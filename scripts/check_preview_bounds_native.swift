// Compiled alongside exact copied Preview module sources; no production edits.
import AppKit
import Foundation
import ObjectiveC
import VixenKit
import VixenBridge

@MainActor final class BoundsDelegateRecorder: NSObject, NSWindowDelegate {
    var moved=0, resized=0
    func windowDidMove(_ notification:Notification) { moved+=1 }
    func windowDidEndLiveResize(_ notification:Notification) { resized+=1 }
}
@main struct PreviewBoundsNative {
    @MainActor static func main() async throws {
        precondition(ProcessInfo.processInfo.environment["VIXEN_UI_TEST"] == nil)
        NSApplication.shared.setActivationPolicy(.prohibited)
        let out=URL(fileURLWithPath:CommandLine.arguments[1]),profile=URL(fileURLWithPath:CommandLine.arguments[2])
        let cornerOnly=CommandLine.arguments.dropFirst(3).contains("--right-corner-only")
        let store=profile.appendingPathComponent("SystemData/ModuleStore.xml")
        let previous=NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        var checks:[[String:Any]]=[],events:[[String:Any]]=[],cursorCalls=0,orderOutCalls=0
        func check(_ name:String,_ passed:Bool,_ detail:[String:Any]=[:]) {
            checks.append(["name":name,"passed":passed,"detail":detail])
        }
        let cursorSel=#selector(NSCursor.set),cursorMethod=class_getInstanceMethod(NSCursor.self,cursorSel)!,cursorIMP=method_getImplementation(class_getInstanceMethod(NSCursor.self,#selector(NSCursor.set))!)
        let cursorBlock:@convention(block)(NSCursor)->Void={_ in MainActor.assumeIsolated { cursorCalls+=1 }}
        class_replaceMethod(NSCursor.self,cursorSel,imp_implementationWithBlock(cursorBlock),method_getTypeEncoding(cursorMethod))
        let orderSel=#selector(NSWindow.order(_:relativeTo:)),orderMethod=class_getInstanceMethod(NSWindow.self,#selector(NSWindow.order(_:relativeTo:)))!
        let orderIMP=method_getImplementation(orderMethod)
        let originalOrder=unsafeBitCast(orderIMP,to:(@convention(c)(AnyObject,Selector,Int,Int)->Void).self)
        let orderBlock:@convention(block)(NSWindow,Int,Int)->Void={window,mode,relative in
            MainActor.assumeIsolated {
                precondition(mode==NSWindow.OrderingMode.out.rawValue,"Unexpected window presentation; refusing onscreen fixture")
                orderOutCalls+=1;originalOrder(window,orderSel,mode,relative)
            }
        }
        class_replaceMethod(NSWindow.self,orderSel,imp_implementationWithBlock(orderBlock),method_getTypeEncoding(orderMethod))
        defer {
            class_replaceMethod(NSCursor.self,cursorSel,cursorIMP,method_getTypeEncoding(cursorMethod))
            class_replaceMethod(NSWindow.self,orderSel,orderIMP,method_getTypeEncoding(orderMethod))
        }
        func readBounds()->[String:Int] {
            let text=(try? String(contentsOf:store,encoding:.utf8)) ?? ""
            var result:[String:Int]=[:]
            for key in ["Left","Top","Width","Height"] {
                let re=try! NSRegularExpression(pattern:"<Setup"+key+">(-?[0-9]+)</Setup"+key+">")
                let matches=re.matches(in:text,range:NSRange(text.startIndex...,in:text))
                precondition(matches.count==1,"Fixture must contain one Preview Setup field")
                result[key]=Int((text as NSString).substring(with:matches[0].range(at:1)))!
            }
            return result
        }
        func seed(_ bounds:[String:Int]) throws {
            var text=try String(contentsOf:store,encoding:.utf8)
            for (key,value) in bounds {
                let re=try NSRegularExpression(pattern:"<Setup"+key+">-?[0-9]+</Setup"+key+">")
                precondition(re.numberOfMatches(in:text,range:NSRange(text.startIndex...,in:text))==1)
                text=re.stringByReplacingMatches(in:text,range:NSRange(text.startIndex...,in:text),withTemplate:"<Setup"+key+">"+String(value)+"</Setup"+key+">")
            }
            try text.write(to:store,atomically:true,encoding:.utf8)
        }
        let screen=NSScreen.screens.first!,area=screen.visibleFrame
        precondition(area.width>800 && area.height>600,"Need ordinary valid-screen geometry")
        let cornerArea=NSScreen.screens.map(\.visibleFrame).min(by:{$0.minX<$1.minX})!
        let stored=cornerOnly ? NSRect(x:cornerArea.minX-1000,y:cornerArea.maxY-720,width:1100,height:680)
            : NSRect(x:area.minX+45,y:area.maxY-440,width:620,height:400)
        let fallback=cornerOnly ? NSRect(x:area.minX+40,y:area.maxY-740,width:1140,height:700)
            : NSRect(x:area.minX+90,y:area.maxY-500,width:580,height:420)
        if cornerOnly {
            let left=NSPoint(x:stored.minX,y:stored.maxY),right=NSPoint(x:stored.maxX-1,y:stored.maxY)
            check("fixture-left-corner-outside-all-working-areas",NSScreen.screens.allSatisfy{!$0.visibleFrame.contains(left)})
            check("fixture-right-minus-one-corner-inside-working-area",NSScreen.screens.contains{$0.visibleFrame.contains(right)})
            events.append(["event":"right-corner-fixture","requestedFrame":[stored.minX,stored.minY,stored.width,stored.height],
                "topLeft":[left.x,left.y],"topRightMinusOne":[right.x,right.y],"workingAreas":NSScreen.screens.map{NSStringFromRect($0.visibleFrame)}])
        }
        let moved=NSRect(x:stored.minX+25,y:stored.minY-20,width:stored.width,height:stored.height)
        let resized=NSRect(x:moved.minX,y:moved.minY,width:moved.width+40,height:moved.height+30)
        // Independent persisted-field expectations: outer frame, top-left in primary-screen coordinates.
        func expected(_ frame:NSRect)->[String:Int] {
            ["Left":Int(frame.minX.rounded()),"Top":Int((screen.frame.maxY-frame.maxY).rounded()),"Width":Int(frame.width),"Height":Int(frame.height)]
        }
        func frameData(_ frame:NSRect)->[Double] {[frame.minX,frame.minY,frame.width,frame.height]}
        try seed(expected(stored))
        events.append(["event":"seed-valid","fields":readBounds(),"frame":frameData(stored)])
        func save(_ model:PreviewDesignerModel,_ name:String,_ fields:[String:Int]) async throws {
            model.save() // Actual production async save path; wait for its private XML outcome.
            let limit=Date().addingTimeInterval(4)
            while readBounds() != fields && Date()<limit { try await Task.sleep(for:.milliseconds(30)) }
            let actual=readBounds()
            check(name+"-saved-xml",actual==fields,["actual":actual,"expected":fields])
            try FileManager.default.copyItem(at:store,to:out.appendingPathComponent(name+"-ModuleStore.xml"))
            events.append(["event":name,"fields":actual])
        }
        for stage in (cornerOnly ? ["right-corner-only"] : ["valid-and-update","reopen-saved","invalid-screen-fallback"]) {
            if stage=="invalid-screen-fallback" {
                try seed(["Left":-30000,"Top":-30000,"Width":620,"Height":400])
            }
            let core=try VixenCore.open(dataRoot:profile.path,muted:true)
            let web=core.webServer(wwwroot:out.appendingPathComponent("private-www").path)
            check(stage+"-web-not-running",!web.settings().running)
            let app=AppModel();app.adopt(core:core)
            let rows=core.previews();precondition(rows.count==1)
            let model=PreviewDesignerModel(previewID:rows[0].id,app:app)
            let recorder=BoundsDelegateRecorder()
            let window=NSWindow(contentRect:fallback,styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
            window.delegate=recorder
            window.isReleasedWhenClosed=false;window.alphaValue=0;window.setFrame(fallback,display:false)
            let canvas=DesignerCanvasView(frame:NSRect(x:0,y:0,width:500,height:300))
            window.contentView=canvas;canvas.metal.metalLayer?.contentsScale=window.backingScaleFactor
            window.layoutIfNeeded();canvas.layoutSubtreeIfNeeded()
            model.open(in:canvas,window:window)
            try await Task.sleep(for:.milliseconds(150))
            check(stage+"-real-designer-opened",model.designer != nil && model.problem==nil,["problem":model.problem as Any? ?? NSNull(),"metalLayer":canvas.metal.layerAddress])
            guard model.designer != nil else { throw NSError(domain:"PreviewBounds",code:1,userInfo:[NSLocalizedDescriptionKey:model.problem ?? "Designer missing"]) }
            check(stage+"-actual-delegate-installed",window.delegate is DesignerWindowDelegate)
            let want=(stage=="valid-and-update" || stage=="right-corner-only") ? stored : stage=="reopen-saved" ? resized : fallback
            check(stage+"-actual-restored-frame",window.frame==want,["actual":frameData(window.frame),"expected":frameData(want)])
            check(stage+"-unshown-unfocused",!window.isVisible && !window.isKeyWindow && !window.isMainWindow)
            events.append(["event":stage,"actualFrame":frameData(window.frame),"fields":readBounds()])
            if stage=="valid-and-update" {
                let beforeMove=recorder.moved
                window.setFrame(moved,display:false)
                try await Task.sleep(for:.milliseconds(40))
                let nativeMoveDelivered=recorder.moved>beforeMove
                if !nativeMoveDelivered {
                    window.delegate?.windowDidMove?(Notification(name:NSWindow.didMoveNotification,object:window))
                }
                check("delegate-original-received-move",recorder.moved>beforeMove)
                events.append(["event":"move-delivery","nativeSetFrameNotification":nativeMoveDelivered,"explicitDelegateNotification":!nativeMoveDelivered,"forwardedCount":recorder.moved-beforeMove])
                try await save(model,"delegate-move",expected(moved))
                window.setFrame(resized,display:false)
                let beforeResize=recorder.resized
                window.delegate?.windowDidEndLiveResize?(Notification(name:NSWindow.didEndLiveResizeNotification,object:window))
                check("delegate-original-received-end-resize",recorder.resized==beforeResize+1)
                events.append(["event":"resize-delivery","explicitDelegateNotification":true,"forwardedCount":recorder.resized-beforeResize])
                try await save(model,"delegate-resize",expected(resized))
            }
            window.close()
            try await Task.sleep(for:.milliseconds(100))
            check(stage+"-native-close-releases-designer",model.designer==nil)
            await Task.detached { core.shutdown() }.value
        }
        check("foreground-unchanged",previous==NSWorkspace.shared.frontmostApplication?.processIdentifier)
        check("all-windows-remain-unshown",NSApp.windows.allSatisfy { !$0.isVisible })
        let report:[String:Any] = ["passed":checks.allSatisfy {$0["passed"] as? Bool==true},"checks":checks,"events":events,
            "screenFrame":frameData(screen.frame),"screenVisibleFrame":frameData(area),"cursorCallsSuppressed":cursorCalls,"orderOutCallsForwarded":orderOutCalls,
            "foregroundBefore":previous,"foregroundAfter":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
            "rightCornerOnly":cornerOnly,
            "scope":"Exact copied unmodified Preview sources compiled with internal fixture entry and optimized Kit/Bridge/FFI dependencies. Real model.open, Metal canvas, restoreBounds, programmatically delivered real delegate move/end-resize callbacks, asynchronous save/private XML and fresh Core reopen. No UI_TEST, ordering/activation/global input; cursor-only side effects suppressed and excluded. No physical drag/Finder/visible-window claim."]
        try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:out.appendingPathComponent("native-result.json"))
    }
}
