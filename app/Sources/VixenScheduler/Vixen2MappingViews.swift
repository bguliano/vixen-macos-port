import AppKit
import SwiftUI
import VixenBridge
import VixenKit

private let vixen2Pasteboard = NSPasteboard.PasteboardType("org.vixen.vixen2.destination")

private final class Vixen2NativeTable: NSTableView {
    var clear: (() -> Void)?
    override func keyDown(with event: NSEvent) { if event.keyCode == 51 || event.keyCode == 117 { clear?() } else { super.keyDown(with: event) } }
}

struct Vixen2MapTable: NSViewRepresentable {
    let model: Vixen2MapModel
    func makeCoordinator() -> Coordinator { Coordinator(model: model) }
    func makeNSView(context: Context) -> NSScrollView {
        let table = Vixen2NativeTable()
        for (id, title, width) in [("number", "V2 Channel", 75.0), ("output", "V2 Output", 70.0), ("name", "V2 Name", 140.0), ("color", "V2 Color", 85.0), ("target", "Destination Element", 150.0), ("import", "Import Color", 85.0), ("mix", "Color Mixing", 85.0)] {
            let c = NSTableColumn(identifier: .init(id)); c.title = title; c.width = width; c.minWidth = 50; table.addTableColumn(c)
        }
        table.allowsMultipleSelection = true; table.rowHeight = 24; table.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        table.delegate = context.coordinator; table.dataSource = context.coordinator; table.clear = { model.clear() }
        table.registerForDraggedTypes([vixen2Pasteboard]); table.setAccessibilityLabel("Vixen 2 channel mappings")
        let scroll = NSScrollView(); scroll.documentView = table; scroll.hasVerticalScroller = true; scroll.hasHorizontalScroller = true; scroll.autohidesScrollers = true
        context.coordinator.table = table; return scroll
    }
    func updateNSView(_ view: NSScrollView, context: Context) {
        let c = context.coordinator
        if c.revision != model.revision { c.revision = model.revision; c.updating = true; c.table?.reloadData(); c.table?.selectRowIndexes(model.selected, byExtendingSelection: false); c.updating = false }
    }
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        let model: Vixen2MapModel
        weak var table: NSTableView?
        var revision = -1
        var updating = false
        init(model: Vixen2MapModel) { self.model = model }
        func numberOfRows(in tableView: NSTableView) -> Int { model.rows.count }
        func tableView(_ tableView: NSTableView, viewFor column: NSTableColumn?, row: Int) -> NSView? {
            let mapping = model.rows[row], key = column?.identifier.rawValue ?? ""
            let field = NSTextField(labelWithString: ""); field.lineBreakMode = .byTruncatingTail
            switch key {
            case "number": field.stringValue = mapping.number
            case "output": field.stringValue = mapping.output
            case "name": field.stringValue = mapping.name
            case "target": field.stringValue = mapping.targetName
            case "mix": field.stringValue = mapping.mixing ? "Yes" : ""
            case "color", "import":
                let argb = key == "color" ? mapping.color : mapping.importColor
                if argb != 0 {
                    let r = Double((argb >> 16) & 255) / 255, g = Double((argb >> 8) & 255) / 255, b = Double(argb & 255) / 255
                    field.stringValue = model.colorNames[argb] ?? ""
                    field.backgroundColor = NSColor(srgbRed: r, green: g, blue: b, alpha: Double(argb >> 24) / 255); field.drawsBackground = true
                    field.textColor = r * 0.299 + g * 0.587 + b * 0.114 > 0.55 ? .black : .white
                }
            default: break
            }
            if ["number", "output", "name"].contains(key) { field.textColor = .secondaryLabelColor; field.backgroundColor = .controlBackgroundColor; field.drawsBackground = true }
            return field
        }
        func tableViewSelectionDidChange(_ notification: Notification) { if !updating { model.selected = table?.selectedRowIndexes ?? [] } }
        func tableView(_ tableView: NSTableView, validateDrop info: NSDraggingInfo, proposedRow row: Int, proposedDropOperation operation: NSTableView.DropOperation) -> NSDragOperation {
            let hitRow = tableView.row(at: tableView.convert(info.draggingLocation, from: nil))
            guard model.rows.indices.contains(hitRow), info.draggingPasteboard.availableType(from: [vixen2Pasteboard]) != nil else { return [] }; tableView.setDropRow(hitRow, dropOperation: .on); return .copy
        }
        func tableView(_ tableView: NSTableView, acceptDrop info: NSDraggingInfo, row: Int, dropOperation: NSTableView.DropOperation) -> Bool {
            let hitRow = tableView.row(at: tableView.convert(info.draggingLocation, from: nil))
            guard model.rows.indices.contains(hitRow) else { return false }
            let nodes = info.draggingPasteboard.pasteboardItems?.compactMap { $0.string(forType: vixen2Pasteboard) } ?? []
            guard !nodes.isEmpty else { return false }; model.drop(start: hitRow, nodes: nodes); return true
        }
    }
}

struct Vixen2MapTree: NSViewRepresentable {
    let model: Vixen2MapModel
    func makeCoordinator() -> Coordinator { Coordinator(model: model) }
    func makeNSView(context: Context) -> NSScrollView {
        let tree = NSOutlineView(); let column = NSTableColumn(identifier: .init("element")); column.title = "Destination Elements"; column.width = 220; tree.addTableColumn(column); tree.outlineTableColumn = column
        tree.allowsMultipleSelection = true; tree.rowHeight = 24; tree.delegate = context.coordinator; tree.dataSource = context.coordinator
        tree.setDraggingSourceOperationMask(.copy, forLocal: true); tree.setAccessibilityLabel("Vixen 2 destination elements")
        let scroll = NSScrollView(); scroll.documentView = tree; scroll.hasVerticalScroller = true; scroll.autohidesScrollers = true; context.coordinator.tree = tree; return scroll
    }
    func updateNSView(_ view: NSScrollView, context: Context) {}
    final class Item { let row: MapperTreeRow; init(_ row: MapperTreeRow) { self.row = row } }
    final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
        let model: Vixen2MapModel
        weak var tree: NSOutlineView?
        var children: [String: [Item]] = [:]
        init(model: Vixen2MapModel) { self.model = model }
        func children(_ item: Any?) -> [Item] {
            let key = (item as? Item)?.row.path ?? ""
            if let cached = children[key] { return cached }
            let loaded = model.dialog.treeRows(parent: key.isEmpty ? nil : key).map(Item.init); children[key] = loaded; return loaded
        }
        func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int { children(item).count }
        func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any { children(item)[index] }
        func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool { (item as? Item)?.row.childCount ?? 0 > 0 }
        func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? { let field = NSTextField(labelWithString: (item as? Item)?.row.name ?? ""); field.lineBreakMode = .byTruncatingTail; return field }
        func outlineViewSelectionDidChange(_ notification: Notification) { guard let tree else { return }; model.nodes = tree.selectedRowIndexes.compactMap { (tree.item(atRow: $0) as? Item)?.row.id } }
        func outlineView(_ outlineView: NSOutlineView, pasteboardWriterForItem item: Any) -> NSPasteboardWriting? { guard let item = item as? Item else { return nil }; let p = NSPasteboardItem(); p.setString(item.row.id, forType: vixen2Pasteboard); return p }
    }
}
