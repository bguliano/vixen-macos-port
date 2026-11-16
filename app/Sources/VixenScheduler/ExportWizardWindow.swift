import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// The Export Wizard, `BulkExportWizard`: F14 to F36. One stage at a time with `<< Previous`, `Next >>` (`Finish` at the end) and `Cancel` at
/// the bottom right, as `WizardForm`. All of the rules are Rust's `ExportWizardSession`.
struct ExportWizardRoot: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model: ExportWizardModel?

    var body: some View {
        Group {
            if let model {
                ExportWizardView(model: model, close: { dismiss() })
            } else {
                ProgressView().frame(minWidth: 627, minHeight: 575)
            }
        }
        .task {
            await app.start()
            for _ in 0..<400 where app.core == nil {
                try? await Task.sleep(for: .milliseconds(50))
            }
            guard let core = app.core else { return }
            let session = await Task.detached { core.exportWizard() }.value
            let made = ExportWizardModel(session: session)
            ExportWizardModel.current = made
            model = made
        }
    }
}

/// What the window shows, refreshed from Rust after each change.
@Observable
final class ExportWizardModel {
    let session: ExportWizardSession
    private(set) var view: WizardView
    var intervalText: String
    var falconFolderText: String
    var hostText: String
    var saveName: String
    private(set) var taskText = "Ready"
    private(set) var taskPercent: Int32 = 0
    private(set) var overallText = "Ready"
    private(set) var overallPercent: Int32 = 0
    private(set) var cancelling = false
    var selectedSequences = Set<String>()
    var selectedControllers = Set<String>()
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored static weak var current: ExportWizardModel?
    @ObservationIgnored private var closed = false

    init(session: ExportWizardSession) {
        self.session = session
        let view = session.view()
        self.view = view
        intervalText = view.intervalText
        falconFolderText = view.falconFolder
        hostText = view.hostText
        saveName = view.saveName
    }

    func refresh() {
        view = session.view()
        if view.falconFolder != falconFolderText { falconFolderText = view.falconFolder }
    }

    /// The device questions wait on the network, so they're asked off the main thread.
    private func afterStageChange() async {
        if view.stage == 3, view.directUpload {
            await checkHost(alert: false)
        } else if view.stage == 4 {
            let session = session
            if view.preparingNetwork {
                let completed = await Task.detached(operation: { session.prepareNetwork() }).value
                refresh()
                guard completed else { return }
            }
            guard !closed, view.stage == 4, view.networkError == nil else { return }
            if view.directUpload, await Task.detached(operation: { session.askDevice() }).value { refresh() }
        }
    }

    /// After a stage change, the boxes that edit text start from the profile.
    private func stageChanged() {
        refresh()
        intervalText = view.intervalText
        falconFolderText = view.falconFolder
        hostText = view.hostText
        saveName = view.saveName
        selectedSequences = []
        selectedControllers = []
    }

    func next() async {
        if view.stage == 4 {
            await run()
        } else if session.next() {
            stageChanged()
            await afterStageChange()
        }
    }

    func previous() async {
        if session.previous() {
            stageChanged()
            await afterStageChange()
        }
    }

    // Stage 1

    func selectProfile(_ index: Int) {
        session.selectProfile(index: UInt32(index))
        refresh()
    }

    // Stage 4

    func setInterval(_ text: String) {
        intervalText = text
        session.setIntervalText(text: text)
        refresh()
    }

    func chooseTiming(_ index: Int) {
        setInterval(view.timingChoices[index])
    }

    func commitFalconFolder() async {
        session.setFalconFolder(path: falconFolderText)
        refresh()
        await offerToCreate(falconFolderText)
    }

    /// F26: File Path or Direct Upload.
    func setDirectUpload(_ on: Bool) async {
        session.setDirectUpload(on: on)
        refresh()
        hostText = view.hostText
        if on { await checkHost(alert: false) }
    }

    func setHost(_ text: String) {
        hostText = text
        session.setHost(text: text)
        refresh()
    }

    /// F26: Next stays off until the host has answered. The alert is for a host the user has just finished typing.
    func checkHost(alert: Bool) async {
        let session = session
        let failure = await Task.detached(operation: { session.checkHost(alert: alert) }).value
        refresh()
        if let failure {
            await SchedulerAlerts.tell(failure.title, failure.message, style: .warning, window: window)
        }
    }

    /// `ValidatePath`: a missing folder can be made.
    func offerToCreate(_ path: String) async {
        guard session.shouldOfferToCreate(path: path) else { return }
        let create = await SchedulerAlerts.confirm(
            "Invalid output path", "The selected output path does not exist. Do you want to create it?", style: .error, confirm: "Create", cancel: "Cancel", window: window
        )
        guard create else { return }
        do {
            try session.createFolder(path: path)
        } catch {
            await SchedulerAlerts.tell("Error creating directory.", "Unable to create the target directory.\n\(error.userMessage)", style: .error, window: window)
        }
        refresh()
    }

    /// The folder buttons: `FolderBrowserDialog` with upstream's description as the panel's message.
    func chooseFolder(message: String, current: String) async -> String? {
        if UITestHarness.isActive {
            return ProcessInfo.processInfo.environment["VIXEN_TEST_FOLDER"]
        }
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.message = message
        if FileManager.default.fileExists(atPath: current) {
            panel.directoryURL = URL(fileURLWithPath: current, isDirectory: true)
        }
        if let window {
            return await panel.beginSheetModal(for: window) == .OK ? panel.url?.path : nil
        }
        return panel.runModal() == .OK ? panel.url?.path : nil
    }

    // Stage 2

    func addSequences() async {
        let folder = session.sequenceFolderPath()
        let files: [String]
        if UITestHarness.isActive, let supplied = ProcessInfo.processInfo.environment["VIXEN_TEST_FILES"] {
            files = supplied.split(separator: "\n").map(String.init)
        } else {
            let panel = NSOpenPanel()
            panel.title = "Select Sequences"
            panel.allowsMultipleSelection = true
            panel.canChooseDirectories = false
            panel.directoryURL = URL(fileURLWithPath: folder, isDirectory: true)
            if let tim = UTType(filenameExtension: "tim") { panel.allowedContentTypes = [tim] }
            // Upstream refuses OK for a file outside the Sequence folder; here those files can't be chosen at all.
            let delegate = InsideFolder(folder: folder)
            panel.delegate = delegate
            let ok: Bool
            if let window {
                ok = await panel.beginSheetModal(for: window) == .OK
            } else {
                ok = panel.runModal() == .OK
            }
            files = ok ? panel.urls.map(\.path) : []
            withExtendedLifetime(delegate) {}
        }
        session.addSequences(paths: files)
        refresh()
    }

    // Stage 5 and the run

    private func run() async {
        session.setSaveName(name: saveName)
        let forward = WizardForwarder(
            task: { [weak self] text, percent in
                self?.taskText = text
                self?.taskPercent = percent
            },
            overall: { [weak self] text, percent in
                self?.overallText = text
                self?.overallPercent = percent
            },
            loadFailed: { [weak self] path in
                await SchedulerAlerts.confirm(
                    "Error Loading Sequence",
                    "An error occurred opening the sequence \(path). \nDo you wish to continue with the export of the remaining ones?",
                    style: .error, confirm: "Continue", cancel: "Stop", window: self?.window
                )
            },
            summaryChanged: { [weak self] in self?.refresh() },
            finished: { [weak self] _, error, uploadAlert in
                guard let self else { return }
                cancelling = false
                stageChanged()
                if !UITestHarness.isActive {
                    NSSound.beep()
                }
                let window = window
                Task {
                    if let uploadAlert {
                        await SchedulerAlerts.tell(uploadAlert.title, uploadAlert.message, style: .error, window: window)
                    }
                    if let error {
                        await SchedulerAlerts.tell("Error", error, style: .error, window: window)
                    }
                }
            }
        )
        do {
            try session.start(listener: forward)
            overallText = "Overall Progress"
            refresh()
        } catch {
            await SchedulerAlerts.tell("Error", error.userMessage, style: .error, window: window)
        }
    }

    /// Cancel. GUARD: during a run the window stays until the run has stopped, saying so.
    func cancel(close: () -> Void) {
        if view.running {
            cancelling = true
            taskText = "Cancelling…"
            session.cancel()
        } else {
            closed = true
            session.cancel()
            close()
        }
    }
}

/// Greys out files outside the profile's Sequence folder.
private final class InsideFolder: NSObject, NSOpenSavePanelDelegate {
    let folder: String

    init(folder: String) {
        self.folder = folder
    }

    func panel(_ sender: Any, shouldEnable url: URL) -> Bool {
        var isDirectory: ObjCBool = false
        FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory)
        return isDirectory.boolValue || url.path.hasPrefix(folder + "/")
    }
}

struct ExportWizardView: View {
    @Bindable var model: ExportWizardModel
    let close: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            Group {
                switch model.view.stage {
                case 0: ConfigStage(model: model)
                case 1: SourcesStage(model: model)
                case 2: ControllersStage(model: model)
                case 3: OutputFormatStage(model: model)
                case 4: SummaryStage(model: model)
                default: FinishedStage(model: model)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(20)
            Divider()
            HStack {
                Spacer()
                if model.view.stage > 0 && model.view.stage < 5 {
                    Button("<< Previous") { Task { await model.previous() } }
                        .disabled(!model.view.canPrevious)
                }
                if model.view.stage == 5 {
                    Button("Finish") { close() }
                        .keyboardShortcut(.defaultAction)
                } else {
                    Button("Next >>") { Task { await model.next() } }
                        .keyboardShortcut(.defaultAction)
                        .disabled(!model.view.canNext)
                    Button("Cancel") { model.cancel(close: close) }
                        .keyboardShortcut(.cancelAction)
                        .disabled(model.cancelling)
                }
            }
            .padding(12)
        }
        .frame(minWidth: 627, minHeight: 575)
        .background(WizardWindowFinder { model.window = $0 })
    }
}

private struct StageLabel: View {
    let text: String

    var body: some View {
        Text(text).font(.title3.weight(.semibold)).padding(.bottom, 8)
    }
}

// MARK: Stage 1, F16 to F20

private struct ConfigStage: View {
    @Bindable var model: ExportWizardModel
    @State private var prompt: NamePrompt?
    @State private var promptText = ""
    @State private var refusal: WizardAlert?
    @State private var confirmingDelete = false

    struct NamePrompt: Identifiable {
        let id = UUID()
        let title: String
        let initial: String
        let rename: Bool
    }

    private var selectedName: String {
        let names = model.view.profileNames
        let index = Int(model.view.selectedProfile)
        return index < names.count ? names[index] : ""
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            StageLabel(text: "Step 1:  Select or start a new export configuration")
            Text("Configurations")
            HStack {
                Picker("Configurations", selection: Binding(get: { Int(model.view.selectedProfile) }, set: { model.selectProfile($0) })) {
                    ForEach(Array(model.view.profileNames.enumerated()), id: \.offset) { index, name in
                        Text(name).tag(index)
                    }
                }
                .labelsHidden()
                .frame(maxWidth: 320)
                Button { ask(NamePrompt(title: "New Profile", initial: "", rename: false)) } label: { Image(systemName: "plus") }
                    .help("Add a profile")
                    .accessibilityLabel("Add Profile")
                Button { confirmingDelete = true } label: { Image(systemName: "minus") }
                    .help("Delete the profile")
                    .accessibilityLabel("Delete Profile")
                    .disabled(model.view.profileNames.isEmpty)
                Button { ask(NamePrompt(title: "Rename Profile", initial: selectedName, rename: true)) } label: { Image(systemName: "pencil.and.list.clipboard") }
                    .help("Rename the profile")
                    .accessibilityLabel("Rename Profile")
                    .disabled(model.view.profileNames.isEmpty)
            }
        }
        .alert(prompt?.title ?? "", isPresented: Binding(get: { prompt != nil }, set: { if !$0 { prompt = nil } }), presenting: prompt) { request in
            TextField("Name", text: $promptText)
            Button("OK") { submit(request) }
            Button("Cancel", role: .cancel) {}
        } message: { _ in
            Text("Enter a name for the profile")
        }
        .alert(refusal?.title ?? "", isPresented: Binding(get: { refusal != nil }, set: { if !$0 { refusal = nil } }), presenting: refusal) { _ in
            Button("OK") {}
        } message: { alert in
            Text(alert.message)
        }
        .confirmationDialog("Confirm Delete", isPresented: $confirmingDelete) {
            Button("Delete", role: .destructive) {
                if let failure = model.session.deleteProfile() { refusal = failure }
                model.refresh()
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Are you sure you want to delete \(selectedName)?")
        }
    }

    private func ask(_ request: NamePrompt) {
        promptText = request.initial
        prompt = request
    }

    /// F19: a refused name shows why, then asks again.
    private func submit(_ request: NamePrompt) {
        let failure = request.rename ? model.session.renameProfile(name: promptText) : model.session.addProfile(name: promptText)
        model.refresh()
        guard let failure else { return }
        refusal = failure
        if failure.title != "Save Error" {
            let retry = NamePrompt(title: request.title, initial: promptText, rename: request.rename)
            Task {
                while refusal != nil { try? await Task.sleep(for: .milliseconds(100)) }
                ask(retry)
            }
        }
    }
}

// MARK: Stage 2, F21 to F24

private struct SourcesStage: View {
    @Bindable var model: ExportWizardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            StageLabel(text: "Step 2:  Select sequences to export.")
            HStack(spacing: 6) {
                Button { Task { await model.addSequences() } } label: { Image(systemName: "folder") }
                    .help("Explore Sequence folder")
                    .accessibilityLabel("Explore Sequence folder")
                Button {
                    model.session.addAllSequences()
                    model.refresh()
                } label: { Image(systemName: "doc.on.doc") }
                    .help("Add All Sequences from Profile")
                    .accessibilityLabel("Add All Sequences from Profile")
                Button(action: removeSelected) { Image(systemName: "trash") }
                    .help("Remove Sequence(s) from list")
                    .accessibilityLabel("Remove Sequence(s) from list")
                    .disabled(model.selectedSequences.isEmpty)
            }
            List(selection: $model.selectedSequences) {
                ForEach(model.view.sequenceFiles, id: \.self) { file in
                    Text(URL(fileURLWithPath: file).lastPathComponent).tag(file)
                }
            }
            .onDeleteCommand(perform: removeSelected)
        }
    }

    private func removeSelected() {
        let rows = model.view.sequenceFiles.enumerated().filter { model.selectedSequences.contains($0.element) }.map { UInt32($0.offset) }
        model.session.removeSequences(rows: rows)
        model.selectedSequences = []
        model.refresh()
    }
}

// MARK: Stage 3, F25

private struct ControllersStage: View {
    @Bindable var model: ExportWizardModel

    var body: some View {
        let rows = model.view.controllers
        VStack(alignment: .leading, spacing: 10) {
            StageLabel(text: "Step 3:  Configure the required outputs and order.")
            HStack {
                Button("Enable All") {
                    model.session.setAllControllers(on: true)
                    model.refresh()
                }
                .disabled(!model.view.enableAllEnabled)
                Button("Disable All") {
                    model.session.setAllControllers(on: false)
                    model.refresh()
                }
                .disabled(!model.view.disableAllEnabled)
            }
            Table(of: ExportControllerRow.self, selection: $model.selectedControllers) {
                TableColumn("Controller") { row in
                    Toggle(row.name, isOn: Binding(get: { row.active }, set: { on in toggle(row: row, on: on, rows: rows) }))
                }
                TableColumn("Channels") { row in Text(String(row.channels)).monospacedDigit() }.width(70)
                TableColumn("Start") { row in Text(row.start.map(String.init) ?? "").monospacedDigit() }.width(60)
                TableColumn("End") { row in Text(row.end.map(String.init) ?? "").monospacedDigit() }.width(60)
            } rows: {
                ForEach(rows) { row in
                    TableRow(row).draggable(row.id)
                }
                .dropDestination(for: String.self) { (index: Int, ids: [String]) in
                    let from = ids.compactMap { id in rows.firstIndex { $0.id == id } }.map { UInt32($0) }
                    model.session.moveControllers(from: from, to: UInt32(index))
                    model.refresh()
                }
            }
            // A focused table owns Space and a selected-row Escape. With no
            // selection, Escape falls through to the wizard's Cancel button.
            .onKeyPress(.space) {
                guard let row = rows.first(where: { model.selectedControllers.contains($0.id) }) else { return .ignored }
                toggle(row: row, on: !row.active, rows: rows)
                return .handled
            }
            .onKeyPress(.escape) {
                guard !model.selectedControllers.isEmpty else { return .ignored }
                model.selectedControllers = []
                return .handled
            }
        }
    }

    /// A tick on a highlighted row ticks every highlighted row, as Space does in the ListView.
    private func toggle(row: ExportControllerRow, on: Bool, rows: [ExportControllerRow]) {
        let targets = model.selectedControllers.contains(row.id) ? model.selectedControllers : [row.id]
        let indexes = rows.enumerated().filter { targets.contains($0.element.id) }.map { UInt32($0.offset) }
        model.session.setControllersActive(rows: indexes, on: on)
        model.refresh()
    }
}

// MARK: Stage 4, F26

private struct OutputFormatStage: View {
    @Bindable var model: ExportWizardModel
    @FocusState private var hostFocused: Bool

    var body: some View {
        let view = model.view
        VStack(alignment: .leading, spacing: 12) {
            StageLabel(text: "Step 4:   Choose the Output Format and Destination")
            GroupBox("Export Format") {
                HStack {
                    Picker("Format:", selection: Binding(get: { Int(view.formatIndex) }, set: { index in
                        model.session.setFormat(index: UInt32(index))
                        model.refresh()
                    })) {
                        ForEach(Array(view.formatNames.enumerated()), id: \.offset) { index, name in Text(name).tag(index) }
                    }
                    .fixedSize()
                    Spacer()
                    Text("Timing (ms):")
                    TextField("Timing (ms)", text: Binding(get: { model.intervalText }, set: { model.setInterval($0) }))
                        .labelsHidden()
                        .monospacedDigit()
                        .multilineTextAlignment(.trailing)
                        .frame(width: 52)
                    Menu {
                        ForEach(Array(view.timingChoices.enumerated()), id: \.offset) { index, choice in
                            Button(choice) { model.chooseTiming(index) }
                        }
                    } label: { EmptyView() }
                        .menuStyle(.borderlessButton)
                        .fixedSize()
                        .accessibilityLabel("Timing choices")
                }
                .padding(6)
            }
            if view.showsFalconGroup {
                GroupBox("Falcon Pi Player 2.x") {
                    VStack(alignment: .leading, spacing: 8) {
                        if view.showsUniverseOptions {
                            HStack(spacing: 20) {
                                toggle("Create Universe File", view.createUniverseFile) { model.session.setCreateUniverseFile(on: $0) }
                                toggle("Backup Universe File", view.backupUniverseFile) { model.session.setBackupUniverseFile(on: $0) }
                            }
                        }
                        HStack(spacing: 20) {
                            toggle("Include Audio", view.falconIncludeAudio) { model.session.setIncludeAudio(falconBox: true, on: $0) }
                            toggle("Enable Compression", view.compress) { model.session.setCompression(on: $0) }
                                .disabled(!view.compressEnabled)
                        }
                        if view.showsDirectUpload {
                            Picker("Send to", selection: Binding(get: { view.directUpload }, set: { on in Task { await model.setDirectUpload(on) } })) {
                                Text("File Path").tag(false)
                                Text("Direct Upload").tag(true)
                            }
                            .pickerStyle(.radioGroup)
                            .horizontalRadioGroupLayout()
                            .labelsHidden()
                        }
                        if view.directUpload {
                            HStack {
                                Text("FPP Host / IP:")
                                TextField("FPP Host / IP", text: Binding(get: { model.hostText }, set: { model.setHost($0) }))
                                    .labelsHidden()
                                    .focused($hostFocused)
                                    .onSubmit { Task { await model.checkHost(alert: true) } }
                            }
                            .onChange(of: hostFocused) { _, focused in
                                if !focused { Task { await model.checkHost(alert: true) } }
                            }
                        } else {
                            Text(view.falconNote).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                            HStack {
                                TextField("Output Folder", text: $model.falconFolderText)
                                    .onSubmit { Task { await model.commitFalconFolder() } }
                                Button("Output Folder") {
                                    Task {
                                        if let folder = await model.chooseFolder(message: "Select the universe file export location", current: view.falconFolder) {
                                            model.falconFolderText = folder
                                            await model.commitFalconFolder()
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .padding(6)
                }
            } else {
                GroupBox("Sequence") {
                    folderRow("Sequence Folder", view.sequenceFolder, button: "Output Folder", message: "Select the sequence export location") {
                        model.session.setSequenceFolder(path: $0)
                    }
                    .padding(6)
                }
                GroupBox("Audio") {
                    VStack(alignment: .leading, spacing: 8) {
                        toggle("Include Audio", view.includeAudio) { model.session.setIncludeAudio(falconBox: false, on: $0) }
                        Group {
                            toggle("Rename file to match sequence", view.renameAudio) { model.session.setRenameAudio(on: $0) }
                            folderRow("Audio Folder", view.audioFolder, button: "Audio Output Folder", message: "Select the audio export location") {
                                model.session.setAudioFolder(path: $0)
                            }
                        }
                        .disabled(!view.includeAudio)
                    }
                    .padding(6)
                }
            }
        }
    }

    private func toggle(_ title: String, _ value: Bool, _ set: @escaping (Bool) -> Void) -> some View {
        Toggle(title, isOn: Binding(get: { value }, set: { on in
            set(on)
            model.refresh()
        }))
    }

    private func folderRow(_ label: String, _ path: String, button: String, message: String, set: @escaping (String) -> Void) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                Spacer()
                Button(button) {
                    Task {
                        if let folder = await model.chooseFolder(message: message, current: path) {
                            set(folder)
                            model.refresh()
                        }
                    }
                }
            }
            Text(path.isEmpty ? " " : path)
                .textSelection(.enabled)
                .lineLimit(1)
                .truncationMode(.middle)
                .foregroundStyle(.secondary)
        }
    }
}

// MARK: Stage 5, F27 and F29

private struct SummaryStage: View {
    @Bindable var model: ExportWizardModel

    var body: some View {
        let view = model.view
        VStack(alignment: .leading, spacing: 10) {
            StageLabel(text: "Action Summary")
            if let summary = view.summary {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 6) {
                    row("Sequences", String(summary.sequences))
                    row("Format", summary.format)
                    row("Timing", summary.timing)
                    row("Destination", summary.destination)
                    row("Audio", summary.audio)
                    if let audio = summary.audioDestination { row("Audio Destination", audio) }
                    if let universe = summary.universeDestination { row("Universe File Destination", universe) }
                }
                if let warning = summary.warning {
                    Text(warning).foregroundStyle(.orange).fixedSize(horizontal: false, vertical: true)
                }
                if let heading = summary.deviceHeading {
                    Text(heading).font(.headline).padding(.top, 4)
                    Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 6) {
                        ForEach(summary.deviceRows, id: \.label) { item in row(item.label, item.value) }
                    }
                }
            }
            HStack {
                Toggle("Save export settings", isOn: Binding(get: { view.saveSettings }, set: { on in
                    model.session.setSaveSettings(on: on)
                    model.refresh()
                }))
                if view.saveSettings {
                    TextField("Profile name", text: Binding(get: { model.saveName }, set: { text in
                        model.saveName = text
                        model.session.setSaveName(name: text)
                        model.refresh()
                    }))
                    .frame(maxWidth: 220)
                    Menu {
                        ForEach(view.profileNames, id: \.self) { name in
                            Button(name) {
                                model.saveName = name
                                model.session.setSaveName(name: name)
                                model.refresh()
                            }
                        }
                    } label: { EmptyView() }
                        .menuStyle(.borderlessButton)
                        .fixedSize()
                        .accessibilityLabel("Profiles")
                }
            }
            Spacer()
            if view.preparingNetwork {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Preparing controller network configuration…")
                    ProgressView().progressViewStyle(.linear)
                }
            } else if let error = view.networkError {
                Text(error).foregroundStyle(.red).fixedSize(horizontal: false, vertical: true)
            } else {
                ProgressBlock(text: model.taskText, percent: model.taskPercent)
            }
            ProgressBlock(text: model.overallText, percent: model.overallPercent)
        }
    }

    private func row(_ label: String, _ value: String) -> some View {
        GridRow {
            Text(label).foregroundStyle(.secondary)
            Text(value).textSelection(.enabled).lineLimit(2).truncationMode(.middle)
        }
    }
}

private struct ProgressBlock: View {
    let text: String
    let percent: Int32

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text).lineLimit(1)
            ProgressView(value: Double(percent), total: 100)
        }
    }
}

// MARK: Stage 6, F36

private struct FinishedStage: View {
    let model: ExportWizardModel

    var body: some View {
        VStack(alignment: .leading) {
            Text(model.view.finalText.isEmpty ? "Export Complete" : model.view.finalText)
                .font(.title2.weight(.semibold))
        }
    }
}

/// Hops a run's reports to the main actor. `loadFailed` blocks the run's thread until the user answers.
private nonisolated final class WizardForwarder: ExportWizardListener, @unchecked Sendable {
    private let onTask: @MainActor (String, Int32) -> Void
    private let onOverall: @MainActor (String, Int32) -> Void
    private let onLoadFailed: @MainActor (String) async -> Bool
    private let onSummaryChanged: @MainActor () -> Void
    private let onFinished: @MainActor (String, String?, WizardAlert?) -> Void

    init(
        task: @escaping @MainActor (String, Int32) -> Void,
        overall: @escaping @MainActor (String, Int32) -> Void,
        loadFailed: @escaping @MainActor (String) async -> Bool,
        summaryChanged: @escaping @MainActor () -> Void,
        finished: @escaping @MainActor (String, String?, WizardAlert?) -> Void
    ) {
        onTask = task
        onOverall = overall
        onLoadFailed = loadFailed
        onSummaryChanged = summaryChanged
        onFinished = finished
    }

    func task(text: String, percent: Int32) {
        let call = onTask
        Task { @MainActor in call(text, percent) }
    }

    func overall(text: String, percent: Int32) {
        let call = onOverall
        Task { @MainActor in call(text, percent) }
    }

    func loadFailed(path: String, message: String) -> Bool {
        let call = onLoadFailed
        let semaphore = DispatchSemaphore(value: 0)
        nonisolated(unsafe) var answer = false
        Task { @MainActor in
            answer = await call(path)
            semaphore.signal()
        }
        semaphore.wait()
        return answer
    }

    func summaryChanged() {
        let call = onSummaryChanged
        Task { @MainActor in call() }
    }

    func finished(finalText: String, error: String?, uploadAlert: WizardAlert?) {
        let call = onFinished
        Task { @MainActor in call(finalText, error, uploadAlert) }
    }
}

private struct WizardWindowFinder: NSViewRepresentable {
    let found: (NSWindow) -> Void

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async { if let window = view.window { found(window) } }
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        if let window = view.window { found(window) }
    }
}
