import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// Which library a strip or panel shows. The window keeps all three, read from Rust, so they stay in step across strips, panels and other windows.
enum LibraryKind: Hashable {
    case color
    case curve
    case gradient

    var strip: ShellStrip {
        switch self {
        case .color: .libraryColor
        case .curve: .libraryCurve
        case .gradient: .libraryGradient
        }
    }
}

/// One item as strips and panels show it.
struct LibraryEntry: Identifiable, Equatable {
    /// The name, or for a colour its place in the list, since duplicates are allowed.
    let id: String
    let name: String
    let value: VixenDragValue
    /// What a colour holds, for the first-match removal.
    let argb: UInt32?
}

/// Thumbnails keyed by what they show, so a redraw of the strips or panels reuses them and only a changed item is drawn again (hazard 13).
@MainActor
enum LibraryImages {
    private enum Key: Hashable {
        case color(UInt32, CGFloat)
        case curve(CurveValue, CGFloat)
        case gradient(GradientValue, CGFloat)
    }

    private static var cache: [Key: NSImage] = [:]

    static func image(_ entry: LibraryEntry, size: CGFloat) -> NSImage {
        let key: Key = switch entry.value {
        case .color(let argb): .color(argb, size)
        case .curve(let value): .curve(value, size)
        case .gradient(let value): .gradient(value, size)
        }
        if let hit = cache[key] {
            return hit
        }
        let image = draw(entry.value, size: size)
        // Scaling steps through many sizes: start again rather than keep every one.
        if cache.count > 600 {
            cache.removeAll(keepingCapacity: true)
        }
        cache[key] = image
        return image
    }

    private static func draw(_ value: VixenDragValue, size: CGFloat) -> NSImage {
        switch value {
        case .color(let argb):
            let image = NSImage(size: NSSize(width: size, height: size))
            image.lockFocus()
            NSColor(argb: argb).setFill()
            NSRect(x: 0, y: 0, width: size, height: size).fill()
            image.unlockFocus()
            return image
        case .curve(let value):
            return CurveThumbnail.image(points: value.points, size: CGSize(width: size, height: size))
        case .gradient(let value):
            return GradientThumbnail.image(value: value, size: CGSize(width: size, height: size))
        }
    }
}

extension SequencerModel {
    func entries(_ kind: LibraryKind) -> [LibraryEntry] {
        switch kind {
        case .color:
            return palette.enumerated().map { LibraryEntry(id: "\($0.offset)", name: rgbTooltip($0.element), value: .color($0.element), argb: $0.element) }
        case .curve:
            return curves.map { LibraryEntry(id: $0.name, name: $0.name, value: .curve($0.curve), argb: nil) }
        case .gradient:
            return gradients.map { LibraryEntry(id: $0.name, name: $0.name, value: .gradient($0.gradient), argb: nil) }
        }
    }

    /// F168, F356: a library item's payload, linked or not.
    func dragValue(for entry: LibraryEntry, link: Bool) -> VixenDragValue {
        switch entry.value {
        case .curve(let value):
            let text = libraryCurveDragText(value: value, name: entry.name, link: link)
            return curveFromDragText(text: text).map(VixenDragValue.curve) ?? entry.value
        case .gradient(let value):
            let text = libraryGradientDragText(value: value, name: entry.name, link: link)
            return gradientFromDragText(text: text).map(VixenDragValue.gradient) ?? entry.value
        case .color:
            ColorPanelSession.shared.finish()
            // A retained cell can still hold the value from before the picker
            // committed. Its position identifies the colour being dragged.
            guard let index = Int(entry.id), palette.indices.contains(index) else { return entry.value }
            return .color(palette[index])
        }
    }

    func setPalette(_ colors: [UInt32]) {
        shell?.setPalette(colors: colors)
        librariesChanged()
    }

    /// F343: the system colour panel, from white.
    func newColor() {
        ColorPanelSession.shared.begin(title: "New Color", argb: 0xFFFF_FFFF, notifyUnchanged: true) { [weak self] argb in
            guard let self else { return }
            self.setPalette(self.palette + [argb])
        }
    }

    /// F344, F173.
    func editColor(at index: Int) {
        ColorPanelSession.shared.finish()
        guard palette.indices.contains(index) else { return }
        ColorPanelSession.shared.begin(title: "Edit Color", argb: palette[index]) { [weak self] argb in
            guard let self, index < self.palette.count else { return }
            var list = self.palette
            list[index] = argb
            self.setPalette(list)
        }
    }

    /// F354, F362: asks for a name, adds it, and opens the editor.
    /// `index`: where a panel drop puts it (F357). Nil leaves it last.
    func newCurve(edit: Bool = true, value: CurveValue = defaultCurveValue(), index: Int? = nil) {
        guard let window, let core = app.core else { return }
        CurveEditorDialog.askForName(on: window, core: core) { [weak self] name in
            guard let self, let name else { return }
            if let index {
                _ = core.libraryInsertCurve(name: name, curve: value, index: UInt32(index))
            } else {
                _ = core.libraryAddCurve(name: name, curve: value)
            }
            self.librariesChanged()
            if edit {
                self.editCurve(name)
            }
        }
    }

    func newGradient(edit: Bool = true, value: GradientValue = defaultGradientValue(), index: Int? = nil) {
        guard let window, let core = app.core else { return }
        GradientEditorDialog.askForName(on: window, core: core) { [weak self] name in
            guard let self, let name else { return }
            if let index {
                _ = core.libraryInsertGradient(name: name, gradient: value, index: UInt32(index))
            } else {
                _ = core.libraryAddGradient(name: name, gradient: value)
            }
            self.librariesChanged()
            if edit {
                self.editGradient(name)
            }
        }
    }

    func editCurve(_ name: String) {
        guard let window, let core = app.core else { return }
        CurveEditorDialog.editLibraryEntry(on: window, core: core, name: name) { [weak self] in self?.librariesChanged() }
    }

    func editGradient(_ name: String) {
        guard let window, let core = app.core else { return }
        GradientEditorDialog.editLibraryEntry(on: window, core: core, name: name) { [weak self] in self?.librariesChanged() }
    }

    func newItem(_ kind: LibraryKind) {
        switch kind {
        case .color: newColor()
        case .curve: newCurve()
        case .gradient: newGradient()
        }
    }

    func edit(_ entry: LibraryEntry, kind: LibraryKind) {
        switch kind {
        case .color:
            if let index = Int(entry.id) {
                editColor(at: index)
            }
        case .curve:
            editCurve(entry.name)
        case .gradient:
            editGradient(entry.name)
        }
    }

    /// F174 and the panels' Delete, after any confirmation.
    func remove(_ entries: [LibraryEntry], kind: LibraryKind) {
        switch kind {
        case .color:
            ColorPanelSession.shared.finish()
            // The pending edit can change a selected colour's value. Resolve
            // its stable position again before upstream's first-value removal.
            let removing = entries.compactMap { entry -> UInt32? in
                guard let index = Int(entry.id), palette.indices.contains(index) else { return nil }
                return palette[index]
            }
            setPalette(shell?.paletteWithout(colors: palette, removing: removing) ?? palette)
        case .curve:
            app.core?.libraryRemoveCurves(names: entries.map(\.name))
            librariesChanged()
        case .gradient:
            app.core?.libraryRemoveGradients(names: entries.map(\.name))
            librariesChanged()
        }
    }

    /// F169, F347, F357: one transaction, as the Mac note asks.
    func moveEntry(from: Int, to: Int, kind: LibraryKind) {
        guard from != to else { return }
        func moved<T>(_ list: [T]) -> [T] {
            var list = list
            guard from < list.count, to < list.count else { return list }
            let item = list.remove(at: from)
            list.insert(item, at: to)
            return list
        }
        switch kind {
        case .color:
            ColorPanelSession.shared.finish()
            setPalette(moved(palette))
        case .curve:
            app.core?.libraryReorderCurves(names: moved(curves.map(\.name)))
            librariesChanged()
        case .gradient:
            app.core?.libraryReorderGradients(names: moved(gradients.map(\.name)))
            librariesChanged()
        }
    }

    /// F347, F357, F362: the panel's Move branch. Every selected item goes to the target's place, or past the last with none under the pointer.
    func moveSelection(_ chosen: Set<String>, to target: LibraryEntry?, kind: LibraryKind) {
        switch kind {
        case .color:
            ColorPanelSession.shared.finish()
            let entries = entries(.color)
            let picked = entries.indices.filter { chosen.contains(entries[$0].id) }.map(UInt32.init)
            let index = target.flatMap { t in entries.firstIndex { $0.id == t.id } }.map(UInt32.init)
            guard !picked.isEmpty, let moved = shell?.paletteMoved(colors: palette, selected: picked, target: index) else { return }
            setPalette(moved)
        case .curve:
            let names = curves.map(\.name).filter(chosen.contains)
            guard !names.isEmpty else { return }
            app.core?.libraryMoveCurves(selected: names, target: target?.name)
            librariesChanged()
        case .gradient:
            let names = gradients.map(\.name).filter(chosen.contains)
            guard !names.isEmpty else { return }
            app.core?.libraryMoveGradients(selected: names, target: target?.name)
            librariesChanged()
        }
    }

    /// F170, F347, F357: a colour goes straight in; a curve or gradient only if it isn't a library reference, and asks for a name.
    func accepts(_ value: VixenDragValue, kind: LibraryKind) -> Bool {
        switch (value, kind) {
        case (.color, .color): true
        case (.curve(let c), .curve): c.libraryName.isEmpty
        case (.gradient(let g), .gradient): g.libraryName.isEmpty
        default: false
        }
    }

    /// `placed`: a panel puts a new curve or gradient at the pointer's item (F357); a toolbar strip adds it last (F170).
    func receive(_ value: VixenDragValue, at index: Int?, placed: Bool = false) {
        switch value {
        case .color(let argb):
            ColorPanelSession.shared.finish()
            var list = palette
            if let index, index < list.count {
                list.insert(argb, at: index)
            } else {
                list.append(argb)
            }
            setPalette(list)
        case .curve(let curve):
            newCurve(edit: false, value: curve, index: placed ? index : nil)
        case .gradient(let gradient):
            newGradient(edit: false, value: gradient, index: placed ? index : nil)
        }
    }

    func exportLibrary(_ kind: LibraryKind) {
        guard let shell else { return }
        // Upstream's gradient form ends its message with a period and the other two don't.
        let failed = "Unable to export data, please check the error log for details" + (kind == .gradient ? "." : "")
        do {
            switch kind {
            case .color:
                ColorPanelSession.shared.finish()
                guard let url = LibraryFilePanels.save(title: "Export Favorite Colors", name: "Favorite Colors", ext: "vfc", typeName: "Vixen 3 Favorite Colors") else { return }
                try shell.exportPalette(path: url.path, colors: palette)
            case .curve:
                guard let url = LibraryFilePanels.save(title: "Export Curve Library", name: "Curve Library", ext: "vcl", typeName: "Vixen 3 Curve Library") else { return }
                try shell.exportCurveLibrary(path: url.path)
            case .gradient:
                guard let url = LibraryFilePanels.save(title: "Export Gradient Library", name: "Gradient Library", ext: "vgl", typeName: "Vixen 3 Color Gradient Library") else { return }
                try shell.exportGradientLibrary(path: url.path)
            }
        } catch {
            FileHandle.standardError.write(Data("library export: \(error.userMessage)\n".utf8))
            Task { await SequencerAlerts.tell(title: "Unable to export", message: failed, style: .error, window: window) }
        }
    }

    func importLibrary(_ kind: LibraryKind) {
        guard let shell else { return }
        do {
            switch kind {
            case .color:
                ColorPanelSession.shared.finish()
                guard let url = LibraryFilePanels.open(title: "Import Favorite Colors", ext: "vfc", typeName: "Vixen 3 Favorite Colors") else { return }
                setPalette(palette + (try shell.importPalette(path: url.path)))
            case .curve:
                guard let url = LibraryFilePanels.open(title: "Import Curve Library", ext: "vcl", typeName: "Vixen 3 Curve Library") else { return }
                _ = try shell.importCurveLibrary(path: url.path)
                librariesChanged()
            case .gradient:
                guard let url = LibraryFilePanels.open(title: "Import Gradient Library", ext: "vgl", typeName: "Vixen 3 Color Gradient Library") else { return }
                _ = try shell.importGradientLibrary(path: url.path)
                librariesChanged()
            }
        } catch {
            // Upstream's wording, form by form: only the gradient one spells "recognize" right.
            let message: String
            switch kind {
            case .color: message = "Sorry, we didn't reconize the data in that file as valid Favorite Color data."
            case .curve: message = "Sorry, we didn't reconize the data in that file as valid Curve Library data."
            case .gradient: message = "Sorry, we didn't recognize the data in that file as valid Color Gradient Library data."
            }
            Task { await SequencerAlerts.tell(title: "Invalid file", message: message, style: .error, window: window) }
        }
    }
}

// MARK: One item

/// Bounded observation for native drag verification; inert in ordinary launches.
@MainActor
private enum LibraryDragTrace {
    private static var count = 0
    static func record(_ message: @autoclosure () -> String) {
        guard UITestHarness.isActive,
              ProcessInfo.processInfo.environment["VIXEN_LIBRARY_DRAG_TRACE"] == "1", count < 200 else { return }
        count += 1
        FileHandle.standardError.write(Data("library-drag \(message())\n".utf8))
    }

    static func installStateProbe(_ model: SequencerModel) {
        guard UITestHarness.isActive else { return }
        UITestHarness.hooks["sequencer.libraryDestinations"] = { [weak model] _ in
            guard let root = model?.window?.contentView else { return "FAIL no sequence window" }
            var rows: [[String: Any]] = []
            @MainActor func visit(_ view: NSView) {
                if let cell = view as? LibraryCellView, cell.tracker != nil {
                    rows.append(["role": "cell", "kind": String(describing: cell.kind), "index": cell.index,
                                 "hidden": cell.isHiddenOrHasHiddenAncestor, "registered": cell.registeredDraggedTypes.count])
                } else if let catcher = view as? LibraryDropCatcher.CatcherView, catcher.tracker != nil {
                    rows.append(["role": "empty", "kind": String(describing: catcher.kind),
                                 "hidden": catcher.isHiddenOrHasHiddenAncestor, "registered": catcher.registeredDraggedTypes.count])
                }
                for child in view.subviews { visit(child) }
            }
            visit(root)
            guard let data = try? JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys]),
                  let text = String(data: data, encoding: .utf8) else { return "FAIL cannot encode destinations" }
            return text
        }
    }
}

/// A thumbnail: presses, a double-click, a drag out, and drops from outside or from its own list. Acts on mouse down, as the strips do (F166).
final class LibraryCellView: ValueDragView {
    var kind = LibraryKind.color
    var index = 0
    var image: NSImage? {
        didSet { needsDisplay = true }
    }
    var title: String?
    var isHighlighted = false {
        didSet {
            if oldValue != isHighlighted {
                needsDisplay = true
            }
        }
    }
    var imageSide: CGFloat = 28
    var pressed: ((NSEvent) -> Void)?
    var released: ((NSEvent) -> Void)?
    var doubleClicked: (() -> Void)?
    var entered: ((Bool) -> Void)?
    var dragPayload: ((NSEvent) -> VixenDragValue?)?
    var moved: ((Int, Int) -> Void)?
    private var downPoint: NSPoint?
    private var pendingRelease: NSEvent?
    private var dragging = false
    private var tracking: NSTrackingArea?

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking {
            removeTrackingArea(tracking)
        }
        let area = NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect], owner: self)
        addTrackingArea(area)
        tracking = area
    }

    override func mouseEntered(with event: NSEvent) {
        entered?(true)
    }

    override func mouseExited(with event: NSEvent) {
        entered?(false)
    }

    override func mouseDown(with event: NSEvent) {
        LibraryDragTrace.record("down kind=\(kind) index=\(index) panel=\(tracker != nil) point=\(event.locationInWindow) modifiers=\(event.modifierFlags.rawValue)")
        downPoint = event.locationInWindow
        dragging = false
        pendingRelease = nil
        if event.clickCount == 2 {
            doubleClicked?()
            return
        }
        pendingRelease = event
        pressed?(event)
    }

    override func mouseUp(with event: NSEvent) {
        LibraryDragTrace.record("up kind=\(kind) index=\(index) dragging=\(dragging) pending=\(pendingRelease != nil)")
        defer {
            pendingRelease = nil
            downPoint = nil
        }
        if !dragging, let pendingRelease {
            released?(pendingRelease)
        }
    }

    override func mouseDragged(with event: NSEvent) {
        LibraryDragTrace.record("dragged kind=\(kind) index=\(index) point=\(event.locationInWindow) down=\(String(describing: downPoint)) dragging=\(dragging)")
        guard let downPoint, let start = pendingRelease, !dragging else { return }
        let distance = hypot(event.locationInWindow.x - downPoint.x, event.locationInWindow.y - downPoint.y)
        guard distance > 3 else { return }
        guard let value = dragPayload?(event) else {
            LibraryDragTrace.record("no-payload kind=\(kind) index=\(index)")
            return
        }
        LibraryDragTrace.record("session-start kind=\(kind) index=\(index) distance=\(distance)")
        dragging = true
        pendingRelease = nil
        startValueDrag(value, event: start)
        LibraryDragTrace.record("session-return kind=\(kind) index=\(index)")
    }

    override func dragImage() -> NSImage? {
        image
    }

    func draggingSession(_ session: NSDraggingSession, willBeginAt screenPoint: NSPoint) {
        LibraryDragTrace.record("session-began kind=\(kind) index=\(index) point=\(screenPoint)")
    }

    func draggingSession(_ session: NSDraggingSession, endedAt screenPoint: NSPoint, operation: NSDragOperation) {
        LibraryDragTrace.record("session-ended kind=\(kind) index=\(index) operation=\(operation.rawValue) point=\(screenPoint)")
    }

    private func isOwnList(_ sender: any NSDraggingInfo) -> LibraryCellView? {
        guard let source = sender.draggingSource as? LibraryCellView, source.kind == kind else { return nil }
        // A panel's own cells share a tracker; toolbar cells both have none.
        // A different sequencer or panel/toolbar crossing copies instead. A
        // floating strip keeps its model identity even in another NSWindow.
        guard source.tracker === tracker, source.libraryModel === libraryModel else { return nil }
        return source
    }

    override func draggingEntered(_ sender: any NSDraggingInfo) -> NSDragOperation {
        guard !isHiddenOrHasHiddenAncestor else { return [] }
        LibraryDragTrace.record("cell-enter kind=\(kind) index=\(index) own=\(isOwnList(sender) != nil) location=\(sender.draggingLocation)")
        tracker?.entered(sender)
        return isOwnList(sender) != nil ? .copy : super.draggingEntered(sender)
    }

    override func draggingUpdated(_ sender: any NSDraggingInfo) -> NSDragOperation {
        guard !isHiddenOrHasHiddenAncestor else { return [] }
        return isOwnList(sender) != nil ? .copy : super.draggingUpdated(sender)
    }

    override func draggingExited(_ sender: (any NSDraggingInfo)?) {
        if let sender { tracker?.exited(sender) }
        super.draggingExited(sender)
    }

    override func performDragOperation(_ sender: any NSDraggingInfo) -> Bool {
        guard !isHiddenOrHasHiddenAncestor else { return false }
        LibraryDragTrace.record("cell-drop kind=\(kind) index=\(index) own=\(isOwnList(sender) != nil) farEnough=\(tracker?.farEnough(sender) ?? true) location=\(sender.draggingLocation)")
        guard tracker?.farEnough(sender) ?? true else { return false }
        if let source = isOwnList(sender) {
            moved?(source.index, index)
            return true
        }
        return super.performDragOperation(sender)
    }

    /// A panel's cells share one; a toolbar strip has none.
    var tracker: LibraryDropTracker?
    weak var libraryModel: SequencerModel?

    override func draw(_ dirtyRect: NSRect) {
        let rect = NSRect(x: (bounds.width - imageSide) / 2, y: bounds.height - imageSide - 1, width: imageSide, height: imageSide)
        image?.draw(in: rect)
        let border = NSBezierPath(rect: rect.insetBy(dx: 0.5, dy: 0.5))
        border.lineWidth = isHighlighted ? 2 : 1
        (isHighlighted ? NSColor.controlAccentColor : NSColor(srgbRed: 136 / 255, green: 136 / 255, blue: 136 / 255, alpha: 1)).setStroke()
        border.stroke()
        if let title {
            // Upstream's large-icon list wraps a name onto a second line before it gives up and truncates.
            let style = NSMutableParagraphStyle()
            style.alignment = .center
            style.lineBreakMode = .byWordWrapping
            let font = NSFont.systemFont(ofSize: max(7, 10 * textScale))
            let lines = ceil(NSLayoutManager().defaultLineHeight(for: font)) * 2
            (title as NSString).draw(
                with: NSRect(x: 0, y: max(0, rect.minY - 2 - lines), width: bounds.width, height: min(lines, rect.minY - 2)),
                options: [.usesLineFragmentOrigin, .truncatesLastVisibleLine],
                attributes: [.font: font, .foregroundColor: NSColor.secondaryLabelColor, .paragraphStyle: style]
            )
        }
    }

    var textScale: CGFloat = 1
}

struct LibraryCell: NSViewRepresentable {
    @Environment(\.panelIsShowing) private var showing
    let model: SequencerModel
    let kind: LibraryKind
    let entry: LibraryEntry
    let index: Int
    let side: CGFloat
    var title: String?
    var textScale: CGFloat = 1
    let highlighted: Bool
    let image: NSImage
    let pressed: (NSEvent) -> Void
    var released: ((NSEvent) -> Void)?
    let doubleClicked: () -> Void
    var entered: ((Bool) -> Void)?
    let payload: (NSEvent) -> VixenDragValue?
    /// Set in a panel: drops follow the list's rules (10 pt, the whole selection moves, a new item lands at the pointer) rather than a strip's.
    var tracker: LibraryDropTracker?
    var moveSelection: (() -> Void)?

    func makeNSView(context: Context) -> LibraryCellView {
        LibraryCellView(frame: NSRect(x: 0, y: 0, width: side, height: side))
    }

    func updateNSView(_ view: LibraryCellView, context: Context) {
        view.setPanelVisible(showing)
        view.kind = kind
        view.index = index
        view.imageSide = side
        view.title = title
        view.textScale = textScale
        view.image = image
        view.isHighlighted = highlighted
        view.toolTip = entry.name
        view.setAccessibilityLabel(entry.name)
        view.pressed = pressed
        view.released = released
        view.doubleClicked = doubleClicked
        view.entered = entered
        view.dragPayload = payload
        view.tracker = tracker
        view.libraryModel = model
        if let moveSelection {
            view.moved = { _, _ in moveSelection() }
        } else {
            view.moved = { [weak model] from, to in model?.moveEntry(from: from, to: to, kind: kind) }
        }
        let placed = tracker != nil
        view.acceptsDrop = { [weak model] value in model?.accepts(value, kind: kind) ?? false }
        view.performDrop = { [weak model] value in model?.receive(value, at: index, placed: placed) }
    }
}

/// F347, F357, F362: a panel ignores a drop that ends within 10 pt of where the drag came in, on both axes, so a click that wobbles moves nothing.
@MainActor
final class LibraryDropTracker {
    private var sequence = -1
    private var entry = NSPoint.zero
    /// The scroll viewport, shared by every cell and its empty-space catcher.
    weak var boundaryView: NSView?

    func entered(_ sender: any NSDraggingInfo) {
        guard sender.draggingSequenceNumber != sequence else { return }
        sequence = sender.draggingSequenceNumber
        entry = sender.draggingLocation
    }

    func exited(_ sender: any NSDraggingInfo) {
        guard sender.draggingSequenceNumber == sequence, let boundaryView else { return }
        let point = boundaryView.convert(sender.draggingLocation, from: nil)
        // AppKit exits each child destination while crossing the list. Upstream
        // resets only when its whole ListView is entered again.
        let viewport = boundaryView.bounds.intersection(boundaryView.visibleRect)
        guard !viewport.contains(point) else { return }
        sequence = -1
    }

    func farEnough(_ sender: any NSDraggingInfo) -> Bool {
        guard sender.draggingSequenceNumber == sequence else { return true }
        let at = sender.draggingLocation
        return abs(at.x - entry.x) > 10 || abs(at.y - entry.y) > 10
    }
}

/// The empty part of a strip or panel: a drop there goes at the end.
struct LibraryDropCatcher: NSViewRepresentable {
    @Environment(\.panelIsShowing) private var showing
    let model: SequencerModel
    let kind: LibraryKind

    final class CatcherView: ValueDragView {
        var clicked: (() -> Void)?
        var kind = LibraryKind.color
        var tracker: LibraryDropTracker?
        var moveToEnd: (() -> Void)?

        override func mouseDown(with event: NSEvent) {
            clicked?()
        }

        private func isOwnList(_ sender: any NSDraggingInfo) -> Bool {
            guard moveToEnd != nil, let source = sender.draggingSource as? LibraryCellView,
                  source.kind == kind else { return false }
            if let tracker { return source.tracker === tracker }
            return true
        }

        override func draggingEntered(_ sender: any NSDraggingInfo) -> NSDragOperation {
            guard !isHiddenOrHasHiddenAncestor else { return [] }
            LibraryDragTrace.record("empty-enter kind=\(kind) own=\(isOwnList(sender)) location=\(sender.draggingLocation)")
            tracker?.entered(sender)
            return isOwnList(sender) ? .copy : super.draggingEntered(sender)
        }

        override func draggingUpdated(_ sender: any NSDraggingInfo) -> NSDragOperation {
            guard !isHiddenOrHasHiddenAncestor else { return [] }
            return isOwnList(sender) ? .copy : super.draggingUpdated(sender)
        }

        override func draggingExited(_ sender: (any NSDraggingInfo)?) {
            if let sender { tracker?.exited(sender) }
            super.draggingExited(sender)
        }

        override func performDragOperation(_ sender: any NSDraggingInfo) -> Bool {
            guard !isHiddenOrHasHiddenAncestor else { return false }
            LibraryDragTrace.record("empty-drop kind=\(kind) own=\(isOwnList(sender)) farEnough=\(tracker?.farEnough(sender) ?? true) location=\(sender.draggingLocation)")
            guard tracker?.farEnough(sender) ?? true else { return false }
            if isOwnList(sender) {
                moveToEnd?()
                return true
            }
            return super.performDragOperation(sender)
        }
    }

    var clicked: (() -> Void)?
    var tracker: LibraryDropTracker?
    /// A panel's own item dropped on the empty part goes past the last.
    var moveToEnd: (() -> Void)?

    func makeNSView(context: Context) -> CatcherView {
        CatcherView(frame: .zero)
    }

    func updateNSView(_ view: CatcherView, context: Context) {
        view.setPanelVisible(showing)
        view.clicked = clicked
        view.kind = kind
        view.tracker = tracker
        view.moveToEnd = moveToEnd
        view.acceptsDrop = { [weak model] value in model?.accepts(value, kind: kind) ?? false }
        view.performDrop = { [weak model] value in model?.receive(value, at: nil) }
    }
}

// MARK: The strips, F164 to F180

struct LibraryStrip: View {
    let context: StripContext
    let kind: LibraryKind
    @State private var checked: Set<String> = []
    @State private var hovered: String?

    var body: some View {
        let model = context.model
        let size = context.imageSize
        StripItems(context) {
            ForEach(Array(model.entries(kind).enumerated()), id: \.element.id) { index, entry in
                if context.shows(entry.name) {
                    cell(model, entry, index, size)
                        .frame(width: size, height: size)
                        .contextMenu { menu(model, entry) }
                        // A menu item can't be dragged or hovered, so in the drop-down an item opens its editor, as a double-click does.
                        .stripEntry(StripEntry(title: entry.name, image: .drawn { LibraryImages.image(entry, size: 16) }, action: { model.edit(entry, kind: kind) }))
                }
            }
            LibraryDropCatcher(model: model, kind: kind)
                .frame(width: max(size / 2, 8), height: size)
                .contextMenu { menu(model, nil) }
        }
        .onHover { inside in
            if !inside {
                checked = []
                hovered = nil
            }
        }
    }

    private func cell(_ model: SequencerModel, _ entry: LibraryEntry, _ index: Int, _ size: CGFloat) -> some View {
        LibraryCell(
            model: model,
            kind: kind,
            entry: entry,
            index: index,
            side: size,
            highlighted: checked.contains(entry.id) || hovered == entry.id,
            image: LibraryImages.image(entry, size: size),
            pressed: { event in press(entry, event) },
            doubleClicked: { model.edit(entry, kind: kind) },
            entered: { inside in hovered = inside ? entry.id : nil },
            payload: { event in model.dragValue(for: entry, link: event.modifierFlags.contains(.command)) }
        )
    }

    /// F166: exclusive, and ⌘ toggles this one and leaves the others.
    private func press(_ entry: LibraryEntry, _ event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            if checked.contains(entry.id) {
                checked.remove(entry.id)
            } else {
                checked.insert(entry.id)
            }
        } else {
            checked = [entry.id]
        }
    }

    /// F171 to F180.
    @ViewBuilder
    private func menu(_ model: SequencerModel, _ entry: LibraryEntry?) -> some View {
        Button("New") { model.newItem(kind) }
        Button("Edit") {
            if let entry {
                model.edit(entry, kind: kind)
            }
        }
        .disabled(entry == nil)
        Button("Delete") {
            // No confirmation here, unlike the panels. Hovering checks a button, so the one under the pointer goes too (F174).
            var ids = checked
            if let entry {
                ids.insert(entry.id)
            }
            model.remove(model.entries(kind).filter { ids.contains($0.id) }, kind: kind)
            checked = []
        }
        .disabled(entry == nil && checked.isEmpty)
        Divider()
        Button("Import") { model.importLibrary(kind) }
        Button("Export") { model.exportLibrary(kind) }
        Divider()
        Button("Show/Hide Items...", action: context.showItems)
        Menu("Toolbars") {
            ToolbarsList(model: model)
        }
        Button("Toggle Image Size") { model.toggleStripSize(kind.strip) }
        Button("Reset Toolbar") { model.resetStrip(kind.strip) }
    }
}

// MARK: The panels, F341 to F363

struct LibraryPanel: View {
    let model: SequencerModel
    let kind: LibraryKind
    @State private var selected: Set<String> = []
    @State private var imageScale: Double?
    @State private var textScale: Double?
    @State private var saveTask: Task<Void, Never>?
    @State private var tracker = LibraryDropTracker()
    @Environment(\.panelIsShowing) private var showing

    private var words: (edit: String, new: String, delete: String, export: String, import: String) {
        switch kind {
        case .color: ("Edit Color", "New Color", "Delete Color", "Export Favorite Colors", "Import Favorite Colors")
        case .curve: ("Edit Curve", "New Curve", "Delete Curve", "Export Curve Library", "Import Curve Library")
        case .gradient: ("Edit Gradient", "New Gradient", "Delete Gradient", "Export Gradient Library", "Import Gradient Library")
        }
    }

    private var scaleKeys: (image: String, text: String?) {
        switch kind {
        case .color: ("Form_ColorLibrary/ColorLibraryScale", nil)
        case .curve: ("Form_CurveLibrary/CurveLibraryImageScale", "Form_CurveLibrary/CurveLibraryTextScale")
        case .gradient: ("Form_GradientLibrary/GradientLibraryImageScale", "Form_GradientLibrary/GradientLibraryTextScale")
        }
    }

    private var link: Bool {
        switch kind {
        case .color: false
        case .curve: model.ui?.prefs.curveLinkCurves ?? false
        case .gradient: model.ui?.prefs.gradientLinkGradients ?? false
        }
    }

    var body: some View {
        let entries = model.entries(kind)
        let chosen = entries.filter { selected.contains($0.id) }
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                iconButton(words.edit, "slider.horizontal.3", enabled: chosen.count == 1) {
                    if let first = chosen.first {
                        model.edit(first, kind: kind)
                    }
                }
                iconButton(words.new, "plus", enabled: true) { model.newItem(kind) }
                iconButton(words.delete, "trash", enabled: !chosen.isEmpty) { confirmDelete(chosen) }
                Divider().frame(height: 16)
                iconButton(words.export, "square.and.arrow.up", enabled: true) { model.exportLibrary(kind) }
                iconButton(words.import, "square.and.arrow.down", enabled: true) { model.importLibrary(kind) }
                Spacer()
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            if kind != .color {
                Toggle("Maintain Library Link", isOn: Binding(get: { link }, set: { model.set(kind == .curve ? .curveLinkCurves : .gradientLinkGradients, $0) }))
                    .controlSize(.small)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 8)
                    .padding(.bottom, 4)
            }
            Divider()
            grid(entries)
        }
        .onAppear {
            loadScale()
            LibraryDragTrace.installStateProbe(model)
        }
    }

    private func iconButton(_ help: String, _ symbol: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol)
        }
        .buttonStyle(.accessoryBar)
        .help(help)
        .accessibilityLabel(help)
        .disabled(!enabled)
    }

    private func grid(_ entries: [LibraryEntry]) -> some View {
        let side = 48 * (imageScale ?? 1)
        let text = textScale ?? 1
        let height = kind == .color ? side : side + 30 * text
        return GeometryReader { geometry in
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: side + 10), spacing: 4, alignment: .top)], spacing: 6) {
                    ForEach(Array(entries.enumerated()), id: \.element.id) { index, entry in
                        LibraryCell(
                            model: model,
                            kind: kind,
                            entry: entry,
                            index: index,
                            side: side,
                            title: kind == .color ? nil : entry.name,
                            textScale: text,
                            highlighted: selected.contains(entry.id),
                            image: LibraryImages.image(entry, size: side),
                            pressed: { event in press(entry, event) },
                            released: { event in
                                if !event.modifierFlags.contains(.command) { selected = [entry.id] }
                            },
                            doubleClicked: {
                                if selected.count == 1, let first = entries.first(where: { selected.contains($0.id) }) {
                                    model.edit(first, kind: kind)
                                }
                            },
                            payload: { _ in
                                // Upstream drags the first selected item, in list order.
                                entries.first { selected.contains($0.id) }.map { model.dragValue(for: $0, link: link) }
                            },
                            tracker: tracker,
                            moveSelection: { model.moveSelection(selected, to: entry, kind: kind) }
                        )
                        .frame(width: side + 10, height: height)
                    }
                }
                .padding(6)
                .frame(minHeight: geometry.size.height, alignment: .top)
                .background(
                    LibraryDropCatcher(
                        model: model,
                        kind: kind,
                        clicked: { selected = [] },
                        tracker: tracker,
                        moveToEnd: { model.moveSelection(selected, to: nil, kind: kind) }
                    )
                )
            }
            .background(ScaleCatcher(active: showing, tracker: tracker) { up, shift in scale(up: up, text: shift) })
        }
    }

    private func press(_ entry: LibraryEntry, _ event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            if selected.contains(entry.id) {
                selected.remove(entry.id)
            } else {
                selected.insert(entry.id)
            }
        } else if !selected.contains(entry.id) {
            selected = [entry.id]
        }
        // Keep an existing selection until mouse-up so a drag moves all of it.
    }

    private func confirmDelete(_ chosen: [LibraryEntry]) {
        let title: String
        let message: String
        switch kind {
        case .color:
            title = "Delete color?"
            message = "Are you sure you want to delete the selected color(s)?"
        case .curve:
            title = "Delete library curve?"
            message = "If you delete the Selected library curve(s), ALL places it is used will be unlinked and will become independent curves. Are you sure you want to continue?"
        case .gradient:
            title = "Delete library gradient?"
            message = "If you delete the selected library gradient(s), ALL places it is used will be unlinked and will become independent gradients. Are you sure you want to continue?"
        }
        Task {
            if await SequencerAlerts.confirm(title: title, message: message, style: .warning, confirm: "Delete", cancel: "Cancel", window: model.window, destructive: true) {
                model.remove(chosen, kind: kind)
                selected = []
            }
        }
    }

    private func loadScale() {
        guard let shell = model.shell, imageScale == nil else { return }
        imageScale = max(0.1, shell.panelScale(key: scaleKeys.image))
        if let key = scaleKeys.text {
            textScale = max(0.2, shell.panelScale(key: key))
        }
    }

    /// F348, F358: ⌘-scroll, by 3 percent a step. With ⇧ in a curve or gradient panel it's the text.
    private func scale(up: Bool, text: Bool) {
        let factor = up ? 1.03 : 1 / 1.03
        if text, kind != .color {
            textScale = max(0.2, (textScale ?? 1) * factor)
        } else {
            imageScale = max(0.1, (imageScale ?? 1) * factor)
        }
        saveTask?.cancel()
        saveTask = Task {
            try? await Task.sleep(for: .seconds(1))
            guard !Task.isCancelled, let shell = model.shell else { return }
            shell.setPanelScale(key: scaleKeys.image, value: imageScale ?? 1)
            if let key = scaleKeys.text {
                shell.setPanelScale(key: key, value: textScale ?? 1)
            }
        }
    }
}

/// ⌘-scroll and pinch.
private struct ScaleCatcher: NSViewRepresentable {
    /// False for a tab kept alive behind the one showing.
    let active: Bool
    let tracker: LibraryDropTracker
    let changed: (_ up: Bool, _ shift: Bool) -> Void

    /// It sits behind the scroll view, which would take the wheel first, so it listens to the window's events over its own area.
    final class CatcherView: NSView {
        var changed: ((Bool, Bool) -> Void)?
        var active = true
        private var monitor: Any?

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            if let monitor {
                NSEvent.removeMonitor(monitor)
                self.monitor = nil
            }
            guard window != nil else { return }
            monitor = NSEvent.addLocalMonitorForEvents(matching: [.scrollWheel, .magnify]) { [weak self] event in
                self?.take(event) == true ? nil : event
            }
        }

        private func take(_ event: NSEvent) -> Bool {
            // An event a script builds has no window, and its place is on the screen.
            guard active, let window, event.window === window || event.window == nil, !isHiddenOrHasHiddenAncestor else { return false }
            let place = event.window == nil ? window.convertPoint(fromScreen: event.locationInWindow) : event.locationInWindow
            guard bounds.contains(convert(place, from: nil)) else { return false }
            let shift = event.modifierFlags.contains(.shift)
            if event.type == .magnify {
                if event.magnification != 0 {
                    changed?(event.magnification > 0, shift)
                }
                return true
            }
            guard event.modifierFlags.contains(.command) else { return false }
            // With ⇧ held a wheel's travel arrives as sideways.
            let travel = event.scrollingDeltaY != 0 ? event.scrollingDeltaY : (shift ? event.scrollingDeltaX : 0)
            if travel != 0 {
                changed?(travel > 0, shift)
            }
            return true
        }

        override func hitTest(_ point: NSPoint) -> NSView? {
            nil
        }
    }

    func makeNSView(context: Context) -> CatcherView {
        let view = CatcherView()
        view.changed = changed
        view.active = active
        tracker.boundaryView = view
        return view
    }

    func updateNSView(_ view: CatcherView, context: Context) {
        view.changed = changed
        view.active = active
        tracker.boundaryView = view
    }
}

