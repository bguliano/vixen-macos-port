// Read-only real SwiftUI/AppKit view-tree and hit-test differential, no drag dispatch.
import AppKit
import SwiftUI
import VixenBridge
import VixenKit
import UniformTypeIdentifiers
@testable import VixenSequencer

// Observer fixture never becomes key/main or orders itself onto WindowServer.
// This is not a test of production window activation/ordering behavior.
enum ProbeFailure: Error { case failed(String) }
func need(_ value:Bool,_ message:String) throws { if !value { throw ProbeFailure.failed(message) } }

@MainActor final class RoutingObserverWindow: NSWindow {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

@MainActor final class LibraryRoutingViews: NSObject, NSApplicationDelegate {
    var window: NSWindow!, model: SequencerModel!, core: VixenCore!
    func applicationDidFinishLaunching(_ notification: Notification) {
        Task { try? await Task.sleep(for:.seconds(12)); exit(3) }
        Task {
            do { try await run(); exit(0) }
            catch {
                let failure:[String:Any]=["passed":false,"error":String(describing:error),"scope":"Read-only observer fixture failure; no WindowServer input or drag callbacks"]
                try? JSONSerialization.data(withJSONObject:failure,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[1]))
                print("FAIL \(error)"); exit(2)
            }
        }
    }
    func run() async throws {
        let previous=NSWorkspace.shared.frontmostApplication?.processIdentifier
        let output=URL(fileURLWithPath:CommandLine.arguments[1])
        core=try VixenCore.open(dataRoot:ProcessInfo.processInfo.environment["VIXEN_DATA"]!,muted:true)
        defer { core.shutdown() }
        try need(core.openDisplaySetup().controllerRows().isEmpty,"Private fixture unexpectedly has controllers")
        let app=AppModel();app.adopt(core:core)
        let sequencePath=ProcessInfo.processInfo.environment["VIXEN_DATA"]!+"/Sequence/Library Drag.tim"
        model=SequencerModel(app:app,ref:.existing(path:sequencePath),registry:SequencerRegistry())
        window=RoutingObserverWindow(contentRect:NSRect(x:10000,y:10000,width:1100,height:758),styleMask:[.titled],backing:.buffered,defer:false)
        window.isReleasedWhenClosed=false;window.alphaValue=0
        window.setFrame(NSRect(x:10000,y:10000,width:1100,height:780),display:false)
        model.window=window
        await model.open()
        model.setPanelWidths(left:153,right:260)
        model.setStrip(.libraryColor,visible:true)
        window.contentView=NSHostingView(rootView:SequencerContent(model:model))
        // An unshown native hierarchy suffices for registration/hitTest inspection.
        // Avoid asking AppKit to order a window merely to inspect its views.
        var safety=[[String:Any]]()
        func recordSafety(_ stage:String) throws {
            let state:[String:Any]=["stage":stage,"previousForegroundPID":previous ?? -1,"currentForegroundPID":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1,"windowKey":window.isKeyWindow,"windowMain":window.isMainWindow,"appActive":NSApp.isActive,"activationPolicy":NSApp.activationPolicy().rawValue,"alpha":window.alphaValue,"windowVisible":window.isVisible,"windowFrame":NSStringFromRect(window.frame),"screenFrames":NSScreen.screens.map { NSStringFromRect($0.frame) },"intersectsAnyScreen":NSScreen.screens.contains { window.frame.intersects($0.frame) },"allVisibleWindows":NSApp.windows.filter(\.isVisible).map { ["class":String(reflecting:type(of:$0)),"title":$0.title,"frame":NSStringFromRect($0.frame),"alpha":$0.alphaValue] as [String:Any] }]
            safety.append(state)
            try JSONSerialization.data(withJSONObject:safety,options:[.prettyPrinted,.sortedKeys]).write(to:output.deletingLastPathComponent().appending(path:"safety.json"),options:.atomic)
        }
        try recordSafety("host-mounted-before-panels")
        try need(!NSScreen.screens.isEmpty && NSScreen.screens.allSatisfy { !window.frame.intersects($0.frame) },"Initial observer frame is not offscreen")
        var observations=[[String:Any]]()
        func describe(_ view:NSView)->[String:Any] {
            var r:[String:Any]=["class":String(reflecting:type(of:view)),"identity":String(describing:ObjectIdentifier(view)),"hidden":view.isHiddenOrHasHiddenAncestor,"alpha":view.alphaValue,"layerOpacity":view.layer?.opacity as Any? ?? NSNull(),"frameInWindow":NSStringFromRect(view.convert(view.bounds,to:nil)),"bounds":NSStringFromRect(view.bounds),"registeredTypes":view.registeredDraggedTypes.map(\.rawValue)]
            if let cell=view as? LibraryCellView { r["kind"]=String(describing:cell.kind);r["index"]=cell.index;r["panel"]=cell.tracker != nil }
            if let parent=view.superview { r["parent"]=String(describing:ObjectIdentifier(parent)) }
            return r
        }
        for (name,panel) in [("color",ShellPanel.colorLibrary),("gradient",.gradientLibrary),("curve",.curveLibrary),("layers",.layers),("color-again",.colorLibrary)] {
            model.selectPanel(panel,side:.right)
            try await Task.sleep(for:.milliseconds(600))
            window.contentView!.layoutSubtreeIfNeeded()
            try recordSafety(name+"-before-hit-tests")
            var all=[NSView]()
            func visit(_ v:NSView) { all.append(v);v.subviews.forEach(visit) }
            visit(window.contentView!)
            let kind=name == "color-again" ? "color" : name
            let cells=all.compactMap { $0 as? LibraryCellView }.filter { !$0.isHiddenOrHasHiddenAncestor && $0.tracker != nil && String(describing:$0.kind)==kind }.sorted { $0.index<$1.index }
            try need(cells.count==(name == "layers" ? 0:4),"Unexpected active \(name) cell count: \(cells.count)")
            var tests=[[String:Any]]()
            for cell in cells {
                let point=cell.convert(NSPoint(x:cell.bounds.midX,y:cell.bounds.midY),to:nil)
                let contentPoint=window.contentView!.convert(point,from:nil)
                let hit=window.contentView!.hitTest(contentPoint)
                var chain=[[String:Any]](),ancestor=hit
                while let view=ancestor { chain.append(describe(view));ancestor=view.superview }
                let covers=all.filter { !$0.isHiddenOrHasHiddenAncestor && !$0.registeredDraggedTypes.isEmpty && $0.bounds.contains($0.convert(point,from:nil)) }.map(describe)
                tests.append(["sourceCell":describe(cell),"pointInWindow":[point.x,point.y],"hitPath":chain,"registeredViewsCoveringPoint":covers])
            }
            let otherDestinations=all.filter { !($0 is ValueDragView) && !$0.registeredDraggedTypes.isEmpty }.map { view -> [String:Any] in
                var ancestors=[[String:Any]](),next:NSView?=view
                while let current=next { ancestors.append(describe(current));next=current.superview }
                let realPoints=[[876.0,595.0],[939.0,595.0],[1002.0,595.0],[1065.0,595.0]]
                return ["view":describe(view),"ancestry":ancestors,"coversRecordedColorPoints":realPoints.map { point in
                    ["point":point,"covered":view.bounds.contains(view.convert(NSPoint(x:point[0],y:point[1]),from:nil))] as [String:Any]
                }]
            }
            observations.append(["panel":name,"selectedRight":String(describing:model.panels?.selectedRight),"allRegisteredViews":all.filter { !$0.registeredDraggedTypes.isEmpty }.map(describe),"otherDestinationAncestry":otherDestinations,"allNativeValueViews":all.filter { $0 is ValueDragView }.map(describe),"cellHitTests":tests,"windowVisible":window.isVisible,"windowAlpha":window.alphaValue,"windowKey":window.isKeyWindow,"windowFrame":NSStringFromRect(window.frame)])
            try recordSafety(name+"-after-layout")
            try need(!window.isKeyWindow && !window.isMainWindow && !NSApp.isActive && !window.isVisible && window.alphaValue==0 && NSScreen.screens.allSatisfy { !window.frame.intersects($0.frame) },"Observer safety invariant failed after \(name); see safety.json")
        }
        let payloads:[VixenDragValue]=[.color(0xffff0000),.gradient(solidGradientValue(argb:0xffff0000)),.curve(defaultCurveValue())]
        var boards=[[String:Any]]()
        for payload in payloads {
            let item=payload.pasteboardItem(),board=NSPasteboard.withUniqueName()
            defer { board.releaseGlobally() }
            try need(board.writeObjects([item]),"Private pasteboard write failed")
            boards.append(["value":String(describing:payload),"itemTypes":item.types.map(\.rawValue),"boardTypes":board.types?.map(\.rawValue) ?? [],"productionRoundTripMatches":VixenDragValue.read(from:board)==payload])
        }
        window.orderOut(nil)
        let typeInfo=[NSPasteboard.PasteboardType.color.rawValue,"com.vixen.color","com.vixen.gradient","public.utf8-plain-text"].map { identifier -> [String:Any] in
            let type=UTType(identifier)
            return ["identifier":identifier,"known":type != nil,"conformsToData":type?.conforms(to:.data) ?? false,"conformsToItem":type?.conforms(to:.item) ?? false,"conformsToText":type?.conforms(to:.text) ?? false]
        }
        let report:[String:Any]=["passed":true,"observations":observations,"safety":safety,"pasteboards":boards,"pasteboardTypeConformance":typeInfo,"foregroundUnchanged":NSWorkspace.shared.frontmostApplication?.processIdentifier==previous,"prohibitedActivation":NSApp.activationPolicy() == .prohibited,"scope":"Actual current compiled debug SequencerContent and native descendants; unshown non-key/main observer window with alpha0. NSView.hitTest, registered destination ancestry and UTI conformance, actual value/pasteboard round trip only. No mouse/drag handlers, WindowServer input, actual drag receiver or optimized whole-app acceptance claim."]
        try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:output)
    }
}
@main struct LibraryRoutingViewsMain {
    @MainActor static func main() {
        precondition(CommandLine.arguments.count==2)
        let app=NSApplication.shared;app.setActivationPolicy(.prohibited)
        let delegate=LibraryRoutingViews();app.delegate=delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
