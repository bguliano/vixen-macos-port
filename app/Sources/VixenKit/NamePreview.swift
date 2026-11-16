import AppKit
import Observation
import SwiftUI

/// The rows of a naming dialog's preview, up to a million of them. The names stay here, out of SwiftUI's sight, and the table asks for the rows it's drawing.
///
/// Making them happens off the main thread, one run at a time. A request made during a run waits, replacing any request already waiting, so typing never queues
/// work, and a finished run that a newer request has overtaken is dropped. The rows shown stay the last good ones until new ones arrive.
@Observable
final class NamePreview: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    /// True once the rows have been behind for long enough to be worth saying so.
    private(set) var showsProgress = false

    @ObservationIgnored private var names: [String]
    @ObservationIgnored private var oldNames: [String]?
    @ObservationIgnored private weak var tableView: NSTableView?
    @ObservationIgnored private var closed = false
    @ObservationIgnored private var token = 0
    @ObservationIgnored private var appliedToken = 0
    @ObservationIgnored private var running = false
    @ObservationIgnored private var waiting: (@Sendable () -> [String])?
    @ObservationIgnored private var whenCurrent: (([String]) -> Void)?

    private static let progressDelay = Duration.milliseconds(250)
    private static let cellID = NSUserInterfaceItemIdentifier("name")
    private static let oldColumnID = NSUserInterfaceItemIdentifier("old")
    private static let newColumnID = NSUserInterfaceItemIdentifier("new")

    /// With `oldNames`, there's one row for each in two columns, and rows past the end of the new names show `-`.
    init(oldNames: [String]?, names: [String] = []) {
        self.oldNames = oldNames
        self.names = names
    }

    /// Replaces whatever is waiting. `make` runs off the main thread.
    func request(_ make: @escaping @Sendable () -> [String]) {
        let wasCurrent = appliedToken == token
        token += 1
        waiting = make
        startWaiting()
        guard wasCurrent else { return }
        let behindSince = token
        Task { [weak self] in
            try? await Task.sleep(for: Self.progressDelay)
            guard let self, self.appliedToken < behindSince else { return }
            self.showsProgress = true
        }
    }

    /// Calls `body` with the names of the newest request, now if they're in or later if they aren't.
    func onceCurrent(_ body: @escaping ([String]) -> Void) {
        if appliedToken == token {
            body(names)
        } else {
            whenCurrent = body
        }
    }

    private func startWaiting() {
        guard !running, !closed, let make = waiting else { return }
        waiting = nil
        running = true
        let run = token
        Task.detached(priority: .userInitiated) {
            let made = make()
            await self.finished(run, made)
        }
    }

    /// `made` is still held by the run, off the main thread, so one that's dropped here gets freed there.
    private func finished(_ run: Int, _ made: [String]) {
        guard !closed else { return }
        running = false
        guard run == token else {
            startWaiting()
            return
        }
        Self.freeOffMain(&names, replacingWith: made)
        appliedToken = run
        showsProgress = false
        tableView?.reloadData()
        if let body = whenCurrent {
            whenCurrent = nil
            body(made)
        }
    }

    /// For when the sheet is finishing, so it doesn't leave a million strings to be freed on the main thread as it goes. The table keeps the rows it has made until then.
    func close() {
        closed = true
        whenCurrent = nil
        Self.freeOffMain(&names, replacingWith: [])
        if var old = oldNames {
            oldNames = nil
            Self.freeOffMain(&old, replacingWith: [])
        }
    }

    /// Freeing a million strings is a visible hitch on the main thread. Moving them into a box first leaves the background task holding the last reference, whoever finishes first.
    private static func freeOffMain(_ names: inout [String], replacingWith new: [String]) {
        let doomed = Doomed(names)
        names = new
        Task.detached(priority: .utility) { doomed.names = [] }
    }

    private nonisolated final class Doomed: @unchecked Sendable {
        var names: [String]

        init(_ names: [String]) {
            self.names = names
        }
    }

    // MARK: The table

    fileprivate func attach(_ table: NSTableView) {
        tableView = table
        let columns: [(NSUserInterfaceItemIdentifier, String)]
        if oldNames == nil {
            columns = [(Self.newColumnID, "Name")]
        } else {
            columns = [(Self.oldColumnID, "Old Name"), (Self.newColumnID, "New Name")]
        }
        for (id, title) in columns {
            let column = NSTableColumn(identifier: id)
            column.title = title
            column.minWidth = 60
            column.resizingMask = [.autoresizingMask, .userResizingMask]
            table.addTableColumn(column)
        }
        table.dataSource = self
        table.delegate = self
    }

    func numberOfRows(in tableView: NSTableView) -> Int {
        oldNames?.count ?? names.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let label = tableView.makeView(withIdentifier: Self.cellID, owner: self) as? NSTextField ?? Self.makeLabel()
        if tableColumn?.identifier == Self.oldColumnID {
            label.stringValue = oldNames.map { row < $0.count ? $0[row] : "" } ?? ""
        } else {
            label.stringValue = row < names.count ? names[row] : "-"
        }
        return label
    }

    private static func makeLabel() -> NSTextField {
        let label = NSTextField(labelWithString: "")
        label.identifier = cellID
        label.lineBreakMode = .byTruncatingTail
        label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        return label
    }
}

/// `listViewNames`. A real `NSTableView`, so a million rows cost what the screenful on view costs.
struct NamePreviewTable: View {
    let preview: NamePreview

    var body: some View {
        NamePreviewScrollView(preview: preview)
            .overlay(alignment: .topTrailing) {
                if preview.showsProgress {
                    ProgressView()
                        .controlSize(.small)
                        .padding(4)
                }
            }
    }
}

private struct NamePreviewScrollView: NSViewRepresentable {
    let preview: NamePreview

    func makeNSView(context: Context) -> NSScrollView {
        let table = NSTableView()
        table.style = .plain
        table.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        table.allowsColumnReordering = false
        table.allowsColumnSelection = false
        table.usesAlternatingRowBackgroundColors = true
        table.refusesFirstResponder = true
        // A million of these stays under 2^24 points, past which layer positions start to lose whole points.
        table.rowHeight = 16
        table.intercellSpacing = NSSize(width: 6, height: 0)
        preview.attach(table)

        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.borderType = .lineBorder
        table.reloadData()
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {}
}
