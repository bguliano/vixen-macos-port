import SwiftUI
import VixenBridge
import VixenKit

/// The Profile Archive Wizard: what the Data Profile dialog calls the Zip Wizard.
struct ArchiveWizardSheet: View {
    let closed: () -> Void

    @State private var module = true
    @State private var program = true
    @State private var sequence = true
    @State private var system = true
    @State private var template = true
    @State private var logs = true
    @State private var media = true
    @State private var userSettings = true
    @State private var choices: [ArchiveProfileChoice] = []
    @State private var choice = 0
    @State private var folder = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.path ?? ""
    @State private var fileName = "VixenProfile"
    @State private var status = ""
    @State private var progress = 0.0
    @State private var task: ArchiveTask?
    @State private var alerts = AlertPresenter()
    @State private var window: NSWindow?

    private var running: Bool { task != nil }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Profile Archive Wizard").font(.headline)
            GroupBox("Profile artifacts to include") {
                Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 6) {
                    GridRow {
                        Toggle("Module", isOn: $module)
                        Toggle("Program", isOn: $program)
                        Toggle("Sequence", isOn: $sequence)
                    }
                    GridRow {
                        Toggle("System", isOn: $system)
                        Toggle("Template", isOn: $template)
                        Toggle("Logs", isOn: $logs)
                    }
                    GridRow {
                        Toggle("Media", isOn: $media)
                        Toggle("User Settings", isOn: $userSettings)
                        Color.clear.frame(width: 1, height: 1)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(4)
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Profile to archive")
                Picker("Profile to archive", selection: $choice) {
                    ForEach(Array(choices.enumerated()), id: \.offset) { index, item in
                        Text(item.name).tag(index)
                    }
                }
                .labelsHidden()
                .disabled(running)
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Folder to save archive to")
                HStack {
                    TextField("Folder to save archive to", text: $folder, prompt: Text(""))
                        .labelsHidden()
                    Button(action: chooseSaveFolder) {
                        Image(systemName: "folder")
                    }
                    .accessibilityLabel("Choose folder")
                }
                .disabled(running)
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Archive file name, without extension")
                TextField("Archive file name, without extension", text: $fileName, prompt: Text(""))
                    .labelsHidden()
                    .disabled(running)
            }
            HStack {
                Spacer()
                Button("Close", role: .cancel) { closed() }
                    .keyboardShortcut(.cancelAction)
                    .disabled(running)
                Button(running ? "Stop" : "Start") { startOrStop() }
                    .keyboardShortcut(.defaultAction)
            }
            HStack(spacing: 8) {
                if running {
                    ProgressView(value: progress)
                        .frame(width: 120)
                }
                Text(status)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            .frame(minHeight: 18)
        }
        .padding(16)
        .frame(width: 400)
        .background(WindowReader { window = $0 })
        .alerts(from: alerts)
        .onAppear { choices = archiveProfileChoices() }
    }

    private var request: ArchiveRequestRecord {
        ArchiveRequestRecord(
            profileFolder: choices.indices.contains(choice) ? choices[choice].folder : "",
            saveFolder: folder,
            fileName: fileName,
            module: module,
            program: program,
            sequence: sequence,
            system: system,
            template: template,
            logs: logs,
            media: media,
            userSettings: userSettings
        )
    }

    private func chooseSaveFolder() {
        Task {
            if let chosen = await chooseFolder(startingAt: folder, on: window) {
                folder = chosen
            }
        }
    }

    private func startOrStop() {
        if let task {
            task.stop()
            return
        }
        Task { await start() }
    }

    /// F82. Every step works from the name as it was typed, which Rust trims once: trimming the field and then passing it on would trim
    /// `a.zip.zip` twice.
    private func start() async {
        let request = self.request
        let issue = checkArchiveRequest(request: request)
        if let issue, !issue.isQuestion {
            await alerts.tell(issue.title, issue.message, critical: issue.isError)
            return
        }
        // A name ending in .zip loses it, in the field too, before the folder and replace questions.
        fileName = trimmedArchiveFileName(name: request.fileName)
        if let issue {
            let yes = issue.replacesFile ? "Replace" : "Create Folder"
            let answer = await alerts.ask(issue.title, issue.message, [.init("Cancel", role: .cancel), .init(yes)])
            guard answer == yes else { return }
            do {
                try resolveArchivePrompt(request: request)
            } catch {
                await alerts.tell("Error Zipping Files", error.userMessage, critical: true)
                return
            }
        }
        progress = 0
        status = ""
        let listener = ArchiveForwarder(
            status: { text in status = text },
            value: { percent in progress = Double(percent) / 100 },
            finished: { message in
                task = nil
                if let message {
                    Task { await alerts.tell("Error Zipping Files", message, critical: true) }
                }
            }
        )
        task = ArchiveTask.start(request: request, listener: listener)
    }
}

/// Hops a run's reports to the main actor.
private nonisolated final class ArchiveForwarder: ArchiveListener, @unchecked Sendable {
    private let statusChanged: @MainActor (String) -> Void
    private let valueChanged: @MainActor (Int32) -> Void
    private let didFinish: @MainActor (String?) -> Void

    init(
        status: @escaping @MainActor (String) -> Void,
        value: @escaping @MainActor (Int32) -> Void,
        finished: @escaping @MainActor (String?) -> Void
    ) {
        statusChanged = status
        valueChanged = value
        didFinish = finished
    }

    func status(text: String) {
        let call = statusChanged
        Task { @MainActor in call(text) }
    }

    func value(percent: Int32) {
        let call = valueChanged
        Task { @MainActor in call(percent) }
    }

    func finished(errorMessage: String?) {
        let call = didFinish
        Task { @MainActor in call(errorMessage) }
    }
}
