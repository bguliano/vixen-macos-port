import AppKit
import Observation
import SwiftUI
import VixenBridge
import VixenKit

/// One row's values. Its own object, so that a commit updates only the rows that changed, and a text edit or a drag in another row stays alive.
@Observable
final class EditorRow: Identifiable {
    let key: EeRowKey
    private(set) var label: String
    private(set) var tooltip: String
    private(set) var readOnly: Bool
    private(set) var control: EeRowControl

    var id: EeRowKey { key }

    init(_ row: EeRow) {
        key = row.key
        label = row.label
        tooltip = row.tooltip
        readOnly = row.readOnly
        control = row.control
    }

    func apply(_ row: EeRow) {
        if label != row.label { label = row.label }
        if tooltip != row.tooltip { tooltip = row.tooltip }
        if readOnly != row.readOnly { readOnly = row.readOnly }
        if control != row.control { control = row.control }
    }
}

/// One category. Its fold state follows the name: moving to another selection that shows this category keeps it, and it's lost if a selection without it comes between (KEEP, F13).
@Observable
final class EditorSection: Identifiable {
    let name: String
    var expanded = true
    private(set) var rows: [EditorRow] = []

    var id: String { name }

    init(name: String, startsCollapsed: Bool) {
        self.name = name
        expanded = !startsCollapsed
    }

    func apply(_ newRows: [EeRow]) {
        let old = Dictionary(uniqueKeysWithValues: rows.map { ($0.key, $0) })
        let updated = newRows.map { row -> EditorRow in
            if let existing = old[row.key] {
                existing.apply(row)
                return existing
            }
            return EditorRow(row)
        }
        // Showing and hiding rows re-sorts, without rebuilding the rows that stay (F18).
        if updated.map(\.key) != rows.map(\.key) {
            rows = updated
        }
    }
}

/// The panel's state: `PanelModel` values and what's going on with the selection. No logic here that a test would want: Rust works it out.
@Observable
public final class EffectEditorModel {
    public let session: SequencerSession
    @ObservationIgnored let api: EffectEditorApi
    @ObservationIgnored let core: VixenCore
    /// What the panel's rows were read for.
    public private(set) var selection: [UInt64] = []
    private(set) var effectName = ""
    private(set) var information = ""
    private(set) var informationLink = ""
    private(set) var effectCount: UInt32 = 0
    private(set) var sections: [EditorSection] = []
    /// The name column is as wide as the widest name (F15).
    private(set) var labelWidth: CGFloat = 60
    private(set) var canSaveDefault = false
    private(set) var hasStoredDefault = false
    /// F5: not saved. Off each time a sequence window opens.
    var previewOn = false {
        didSet {
            if oldValue != previewOn {
                api.previewSetOn(on: previewOn)
            }
        }
    }

    /// F58: one selected list row in the panel, held here so that a press elsewhere can clear it.
    var selectedListRow: ListRow?
    @ObservationIgnored var listRegions: [String: WeakListRegion] = [:]
    @ObservationIgnored private var pressMonitor: Any?

    /// An entry went on the session's undo stack. Return true if the window's `UndoManager` has been told; otherwise the panel tells it.
    @ObservationIgnored public var onUndoRecorded: ((String) -> Bool)?
    @ObservationIgnored weak var hostWindow: NSWindow?
    @ObservationIgnored private var debounce: Task<Void, Never>?
    @ObservationIgnored private var lastRevision: UInt64 = 0

    public init(session: SequencerSession) {
        self.session = session
        api = session.effectEditor()
        core = session.applicationCore()
        reload()
    }

    /// F2: each change restarts a 200 ms timer, and only when it fires does the panel read the new selection. An identical selection does nothing.
    public func selectionChanged(_ effects: [UInt64]) {
        debounce?.cancel()
        debounce = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled, let self else { return }
            self.setSelectionNow(effects)
        }
    }

    public func setSelectionNow(_ effects: [UInt64]) {
        // F32's flush rule: what's pending belongs to the old selection.
        PendingCommits.flushAll()
        // F2's note: an identical selection still restarts the preview, which runs before the panel's comparison. F4: and it isn't limited.
        api.previewSetSelection(effects: effects)
        guard effects != selection else { return }
        selection = effects
        reload()
    }

    /// Reads every row for the selection, reusing the rows whose keys stay.
    public func reload() {
        let panel = api.panel(effects: selection)
        lastRevision = api.editRevision()
        if effectName != panel.effectName { effectName = panel.effectName }
        if information != panel.information { information = panel.information }
        if informationLink != panel.informationLink { informationLink = panel.informationLink }
        if effectCount != panel.effectCount { effectCount = panel.effectCount }
        if canSaveDefault != panel.canSaveDefault { canSaveDefault = panel.canSaveDefault }
        if hasStoredDefault != panel.hasStoredDefault { hasStoredDefault = panel.hasStoredDefault }
        let old = Dictionary(uniqueKeysWithValues: sections.map { ($0.name, $0) })
        let updated = panel.categories.map { category -> EditorSection in
            let section = old[category.name] ?? EditorSection(name: category.name, startsCollapsed: category.startsCollapsed)
            section.apply(category.rows)
            return section
        }
        if updated.map(\.name) != sections.map(\.name) {
            sections = updated
        }
        let width = measuredLabelWidth(panel)
        if abs(width - labelWidth) > 0.5 {
            labelWidth = width
        }
    }

    /// Someone else changed the document (the timeline, undo, a library): read the values again. Called a few times a second; costs one atomic read when nothing changed.
    /// Real playback takes the engine from the Effect Preview by itself, and the preview comes back here once the engine is free (F6's Rules, F267's `ResumePreview`).
    func checkForOutsideEdits() {
        if previewOn {
            api.previewPoll()
        }
        guard api.editRevision() != lastRevision else { return }
        reload()
    }

    private func measuredLabelWidth(_ panel: EePanel) -> CGFloat {
        let font = NSFont.systemFont(ofSize: NSFont.smallSystemFontSize)
        var widest: CGFloat = 0
        func measure(_ rows: [EeRow]) {
            for row in rows {
                if case let .recordList(style, items) = row.control {
                    if !style.inline {
                        for item in items {
                            measure(item.rows)
                        }
                    }
                } else if !row.control.isCollection {
                    widest = max(widest, ceil((row.label as NSString).size(withAttributes: [.font: font]).width))
                }
            }
        }
        for category in panel.categories {
            measure(category.rows)
        }
        return max(widest, 20)
    }

    /// Export is starting: the preview stops, and `resumePreview` brings it back. Real playback needs neither: it takes the engine, and the preview returns after it.
    /// Without the panel to hand, `VixenCore.holdEffectPreview(held:)` does the same for the whole app.
    public func suspendPreview() {
        api.previewSuspend()
    }

    public func resumePreview() {
        api.previewResume()
    }

    /// The panel is going away: the context is released (F6's teardown).
    func panelClosed() {
        PendingCommits.flushAll()
        api.previewSetOn(on: false)
        if let pressMonitor {
            NSEvent.removeMonitor(pressMonitor)
        }
        pressMonitor = nil
    }

    /// F58: losing focus clears the selected row, and the list's own buttons don't count. Upstream does it with focus scopes. Here a press is what moves the focus, so a press outside the row's list clears it, before that press does anything else.
    func watchPresses() {
        guard pressMonitor == nil else { return }
        pressMonitor = NSEvent.addLocalMonitorForEvents(matching: .leftMouseDown) { [weak self] event in
            guard let self, let row = self.selectedListRow else { return event }
            let region = self.listRegions[row.param]?.view
            let inside = region.flatMap { view -> Bool? in
                guard view.window === event.window else { return false }
                return view.convert(view.bounds, to: nil).contains(event.locationInWindow)
            } ?? false
            if !inside {
                self.selectedListRow = nil
            }
            return event
        }
    }

    /// F8.
    func saveAsDefault() {
        guard selection.count == 1 else { return }
        do {
            try api.saveAsDefault(effect: selection[0])
        } catch {
            showError(error, title: "Save as Default")
        }
        reload()
    }

    /// F9: asks first. Changes no effect; only those created later.
    func resetDefault() {
        guard selection.count == 1, let window = hostWindow else { return }
        let id = selection[0]
        let name = effectName
        Task { @MainActor in
            let answer = await askInSheet(
                on: window, message: "Remove the saved default for \(name)?", title: "Reset Effect Default", buttons: ["Remove Default", "Cancel"], destructiveIndex: 0
            )
            guard answer == 0 else { return }
            do {
                try api.resetDefault(effect: id)
            } catch {
                showError(error, title: "Reset Effect Default")
            }
            reload()
        }
    }

    private func showError(_ error: Error, title: String) {
        guard let window = hostWindow else { return }
        Task { @MainActor in
            _ = await askInSheet(on: window, message: error.localizedDescription, title: title, buttons: ["OK"], style: .critical)
        }
    }

    // MARK: Commits. One call to Rust sets the value on the selection and records one undo step.

    func commit(_ key: EeRowKey, _ value: EeValue, gesture: UInt64? = nil) {
        finish(api.commit(effects: selection, key: key, value: value, gesture: gesture))
    }

    func listAdd(_ param: String) {
        finish(api.listAdd(effects: selection, param: param))
    }

    func listRemove(_ param: String, index: Int) {
        finish(api.listRemove(effects: selection, param: param, index: UInt32(index)))
    }

    func listMove(_ param: String, from: Int, to: Int) {
        finish(api.listMove(effects: selection, param: param, from: UInt32(from), to: UInt32(to)))
    }

    func canDrop(_ key: EeRowKey, _ value: VixenDragValue) -> Bool {
        api.canDrop(effects: selection, key: key, dragText: value.dragText)
    }

    func drop(_ key: EeRowKey, _ value: VixenDragValue) {
        finish(api.dropValue(effects: selection, key: key, dragText: value.dragText))
    }

    /// Another area wrote through Rust for this panel, such as the polygon editor: the rows are read again, and its undo step goes to the window's `UndoManager` like the panel's own.
    func externalCommitFinished(undoLabel: String?) {
        reload()
        if let undoLabel {
            undoRecorded(undoLabel)
        }
    }

    private func finish(_ result: EeCommitResult) {
        if UITestHarness.isActive {
            UITestHarness.note("commit changed=\(result.changed) newStep=\(result.newUndoStep) label=\(result.undoLabel)")
        }
        // Even with no change: the row goes back to what's stored, as a revert or a Gain that reads back rounded.
        reload()
        if result.newUndoStep {
            undoRecorded(result.undoLabel)
        }
    }

    // MARK: Undo. The session's stack, mirrored in the window's `UndoManager` so the Edit menu names the action.

    private func undoRecorded(_ label: String) {
        if onUndoRecorded?(label) == true {
            return
        }
        guard let manager = hostWindow?.undoManager else { return }
        manager.registerUndo(withTarget: self) { model in
            MainActor.assumeIsolated { model.undoOne(manager) }
        }
        manager.setActionName(label)
    }

    private func undoOne(_ manager: UndoManager) {
        PendingCommits.flushAll()
        let label = session.undoDescriptions().first ?? ""
        _ = session.undo(steps: 1)
        reload()
        manager.registerUndo(withTarget: self) { model in
            MainActor.assumeIsolated { model.redoOne(manager) }
        }
        manager.setActionName(label)
    }

    private func redoOne(_ manager: UndoManager) {
        PendingCommits.flushAll()
        let label = session.redoDescriptions().first ?? ""
        _ = session.redo(steps: 1)
        reload()
        manager.registerUndo(withTarget: self) { model in
            MainActor.assumeIsolated { model.undoOne(manager) }
        }
        manager.setActionName(label)
    }
}

/// A list row: the property, and its place in the list.
struct ListRow: Equatable {
    let param: String
    let index: Int
}

/// Where a list is on screen.
struct WeakListRegion {
    weak var view: NSView?
}

extension EeRowControl {
    /// F16: a list has its name as a header above it, and the editor takes the full width.
    var isCollection: Bool {
        switch self {
        case .gradientList, .colorList, .gradientLevels, .stringList, .recordList: true
        default: false
        }
    }
}

extension VixenDragValue {
    var dragText: String {
        switch self {
        case let .curve(value): curveDragText(value: value)
        case let .gradient(value): gradientDragText(value: value)
        case let .color(argb): colorDragText(argb: argb)
        }
    }
}
