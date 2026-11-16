import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

struct PackageWizardRoot: View {
    let importing: Bool
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model: PackageWizardModel?
    var body: some View {
        Group {
            if let model { PackageWizardView(model: model) }
            else { ProgressView().frame(minWidth: 640, minHeight: 440) }
        }
        .task {
            await app.start()
            guard let core = app.core else { return }
            let session = await Task.detached { core.packageWizard(importing: importing) }.value
            let made = PackageWizardModel(session: session)
            made.dismiss = { dismiss() }
            if importing { PackageWizardModel.currentImport = made } else { PackageWizardModel.currentExport = made }
            model = made
        }
        .onDisappear { model?.disappeared() }
    }
}

@Observable
final class PackageWizardModel {
    let session: PackageSession
    var view: PackageView
    var selection = Set<UInt32>()
    var taskText = ""
    var taskPercent: UInt32 = 0
    var overallPercent: UInt32 = 0
    var busy = false
    var cancelling = false
    @ObservationIgnored var dismiss: () -> Void = {}
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored static weak var currentImport: PackageWizardModel?
    @ObservationIgnored static weak var currentExport: PackageWizardModel?
    @ObservationIgnored private var closed = false
    init(session: PackageSession) { self.session = session; view = session.view() }
    func refresh() { view = session.view() }
    func next() {
        if view.stage == 2 { start() }
        else { _ = session.next(); selection = []; refresh() }
    }
    func previous() { session.previous(); selection = []; refresh() }
    func remove() {
        if view.importing { session.setSelected(rows: Array(selection), selected: false) }
        else { session.removeRows(rows: Array(selection)); selection = [] }
        refresh()
    }
    func toggleSelection() {
        let checked = !selection.contains { row in view.rows.indices.contains(Int(row)) && view.rows[Int(row)].selected }
        session.setSelected(rows: Array(selection), selected: checked); refresh()
    }
    func choose(_ kind: String) {
        Task {
            if kind == "output" {
                let panel = NSSavePanel(); panel.title = "Sequence Export Location"
                panel.allowedContentTypes = [UTType(filenameExtension: "vpkg") ?? .zip]
                let current = URL(fileURLWithPath: view.output)
                panel.nameFieldStringValue = current.lastPathComponent
                panel.directoryURL = current.deletingLastPathComponent()
                let response = if let window { await panel.beginSheetModal(for: window) } else { await panel.begin() }
                if response == .OK, let url = panel.url { session.setOutput(path: url.path); refresh() }
                return
            }
            let panel = NSOpenPanel(); panel.canChooseDirectories = false
            panel.allowsMultipleSelection = kind == "sources"
            panel.title = kind == "sources" ? "Select Sequences" : kind == "input" ? "Sequence Package File" : "Profile Map File"
            let extensions = kind == "sources" ? ["tim", "vix"] : kind == "input" ? ["vpkg"] : ["vmap"]
            panel.allowedContentTypes = extensions.compactMap { UTType(filenameExtension: $0) }
            if kind == "sources" { panel.directoryURL = URL(fileURLWithPath: view.sequenceFolder) }
            else {
                let saved = kind == "input" ? view.input : view.map
                if !saved.isEmpty { let folder = URL(fileURLWithPath: saved).deletingLastPathComponent(); if FileManager.default.fileExists(atPath: folder.path) { panel.directoryURL = folder } }
            }
            let response = if let window { await panel.beginSheetModal(for: window) } else { await panel.begin() }
            guard response == .OK else { return }
            busy = true
            defer { busy = false }
            do {
                let paths = panel.urls.map(\.path), session = session
                if kind == "sources" {
                    let foreign = await Task.detached { session.addSequences(paths: paths) }.value
                    refresh()
                    if foreign { await SchedulerAlerts.tell("Warning", "Some selected sequences are not located in the current profile sequence folder. Please ensure all sequences are compatible and play with the current setup before exporting for best results.", style: .warning, window: window) }
                } else if let path = paths.first {
                    if kind == "input" { try await Task.detached { try session.setInput(path: path) }.value }
                    else { session.setMap(path: path) }
                    selection = []; refresh()
                }
            } catch { await tell(error.userMessage) }
        }
    }
    func addAll() {
        busy = true
        Task { let session = session; await Task.detached { session.addAll() }.value; refresh(); busy = false }
    }
    func editMap(_ edit: Bool) {
        busy = true
        Task {
            do {
                let session = session
                let dialog = try await Task.detached { try session.mapper(edit: edit) }.value
                ElementMapperPresenter.show(dialog: dialog, over: window) { [weak self] result, path in
                    guard let self else { return }
                    if result != nil, let path { session.setMap(path: path); refresh() }
                    busy = false
                }
            } catch { busy = false; await tell(error.userMessage) }
        }
    }
    func tell(_ message: String) async {
        await SchedulerAlerts.tell(view.importing ? "Error Importing Package" : "Error Exporting Package", message, style: .error, window: window)
    }
    func start() {
        let forward = PackageForwarder(
            progress: { [weak self] task, percent, overall in self?.taskText = task; self?.taskPercent = percent; self?.overallPercent = overall },
            overwrite: { [weak self] path in
                guard let self else { return false }
                if UITestHarness.isActive, let answer = ProcessInfo.processInfo.environment["VIXEN_TEST_PACKAGE_OVERWRITE"] { return answer == "1" }
                return await SchedulerAlerts.confirm("Export package exists.", "The Export package \(URL(fileURLWithPath: path).lastPathComponent) already exists. Overwrite?", style: .warning, confirm: "Overwrite", cancel: "Cancel", window: window)
            },
            destination: { [weak self] name, path in
                guard let self else { return nil }
                if UITestHarness.isActive, let answer = ProcessInfo.processInfo.environment["VIXEN_TEST_PACKAGE_OVERWRITE"] { return answer == "1" ? path : nil }
                if await SchedulerAlerts.confirm("Sequence exists.", "The sequence \(name) already exists. Overwrite?", style: .warning, confirm: "Overwrite", cancel: "Choose Another Name", window: window) { return path }
                let panel = NSSavePanel(); panel.title = "Timed Sequence"; panel.allowedContentTypes = [UTType(filenameExtension: "tim") ?? .xml]
                panel.directoryURL = URL(fileURLWithPath: view.sequenceFolder); panel.nameFieldStringValue = name + ".tim"
                let response = if let window { await panel.beginSheetModal(for: window) } else { await panel.begin() }
                return response == .OK ? panel.url?.path : nil
            },
            warning: { [weak self] message in
                guard let self else { return }
                if UITestHarness.isActive { UITestHarness.note("package warning \(message)") }
                else { await tell(message) }
            },
            finished: { [weak self] in
                guard let self else { return }
                refresh()
                if cancelling { Task { await close(accept: false) }; return }
                if !UITestHarness.isActive { NSSound.beep() }
            }
        )
        do { try session.start(listener: forward); refresh() }
        catch { Task { await tell(error.userMessage) } }
    }
    func cancel() {
        if view.running { cancelling = true; session.cancel() }
        else { Task { await close(accept: false) } }
    }
    func close(accept: Bool) async {
        do {
            let session = session
            try await Task.detached { try session.close(accept: accept) }.value
            closed = true; dismiss()
        } catch { await tell(error.userMessage) }
    }
    func disappeared() {
        guard !closed else { return }
        session.cancel()
        let session = session
        Task.detached {
            while session.view().running { try? await Task.sleep(for: .milliseconds(100)) }
            try? session.close(accept: false)
        }
    }
}

private struct PackageListRow: Identifiable {
    let id: UInt32
    let row: PackageRow
}
private struct PackageWizardView: View {
    @Bindable var model: PackageWizardModel
    private var rows: [PackageListRow] { model.view.rows.enumerated().map { PackageListRow(id: UInt32($0.offset), row: $0.element) } }
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if model.view.stage == 3 {
                Spacer()
                Text(model.view.finalText).font(.title2)
                Text("\(model.view.completed) sequence(s) completed; \(model.view.skipped) skipped.").foregroundStyle(.secondary)
                if !model.view.errors.isEmpty { ScrollView { Text(model.view.errors.joined(separator: "\n\n")).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading) }.frame(maxHeight: 180) }
                Spacer()
            } else if model.view.stage == 2 {
                Text(model.view.importing ? "Import Sequence Package Summary" : "Export Sequence Package Summary").font(.title2)
                Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 12) {
                    GridRow { Text("Sequences"); Text("\(model.view.rows.filter(\.selected).count)") }
                    GridRow { Text("Destination"); Text(model.view.importing ? model.view.sequenceFolder : model.view.output).textSelection(.enabled) }
                    GridRow { Text("Audio"); Text(model.view.importing ? "Ask on duplicate name." : model.view.includeAudio ? "Included" : "Not included.") }
                }
                Spacer()
                if model.view.running {
                    Text(model.taskText)
                    ProgressView(value: Double(model.taskPercent), total: 100)
                    Text(model.cancelling ? "Canceling…" : "Overall Progress")
                    ProgressView(value: Double(model.overallPercent), total: 100)
                }
            } else if model.view.importing && model.view.stage == 0 {
                Text("Step 1:   Select the Sequence Package to import").font(.headline)
                GroupBox("Package Location") { pathRow("Sequence Package File", model.view.input, kind: "input").padding(8) }
                GroupBox("Profile Map Configuration") {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Select the map file locaton if you have previously mapped this pacakge file. If you have not previously mapped it, use the Create Map button to create one.").foregroundStyle(.secondary)
                        pathRow("Profile Map File", model.view.map, kind: "map")
                        HStack { Button("Create Map") { model.editMap(false) }.disabled(!model.view.canCreateMap); Button("Edit Map") { model.editMap(true) }.disabled(!model.view.canEditMap || !model.view.canCreateMap) }
                    }.padding(8)
                }
                Spacer()
            } else if !model.view.importing && model.view.stage == 1 {
                Text("Step 2:   Configure the Output Folder").font(.headline)
                GroupBox("Export Configuration") {
                    VStack(alignment: .leading, spacing: 14) {
                        pathRow("Sequence Export Location", model.view.output, kind: "output")
                        Toggle("Include Audio", isOn: Binding(get: { model.view.includeAudio }, set: { model.session.setAudio(include: $0); model.refresh() }))
                    }.padding(8)
                }
                Spacer()
            } else {
                Text(model.view.importing ? "Step 2:  Select sequences to import." : "Step 2:  Select sequences to export.").font(.headline)
                HStack(spacing: 10) {
                    if !model.view.importing {
                        Button { model.choose("sources") } label: { Image(systemName: "folder") }.help("Explore Sequence folder")
                        Button { model.addAll() } label: { Image(systemName: "plus.rectangle.on.folder") }.help("Add All Sequences from Profile")
                    }
                    Button { model.remove() } label: { Image(systemName: "minus") }.help("Remove Sequence(s) from list").disabled(model.selection.isEmpty)
                }
                Table(rows, selection: $model.selection) {
                    TableColumn("Sequence Names") { item in
                        if model.view.importing {
                            Toggle(item.row.name, isOn: Binding(get: { item.row.selected }, set: { model.session.setSelected(rows: [item.id], selected: $0); model.refresh() }))
                        } else { Text(item.row.path).help(item.row.path) }
                    }
                }
                .onDeleteCommand { model.remove() }
                .onKeyPress(.space) { if model.view.importing { model.toggleSelection(); return .handled }; return .ignored }
            }
            Divider()
            HStack {
                if model.busy { ProgressView().controlSize(.small) }
                Spacer()
                if model.view.stage < 3 {
                    Button("<< Previous") { model.previous() }.disabled(model.view.stage == 0 || model.view.running)
                    Button("Next >>") { model.next() }.disabled(!model.view.canNext)
                    Button("Cancel") { model.cancel() }.keyboardShortcut(.cancelAction).disabled(model.cancelling)
                } else {
                    Button("Finish") { Task { await model.close(accept: true) } }.keyboardShortcut(.defaultAction)
                }
            }
        }
        .padding(20).frame(minWidth: 640, idealWidth: 680, minHeight: 440, idealHeight: 500)
        .disabled(model.busy)
        .background(PackageWindowFinder { model.window = $0 })
    }
    private func pathRow(_ title: String, _ value: String, kind: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
            HStack {
                Text(value.isEmpty ? "None selected" : value).textSelection(.enabled)
                    .lineLimit(2).truncationMode(.middle)
                    .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading).layoutPriority(-1)
                Button { model.choose(kind) } label: { Image(systemName: "folder") }
                    .fixedSize().help("Browse").accessibilityLabel("Browse \(title)")
            }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
}

private nonisolated final class PackageForwarder: PackageListener, @unchecked Sendable {
    let onProgress: @MainActor (String, UInt32, UInt32) -> Void
    let onOverwrite: @MainActor (String) async -> Bool
    let onDestination: @MainActor (String, String) async -> String?
    let onWarning: @MainActor (String) async -> Void
    let onFinished: @MainActor () -> Void
    init(progress: @escaping @MainActor (String, UInt32, UInt32) -> Void, overwrite: @escaping @MainActor (String) async -> Bool, destination: @escaping @MainActor (String, String) async -> String?, warning: @escaping @MainActor (String) async -> Void, finished: @escaping @MainActor () -> Void) {
        onProgress = progress; onOverwrite = overwrite; onDestination = destination; onWarning = warning; onFinished = finished
    }
    func progress(task: String, taskPercent: UInt32, overallPercent: UInt32) { Task { @MainActor in onProgress(task, taskPercent, overallPercent) } }
    func overwritePackage(path: String) -> Bool {
        let wait = DispatchSemaphore(value: 0); nonisolated(unsafe) var answer = false
        Task { @MainActor in answer = await onOverwrite(path); wait.signal() }; wait.wait(); return answer
    }
    func sequenceDestination(name: String, path: String) -> String? {
        let wait = DispatchSemaphore(value: 0); nonisolated(unsafe) var answer: String?
        Task { @MainActor in answer = await onDestination(name, path); wait.signal() }; wait.wait(); return answer
    }
    func warning(message: String) {
        let wait = DispatchSemaphore(value: 0)
        Task { @MainActor in await onWarning(message); wait.signal() }; wait.wait()
    }
    func finished() { Task { @MainActor in onFinished() } }
}
private struct PackageWindowFinder: NSViewRepresentable {
    let found: (NSWindow) -> Void
    func makeNSView(context: Context) -> Finder { Finder(found: found) }
    func updateNSView(_ nsView: Finder, context: Context) {}
    final class Finder: NSView {
        let found: (NSWindow) -> Void
        init(found: @escaping (NSWindow) -> Void) { self.found = found; super.init(frame: .zero) }
        required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
        override func viewDidMoveToWindow() { if let window { found(window) } }
    }
}
