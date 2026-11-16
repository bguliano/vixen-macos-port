// Actual optimized production CurveStripNSView handlers; no global input or Core.
import AppKit
import Foundation
import ObjectiveC
import VixenKit
import VixenBridge

@main struct CurveHandleNative {
    @MainActor static func main() throws {
        precondition(ProcessInfo.processInfo.environment["VIXEN_UI_TEST"] == nil)
        NSApplication.shared.setActivationPolicy(.prohibited)
        let previous = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        var checks: [[String:Any]] = [], ordering: [[String:Any]] = []
        func check(_ name: String, _ passed: Bool, _ detail: [String:Any] = [:]) {
            checks.append(["name":name,"passed":passed,"detail":detail])
        }
        func labels(_ view: NSView?) -> [String] {
            guard let view else { return [] }
            return (view as? NSTextField).map { [$0.stringValue] } ?? view.subviews.flatMap { labels($0) }
        }
        let selector = #selector(NSWindow.orderFront(_:))
        let method = class_getInstanceMethod(NSPanel.self, selector)!
        let original = unsafeBitCast(method_getImplementation(method), to: (@convention(c) (AnyObject, Selector, AnyObject?) -> Void).self)
        let guarded: @convention(block) (NSPanel, AnyObject?) -> Void = { panel, sender in
            MainActor.assumeIsolated {
                let outside = NSScreen.screens.allSatisfy { !$0.frame.intersects(panel.frame) }
                precondition(outside && panel.styleMask.contains(.nonactivatingPanel), "Unexpected panel presentation")
                panel.alphaValue = 0
                original(panel, selector, sender)
                ordering.append(["forwarded":true,"offscreen":outside,"transparent":panel.alphaValue == 0,
                    "nonactivating":panel.styleMask.contains(.nonactivatingPanel),"key":panel.isKeyWindow,
                    "visible":panel.isVisible,"texts":labels(panel.contentView),"frame":NSStringFromRect(panel.frame)])
            }
        }
        class_replaceMethod(NSPanel.self, selector, imp_implementationWithBlock(guarded), method_getTypeEncoding(method))
        defer { class_replaceMethod(NSPanel.self, selector, unsafeBitCast(original, to: IMP.self), method_getTypeEncoding(method)) }
        var caseReports: [[String:Any]] = []
        for (name, moves, expectedCommits, expectedLevel) in [
            ("below-two", [51.0], 0, 25.0), ("exactly-two", [52.0], 0, 25.0),
            ("first-crossing-only", [53.0], 1, 27.0),
            ("crossing-then-move", [53.0, 67.0], 1, 34.0),
            ("required-preview-twenty-forty", [40.0, 80.0], 1, 40.0)
        ] {
            let window = NSWindow(contentRect: NSRect(x:-16000,y:-16000,width:200,height:48), styleMask:[.borderless], backing:.buffered, defer:false)
            window.isReleasedWhenClosed = false
            let view = CurveStripNSView(frame: NSRect(x:0,y:0,width:200,height:48))
            window.contentView = view
            view.setValue(CurveValue(points:[KitPoint(x:0,y:25),KitPoint(x:100,y:25)], libraryName:"", isCurrentLibrary:false))
            var commits: [CurveValue] = []
            view.onCommit = { commits.append($0) }
            func event(_ type: NSEvent.EventType, _ x: Double) -> NSEvent {
                NSEvent.mouseEvent(with:type,location:view.convert(NSPoint(x:x,y:40),to:nil),modifierFlags:[.shift],
                    timestamp:ProcessInfo.processInfo.systemUptime,windowNumber:window.windowNumber,context:nil,
                    eventNumber:0,clickCount:1,pressure:1)!
            }
            // Public entered handler exposes the Shift level bar without changing cursor/global input.
            view.mouseEntered(with:event(.mouseMoved,50))
            check(name+"-level-bar-exposed", view.intrinsicContentSize.height == 48)
            let beforePressOrdering = ordering.count
            view.mouseDown(with:event(.leftMouseDown,50))
            if name == "below-two" {
                let pressOrders = Array(ordering.dropFirst(beforePressOrdering))
                check("press-tooltip-opens-with-current-level", pressOrders.last?["texts"] as? [String] == ["25"]
                    && NSApp.windows.contains { $0 is NSPanel && $0.isVisible }, ["actualPressOrders":pressOrders])
            }
            let orderingStart = ordering.count
            for x in moves { view.mouseDragged(with:event(.leftMouseDragged,x)) }
            check(name+"-deferred-until-release", commits.isEmpty)
            let tooltipTexts = ordering.dropFirst(orderingStart).flatMap { $0["texts"] as? [String] ?? [] }
            view.mouseUp(with:event(.leftMouseUp,moves.last!))
            let values = view.value.points.map { [$0.x,$0.y] }
            check(name+"-commit-count", commits.count == expectedCommits, ["actual":commits.count,"expected":expectedCommits])
            check(name+"-final-level", view.value.points.map(\.y) == [expectedLevel,expectedLevel], ["actual":values,"expectedLevel":expectedLevel])
            if name == "crossing-then-move" {
                check("actual-tooltip-rounds-33-point-5-to-34", tooltipTexts.last == "34", ["actualTexts":tooltipTexts])
                view.mouseUp(with:event(.leftMouseUp,67))
                check("repeat-release-does-not-commit-again", commits.count == 1)
            }
            if name == "required-preview-twenty-forty" {
                check("required-distinct-moves-preview-each-level", tooltipTexts == ["20", "40"], ["actualTexts":tooltipTexts])
            }
            check(name+"-host-remains-unshown", !window.isVisible && !window.isKeyWindow)
            check(name+"-tooltip-hidden-after-release", NSApp.windows.filter { $0 is NSPanel }.allSatisfy { !$0.isVisible })
            caseReports.append(["name":name,"moves":moves,"commits":commits.map { $0.points.map { [$0.x,$0.y] } },"final":values,"tooltipTexts":tooltipTexts])
            window.close()
        }
        check("actual-tooltip-ordering-forwarded", !ordering.isEmpty && ordering.allSatisfy { $0["forwarded"] as? Bool == true && $0["transparent"] as? Bool == true && $0["offscreen"] as? Bool == true && $0["nonactivating"] as? Bool == true && $0["key"] as? Bool == false })
        check("foreground-unchanged", previous == NSWorkspace.shared.frontmostApplication?.processIdentifier)
        check("no-visible-window-on-exit", NSApp.windows.allSatisfy { !$0.isVisible })
        let report: [String:Any] = ["passed":checks.allSatisfy { $0["passed"] as? Bool == true },"checks":checks,"cases":caseReports,"ordering":ordering,
            "foregroundBefore":previous,"foregroundAfter":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
            "expectedContract":"Port F42: strict >2pt travel, apply every qualifying move (explicit FIX), one commit on release, tooltip open from press to release. The first qualifying event intentionally differs from upstream SliderPoint arming-only branch.",
            "scope":"Real compiled CurveStripNSView mouse handlers and real generated tooltip label. Private offscreen unshown host; native nonactivating tooltip orderFront forwarded transparently. No Core/show/global input; no physical gesture or visible tooltip claim."]
        try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[1]))
    }
}
