import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// Scheduler > Shows, `ShowListForm`: F37 to F41. The rules are Rust's `vixen_session::shows`.
struct ShowsRoot: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model: ShowsModel?

    var body: some View {
        Group {
            if let model {
                ShowsView(model: model, close: { dismiss() })
            } else {
                ProgressView().frame(width: 340, height: 360)
            }
        }
        .task {
            await app.start()
            for _ in 0..<400 where app.core == nil {
                try? await Task.sleep(for: .milliseconds(50))
            }
            guard let core = app.core else { return }
            let made = ShowsModel(core: core)
            ShowsModel.current = made
            model = made
        }
    }
}

@Observable
final class ShowsModel {
    let core: VixenCore
    private(set) var names: [String]
    var selection: Int?
    var renaming: Int?
    var renameText = ""
    var editor: ShowEditorModel?
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored static weak var current: ShowsModel?

    init(core: VixenCore) {
        self.core = core
        names = core.showNames()
    }

    func refresh() {
        names = core.showNames()
        if let selection, selection >= names.count { self.selection = names.isEmpty ? nil : names.count - 1 }
        SchedulerHub.current?.refresh()
    }

    /// F38.
    func add() {
        commitRename()
        let row = Int(core.showAdd())
        refresh()
        selection = row
    }

    /// F39. Nothing without a selection.
    func edit() {
        commitRename()
        guard let selection, let session = core.showEditor(index: UInt32(selection)) else { return }
        editor = ShowEditorModel(session: session) { [weak self] in
            self?.editor = nil
            self?.refresh()
        }
    }

    func beginRename(_ row: Int) {
        guard names.indices.contains(row) else { return }
        selection = row
        renameText = names[row]
        renaming = row
    }

    /// F40: an empty name is ignored.
    func commitRename() {
        guard let row = renaming else { return }
        renaming = nil
        guard names.indices.contains(row), renameText != names[row] else { return }
        let failure = core.showRename(index: UInt32(row), name: renameText)
        refresh()
        report(failure)
    }

    /// F41.
    func delete() async {
        commitRename()
        guard let selection, names.indices.contains(selection) else { return }
        let message = core.showDeleteMessage(index: UInt32(selection))
        guard await SchedulerAlerts.confirm("Delete Show", message, style: .warning, confirm: "Delete", cancel: "Cancel", destructive: true, window: window) else { return }
        let failure = core.showDelete(index: UInt32(selection))
        self.selection = nil
        refresh()
        report(failure)
    }

    private func report(_ failure: String?) {
        guard let failure else { return }
        let window = window
        Task { await SchedulerAlerts.tell("Shows", "The shows could not be saved.\n\n\(failure)", style: .error, window: window) }
    }
}

struct ShowsView: View {
    @Bindable var model: ShowsModel
    let close: () -> Void
    @FocusState private var renameFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            GroupBox("Shows") {
                VStack(spacing: 8) {
                    list
                    HStack(spacing: 6) {
                        Spacer()
                        iconButton("plus", "Add a Show") { model.add() }
                        iconButton("pencil", "Edit the Selected Show") { model.edit() }
                            .disabled(model.selection == nil)
                        iconButton("minus", "Delete the Selected Show") { Task { await model.delete() } }
                            .disabled(model.selection == nil)
                    }
                }
                .padding(4)
            }
            HStack {
                Button("Help") { SchedulerLinks.open(model.core.showsHelpUrl()) }
                Spacer()
                Button("Close") {
                    model.commitRename()
                    close()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 340, height: 360)
        .background(WindowReader { model.window = $0 })
        .sheet(item: $model.editor) { editor in
            ShowEditorSheet(model: editor)
        }
    }

    private var list: some View {
        List(selection: $model.selection) {
            ForEach(Array(model.names.enumerated()), id: \.offset) { row, name in
                Group {
                    if model.renaming == row {
                        TextField("", text: $model.renameText)
                            .textFieldStyle(.plain)
                            .focused($renameFocused)
                            .onSubmit { model.commitRename() }
                            .onExitCommand { model.renaming = nil }
                            .onAppear { renameFocused = true }
                    } else {
                        Text(name)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .contentShape(Rectangle())
                            .onTapGesture(count: 2) {
                                model.selection = row
                                model.edit()
                            }
                            .onTapGesture {
                                // A click on the selected row's name edits it, as a list with editable labels does.
                                if model.selection == row {
                                    model.beginRename(row)
                                } else {
                                    model.commitRename()
                                    model.selection = row
                                }
                            }
                    }
                }
                .tag(row)
            }
        }
        .listStyle(.bordered)
        .alternatingRowBackgrounds(.disabled)
        .onChange(of: renameFocused) { _, focused in
            if !focused { model.commitRename() }
        }
    }

    private func iconButton(_ symbol: String, _ help: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol).frame(width: 16, height: 16)
        }
        .help(help)
        .accessibilityLabel(help)
    }
}

enum SchedulerLinks {
    /// Help pages open in the browser. Under test nothing leaves the app.
    static func open(_ text: String) {
        guard !UITestHarness.isActive, let url = URL(string: text) else { return }
        NSWorkspace.shared.open(url)
    }
}

/// Hands the hosting window to a model, for alerts and panels.
struct WindowReader: NSViewRepresentable {
    let found: (NSWindow?) -> Void

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async { found(view.window) }
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        DispatchQueue.main.async { found(view.window) }
    }
}
