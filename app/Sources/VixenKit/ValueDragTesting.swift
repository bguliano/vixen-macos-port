import AppKit

/// The test harness can't run a drag session, which the window server drives. This does everything around one: what the view under one point would drag, on a pasteboard, offered to the
/// view under another point through its `NSDraggingDestination` methods. `{ "action": "hook", "name": "valueDrag", "path": ["x1", "y1", "x2", "y2"] }`, in content coordinates from the
/// top left as the `mouse` step uses, answers what was dragged and whether the drop took it.
@MainActor
public enum ValueDragTesting {
    public static func register() {
        guard UITestHarness.isActive, UITestHarness.hooks["valueDrag"] == nil else { return }
        UITestHarness.hooks["valueDrag"] = { path in
            let numbers = path.compactMap(Double.init)
            guard numbers.count == 4 else { return "FAIL: valueDrag needs x1, y1, x2, y2" }
            guard let window = NSApp.orderedWindows.first(where: { $0.isVisible && !($0 is NSPanel) && $0.attachedSheet == nil }) else { return "FAIL: no window" }
            guard let source = view(in: window, x: numbers[0], y: numbers[1]) else { return "FAIL: nothing to drag from there" }
            guard let value = source.dragValue else { return "nothing to drag" }
            guard let target = view(in: window, x: numbers[2], y: numbers[3]) else { return "FAIL: nothing to drop on there" }
            let pasteboard = NSPasteboard(name: NSPasteboard.Name("com.vixen.test-drag.\(UUID().uuidString)"))
            pasteboard.clearContents()
            pasteboard.writeObjects([value.pasteboardItem()])
            defer { pasteboard.releaseGlobally() }
            let location = point(in: window, x: numbers[2], y: numbers[3])
            let info = TestDraggingInfo(pasteboard: pasteboard, location: location, window: window)
            let kind = describe(value)
            guard target.draggingEntered(info).contains(.copy) else { return "\(kind) refused" }
            return target.performDragOperation(info) ? "\(kind) dropped" : "FAIL: \(kind) accepted, then not taken"
        }
    }

    private static func describe(_ value: VixenDragValue) -> String {
        switch value {
        case .curve: "curve"
        case .gradient: "gradient"
        case let .color(argb): String(format: "color %08X", argb)
        }
    }

    private static func point(in window: NSWindow, x: Double, y: Double) -> NSPoint {
        let height = window.contentView?.bounds.height ?? 0
        return NSPoint(x: x, y: height - y)
    }

    private static func view(in window: NSWindow, x: Double, y: Double) -> ValueDragView? {
        guard let content = window.contentView else { return nil }
        let root = content.superview ?? content
        var hit = root.hitTest(root.convert(point(in: window, x: x, y: y), from: nil))
        while let current = hit, !(current is ValueDragView) {
            hit = current.superview
        }
        return hit as? ValueDragView
    }
}

/// The parts of a drag a destination reads.
@MainActor
private final class TestDraggingInfo: NSObject, NSDraggingInfo {
    let draggingPasteboard: NSPasteboard
    let draggingLocation: NSPoint
    let draggingDestinationWindow: NSWindow?
    var draggingFormation: NSDraggingFormation = .default
    var animatesToDestination = false
    var numberOfValidItemsForDrop = 1

    init(pasteboard: NSPasteboard, location: NSPoint, window: NSWindow) {
        draggingPasteboard = pasteboard
        draggingLocation = location
        draggingDestinationWindow = window
    }

    var draggingSourceOperationMask: NSDragOperation { .copy }
    var draggedImageLocation: NSPoint { draggingLocation }
    var draggedImage: NSImage? { nil }
    var draggingSource: Any? { nil }
    var draggingSequenceNumber: Int { 1 }
    var springLoadingHighlight: NSSpringLoadingHighlight { .none }

    func slideDraggedImage(to screenPoint: NSPoint) {}

    func enumerateDraggingItems(
        options enumOpts: NSDraggingItemEnumerationOptions,
        for view: NSView?,
        classes classArray: [AnyClass],
        searchOptions: [NSPasteboard.ReadingOptionKey: Any],
        using block: (NSDraggingItem, Int, UnsafeMutablePointer<ObjCBool>) -> Void
    ) {}

    func resetSpringLoading() {}
}
