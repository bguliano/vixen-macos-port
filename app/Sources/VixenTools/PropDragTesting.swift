import AppKit
import VixenKit

// The window server owns real drag sessions. This harness sends the table's
// actual pasteboard payload through its destination callbacks without claiming
// to verify the offscreen window-server routing.
@MainActor
final class PropTestDraggingInfo: NSObject, NSDraggingInfo {
    let draggingPasteboard: NSPasteboard
    let draggingDestinationWindow: NSWindow?
    let draggingSource: Any?
    let draggingLocation = NSPoint.zero
    var draggingFormation: NSDraggingFormation = .default
    var animatesToDestination = false
    var numberOfValidItemsForDrop = 1
    init(pasteboard: NSPasteboard, table: NSTableView, foreign: Bool) {
        draggingPasteboard = pasteboard; draggingDestinationWindow = table.window
        draggingSource = foreign ? NSView() : table
    }
    var draggingSourceOperationMask: NSDragOperation { .move }
    var draggedImageLocation: NSPoint { .zero }
    var draggedImage: NSImage? { nil }
    var draggingSequenceNumber: Int { 1 }
    var springLoadingHighlight: NSSpringLoadingHighlight { .none }
    func slideDraggedImage(to screenPoint: NSPoint) {}
    func enumerateDraggingItems(options: NSDraggingItemEnumerationOptions, for view: NSView?, classes: [AnyClass], searchOptions: [NSPasteboard.ReadingOptionKey: Any], using block: (NSDraggingItem, Int, UnsafeMutablePointer<ObjCBool>) -> Void) {}
    func resetSpringLoading() {}
}
