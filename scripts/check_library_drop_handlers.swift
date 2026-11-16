// Invokes the real compiled destination handlers with supplied dragging metadata.
// The NSDraggingInfo probe has no copied threshold, acceptance, or move logic.
// This is not evidence of WindowServer routing; foreground tests cover that path.
import AppKit
import Foundation
import SwiftUI
@testable import VixenSequencer
import VixenKit
import VixenBridge

@MainActor
final class DragInfo: NSObject, NSDraggingInfo {
    var draggingDestinationWindow: NSWindow?
    var draggingSourceOperationMask: NSDragOperation = [.copy, .move]
    var draggingLocation = NSPoint.zero
    var draggedImageLocation = NSPoint.zero
    nonisolated var draggedImage: NSImage? { nil }
    let draggingPasteboard = NSPasteboard.withUniqueName()
    var draggingSource: Any?
    var draggingSequenceNumber = 1
    var draggingFormation: NSDraggingFormation = .none
    var animatesToDestination = false
    var numberOfValidItemsForDrop = 1
    var springLoadingHighlight: NSSpringLoadingHighlight { .none }
    func slideDraggedImage(to screenPoint: NSPoint) {}
    nonisolated override func namesOfPromisedFilesDropped(atDestination dropDestination: URL) -> [String]? { nil }
    func resetSpringLoading() {}
    func enumerateDraggingItems(options enumOpts: NSDraggingItemEnumerationOptions,
                                for view: NSView?, classes classArray: [AnyClass],
                                searchOptions: [NSPasteboard.ReadingOptionKey: Any],
                                using block: (NSDraggingItem, Int, UnsafeMutablePointer<ObjCBool>) -> Void) {}
}

@main
struct CheckLibraryDropHandlers {
    @MainActor static func main() throws {
        guard CommandLine.arguments.count == 2 else { fatalError("private result path required") }
        let output = URL(fileURLWithPath: CommandLine.arguments[1])
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.prohibited)
        let previous = NSWorkspace.shared.frontmostApplication?.processIdentifier
        var checks: [[String: Any]] = []
        var sequence = 0
        var pasteboards: [NSPasteboard] = []
        defer { pasteboards.forEach { $0.releaseGlobally() } }

        func check(_ name: String, actual: Bool, expected: Bool, evidence: [String: Any] = [:]) {
            checks.append(["name": name, "passed": actual == expected, "actual": actual,
                           "expected": expected, "evidence": evidence])
        }
        func cell(_ tracker: LibraryDropTracker?) -> LibraryCellView {
            let value = LibraryCellView(frame: NSRect(x: 80, y: 180, width: 58, height: 78))
            value.kind = .curve
            value.tracker = tracker
            return value
        }
        func info(_ source: LibraryCellView) -> DragInfo {
            sequence += 1
            let value = DragInfo()
            pasteboards.append(value.draggingPasteboard)
            value.draggingSource = source
            value.draggingSequenceNumber = sequence
            return value
        }
        func viewport() -> (NSWindow, NSView) {
            let window = NSWindow(contentRect: NSRect(x: 10000, y: 10000, width: 800, height: 800),
                                  styleMask: [.titled], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            let panel = NSView(frame: NSRect(x: 50, y: 60, width: 260, height: 600))
            window.contentView!.addSubview(panel)
            return (window, panel) // Never ordered, activated, or made key.
        }

        // Fixed expectations are from the original form's strict per-axis >10 test.
        let boundaries: [(String, CGFloat, CGFloat, Bool)] = [
            ("zero", 0, 0, false), ("below-positive-x", 9.999, 0, false),
            ("positive-x-exactly-ten", 10, 0, false), ("negative-x-exactly-ten", -10, 0, false),
            ("positive-y-exactly-ten", 0, 10, false), ("negative-y-exactly-ten", 0, -10, false),
            ("diagonal-ten-ten", 10, 10, false), ("diagonal-negative-ten", -10, -10, false),
            ("positive-x-above-ten", 10.001, 0, true), ("negative-x-above-ten", -10.001, 0, true),
            ("positive-y-above-ten", 0, 10.001, true), ("negative-y-above-ten", 0, -10.001, true),
        ]
        for empty in [false, true] {
            for (name, x, y, expected) in boundaries {
                let tracker = LibraryDropTracker()
                let source = cell(tracker)
                source.index = 3
                let sender = info(source)
                sender.draggingLocation = NSPoint(x: 100, y: 200)
                var invoked = 0
                let destination: NSView
                if empty {
                    let catcher = LibraryDropCatcher.CatcherView(frame: NSRect(x: 0, y: 0, width: 260, height: 600))
                    catcher.kind = .curve; catcher.tracker = tracker
                    catcher.moveToEnd = { invoked += 1 }
                    destination = catcher
                } else {
                    let target = cell(tracker); target.index = 1
                    target.moved = { from, to in
                        precondition(from == 3 && to == 1)
                        invoked += 1
                    }
                    destination = target
                }
                let accepted = destination.draggingEntered(sender)
                sender.draggingLocation = NSPoint(x: 100 + x, y: 200 + y)
                let result = destination.performDragOperation(sender)
                check("\(empty ? "empty" : "cell")-\(name)", actual: result && invoked == 1,
                      expected: expected, evidence: ["delta": [x, y], "callbacks": invoked,
                                                     "acceptedOperation": accepted.rawValue, "handlerReturned": result])
                check("\(empty ? "empty" : "cell")-\(name)-callback-count", actual: invoked == (expected ? 1 : 0), expected: true)
            }
        }

        do {
            let tracker = LibraryDropTracker(), source = cell(nil)
            let sender = info(source), target = cell(tracker)
            var copied = 0, moved = 0
            _ = sender.draggingPasteboard.writeObjects([VixenDragValue.curve(defaultCurveValue()).pasteboardItem()])
            target.acceptsDrop = { if case .curve = $0 { return true }; return false }
            target.performDrop = { _ in copied += 1 }
            target.moved = { _, _ in moved += 1 }
            _ = target.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 11, y: 0)
            check("toolbar-value-uses-copy-callback", actual: target.performDragOperation(sender) && copied == 1 && moved == 0,
                  expected: true)
        }

        // Use the production model's acceptance callback, without opening a
        // core, profile, document, or window. Panel values are external to a
        // toolbar: unlinked values copy; linked curves/gradients are rejected.
        // A toolbar's own drag must still reorder even when its payload links.
        do {
            let model = SequencerModel(app: AppModel(), ref: .new(), registry: SequencerRegistry())
            var linkedCurve = defaultCurveValue()
            linkedCurve.libraryName = "Existing curve"
            var linkedGradient = defaultGradientValue()
            linkedGradient.libraryName = "Existing gradient"
            let values: [(String, LibraryKind, VixenDragValue, Bool)] = [
                ("color", .color, .color(0xff00ff00), true),
                ("curve", .curve, .curve(defaultCurveValue()), true),
                ("gradient", .gradient, .gradient(defaultGradientValue()), true),
                ("linked-curve", .curve, .curve(linkedCurve), false),
                ("linked-gradient", .gradient, .gradient(linkedGradient), false),
            ]
            for panelSource in [true, false] {
                for (name, kind, payload, acceptsExternal) in values {
                    let source = cell(panelSource ? LibraryDropTracker() : nil)
                    source.kind = kind
                    source.index = 3
                    let target = cell(nil)
                    target.kind = kind
                    target.index = 1
                    let sender = info(source)
                    precondition(sender.draggingPasteboard.writeObjects([payload.pasteboardItem()]))
                    var copied = 0, moved = 0
                    target.acceptsDrop = { model.accepts($0, kind: kind) }
                    target.performDrop = { _ in copied += 1 }
                    target.moved = { from, to in
                        precondition(from == 3 && to == 1)
                        moved += 1
                    }
                    let enter = target.draggingEntered(sender)
                    let update = target.draggingUpdated(sender)
                    let result = target.performDragOperation(sender)
                    let expectedAccepted = !panelSource || acceptsExternal
                    let routeIsCorrect = result == expectedAccepted
                        && enter.isEmpty == !expectedAccepted && update.isEmpty == !expectedAccepted
                        && copied == (panelSource && acceptsExternal ? 1 : 0)
                        && moved == (panelSource ? 0 : 1)
                    check("\(panelSource ? "panel-to-toolbar" : "same-toolbar")-\(name)-route",
                          actual: routeIsCorrect, expected: true,
                          evidence: ["enteredOperation": enter.rawValue, "updatedOperation": update.rawValue,
                                     "handlerReturned": result, "copyCallbacks": copied, "moveCallbacks": moved,
                                     "expectedAccepted": expectedAccepted])
                }
            }

            // Mount the actual representable so updateNSView assigns the
            // owner, payload, kind and acceptance callback. Neither model is
            // opened. Separate offscreen windows also prove that ownership is
            // the sequencer model rather than the NSWindow (a strip can float).
            func mountedCell(_ owner: SequencerModel, kind: LibraryKind, payload: VixenDragValue, index: Int) -> (NSWindow, LibraryCellView) {
                let entry = LibraryEntry(id: String(index), name: "Routing probe", value: payload, argb: nil)
                let host = NSHostingView(rootView: LibraryCell(
                    model: owner, kind: kind, entry: entry, index: index, side: 28,
                    highlighted: false, image: NSImage(size: NSSize(width: 28, height: 28)),
                    pressed: { _ in }, doubleClicked: {}, payload: { _ in payload }
                ).frame(width: 28, height: 28))
                let window = NSWindow(contentRect: NSRect(x: 10000, y: 10000, width: 40, height: 40),
                                      styleMask: [.titled], backing: .buffered, defer: false)
                window.isReleasedWhenClosed = false
                window.contentView = host
                func find(_ view: NSView) -> LibraryCellView? {
                    if let cell = view as? LibraryCellView { return cell }
                    return view.subviews.lazy.compactMap(find).first
                }
                for _ in 0..<20 {
                    host.layoutSubtreeIfNeeded()
                    if let cell = find(host) { return (window, cell) }
                    RunLoop.current.run(until: Date().addingTimeInterval(0.025))
                }
                preconditionFailure("Production LibraryCell representable did not mount")
            }
            for crossModel in [true, false] {
                for (name, kind, payload, acceptsExternal) in values {
                    let otherModel = SequencerModel(app: model.app, ref: .new(), registry: SequencerRegistry())
                    let (sourceWindow, source) = mountedCell(model, kind: kind, payload: payload, index: 3)
                    let (targetWindow, target) = mountedCell(crossModel ? otherModel : model, kind: kind, payload: payload, index: 1)
                    let sender = info(source)
                    sender.draggingDestinationWindow = targetWindow
                    precondition(sender.draggingPasteboard.writeObjects([payload.pasteboardItem()]))
                    // Keep the production acceptsDrop callback. Count only the
                    // selected route, without opening a naming dialog or core.
                    var copied = 0, moved = 0
                    target.performDrop = { _ in copied += 1 }
                    target.moved = { from, to in
                        precondition(from == 3 && to == 1)
                        moved += 1
                    }
                    let enter = target.draggingEntered(sender)
                    let update = target.draggingUpdated(sender)
                    let result = target.performDragOperation(sender)
                    let expectedAccepted = !crossModel || acceptsExternal
                    let routeIsCorrect = result == expectedAccepted
                        && enter.isEmpty == !expectedAccepted && update.isEmpty == !expectedAccepted
                        && copied == (crossModel && acceptsExternal ? 1 : 0)
                        && moved == (crossModel ? 0 : 1)
                    check("\(crossModel ? "cross-model-toolbar" : "same-model-floating-toolbar")-\(name)-route",
                          actual: routeIsCorrect, expected: true,
                          evidence: ["enteredOperation": enter.rawValue, "updatedOperation": update.rawValue,
                                     "handlerReturned": result, "copyCallbacks": copied, "moveCallbacks": moved,
                                     "expectedAccepted": expectedAccepted, "actualRepresentableMounted": true,
                                     "separateNativeWindows": source.window !== target.window,
                                     "windowsVisible": sourceWindow.isVisible || targetWindow.isVisible])
                    withExtendedLifetime((sourceWindow, targetWindow, otherModel)) {}
                }
            }
        }

        do {
            let tracker = LibraryDropTracker(), source = cell(nil)
            source.tracker = tracker
            let sender = info(source), first = cell(tracker), second = cell(tracker)
            let (window, panel) = viewport()
            sender.draggingDestinationWindow = window
            panel.addSubview(first)
            panel.addSubview(second)
            #if LIBRARY_REENTRY_BOUNDARY
                tracker.boundaryView = panel
            #endif
            second.frame.origin.x = 140
            var moved = 0
            second.moved = { _, _ in moved += 1 }
            sender.draggingLocation = NSPoint(x: 150, y: 260)
            _ = first.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 200, y: 260)
            first.draggingExited(sender)
            _ = second.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 201, y: 260)
            check("child-cell-transition-keeps-list-entry", actual: second.performDragOperation(sender) && moved == 1,
                  expected: true)
            sender.draggingSequenceNumber += 1000
            sender.draggingLocation = NSPoint(x: 205, y: 260)
            _ = second.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 210, y: 260)
            check("new-session-resets-entry", actual: second.performDragOperation(sender), expected: false)
            withExtendedLifetime(window) {}
        }

        do {
            let tracker = LibraryDropTracker(), source = cell(nil)
            source.tracker = tracker
            let sender = info(source), target = cell(tracker)
            let (window, panel) = viewport()
            sender.draggingDestinationWindow = window
            panel.addSubview(target)
            #if LIBRARY_REENTRY_BOUNDARY
                tracker.boundaryView = panel
            #endif
            let empty = LibraryDropCatcher.CatcherView(frame: panel.bounds)
            empty.kind = .curve; empty.tracker = tracker
            panel.addSubview(empty, positioned: .below, relativeTo: target)
            var invoked = 0
            empty.moveToEnd = { invoked += 1 }
            target.moved = { _, _ in invoked += 1 }
            sender.draggingLocation = NSPoint(x: 150, y: 260)
            _ = target.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 250, y: 360)
            target.draggingExited(sender)
            _ = empty.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 251, y: 360)
            check("cell-exit-to-empty-keeps-list-entry", actual: empty.performDragOperation(sender) && invoked == 1, expected: true)
            sender.draggingLocation = NSPoint(x: 170, y: 260)
            empty.draggingExited(sender)
            _ = target.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 171, y: 260)
            check("empty-exit-to-cell-keeps-list-entry", actual: target.performDragOperation(sender) && invoked == 2, expected: true)
            withExtendedLifetime(window) {}
        }

        for ancestor in [false, true] {
            let tracker = LibraryDropTracker(), source = cell(nil)
            source.tracker = tracker
            let target = cell(tracker), parent = NSView(frame: NSRect(x: 0, y: 0, width: 300, height: 300))
            parent.addSubview(target)
            if ancestor { parent.isHidden = true } else { target.isHidden = true }
            let sender = info(source)
            var invoked = 0
            target.moved = { _, _ in invoked += 1 }
            let operation = target.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 50, y: 50)
            check("hidden-\(ancestor ? "ancestor" : "cell")-rejects-enter-and-drop",
                  actual: operation.isEmpty && !target.performDragOperation(sender) && invoked == 0, expected: true)
        }

        // Real destination exit/re-entry callbacks, supplied with an outside point.
        // The current tracker may expose a gap here; report it instead of matching it.
        for empty in [false, true] {
            let tracker = LibraryDropTracker(), source = cell(nil)
            source.tracker = tracker
            let sender = info(source)
            let (window, panel) = viewport()
            sender.draggingDestinationWindow = window
            #if LIBRARY_REENTRY_BOUNDARY
                tracker.boundaryView = panel
            #endif
            let destination: NSView
            if empty {
                let catcher = LibraryDropCatcher.CatcherView(frame: NSRect(x: 0, y: 0, width: 260, height: 600))
                catcher.kind = .curve; catcher.tracker = tracker; catcher.moveToEnd = {}
                destination = catcher
            } else {
                let target = cell(tracker); target.moved = { _, _ in }
                target.frame.origin = NSPoint(x: 180, y: 180)
                destination = target
            }
            panel.addSubview(destination)
            sender.draggingLocation = NSPoint(x: 250, y: 260)
            _ = destination.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 400, y: 400)
            destination.draggingExited(sender)
            sender.draggingLocation = NSPoint(x: 280, y: 260)
            _ = destination.draggingEntered(sender)
            sender.draggingLocation = NSPoint(x: 285, y: 260)
            check("whole-panel-reentry-\(empty ? "empty" : "cell")-uses-new-entry",
                  actual: destination.performDragOperation(sender), expected: false,
                  evidence: ["firstEntry": [250, 260], "exit": [400, 400], "reentry": [280, 260], "drop": [285, 260],
                             "viewportBoundsInWindow": NSStringFromRect(panel.convert(panel.bounds, to: nil)),
                             "visibleRectInWindow": NSStringFromRect(panel.convert(panel.visibleRect, to: nil)),
                             "windowVisible": window.isVisible])
            withExtendedLifetime(window) {}
        }
        let foregroundUnchanged = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous
        check("foreground-unchanged", actual: foregroundUnchanged, expected: true)
        let failures = checks.filter { $0["passed"] as? Bool != true }.map { $0["name"] as! String }
        let result: [String: Any] = ["passed": failures.isEmpty, "checks": checks, "failures": failures,
                                   "invokedActualCompiledHandlers": true, "windowServerDragSession": false,
                                   "scope": "Exact destination-handler boundary and transition checks; supplied NSDraggingInfo metadata; no foreground input or application profile."]
        let data = try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: output)
        print(String(data: data, encoding: .utf8)!)
    }
}
