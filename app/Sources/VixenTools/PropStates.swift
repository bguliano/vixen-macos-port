import AppKit
import SwiftUI
import VixenBridge
import VixenKit

struct PropStatePanel: View {
    let model: PropEditorModel
    private var selected: PropEdStateDefinition? { model.states.definitions.first { $0.id == model.states.selectedDefinition } }
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Name")
                Picker("State definition", selection: Binding(get: { model.states.selectedDefinition ?? "" }, set: { value in model.editor.selectStateDefinition(id: value); model.refresh() })) {
                    if model.states.definitions.isEmpty { Text("None").tag("") }
                    ForEach(model.states.definitions, id: \.id) { Text($0.name).tag($0.id) }
                }.labelsHidden().frame(maxWidth: .infinity)
            }
            HStack {
                Spacer()
                Button("Add") { model.editDefinition(.add) }
                Button("Delete") { model.deleteDefinition() }.disabled(selected == nil)
                Menu { Button("Rename") { model.editDefinition(.rename) }; Button("Copy") { model.editDefinition(.copy) } } label: { Image(systemName: "ellipsis") }.disabled(selected == nil).fixedSize()
            }
            if let selected {
                TextField("Description", text: Binding(get: { selected.description }, set: { value in model.mutate { _ = $0.setStateDescription(text: value) } }), axis: .vertical).lineLimit(2...4).textFieldStyle(.roundedBorder)
            }
            Text("State Items").font(.headline)
            PropStateTable(model: model).frame(minHeight: 100)
            HStack(spacing: 5) {
                Spacer()
                Button("Add") { model.mutate { _ = $0.addStateItem(copy: false) } }.disabled(selected == nil)
                Button("Copy") { model.mutate { _ = $0.addStateItem(copy: true) } }.disabled(model.states.items.filter(\.selected).count != 1)
                Button("Remove") { model.removeStateItems() }.disabled(!model.states.items.contains(where: \.selected))
                Button { model.mutate { _ = $0.moveStateItem(down: false) } } label: { Image(systemName: "arrow.up") }.help("Move selected State Item up").disabled(!model.states.canMoveUp)
                Button { model.mutate { _ = $0.moveStateItem(down: true) } } label: { Image(systemName: "arrow.down") }.help("Move selected State Item down").disabled(!model.states.canMoveDown)
            }
            if !model.states.validation.isEmpty {
                ScrollView { Text(model.states.validation.joined(separator: "\n")).font(.caption).foregroundStyle(Color(red: 0.804, green: 0.361, blue: 0.361)).frame(maxWidth: .infinity, alignment: .leading) }.frame(maxHeight: 90)
            }
        }.padding(6)
    }
}

extension PropEditorModel {
    enum DefinitionEdit { case add, rename, copy }
    func editDefinition(_ mode: DefinitionEdit) {
        let selected = states.definitions.first { $0.id == states.selectedDefinition }
        let title = mode == .add ? "Add State Definition" : mode == .rename ? "Rename State Definition" : "Copy State Definition"
        let initial = mode == .add ? states.suggestedName : (selected?.name ?? "") + (mode == .copy ? " Copy" : "")
        Task {
            let names = states.definitions.filter { mode != .rename || $0.id != selected?.id }.map(\.name)
            guard let name = await prompt("Name", title: title, initial: initial, validator: { !names.contains($0.trimmingCharacters(in: .whitespacesAndNewlines)) }) else { return }
            let before = state.revision
            do {
                switch mode { case .add: try editor.addStateDefinition(name: name); case .rename: _ = try editor.renameStateDefinition(name: name); case .copy: _ = try editor.copyStateDefinition(name: name) }
                refresh(); registerChange(after: before)
            } catch { await tell(title, error.userMessage) }
        }
    }
    func deleteDefinition() {
        guard let selected = states.definitions.first(where: { $0.id == states.selectedDefinition }) else { return }
        Task {
            let alert = NSAlert(); alert.messageText = "Delete State Definition"; alert.informativeText = "Delete state definition \"\(selected.name)\"?"; alert.addButton(withTitle: "Delete"); alert.addButton(withTitle: "Cancel")
            if await show(alert) == .alertFirstButtonReturn { mutate { _ = $0.deleteStateDefinition() } }
        }
    }
    func removeStateItems() {
        let selected = states.items.filter(\.selected); guard !selected.isEmpty else { return }
        Task {
            let alert = NSAlert(); alert.messageText = "Remove State Items"; alert.informativeText = selected.count == 1 ? "Remove state item \"\(selected[0].name)\"?" : "Remove \(selected.count) selected state items?"; alert.addButton(withTitle: "Remove"); alert.addButton(withTitle: "Cancel")
            if await show(alert) == .alertFirstButtonReturn { mutate { _ = $0.removeStateItems() } }
        }
    }
}

private struct PropStateTable: NSViewRepresentable {
    let model: PropEditorModel
    func makeCoordinator() -> Coordinator { Coordinator(model) }
    func makeNSView(context: Context) -> NSScrollView {
        let table = NSTableView(); table.allowsMultipleSelection = true; table.style = .fullWidth; table.columnAutoresizingStyle = .firstColumnOnlyAutoresizingStyle
        for (id, title, width) in [("name", "Name", 130.0), ("color", "Color", 110.0), ("count", "Count", 50.0)] {
            let column = NSTableColumn(identifier: .init(id)); column.title = title; column.width = width; column.minWidth = id == "name" ? 70 : width; if id != "name" { column.maxWidth = width }; column.sortDescriptorPrototype = NSSortDescriptor(key: id, ascending: true); table.addTableColumn(column)
        }
        table.delegate = context.coordinator; table.dataSource = context.coordinator; table.target = context.coordinator; table.doubleAction = #selector(Coordinator.doubleClick)
        table.setAccessibilityLabel("State Items")
        let scroll = StateScrollView(); scroll.documentView = table; scroll.hasVerticalScroller = true; context.coordinator.table = table; return scroll
    }
    func updateNSView(_ view: NSScrollView, context: Context) {
        let c = context.coordinator; guard c.revision != model.revision else { return }; c.revision = model.revision
        c.syncing = true; c.table?.reloadData(); c.table?.selectRowIndexes(IndexSet(model.states.items.indices.filter { model.states.items[$0].selected }), byExtendingSelection: false); c.syncing = false
    }
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate {
        let model: PropEditorModel
        weak var table: NSTableView?
        var revision = -1
        var syncing = false
        init(_ model: PropEditorModel) { self.model = model }
        func numberOfRows(in tableView: NSTableView) -> Int { model.states.items.count }
        func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
            guard let column = tableColumn?.identifier.rawValue else { return nil }
            let item = model.states.items[row]
            let field = NSTextField(string: column == "name" ? item.name : column == "color" ? item.colorHex : String(item.count))
            field.identifier = .init(item.id + "/" + column); field.delegate = self; field.isBordered = false; field.drawsBackground = column == "color"; field.isEditable = column != "count"; field.lineBreakMode = .byTruncatingTail
            if column == "color" {
                let color = NSColor(argb: item.color); field.backgroundColor = color
                field.textColor = color.brightnessComponent > 0.6 ? .black : .white
                field.alignment = .center
                let well = StateColorWell(); well.color = color; well.identifier = .init(item.id)
                well.target = self; well.action = #selector(wellChanged); well.isBordered = false
                well.widthAnchor.constraint(equalToConstant: 18).isActive = true
                well.heightAnchor.constraint(equalToConstant: 18).isActive = true
                field.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
                let row = NSStackView(views: [well, field]); row.orientation = .horizontal; row.spacing = 3
                return row
            }
            return field
        }
        func controlTextDidEndEditing(_ notification: Notification) {
            guard let field = notification.object as? NSTextField, let identifier = field.identifier?.rawValue else { return }
            let parts = identifier.split(separator: "/"); guard parts.count == 2 else { return }
            model.mutate { _ = $0.editStateItem(id: String(parts[0]), name: parts[1] == "name" ? field.stringValue : nil, colorText: parts[1] == "color" ? field.stringValue : nil) }
        }
        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSTextField, let identifier = field.identifier?.rawValue else { return }
            let parts = identifier.split(separator: "/"); guard parts.count == 2 else { return }
            let before = model.state.revision
            _ = model.editor.editStateItem(id: String(parts[0]), name: parts[1] == "name" ? field.stringValue : nil, colorText: parts[1] == "color" ? field.stringValue : nil)
            model.refresh(full: false); model.states = model.editor.states(); model.registerChange(after: before)
        }
        func tableViewSelectionDidChange(_ notification: Notification) {
            guard !syncing, let table else { return }
            let ids = table.selectedRowIndexes.compactMap { model.states.items.indices.contains($0) ? model.states.items[$0].id : nil }
            model.editor.selectStateItems(ids: ids); model.refresh()
        }
        func tableView(_ tableView: NSTableView, sortDescriptorsDidChange oldDescriptors: [NSSortDescriptor]) {
            guard let sort = tableView.sortDescriptors.first, let key = sort.key else { return }
            model.mutate { _ = $0.sortStateItems(column: key, ascending: sort.ascending) }
        }
        @objc func doubleClick() {
            guard let table, table.clickedRow >= 0, table.clickedColumn == 1, model.states.items.indices.contains(table.clickedRow) else { return }
            let item = model.states.items[table.clickedRow]
            let apply: (UInt32) -> Void = { [weak self] color in self?.applyColor(NSColor(argb: color), id: item.id) }
            ColorPanelSession.shared.begin(title: "State Item Color", argb: item.color, onLive: apply, onFinish: apply)
            // Keep the existing per-gesture edits; other session owners reset
            // continuous delivery when they begin their own edit.
            NSColorPanel.shared.isContinuous = false
        }
        @objc func wellChanged(_ well: NSColorWell) {
            guard let id = well.identifier?.rawValue, let rgb = well.color.usingColorSpace(.deviceRGB) else { return }
            let hex = String(format: "#%02X%02X%02X", Int((rgb.redComponent * 255).rounded()), Int((rgb.greenComponent * 255).rounded()), Int((rgb.blueComponent * 255).rounded()))
            let before = model.state.revision
            _ = model.editor.editStateItem(id: id, name: nil, colorText: hex)
            // Keep the active native well alive while the shared panel sends continuous changes.
            model.refresh(full: false); model.states = model.editor.states(); model.registerChange(after: before)
            if let row = well.superview as? NSStackView, let field = row.arrangedSubviews.compactMap({ $0 as? NSTextField }).first {
                field.stringValue = hex; field.backgroundColor = rgb
                field.textColor = rgb.brightnessComponent > 0.6 ? .black : .white
            }
        }
        private func applyColor(_ color: NSColor, id: String) {
            guard let rgb = color.usingColorSpace(.deviceRGB) else { return }
            let hex = String(format: "#%02X%02X%02X", Int((rgb.redComponent * 255).rounded()), Int((rgb.greenComponent * 255).rounded()), Int((rgb.blueComponent * 255).rounded()))
            let before = model.state.revision
            // A final color equal to its live value must not reload the table:
            // a native well in that table may be taking over the shared panel.
            guard model.editor.editStateItem(id: id, name: nil, colorText: hex) else { return }
            model.refresh(); model.registerChange(after: before)
        }
    }
}

private final class StateColorWell: NSColorWell {
    override func activate(_ exclusive: Bool) {
        super.activate(exclusive)
        if UITestHarness.isActive {
            let panel = NSColorPanel.shared; panel.setFrameAutosaveName("")
            panel.setFrameOrigin(NSPoint(x: -12000, y: -12000))
        }
    }
}

private final class StateScrollView: NSScrollView {
    override func tile() {
        super.tile()
        guard let table = documentView as? NSTableView, table.tableColumns.count == 3, contentSize.width > 0 else { return }
        let columns = table.tableColumns
        let width = max(columns[0].minWidth, contentSize.width - columns[1].width - columns[2].width - table.intercellSpacing.width * 3)
        if abs(columns[0].width - width) > 0.5 { columns[0].width = width }
    }
}
