// External, deadline-limited WindowServer gestures for one frozen private host.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit
import ImageIO

func flagsForKeys(_ keys:Set<CGKeyCode>)->CGEventFlags {
    var flags=CGEventFlags();if keys.contains(55){flags.insert(.maskCommand)};if keys.contains(56){flags.insert(.maskShift)};return flags
}
struct KeyUpAcknowledgements {
    private(set) var pending=Set<CGKeyCode>()
    private var absentSince=[CGKeyCode:Double]()
    mutating func posted(_ key:CGKeyCode,held:Set<CGKeyCode>,leased:Set<CGKeyCode>)->Bool {
        guard held.contains(key),leased.contains(key),!pending.contains(key) else{return false}
        pending.insert(key);absentSince[key]=nil;return true
    }
    mutating func observe(time:Double,held:Set<CGKeyCode>,keys:Set<CGKeyCode>,buttons:Set<CGMouseButton.RawValue>,flags:CGEventFlags,mouseHeld:Bool)->Bool {
        let allowed=held.union(pending)
        guard time.isFinite,keys.isSubset(of:allowed),buttons.allSatisfy({mouseHeld && $0==CGMouseButton.left.rawValue}),flags.subtracting(flagsForKeys(allowed)).isEmpty else{return false}
        for key in pending {
            if keys.contains(key) || !flags.intersection(flagsForKeys([key])).isEmpty {absentSince[key]=nil}
            else if let since=absentSince[key],time-since>=0.025 {pending.remove(key);absentSince[key]=nil}
            else if absentSince[key]==nil {absentSince[key]=time}
        }
        return true
    }
}
func keyUpAcknowledgementChecks()->[(String,Bool)] {
    var checks=[(String,Bool)](),ack=KeyUpAcknowledgements()
    let leased:Set<CGKeyCode>=[55,56,5];var held=leased
    func sample(_ time:Double,_ keys:Set<CGKeyCode>,_ flags:CGEventFlags)->Bool {ack.observe(time:time,held:held,keys:keys,buttons:[],flags:flags,mouseHeld:false)}
    checks.append(("G UP must be tracked and leased",ack.posted(5,held:held,leased:leased)));held.remove(5)
    checks.append(("delayed G UP stays pending",sample(0,leased,[.maskCommand,.maskShift]) && ack.pending==[5]))
    checks.append(("first absent G sample does not authorize new input",sample(0.1,[55,56],[.maskCommand,.maskShift]) && !ack.pending.isEmpty))
    checks.append(("stable G absence acknowledged",sample(0.13,[55,56],[.maskCommand,.maskShift]) && ack.pending.isEmpty))
    checks.append(("Shift UP registered",ack.posted(56,held:held,leased:leased)));held.remove(56)
    checks.append(("delayed Shift UP is owned",sample(0.2,[55,56],[.maskCommand,.maskShift]) && ack.pending==[56]))
    checks.append(("lagging Shift modifier alone remains pending",sample(0.3,[55],[.maskCommand,.maskShift]) && ack.pending==[56]))
    _=sample(0.4,[55],[.maskCommand])
    checks.append(("Shift ack permits remaining owned Cmd release",sample(0.43,[55],[.maskCommand]) && ack.pending.isEmpty))
    checks.append(("already acknowledged Shift reappearing is foreign",!sample(0.44,[55,56],[.maskCommand,.maskShift])))
    checks.append(("Cmd UP registered",ack.posted(55,held:held,leased:leased)));held.remove(55)
    checks.append(("foreign key refused during pending Cmd",!sample(0.5,[55,7],[.maskCommand])))
    checks.append(("foreign modifier refused during pending Cmd",!sample(0.5,[55],[.maskCommand,.maskAlternate])))
    checks.append(("foreign mouse press refused during pending Cmd",!ack.observe(time:0.5,held:held,keys:[55],buttons:[0],flags:[.maskCommand],mouseHeld:false)))
    checks.append(("unacknowledged UP never becomes ready just from elapsed time",sample(10,[55],[.maskCommand]) && ack.pending==[55]))
    _=sample(10.1,[],[]);_=sample(10.12,[55],[.maskCommand]);_=sample(10.2,[],[])
    checks.append(("reappearance resets stable absence interval",sample(10.21,[],[]) && ack.pending==[55]))
    checks.append(("all posted UPs eventually acknowledged",sample(10.24,[],[]) && ack.pending.isEmpty))
    checks.append(("untracked UP cannot enter pending",!ack.posted(55,held:[],leased:leased)))
    checks.append(("unleased UP cannot enter pending",!ack.posted(55,held:[55],leased:[])))
    return checks
}


enum GestureFailure: Error { case failed(String) }
func require(_ value: Bool, _ message: String) throws { if !value { throw GestureFailure.failed(message) } }
func sha(_ path: String) throws -> String { SHA256.hash(data: try Data(contentsOf: URL(fileURLWithPath: path))).map { String(format: "%02x", $0) }.joined() }
func pump(_ duration: Double) { RunLoop.current.run(until: Date().addingTimeInterval(duration)) }
func identity(_ app: NSRunningApplication?) -> [String: Any] {
    ["pid": Int(app?.processIdentifier ?? -1), "bundleIdentifier": app?.bundleIdentifier ?? "", "executable": app?.executableURL?.standardizedFileURL.path ?? ""]
}

@main struct InlineGestureDriver {
    @MainActor static func main() throws {
        let args = CommandLine.arguments
        if args.count == 2 && args[1] == "--pure-checks" {
            let checks=keyUpAcknowledgementChecks();precondition(checks.allSatisfy{$0.1})
            print("{\"passed\":true,\"checks\":\(checks.count),\"nativeOperations\":0}");return
        }
        if args.count == 3 && args[1] == "--desktop" {
            let result: [String: Any] = ["application": identity(NSWorkspace.shared.frontmostApplication),
                "accessibilityTrusted": AXIsProcessTrusted(), "eventPostingPreflight": CGPreflightPostEventAccess(),
                "screenCapturePreflight":CGPreflightScreenCaptureAccess(), "inputPosted": false, "permissionRequested": false]
            try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted,.sortedKeys]).write(to: URL(fileURLWithPath: args[2]))
            return
        }
        guard args.count == 2 else { exit(2) }
        let config = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: args[1]))) as! [String: Any]
        let pid = pid_t(config["pid"] as! Int), executable = config["executable"] as! String
        let bundle = config["bundleIdentifier"] as! String
        let output = URL(fileURLWithPath: config["output"] as! String)
        let events = URL(fileURLWithPath: config["events"] as! String)
        let previous = config["previousApplication"] as! [String: Any]
        let began = Date(), deadline = began.addingTimeInterval(20), cleanupDeadline = began.addingTimeInterval(25)
        let oldPointer = CGEvent(source: nil)?.location
        var heldMouse = false, heldShift = false, posted = false, activated = false, pointerInterference = false
        var lastPoint: CGPoint?, frozenWindowNumber: Int?, frozenFrame: [Double]?
        var ownershipLost=false, keyAcks=KeyUpAcknowledgements(), captures=[[String:Any]](), mouseDowns=0,mouseUps=0,keyDowns=0,keyUps=0
        let systemAX=AXUIElementCreateSystemWide()
        func physical() -> Bool {
            let keys=Set((0..<128).map{CGKeyCode($0)}.filter{CGEventSource.keyState(.combinedSessionState,key:$0)})
            let buttons=Set([CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}.map(\.rawValue))
            let flags=CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand,.maskSecondaryFn])
            return keyAcks.observe(time:ProcessInfo.processInfo.systemUptime,held:heldShift ? [56]:[],keys:keys,buttons:buttons,flags:flags,mouseHeld:heldMouse)
        }
        func pointerStable() -> Bool {
            guard let actual=CGEvent(source:nil)?.location,let expected=lastPoint ?? oldPointer else{return false}
            return hypot(actual.x-expected.x,actual.y-expected.y)<1
        }
        var report: [String: Any] = ["passed":false, "inputDeadlineSeconds":20, "cleanupDeadlineSeconds":25,
            "pid":Int(pid), "executable":executable, "bundleIdentifier":bundle,
            "scope":"Four cursor-inclusive crops of previously accepted three inline gestures in unchanged optimized native host. No full-app visual claim, handlers, Core or show."]
        var observations = [[String: Any]](), pointerEvents = [[String: Any]]()
        func read() throws -> [String: Any] {
            let value = try JSONSerialization.jsonObject(with: Data(contentsOf: events)) as! [String: Any]
            try require(value["pid"] as? Int == Int(pid) && value["executable"] as? String == executable && value["bundleIdentifier"] as? String == bundle && value["testMode"] as? Bool == false, "Passive observation identity/test-mode mismatch")
            return value
        }
        func target() -> NSRunningApplication? { NSRunningApplication(processIdentifier: pid) }
        func same(_ app: NSRunningApplication?, _ expected: [String: Any]) -> Bool {
            app?.processIdentifier == pid_t(expected["pid"] as! Int) && app?.bundleIdentifier == expected["bundleIdentifier"] as? String && app?.executableURL?.standardizedFileURL.path == expected["executable"] as? String
        }
        func owned() -> Bool {
            let app = target()
            return app?.isTerminated == false && app?.executableURL?.standardizedFileURL.path == executable && app?.bundleIdentifier == bundle
        }
        func active() -> Bool { owned() && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && target()?.isActive == true }
        func nodeOwned(_ node: AXUIElement) -> Bool { var owner: pid_t = -1; return AXUIElementGetPid(node, &owner) == .success && owner == pid }
        func attr(_ node: AXUIElement, _ name: String) -> CFTypeRef? {
            guard Date() < cleanupDeadline, nodeOwned(node) else { return nil }
            AXUIElementSetMessagingTimeout(node, 0.15)
            var value: CFTypeRef?
            return AXUIElementCopyAttributeValue(node, name as CFString, &value) == .success ? value : nil
        }
        let root = AXUIElementCreateApplication(pid)
        func windows() -> [AXUIElement] {
            (attr(root, kAXWindowsAttribute) as? [AXUIElement] ?? []).filter { attr($0,kAXTitleAttribute) as? String == "Private Inline Gesture Verification" }
        }
        func guardInput() throws {
            try require(Date()<deadline && !FileManager.default.fileExists(atPath:output.deletingLastPathComponent().appendingPathComponent("abort-request.json").path),"Input cutoff or cooperative abort")
            try require(keyAcks.pending.isEmpty,"Posted keyUP pending; no new input")
            let ok = !ownershipLost && active() && pointerStable() && physical()
            if !ok {ownershipLost=true};try require(ok,"Private process, pointer or physical input ownership lost")
            let value = try read()
            try require(value["windowNumber"] as? Int == frozenWindowNumber && value["windowFrame"] as? [Double] == frozenFrame,
                        "Owned window moved or was replaced")
            let live=windows()
            try require(live.count==1 && (value["current"] as? [String:Any])?["windowKey"] as? Bool==true,"Private window key state or count changed")
            try require(attr(root,kAXFocusedWindowAttribute).map{CFEqual($0,live[0])}==true && attr(root,kAXMainWindowAttribute).map{CFEqual($0,live[0])}==true && Date()<deadline && active(),"Exact private AX focused/main window changed")
            let fresh = !ownershipLost && active() && pointerStable() && physical()
            if !fresh {ownershipLost=true;pointerInterference = !pointerStable()}
            try require(fresh && Date()<deadline,"Immediate input ownership/deadline changed after AX traversal")
        }
        func pointerOwned(_ point: CGPoint) -> Bool {
            guard Date()<cleanupDeadline,let expected=frozenFrame,let id=frozenWindowNumber,
                  let snapshot=try? read(),let top=snapshot["screenTop"] as? Double,
                  let rows=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] else{return false}
            let expectedCG=CGRect(x:expected[0],y:top-expected[1]-expected[3],width:expected[2],height:expected[3])
            let matches=rows.filter{ row in
                guard row[kCGWindowOwnerPID as String] as? Int==Int(pid),row[kCGWindowNumber as String] as? Int==id,
                      (row[kCGWindowAlpha as String] as? Double ?? 0)>0,let raw=row[kCGWindowBounds as String] as? [String:Any],let frame=CGRect(dictionaryRepresentation:raw as CFDictionary) else{return false}
                return frame==expectedCG && frame.contains(point)
            }
            guard matches.count==1,let window=windows().first,windows().count==1 else{return false}
            AXUIElementSetMessagingTimeout(systemAX,0.1)
            var hit:AXUIElement?
            guard AXUIElementCopyElementAtPosition(systemAX,Float(point.x),Float(point.y),&hit) == .success,let hit,nodeOwned(hit) else{return false}
            if let raw=attr(hit,kAXWindowAttribute),CFGetTypeID(raw)==AXUIElementGetTypeID(),CFEqual(raw,window){return Date()<cleanupDeadline}
            var current:AXUIElement?=hit
            for _ in 0..<14 {
                guard let node=current,Date()<cleanupDeadline,nodeOwned(node) else{return false}
                if CFEqual(node,window){return true}
                guard let parent=attr(node,kAXParentAttribute),CFGetTypeID(parent)==AXUIElementGetTypeID() else{return false}
                current=(parent as! AXUIElement)
            }
            return false
        }
        func mouse(_ kind: CGEventType, _ point: CGPoint, shift: Bool = false) throws {
            try guardInput()
            try require(pointerOwned(point), "Private WindowServer window does not own target point")
            guard let event = CGEvent(mouseEventSource:nil, mouseType:kind, mouseCursorPosition:point, mouseButton:.left) else { throw GestureFailure.failed("Cannot create pointer event") }
            event.flags = shift ? .maskShift : []
            event.setIntegerValueField(.mouseEventClickState, value:1)
            try guardInput();try require(pointerOwned(point),"Immediate exact native point changed");try guardInput()
            event.post(tap:.cghidEventTap); posted = true; lastPoint = point
            if kind == .leftMouseDown { heldMouse = true;mouseDowns+=1 }
            if kind == .leftMouseUp { heldMouse = false;mouseUps+=1 }
            pointerEvents.append(["type":kind.rawValue,"point":[point.x,point.y],"shift":shift,"elapsedSeconds":Date().timeIntervalSince(began)])
            pump(0.18)
        }
        func releaseSafe() -> Bool {
            let ok = !ownershipLost && active() && pointerStable() && physical()
            if !ok{ownershipLost=true};return ok && Date()<cleanupDeadline
        }
        func acknowledge(_ cleanup:Bool=false) throws {
            let end=min(cleanup ? cleanupDeadline:deadline,Date().addingTimeInterval(0.75))
            while !keyAcks.pending.isEmpty && Date()<end {
                try require(releaseSafe(),"Key acknowledgement ownership lost")
                if !keyAcks.pending.isEmpty{pump(0.015)}
            }
            try require(keyAcks.pending.isEmpty && Date()<end,"KeyUP physical acknowledgement timed out")
        }
        func shift(_ down: Bool) throws {
            if down{try guardInput()}else{try require(heldShift && releaseSafe() && Date()<deadline,"Tracked Shift release lost owner")}
            guard let event=CGEvent(keyboardEventSource:nil,virtualKey:56,keyDown:down) else{throw GestureFailure.failed("Cannot create Shift")}
            event.flags=down ? .maskShift:[]
            if down{try guardInput()}else{try require(heldShift && releaseSafe() && Date()<deadline,"Immediate Shift release lost owner")}
            event.post(tap:.cghidEventTap);posted=true
            if down{heldShift=true;keyDowns+=1;pump(0.12)}
            else{try require(keyAcks.posted(56,held:[56],leased:[56]),"Shift release tracking");heldShift=false;keyUps+=1;try acknowledge()}
        }
        func point(_ kind: String, _ x: Double, _ y: Double) throws -> CGPoint {
            let value = try read(), all = value["geometry"] as! [String:[String:Any]], geometry = all[kind]!
            let start = geometry["topLeftCG"] as! [Double], end = geometry["bottomRightCG"] as! [Double], bounds = geometry["bounds"] as! [Double]
            try require(geometry["flipped"] as? Bool == true && bounds[0] == 0 && bounds[1] == 0 && bounds[2] == 200 && end[0]-start[0] == 200 && end[1]-start[1] == bounds[3] && x >= 0 && x < 200 && y >= 0 && y < bounds[3], "Exported control geometry is not the expected unscaled 200pt native view")
            return CGPoint(x:start[0]+x,y:start[1]+y)
        }
        func observe(_ name: String, _ predicate: ([String:Any]) -> Bool) throws -> [String:Any] {
            let until = min(deadline, Date().addingTimeInterval(1.5))
            var value: [String:Any] = [:]
            repeat {
                try guardInput(); value = try read()
                if predicate(value["current"] as! [String:Any]) { observations.append(["name":name,"observation":value]); return value }
                pump(0.05)
            } while Date() < until
            observations.append(["name":name,"failedObservation":value]); throw GestureFailure.failed("Passive state assertion failed: "+name)
        }
        func tips(_ state: [String:Any]) -> [String] { (state["tooltips"] as? [[String:Any]] ?? []).flatMap { $0["texts"] as? [String] ?? [] } }
        func count(_ state: [String:Any], _ name: String) -> Int { (state["commits"] as! [String:Int])[name]! }
        func points(_ state: [String:Any], _ name: String) -> [[Double]] { (state[name] as! [String:Any])["points"] as! [[Double]] }
        func equal(_ a: Any, _ b: Any) -> Bool { NSDictionary(dictionary:["value":a]).isEqual(to:["value":b]) }
        func visibleTip(_ state: [String:Any], _ text: String) -> Bool {
            let panels = state["tooltips"] as? [[String:Any]] ?? []
            guard tips(state) == [text] && panels.count == 1,
                  let frame = panels[0]["frame"] as? [Double], frame.count == 4 else { return false }
            let rect = NSRect(x:frame[0],y:frame[1],width:frame[2],height:frame[3])
            return panels[0]["alpha"] as? Double == 1 && panels[0]["nonactivating"] as? Bool == true && panels[0]["key"] as? Bool == false && panels[0]["visible"] as? Bool == true && NSScreen.screens.contains { $0.visibleFrame.contains(rect) }
        }
        func capture(_ name:String,_ kind:String) throws {
            try guardInput();try require(CGPreflightScreenCaptureAccess(),"Screen recording permission unavailable; no prompt requested")
            let value=try read(),geometry=(value["geometry"] as! [String:[String:Any]])[kind]!,a=geometry["topLeftCG"] as! [Double],b=geometry["bottomRightCG"] as! [Double]
            let state=value["current"] as! [String:Any],top=value["screenTop"] as! Double,frame=frozenFrame!
            var crop=CGRect(x:a[0],y:a[1],width:b[0]-a[0],height:b[1]-a[1]).insetBy(dx:-12,dy:-12)
            for tip in state["tooltips"] as! [[String:Any]] {
                let t=tip["frame"] as! [Double];crop=crop.union(CGRect(x:t[0],y:top-t[1]-t[3],width:t[2],height:t[3]).insetBy(dx:-6,dy:-6))
            }
            guard let pointer=CGEvent(source:nil)?.location else{throw GestureFailure.failed("Pointer unavailable")}
            crop=crop.union(CGRect(x:pointer.x-14,y:pointer.y-14,width:36,height:36)).integral
            let ownedFrame=CGRect(x:frame[0],y:top-frame[1]-frame[3],width:frame[2],height:frame[3])
            try require(ownedFrame.contains(crop) && crop.width<=300 && crop.height<=140,"Crop escaped owned host or expected small geometry")
            for p in [CGPoint(x:crop.minX+1,y:crop.minY+1),CGPoint(x:crop.maxX-1,y:crop.minY+1),CGPoint(x:crop.minX+1,y:crop.maxY-1),CGPoint(x:crop.maxX-1,y:crop.maxY-1),pointer]{try require(pointerOwned(p),"Crop ownership changed")}
            let destination=output.deletingLastPathComponent().appendingPathComponent(name+".png")
            try require(!FileManager.default.fileExists(atPath:destination.path),"Crop already exists")
            let child=Process();child.executableURL=URL(fileURLWithPath:"/usr/sbin/screencapture")
            child.arguments=["-x","-C","-R\(Int(crop.minX)),\(Int(crop.minY)),\(Int(crop.width)),\(Int(crop.height))",destination.path]
            child.standardOutput=FileHandle.nullDevice;child.standardError=FileHandle.nullDevice
            try guardInput();try require(deadline.timeIntervalSinceNow>2,"Insufficient complete capture budget");try child.run()
            let until=min(deadline,Date().addingTimeInterval(1.5))
            report["lastCapturePID"]=Int(child.processIdentifier)
            while child.isRunning && Date()<until{pump(0.015)}
            if child.isRunning{child.terminate();let end=min(cleanupDeadline,Date().addingTimeInterval(0.15));while child.isRunning && Date()<end{pump(0.01)};if child.isRunning{kill(child.processIdentifier,SIGKILL);let hard=min(cleanupDeadline,Date().addingTimeInterval(0.15));while child.isRunning && Date()<hard{pump(0.01)}}}
            report["lastCaptureExited"] = !child.isRunning
            try require(!child.isRunning && child.terminationStatus==0,"Bounded screencapture failed")
            try guardInput();try require(pointerOwned(pointer),"Capture ended under another window")
            guard let image=CGImageSourceCreateWithURL(destination as CFURL,nil),let bitmap=CGImageSourceCreateImageAtIndex(image,0,nil) else{throw GestureFailure.failed("Capture PNG unreadable")}
            let scale=Double(bitmap.width)/crop.width
            try require([1.0,2.0].contains(scale) && Double(bitmap.height)==crop.height*scale,"Unexpected capture backing geometry")
            captures.append(["name":name,"path":destination.path,"sha256":try sha(destination.path),"cropCG":[crop.minX,crop.minY,crop.width,crop.height],"pixelSize":[bitmap.width,bitmap.height],"cursorCG":[pointer.x,pointer.y],"cursorRequested":true,"screenCapturePreflight":true,"captureArguments":child.arguments!,"passiveState":state,"windowNumber":frozenWindowNumber!,"elapsedSeconds":Date().timeIntervalSince(began)])
        }
        defer {
            if heldMouse,releaseSafe(),let p=CGEvent(source:nil)?.location,pointerOwned(p),let event=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:p,mouseButton:.left),releaseSafe() {
                event.flags=heldShift ? .maskShift:[];event.post(tap:.cghidEventTap);heldMouse=false;mouseUps+=1;pump(0.08)
            }
            if heldShift,releaseSafe(),let event=CGEvent(keyboardEventSource:nil,virtualKey:56,keyDown:false),releaseSafe() {
                event.flags=[];event.post(tap:.cghidEventTap);_ = keyAcks.posted(56,held:[56],leased:[56]);heldShift=false;keyUps+=1
            }
            if !keyAcks.pending.isEmpty{try? acknowledge(true)}
            if owned() && Date() < cleanupDeadline { _ = target()?.terminate() }
            let softExitDeadline = min(Date().addingTimeInterval(1.5),cleanupDeadline.addingTimeInterval(-2))
            while owned() && Date() < softExitDeadline { pump(0.05) }
            if owned() && Date() < cleanupDeadline { _ = target()?.forceTerminate() }
            let hardExitDeadline = min(Date().addingTimeInterval(0.5),cleanupDeadline.addingTimeInterval(-1))
            while owned() && Date() < hardExitDeadline { pump(0.05) }
            func restorable() -> Bool {
                let current = NSWorkspace.shared.frontmostApplication
                return !ownershipLost && !heldMouse && !heldShift && keyAcks.pending.isEmpty && physical() && pointerStable() && Date() < cleanupDeadline && (current?.processIdentifier == pid || same(current,previous)) && same(NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int)),previous)
            }
            if activated && restorable() {
                if posted && !pointerInterference, let oldPointer, restorable() { CGWarpMouseCursorPosition(oldPointer);lastPoint=oldPointer }
                if restorable() { _ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[]) }
                while !same(NSWorkspace.shared.frontmostApplication,previous) && Date() < cleanupDeadline { pump(0.05) }
            }
            report["restoredPrevious"] = same(NSWorkspace.shared.frontmostApplication,previous)
            report["targetExited"] = !owned()
            report["noInputHeld"] = !heldMouse && !heldShift && keyAcks.pending.isEmpty && physical()
            report["restoredPointer"] = pointerStable() && (lastPoint == oldPointer || !posted)
            report["pendingKeyUpAcknowledgements"] = keyAcks.pending.sorted().map(Int.init)
            report["mouseDowns"]=mouseDowns;report["mouseUps"]=mouseUps;report["keyDowns"]=keyDowns;report["keyUps"]=keyUps;report["captures"]=captures;report["ownershipLost"]=ownershipLost
            report["heldMouse"] = heldMouse;report["heldShift"] = heldShift;report["inputPosted"] = posted
            report["pointerInterferenceDetected"] = pointerInterference
            report["elapsedSeconds"] = Date().timeIntervalSince(began)
            report["observations"] = observations;report["pointerEvents"] = pointerEvents
            if report["restoredPointer"] as? Bool != true || report["targetExited"] as? Bool != true || report["noInputHeld"] as? Bool != true || report["restoredPrevious"] as? Bool != true { report["passed"] = false }
            if let data = try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]) { try? data.write(to:output,options:.atomic) }
        }
        do {
            try require(try sha(executable) == config["executableSha256"] as? String && bundle.hasPrefix("com.vixenlights.verification."), "Private executable pin mismatch")
            try require(AXIsProcessTrusted() && CGPreflightPostEventAccess() && CGPreflightScreenCaptureAccess(), "Existing permissions unavailable; no prompt requested")
            try require(oldPointer != nil && physical() && pointerStable(),"Physical input held or pointer unavailable; no input")
            while (!owned() || !FileManager.default.fileExists(atPath:events.path)) && Date() < min(deadline,began.addingTimeInterval(4)) { pump(0.05) }
            let initial = try read(); frozenWindowNumber = initial["windowNumber"] as? Int; frozenFrame = initial["windowFrame"] as? [Double]
            try require(owned() && frozenWindowNumber != nil && frozenFrame?.count == 4, "Private host not ready")
            try require(same(NSWorkspace.shared.frontmostApplication,previous) || NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Desktop changed before activation")
            try require(!(previous["bundleIdentifier"] as! String).lowercased().contains("securityagent"), "System authorization owns desktop")
            try require(pointerStable() && physical() && Date() < deadline && target()?.activate(options:[]) == true, "Private activation refused")
            activated = true
            while !active() && Date() < deadline { pump(0.05) }
            let first = try observe("initial-native-values") { state in
                count(state,"level") == 0 && count(state,"point") == 0 && count(state,"gradient") == 0 && points(state,"level") == [[0,25],[100,25]] && points(state,"point") == [[0,0],[50,50],[100,100]] && tips(state).isEmpty
            }
            let originalGradient = (first["current"] as! [String:Any])["gradient"] as! [String:Any]
            try mouse(.mouseMoved, try point("level",50,16)); try shift(true)
            try mouse(.mouseMoved, try point("level",50,16), shift:true)
            _ = try observe("shift-exposes-level") { $0["levelIntrinsicHeight"] as? Double == 48 }
            try mouse(.mouseMoved,try point("level",50,40),shift:true)
            try mouse(.leftMouseDown,try point("level",50,40),shift:true)
            _ = try observe("level-press-tooltip25") { visibleTip($0,"25") && count($0,"level") == 0 && points($0,"level") == [[0,25],[100,25]] }
            try capture("01-level-tooltip-cursor","level")
            for (x,tip) in [(40.0,"20"),(80.0,"40")] {
                try mouse(.leftMouseDragged,try point("level",x,40),shift:true)
                _ = try observe("level-preview-"+tip) { visibleTip($0,tip) && count($0,"level") == 0 && points($0,"level") == [[0,25],[100,25]] }
            }
            try mouse(.leftMouseUp,try point("level",80,40),shift:true); try shift(false)
            _ = try observe("level-one-release-commit") { count($0,"level") == 1 && points($0,"level") == [[0,40],[100,40]] && tips($0).isEmpty }
            try mouse(.mouseMoved,try point("point",100,16));try mouse(.leftMouseDown,try point("point",100,16))
            _ = try observe("point-press-no-tooltip") { tips($0).isEmpty && count($0,"point") == 0 }
            try mouse(.leftMouseDragged,try point("point",103,16))
            _ = try observe("point-first-crossing-arms") { tips($0).isEmpty && count($0,"point") == 0 }
            for (x,y,tip) in [(125.0,12.0,"63, 63"),(140.0,8.0,"70, 75")] {
                try mouse(.leftMouseDragged,try point("point",x,y))
                _ = try observe("point-preview-"+tip) { visibleTip($0,tip) && count($0,"point") == 0 && points($0,"point") == [[0,0],[50,50],[100,100]] }
            }
            try capture("02-point-tooltip-cursor","point")
            try mouse(.leftMouseUp,try point("point",140,8))
            _ = try observe("point-one-release-commit") { count($0,"point") == 1 && points($0,"point") == [[0,0],[70,75],[100,100]] && tips($0).isEmpty }
            try mouse(.mouseMoved,try point("gradient",50,37));try mouse(.leftMouseDown,try point("gradient",50,37))
            _ = try observe("gradient-press-no-tooltip") { tips($0).isEmpty && count($0,"gradient") == 0 }
            try mouse(.leftMouseDragged,try point("gradient",53,37))
            _ = try observe("gradient-first-crossing-arms") { tips($0).isEmpty && count($0,"gradient") == 0 }
            for (x,tip) in [(67.0,"34"),(80.0,"40")] {
                try mouse(.leftMouseDragged,try point("gradient",x,37))
                _ = try observe("gradient-preview-"+tip) { visibleTip($0,tip) && count($0,"gradient") == 0 && equal($0["gradient"]!,originalGradient) }
            }
            try capture("03-gradient-tooltip-cursor","gradient")
            try mouse(.leftMouseUp,try point("gradient",80,37))
            var expectedGradient = originalGradient
            var stops = originalGradient["stops"] as! [[String:Any]]
            try require(stops.count == 3 && stops.map { $0["position"] as! Double } == [0.25,0.25,0.75], "Initial paired gradient fixture invalid")
            stops[0]["position"] = 0.4;stops[1]["position"] = 0.4;expectedGradient["stops"] = stops
            _ = try observe("gradient-shared-stops-one-release-full-value") { count($0,"gradient") == 1 && equal($0["gradient"]!,expectedGradient) && tips($0).isEmpty }
            _ = try observe("all-final-values-one-commit") { count($0,"level") == 1 && count($0,"point") == 1 && count($0,"gradient") == 1 && points($0,"level") == [[0,40],[100,40]] && points($0,"point") == [[0,0],[70,75],[100,100]] && equal($0["gradient"]!,expectedGradient) && tips($0).isEmpty }
            let g=(try read()["geometry"] as! [String:[String:Any]])["gradient"]!["topLeftCG"] as! [Double]
            try mouse(.mouseMoved,CGPoint(x:g[0]-18,y:g[1]+20))
            try capture("04-exit-arrow-no-tooltip","gradient")
            try require(captures.count==4 && mouseDowns==3 && mouseUps==3 && keyDowns==1 && keyUps==1,"Expected four captures and balanced gestures")
            try guardInput()
            var matches = [AXUIElement](), visits = 0
            func findFinish(_ node: AXUIElement, depth: Int) {
                guard depth < 8 && visits < 100 && Date() < deadline else { return };visits += 1
                if attr(node,kAXRoleAttribute) as? String == "AXButton" && attr(node,kAXTitleAttribute) as? String == "Finish Check" { matches.append(node) }
                for child in attr(node,kAXChildrenAttribute) as? [AXUIElement] ?? [] { findFinish(child,depth:depth+1) }
            }
            for window in windows() { findFinish(window,depth:0) }
            try require(matches.count == 1 && nodeOwned(matches[0]), "Unique owned Finish Check unavailable")
            try guardInput();let status = AXUIElementPerformAction(matches[0],kAXPressAction as CFString)
            let finishDeadline = min(deadline,Date().addingTimeInterval(1.5))
            while owned() && Date() < finishDeadline { pump(0.05) }
            try require(status == .success || !owned(), "Native finish rejected")
            try require(!owned(), "Host did not exit after finish")
            let final = try read(), names = (final["events"] as! [[String:Any]]).map { $0["event"] as! String }
            try require(final["terminated"] as? Bool == true && !names.contains("watchdog") && names.suffix(2) == ["finished","terminated"], "Normal host finish unproven")
            report["passed"] = true;report["normalFinish"] = true
        } catch { report["error"] = String(describing:error) }
    }
}
