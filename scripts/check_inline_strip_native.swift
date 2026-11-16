// Actual optimized production inline strip handlers; no global input or Core.
import AppKit
import Foundation
import ObjectiveC
import VixenKit
import VixenBridge

@main struct InlineStripNative {
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
        func host(_ view: NSView, height: CGFloat) -> NSWindow {
            let window=NSWindow(contentRect:NSRect(x:-16000,y:-16000,width:200,height:height),styleMask:[.borderless],backing:.buffered,defer:false)
            window.isReleasedWhenClosed=false;window.contentView=view;return window
        }
        func event(_ type: NSEvent.EventType, _ view: NSView, _ x: Double, _ y: Double) -> NSEvent {
            NSEvent.mouseEvent(with:type,location:view.convert(NSPoint(x:x,y:y),to:nil),modifierFlags:[],
                timestamp:ProcessInfo.processInfo.systemUptime,windowNumber:view.window!.windowNumber,context:nil,eventNumber:0,clickCount:1,pressure:1)!
        }
        func hiddenTooltips() -> Bool { NSApp.windows.filter { $0 is NSPanel }.allSatisfy { !$0.isVisible } }
        for (name,moves,expectedTips,expectedMiddle) in [
            ("curve-below-two", [[101.0,16.0]], [String](), [50.0,50.0]),
            ("curve-exactly-two", [[102.0,16.0]], [], [50.0,50.0]),
            ("curve-first-crossing-arms-only", [[103.0,16.0]], [], [50.0,50.0]),
            ("curve-repeated-point-moves", [[103.0,16.0],[125.0,12.0],[140.0,8.0]], ["63, 63","70, 75"], [70.0,75.0])
        ] {
            let view=CurveStripNSView(frame:NSRect(x:0,y:0,width:200,height:32)),window=host(view,height:32)
            let original=CurveValue(points:[KitPoint(x:0,y:0),KitPoint(x:50,y:50),KitPoint(x:100,y:100)],libraryName:"",isCurrentLibrary:false)
            view.setValue(original);var commits:[CurveValue]=[];view.onCommit={commits.append($0)}
            let start=ordering.count
            view.mouseDown(with:event(.leftMouseDown,view,100,16))
            check(name+"-no-tip-on-point-press",ordering.count==start && hiddenTooltips())
            for point in moves { view.mouseDragged(with:event(.leftMouseDragged,view,point[0],point[1])) }
            let texts=ordering.dropFirst(start).flatMap { $0["texts"] as? [String] ?? [] }
            check(name+"-actual-point-tooltip",texts==expectedTips,["actual":texts,"expected":expectedTips])
            check(name+"-deferred-value-and-commit",commits.isEmpty && view.value==original)
            let last=moves.last!;view.mouseUp(with:event(.leftMouseUp,view,last[0],last[1]))
            let expected=[[0.0,0.0],expectedMiddle,[100.0,100.0]],actual=view.value.points.map { [$0.x,$0.y] }
            check(name+"-final-points",actual==expected,["actual":actual,"expected":expected])
            check(name+"-one-release-callback",commits.count==1)
            view.mouseUp(with:event(.leftMouseUp,view,last[0],last[1]))
            check(name+"-repeated-release-inert",commits.count==1)
            check(name+"-tooltip-closed-and-host-unshown",hiddenTooltips() && !window.isVisible && !window.isKeyWindow)
            caseReports.append(["name":name,"moves":moves,"tooltipTexts":texts,"actualPoints":actual,"commitCount":commits.count])
            window.close()
        }
        func gradient(_ shared: Bool) -> GradientValue {
            var result=solidGradientValue(argb:0xffff0000)
            var red=solidGradientValue(argb:0xffff0000).stops[0],green=solidGradientValue(argb:0xff00ff00).stops[0],blue=solidGradientValue(argb:0xff0000ff).stops[0]
            red.position=0.25;green.position=0.25;blue.position=0.75
            result.stops=shared ? [red,green,blue] : [red,blue]
            return result
        }
        for (name,shared,moves,expectedTips,expectedPosition,expectedCommits) in [
            ("gradient-below-two",false,[51.0],[String](),0.25,0),
            ("gradient-exactly-two",false,[52.0],[],0.25,0),
            ("gradient-first-crossing-arms-only",false,[53.0],[],0.25,0),
            ("gradient-repeated-moves",false,[53.0,67.0,80.0],["34","40"],0.4,1),
            ("gradient-shared-stop-moves",true,[53.0,67.0,80.0],["34","40"],0.4,1),
            ("gradient-bankers-center",false,[53.0,66.5,67.5],["33","34"],0.34,1)
        ] {
            let view=GradientStripNSView(frame:NSRect(x:0,y:0,width:200,height:42)),window=host(view,height:42)
            let original=gradient(shared);view.setValue(original,discrete:false)
            var commits:[GradientValue]=[];view.onCommit={commits.append($0)}
            let start=ordering.count
            view.mouseDown(with:event(.leftMouseDown,view,50,37))
            let pressTipVisible = !hiddenTooltips()
            // Original gradient SliderPoint disables tooltip; press absence is observational, not a missing feature.
            for x in moves { view.mouseDragged(with:event(.leftMouseDragged,view,x,37)) }
            let texts=ordering.dropFirst(start).flatMap { $0["texts"] as? [String] ?? [] }
            check(name+"-actual-drag-feedback",texts==expectedTips,["actual":texts,"expected":expectedTips])
            check(name+"-deferred-value-and-commit",commits.isEmpty && view.value==original)
            view.mouseUp(with:event(.leftMouseUp,view,moves.last!,37))
            var expected=original
            for i in 0..<(shared ? 2:1) { expected.stops[i].position=expectedPosition }
            check(name+"-full-value-shared-colors-alphas-preserved",view.value==expected,["actualPositions":view.value.stops.map(\.position),"expectedPositions":expected.stops.map(\.position)])
            check(name+"-commit-count",commits.count==expectedCommits,["actual":commits.count,"expected":expectedCommits])
            view.mouseUp(with:event(.leftMouseUp,view,moves.last!,37))
            check(name+"-repeated-release-inert",commits.count==expectedCommits)
            check(name+"-tooltip-closed-and-host-unshown",hiddenTooltips() && !window.isVisible && !window.isKeyWindow)
            caseReports.append(["name":name,"moves":moves,"pressTooltipVisible":pressTipVisible,"tooltipTexts":texts,"positions":view.value.stops.map(\.position),"commitCount":commits.count])
            window.close()
        }
        // Disabled/linked controls in this same press/release flow must not mutate or show drag feedback.
        // No linked value drag is started: no global NSDraggingSession is requested.
        for linked in [false,true] {
            let label=linked ? "linked":"disabled"
            let curve=CurveStripNSView(frame:NSRect(x:0,y:0,width:200,height:32)),cw=host(curve,height:32)
            var cv=CurveValue(points:[KitPoint(x:0,y:0),KitPoint(x:50,y:50),KitPoint(x:100,y:100)],libraryName:linked ? "Fixture Curve":"",isCurrentLibrary:false)
            curve.setValue(cv);curve.isEnabled=linked;var cc=0;curve.onCommit={_ in cc+=1}
            let before=ordering.count
            curve.mouseDown(with:event(.leftMouseDown,curve,100,16));curve.mouseUp(with:event(.leftMouseUp,curve,100,16))
            check("curve-"+label+"-press-release-inert",cc==0 && curve.value==cv && ordering.count==before && hiddenTooltips())
            cw.close()
            let grad=GradientStripNSView(frame:NSRect(x:0,y:0,width:200,height:42)),gw=host(grad,height:42)
            var gv=gradient(true);gv.libraryName=linked ? "Fixture Gradient":"";grad.setValue(gv,discrete:false);grad.isEnabled=linked
            var gc=0;grad.onCommit={_ in gc+=1}
            let gBefore=ordering.count
            grad.mouseDown(with:event(.leftMouseDown,grad,50,37));grad.mouseUp(with:event(.leftMouseUp,grad,50,37))
            check("gradient-"+label+"-press-release-inert",gc==0 && grad.value==gv && ordering.count==gBefore && hiddenTooltips())
            gw.close()
        }
        check("actual-tooltip-ordering-forwarded", !ordering.isEmpty && ordering.allSatisfy { $0["forwarded"] as? Bool == true && $0["transparent"] as? Bool == true && $0["offscreen"] as? Bool == true && $0["nonactivating"] as? Bool == true && $0["key"] as? Bool == false })
        check("foreground-unchanged", previous == NSWorkspace.shared.frontmostApplication?.processIdentifier)
        check("no-visible-window-on-exit", NSApp.windows.allSatisfy { !$0.isVisible })
        let report: [String:Any] = ["passed":checks.allSatisfy { $0["passed"] as? Bool == true },"checks":checks,"cases":caseReports,"ordering":ordering,
            "foregroundBefore":previous,"foregroundAfter":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
            "expectedContract":"F39 point and F46 gradient retain arming-only first >2pt move. F46 updates every subsequent distinct move, including all stops sharing a handle, and commits once on release. Original gradient SliderPoint showToolTip defaults false; existing Mac drag tooltip is current feedback, not upstream press-tooltip parity.",
            "scope":"Actual compiled CurveStripNSView/GradientStripNSView public mouse handlers and generated tooltip NSTextField. Private offscreen unshown host; native nonactivating tooltip orderFront forwarded transparently. No Core/show/global input; no physical gesture or visible tooltip claim."]
        try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[1]))
    }
}
