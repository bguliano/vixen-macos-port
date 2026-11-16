import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// `ShowEditorForm` on a copy of a show: F42 to F48b. The rules are Rust's `ShowEditor`.
@Observable
final class ShowEditorModel: Identifiable {
    let session: ShowEditorSession
    private(set) var view: ShowEditorView
    private let finished: () -> Void
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored private var tester: ProgramOutputWindow?
    @ObservationIgnored static weak var current: ShowEditorModel?

    init(session: ShowEditorSession, finished: @escaping () -> Void) {
        self.session = session
        view = session.view()
        self.finished = finished
        ShowEditorModel.current = self
    }

    func refresh() {
        let next = session.view()
        if next != view { view = next }
    }

    func change(_ work: (ShowEditorSession) -> Void) {
        work(session)
        refresh()
    }

    /// F46: `destination` as `onMove` gives it, the place before the rows are removed.
    func move(from source: IndexSet, to destination: Int) {
        guard let from = source.first else { return }
        let to = destination > from ? destination - 1 : destination
        change { $0.moveItem(from: UInt32(from), to: UInt32(to)) }
    }

    func browseSequence() async {
        let panel = NSOpenPanel()
        panel.directoryURL = URL(fileURLWithPath: view.sequenceDir, isDirectory: true)
        panel.allowedContentTypes = [UTType(filenameExtension: "tim") ?? .data]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        guard let window, await panel.beginSheetModal(for: window) == .OK, let url = panel.url else { return }
        setSequence(url.path)
    }

    func setSequence(_ path: String) {
        let refusal = session.setSequence(path: path)
        refresh()
        if let refusal {
            let window = window
            Task { await SchedulerAlerts.tell("Sequence", refusal, style: .error, window: window) }
        }
    }

    func browseProgram() async {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.treatsFilePackagesAsDirectories = false
        panel.allowedContentTypes = [.application, .unixExecutable, .shellScript, .script, .item]
        guard let window, await panel.beginSheetModal(for: window) == .OK, let url = panel.url else { return }
        change { $0.setLaunchProgram(text: url.path) }
    }

    /// F48a.
    func testLaunch() {
        tester?.close()
        tester = ProgramOutputWindow.run(session: session)
    }

    /// F48b.
    func ok() async {
        let outcome = session.ok()
        if let alert = outcome.alert {
            await SchedulerAlerts.tell(alert.title, alert.message, style: .error, window: window)
            return
        }
        tester?.close()
        finished()
        if let failure = outcome.saveError {
            await SchedulerAlerts.tell("Shows", "The show could not be saved.\n\n\(failure)", style: .error, window: nil)
        }
    }

    func cancel() async {
        let alert = session.cancelAlert()
        guard await SchedulerAlerts.confirm(alert.title, alert.message, style: .warning, confirm: "Discard Changes", cancel: "Keep Editing", window: window) else { return }
        tester?.close()
        finished()
    }
}

struct ShowEditorSheet: View {
    let model: ShowEditorModel

    private var view: ShowEditorView { model.view }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Setup a Show").font(.headline)
            HStack {
                Text("Show Name:")
                TextField("", text: Binding(get: { view.name }, set: { text in model.change { $0.setName(name: text) } }))
                    .accessibilityLabel("Show Name")
            }
            HStack(alignment: .top, spacing: 16) {
                items.frame(width: 330)
                VStack(alignment: .leading, spacing: 12) {
                    GroupBox("Action") {
                        HStack {
                            Text("Action:").foregroundStyle(view.selected == nil ? .secondary : .primary)
                            Picker("", selection: Binding(get: { view.action.map(Int.init) ?? -1 }, set: { index in model.change { $0.setAction(action: UInt32(index)) } })) {
                                ForEach(Array(view.actionNames.enumerated()), id: \.offset) { index, name in
                                    Text(name).tag(index)
                                }
                            }
                            .labelsHidden()
                            .accessibilityLabel("Action")
                            .frame(width: 160)
                            Spacer()
                        }
                        .padding(4)
                    }
                    .disabled(view.selected == nil)
                    GroupBox("Item Information") {
                        information
                            .padding(4)
                            .frame(maxWidth: .infinity, minHeight: 150, alignment: .topLeading)
                    }
                    Spacer(minLength: 0)
                }
                .frame(width: 400)
            }
            HStack {
                Spacer()
                Button("OK") { Task { await model.ok() } }
                    .keyboardShortcut(.defaultAction)
                Button("Cancel") { Task { await model.cancel() } }
            }
        }
        .padding(16)
        .frame(width: 780)
        .background(WindowReader { model.window = $0 })
        .interactiveDismissDisabled()
    }

    private var items: some View {
        VStack(alignment: .leading, spacing: 8) {
            Picker("", selection: Binding(get: { Int(view.tab) }, set: { tab in model.change { $0.setTab(tab: UInt32(tab)) } })) {
                Text("Startup").tag(0)
                Text("Background").tag(1)
                Text("Sequential").tag(2)
                Text("Shutdown").tag(3)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            List(selection: Binding(get: { view.selected.map(Int.init) }, set: { row in model.change { $0.select(row: row.map(UInt32.init)) } })) {
                ForEach(Array(view.rows.enumerated()), id: \.offset) { row, name in
                    Text(name).tag(row)
                }
                .onMove { model.move(from: $0, to: $1) }
            }
            .listStyle(.bordered)
            .alternatingRowBackgrounds(.disabled)
            .frame(height: 220)
            .help("Left click Sequence and drag to re-arrange show order")
            Text(view.help)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 6) {
                Button {
                    model.change { $0.deleteItem() }
                } label: {
                    Image(systemName: "minus").frame(width: 16, height: 16)
                }
                .help("Delete the Selected Item")
                .accessibilityLabel("Delete the Selected Item")
                .disabled(view.selected == nil)
                Button {
                    model.change { $0.addItem() }
                } label: {
                    Image(systemName: "plus").frame(width: 16, height: 16)
                }
                .help("Add an Item")
                .accessibilityLabel("Add an Item")
            }
            Button("Help") { SchedulerLinks.open(SchedulerHub.current?.core.showsHelpUrl() ?? "") }
        }
    }

    @ViewBuilder private var information: some View {
        if view.selected == nil {
            EmptyView()
        } else if view.action == 0 {
            launch
        } else if view.action == 1 {
            sequence
        } else if view.action == 2 {
            pause
        }
    }

    private var sequence: some View {
        Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 8) {
            GridRow {
                Text("Name:")
                Text(view.sequenceName).lineLimit(1).truncationMode(.middle)
                Button {
                    Task { await model.browseSequence() }
                } label: {
                    Image(systemName: "folder").frame(width: 16, height: 16)
                }
                .help("Select a Sequence")
                .accessibilityLabel("Select a Sequence")
            }
            GridRow {
                Text("Location:")
                Text(view.sequenceLocation)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .gridCellColumns(2)
            }
        }
    }

    private var launch: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Program to Run:")
            HStack {
                TextField("", text: Binding(get: { view.launchProgram }, set: { text in model.change { $0.setLaunchProgram(text: text) } }))
                    .accessibilityLabel("Program to Run")
                Button {
                    Task { await model.browseProgram() }
                } label: {
                    Image(systemName: "folder").frame(width: 16, height: 16)
                }
                .help("Select a File")
                .accessibilityLabel("Select a File")
            }
            Text("Command Line:")
            HStack {
                TextField("", text: Binding(get: { view.launchCommandLine }, set: { text in model.change { $0.setLaunchCommandLine(text: text) } }))
                    .accessibilityLabel("Command Line")
                Button {
                    model.testLaunch()
                } label: {
                    Image(systemName: "gearshape.arrow.triangle.2.circlepath").frame(width: 16, height: 16)
                }
                .help("Test")
                .accessibilityLabel("Test")
            }
            Toggle("Show Command Window", isOn: Binding(get: { view.launchShowCommandWindow }, set: { on in model.change { $0.setLaunchShowCommandWindow(on: on) } }))
            Toggle("Wait for Exit", isOn: Binding(get: { view.launchWaitForExit }, set: { on in model.change { $0.setLaunchWaitForExit(on: on) } }))
        }
    }

    private var pause: some View {
        let seconds = Binding(get: { Int(view.pauseSeconds) }, set: { value in
            let clamped = min(max(value, Int(view.pauseMin)), Int(view.pauseMax))
            model.change { $0.setPause(seconds: Int32(clamped)) }
        })
        return HStack {
            Text("Pause (seconds):")
            TextField("", value: seconds, format: .number.grouping(.never))
                .frame(width: 60)
                .multilineTextAlignment(.trailing)
                .accessibilityLabel("Pause (seconds)")
            Stepper("", value: seconds, in: Int(view.pauseMin)...Int(view.pauseMax))
                .labelsHidden()
        }
    }
}

/// `LaunchTypeTester`: F48a. Runs the program as it opens, and stops it however it closes.
final class ProgramOutputWindow: NSWindow, NSWindowDelegate {
    private let state = ProgramOutputState()
    private var test: LaunchTest?

    static func run(session: ShowEditorSession) -> ProgramOutputWindow? {
        let window = ProgramOutputWindow(contentRect: NSRect(x: 0, y: 0, width: 520, height: 340), styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.title = "Program Output Window"
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = NSHostingView(rootView: ProgramOutputContent(state: window.state, close: { [weak window] in window?.close() }))
        window.delegate = window
        guard let test = session.testLaunch(listener: ProgramOutputForwarder(state: window.state)) else { return nil }
        window.test = test
        window.center()
        window.makeKeyAndOrderFront(nil)
        return window
    }

    func windowWillClose(_ notification: Notification) {
        test?.stop()
        test = nil
    }
}

@Observable
final class ProgramOutputState {
    var text = ""
    var complete = false
}

private nonisolated final class ProgramOutputForwarder: LaunchTestListener, @unchecked Sendable {
    private let state: ProgramOutputState

    init(state: ProgramOutputState) {
        self.state = state
    }

    func complete(text: String) {
        let state = state
        Task { @MainActor in
            state.text = text
            state.complete = true
        }
    }
}

private struct ProgramOutputContent: View {
    let state: ProgramOutputState
    let close: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ScrollView {
                Text(state.text)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .topLeading)
                    .padding(6)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .textBackgroundColor))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color(nsColor: .separatorColor)))
            HStack {
                Text(state.complete ? "Status: Run Complete" : "Status: Running")
                    .accessibilityIdentifier("programOutputStatus")
                Spacer()
                Button("Close", action: close)
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(14)
        .frame(minWidth: 400, minHeight: 240)
    }
}
