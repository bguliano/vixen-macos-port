import SwiftUI
import VixenBridge
import VixenKit

/// Setup Previews. List edits apply live to the running previews, as upstream, so OK and Cancel only decide what happens next.
struct SetupPreviewsSheet: View {
    @Environment(AdminModel.self) private var admin
    @Environment(\.openWindow) private var openWindow
    @State private var rows: [SetupPreviewRow] = []
    @State private var selection = Set<String>()
    @State private var nameText = ""
    /// The preview shown in the group, kept selected across each fill.
    @State private var displayed: String?
    /// A preview stopped for its designer, to start again when that closes.
    @State private var restartAfterDesign: String?
    @State private var alerts = AlertPresenter()

    private var core: VixenCore? { admin.app.core }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Setup Previews").font(.headline)
            Table(rows, selection: $selection) {
                TableColumn("") { row in
                    Toggle("", isOn: Binding(get: { row.isRunning }, set: { setRunning(row, $0) }))
                        .labelsHidden()
                        .accessibilityLabel("\(row.name) running")
                }
                .width(24)
                TableColumn("Name", value: \.name)
                TableColumn("Type", value: \.typeName)
            }
            .frame(minHeight: 160)
            HStack {
                Button("Add New Preview", action: add)
                Button("Duplicate Selected", action: duplicate)
                    .disabled(selection.isEmpty)
                Button("Delete Selected", action: delete)
                    .disabled(selection.isEmpty)
                Spacer()
            }
            GroupBox("Selected Preview") {
                Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 10) {
                    GridRow {
                        Text("Name:")
                        TextField("Name:", text: $nameText, prompt: Text(""))
                            .labelsHidden()
                        Button("Update", action: update)
                    }
                    GridRow {
                        Button("Configure Preview", action: configure)
                            .gridCellColumns(1)
                        Text("Configure details specific to the preview.")
                            .gridCellColumns(2)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(4)
            }
            .disabled(selection.count != 1)
            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: cancel)
                    .keyboardShortcut(.cancelAction)
                Button("OK") { finish(saved: true) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 482, height: 569)
        .alerts(from: alerts)
        .onAppear {
            fill()
            admin.app.previewDesignerDidClose = { id in
                if id == restartAfterDesign {
                    restartStoppedPreview()
                }
            }
        }
        .onChange(of: selection) { showSelection() }
    }

    /// `_PopulatePreviewList`.
    private func fill() {
        rows = core?.setupPreviewRows() ?? []
        if let displayed, rows.contains(where: { $0.id == displayed }) {
            selection = [displayed]
        }
    }

    /// One row fills the group, and none or many clear it.
    private func showSelection() {
        if selection.count == 1, let id = selection.first, let row = rows.first(where: { $0.id == id }) {
            displayed = id
            nameText = row.name
        } else {
            displayed = nil
            nameText = ""
        }
    }

    /// F90. The box follows the device, so with the engine closed it doesn't stay ticked.
    private func setRunning(_ row: SetupPreviewRow, _ running: Bool) {
        _ = core?.setupSetPreviewRunning(id: row.id, running: running)
        rows = core?.setupPreviewRows() ?? []
    }

    /// F91: the new preview opens in the designer at once.
    private func add() {
        guard let core else { return }
        let id = core.setupAddPreview()
        displayed = id
        fill()
        showSelection()
        design(id: id)
    }

    /// F92.
    private func duplicate() {
        guard let core else { return }
        let made = core.setupDuplicatePreviews(ids: rows.filter { selection.contains($0.id) }.map(\.id))
        if let last = made.last {
            displayed = last
        }
        fill()
        showSelection()
    }

    /// F93.
    private func delete() {
        let plural = selection.count > 1
        Task {
            let answer = await alerts.ask(
                plural ? "Delete previews?" : "Delete preview?",
                plural ? "Are you sure you want to delete the selected previews?" : "Are you sure you want to delete the selected preview?",
                [.init("Cancel", role: .cancel), .init("Delete", role: .destructive)],
                critical: true
            )
            guard answer == "Delete", let core else { return }
            core.setupDeletePreviews(ids: Array(selection))
            selection = []
            displayed = nil
            fill()
            showSelection()
        }
    }

    /// F94: no validation, so an empty or a repeated name is allowed.
    private func update() {
        guard let displayed, let core else { return }
        core.setupRenamePreview(id: displayed, name: nameText)
        fill()
    }

    /// F95.
    private func configure() {
        guard let id = selection.first else { return }
        core?.setupMarkPreviewChanged()
        design(id: id)
    }

    private func design(id: String) {
        guard let core else { return }
        if rows.first(where: { $0.id == id })?.isRunning == true {
            _ = core.setupSetPreviewRunning(id: id, running: false)
            restartAfterDesign = id
            rows = core.setupPreviewRows()
        }
        openWindow(id: WindowID.previewSetup, value: id)
    }

    private func restartStoppedPreview() {
        guard let id = restartAfterDesign, let core else { return }
        restartAfterDesign = nil
        _ = core.setupSetPreviewRunning(id: id, running: true)
        rows = core.setupPreviewRows()
    }

    /// F96.
    private func cancel() {
        guard core?.setupPreviewChangesMade() == true else {
            finish(saved: false)
            return
        }
        Task {
            let answer = await alerts.ask(
                "Are you sure?",
                "All changes will be lost if you continue, do you wish to continue?",
                [.init("Keep Editing", role: .cancel), .init("Discard Changes", role: .destructive)]
            )
            if answer == "Discard Changes" {
                finish(saved: false)
            }
        }
    }

    private func finish(saved: Bool) {
        restartStoppedPreview()
        admin.app.previewDesignerDidClose = nil
        admin.finishSetupPreviews(saved: saved)
    }
}
