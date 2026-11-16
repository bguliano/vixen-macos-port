// Test-only WindowServer input for one private Vixen process. No app callbacks.
import AppKit
import ApplicationServices
import Foundation

enum CheckFailure: Error { case failed(String) }
func require(_ ok: Bool, _ message: String) throws { if !ok { throw CheckFailure.failed(message) } }
let args = CommandLine.arguments
guard (4...5).contains(args.count), let pid = Int32(args[1]) else { fatalError("PID, output folder, exact executable, optional mode required") }
let edgeMode = args.count == 5 && args[4] == "edges"
let panelToToolbar = args.count == 5 && args[4] == "panel-to-toolbar"
let out = URL(fileURLWithPath: args[2])
let executable = URL(fileURLWithPath: args[3]).standardizedFileURL
var target = NSRunningApplication(processIdentifier: pid)
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let deadline = Date().addingTimeInterval(65)
var report: [String:Any] = ["passed":false,"targetPID":pid,"previousPID":previous?.processIdentifier ?? -1,
                          "scope":"Actual OS CG pointer drags; calibrated private-window points; no drag callbacks or library mutation hooks"]
var observations: [[String:Any]] = []
var pointerChecks: [[String:Any]] = []
var held = false
var lastPoint = CGPoint.zero
func pump(_ seconds: Double = 0.1) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func value(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    guard Date() < deadline else { return nil }
    var owner: pid_t = -1
    guard AXUIElementGetPid(element, &owner) == .success, owner == pid else { return nil }
    AXUIElementSetMessagingTimeout(element, 0.3)
    var result: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &result) == .success ? result : nil
}
func element(_ value: CFTypeRef?) -> AXUIElement? {
    guard let value, CFGetTypeID(value)==AXUIElementGetTypeID() else { return nil }
    return (value as! AXUIElement)
}
func children(_ e: AXUIElement) -> [AXUIElement] { value(e,kAXChildrenAttribute) as? [AXUIElement] ?? [] }
func label(_ e: AXUIElement) -> String {
    for name in [kAXTitleAttribute,kAXDescriptionAttribute,kAXValueAttribute] {
        if let text = value(e,name) as? String, !text.isEmpty { return text }
    }
    return ""
}
func find(_ e: AXUIElement, role: String? = nil, name: String? = nil, depth: Int = 0) -> AXUIElement? {
    guard depth < 24 else { return nil }
    if (role == nil || value(e,kAXRoleAttribute) as? String == role), (name == nil || label(e) == name) { return e }
    for child in children(e) { if let found = find(child,role:role,name:name,depth:depth+1) { return found } }
    return nil
}
func bounds(_ e: AXUIElement) throws -> CGRect {
    guard let p=value(e,kAXPositionAttribute), let s=value(e,kAXSizeAttribute),
          CFGetTypeID(p)==AXValueGetTypeID(), CFGetTypeID(s)==AXValueGetTypeID() else { throw CheckFailure.failed("AX window geometry unavailable") }
    var point=CGPoint.zero, size=CGSize.zero
    try require(AXValueGetValue(p as! AXValue,.cgPoint,&point) && AXValueGetValue(s as! AXValue,.cgSize,&size),"Invalid AX geometry")
    return CGRect(origin:point,size:size)
}
func frontmost() -> Bool { NSWorkspace.shared.frontmostApplication?.processIdentifier==pid && target?.isActive==true }
func owns(_ point: CGPoint) -> Bool {
    guard let windows=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] else { return false }
    for w in windows {
        guard (w[kCGWindowLayer as String] as? Int)==0, (w[kCGWindowAlpha as String] as? Double ?? 0)>0,
              let raw=w[kCGWindowBounds as String] as? [String:Any], let rect=CGRect(dictionaryRepresentation:raw as CFDictionary),rect.contains(point) else { continue }
        return (w[kCGWindowOwnerPID as String] as? Int)==Int(pid)
    }
    return false
}
func mouse(_ kind: CGEventType, _ point: CGPoint, flags: CGEventFlags = []) throws {
    try require(frontmost(),"Foreground ownership lost before input")
    try require(owns(point),"Private window does not own the pointer point")
    guard let event=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else { throw CheckFailure.failed("Cannot construct CG mouse event") }
    event.flags=flags;event.setIntegerValueField(.mouseEventClickState,value:1);event.post(tap:.cghidEventTap)
    lastPoint=point
    if kind == .leftMouseDown { held=true }
    if kind == .leftMouseUp { held=false }
}
func settlePointer(_ point: CGPoint) throws {
    try mouse(.mouseMoved,point)
    pump(0.18)
    guard let observed=CGEvent(source:nil)?.location else { throw CheckFailure.failed("Cannot read settled pointer") }
    try require(hypot(observed.x-point.x,observed.y-point.y)<1,"Pointer did not settle at the calibrated source")
    pointerChecks.append(["requested":[point.x,point.y],"observed":[observed.x,observed.y]])
}
func click(_ point: CGPoint, flags: CGEventFlags = []) throws {
    try settlePointer(point)
    try mouse(.leftMouseDown,point,flags:flags);pump(0.1);try mouse(.leftMouseUp,point,flags:flags);pump(0.25)
}
func drag(_ start: CGPoint, _ end: CGPoint) throws {
    try settlePointer(start)
    try mouse(.leftMouseDown,start);pump(0.12)
    for i in 1...24 {
        let t=CGFloat(i)/24
        try mouse(.leftMouseDragged,CGPoint(x:start.x+(end.x-start.x)*t,y:start.y+(end.y-start.y)*t));pump(0.035)
    }
    try mouse(.leftMouseUp,end);pump(0.35)
}
func marker(_ name: String) throws {
    while !FileManager.default.fileExists(atPath:out.appending(path:"ui/\(name).png").path) {
        try require(Date()<deadline && target?.isTerminated==false,"Timeout/exit awaiting \(name)");pump()
    }
    pump(0.5) // The harness hides its window after the snapshot.
}
func publish() {
    report["observations"]=observations
    report["pointerChecks"]=pointerChecks
    if let data=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]) {
        try? data.write(to:out.appending(path:"foreground.json"))
    }
}

do {
    defer {
        if held, frontmost(), let event=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left) { event.post(tap:.cghidEventTap) }
        let current=NSWorkspace.shared.frontmostApplication
        let authorization=(current?.bundleIdentifier ?? "").lowercased().contains("securityagent")
        if !authorization, current?.processIdentifier==pid || current?.processIdentifier==previous?.processIdentifier {
            if let oldPointer { CGWarpMouseCursorPosition(oldPointer) }
            report["restoredPrevious"]=previous?.activate(options:[.activateIgnoringOtherApps]) ?? true
        } else { report["restoredPrevious"]=false }
        report["targetExited"]=target?.isTerminated ?? true
        publish()
    }
    do {
        // A directly spawned app may not yet be registered with Launch Services.
        // Resolve its identity before any activation/input, within a fixed limit.
        for _ in 0..<100 where target?.executableURL == nil {
            pump(0.1)
            target = NSRunningApplication(processIdentifier: pid)
        }
        try require(target?.executableURL?.standardizedFileURL==executable,"Owned PID executable mismatch")
        try require((target?.bundleIdentifier ?? "").hasPrefix("com.vixenlights.verification."),"Non-private Cocoa domain")
        try require(!(previous?.bundleIdentifier ?? "").lowercased().contains("securityagent"),"Authorization window is foreground; no input sent")
        try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Existing AX/CG permissions unavailable; no prompts requested")
        try marker("ready")
        guard let image=NSBitmapImageRep(data:try Data(contentsOf:out.appending(path:"ui/ready.png"))) else { throw CheckFailure.failed("Missing calibration image") }
        // Both observed rasters have identical 1100x780-point layout. The
        // host now produces 1x backing pixels instead of its earlier 2x raster.
        let scale = Double(image.pixelsWide) / 1100
        try require([1.0, 2.0].contains(scale) && Double(image.pixelsHigh) == 780 * scale,"Unexpected calibration raster size")
        report["observedRasterScale"] = scale
        try require(target?.activate(options:[.activateIgnoringOtherApps])==true,"Activation failed")
        for _ in 0..<25 where !frontmost() { pump() }
        try require(frontmost(),"Private app did not become foreground")
        let app=AXUIElementCreateApplication(pid);AXUIElementSetMessagingTimeout(app,1)
        guard let window=(value(app,kAXWindowsAttribute) as? [AXUIElement])?.first(where:{label($0).contains("Library Drag")}) else { throw CheckFailure.failed("No Library Drag AX window") }
        guard let screen=NSScreen.screens.first else { throw CheckFailure.failed("No screen") }
        let width=screen.visibleFrame.width,height=screen.visibleFrame.height
        try require(width>=1140 && height>=820,"Screen cannot contain calibrated private window")
        var position=CGPoint(x:screen.visibleFrame.minX+20,y:screen.frame.maxY-screen.visibleFrame.maxY+20)
        guard let positionValue=AXValueCreate(.cgPoint,&position) else { throw CheckFailure.failed("Position encoding failed") }
        func show(_ stage: String) throws -> CGRect {
            try require(frontmost(),"Foreground ownership lost before \(stage)")
            try require(AXUIElementSetAttributeValue(window,kAXPositionAttribute as CFString,positionValue) == .success,"Cannot place private window")
            _=AXUIElementPerformAction(window,kAXRaiseAction as CFString)
            _=AXUIElementSetAttributeValue(window,kAXMainAttribute as CFString,kCFBooleanTrue)
            pump(0.2)
            let key=element(value(app,kAXFocusedWindowAttribute)),main=element(value(app,kAXMainWindowAttribute))
            try require(key.map{CFEqual($0,window)}==true && main.map{CFEqual($0,window)}==true,"Library window is not actual key/main")
            let rect=try bounds(window)
            try require(abs(rect.width-1100)<1 && abs(rect.height-780)<1,"Window differs from snapshot calibration")
            func actualFrameMatches() -> Bool {
                guard let records=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] else { return false }
                return records.contains { record in
                    guard (record[kCGWindowOwnerPID as String] as? Int)==Int(pid),
                          (record[kCGWindowLayer as String] as? Int)==0,
                          let raw=record[kCGWindowBounds as String] as? [String:Any],
                          let actual=CGRect(dictionaryRepresentation:raw as CFDictionary) else { return false }
                    return abs(actual.minX-rect.minX)<1 && abs(actual.minY-rect.minY)<1 &&
                           abs(actual.width-rect.width)<1 && abs(actual.height-rect.height)<1
                }
            }
            for _ in 0..<15 where !actualFrameMatches() { pump(0.05) }
            try require(actualFrameMatches(),"WindowServer frame has not settled at AX calibration")
            pump(0.15)
            observations.append(["stage":stage,"active":target?.isActive ?? false,"frontmostPID":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1,
                                 "key":key.map(label) ?? "","main":main.map(label) ?? "","frame":[rect.minX,rect.minY,rect.width,rect.height]])
            return rect
        }
        func point(_ frame: CGRect,_ x: CGFloat,_ y: CGFloat)->CGPoint { CGPoint(x:frame.minX+x,y:frame.minY+y) }
        if edgeMode {
            report["edgeCases"] = ["reentry-five", "boundary-ten", "boundary-eleven"]
            for (name, delta) in [("reentry-five", 5.0), ("boundary-ten", 10.0), ("boundary-eleven", 11.0)] {
                let frame = try show(name)
                let start = point(frame,875.75,200)
                let outside = point(frame,800,350)
                let entry = point(frame,875.75,350)
                let drop = point(frame,875.75+delta,350)
                try click(start)
                try settlePointer(start)
                try mouse(.leftMouseDown,start); pump(0.12)
                try mouse(.leftMouseDragged,point(frame,887.75,200)); pump(0.2)
                try mouse(.leftMouseDragged,outside); pump(0.2)
                try mouse(.leftMouseDragged,entry); pump(0.2)
                try mouse(.leftMouseDragged,drop); pump(0.2)
                try mouse(.leftMouseUp,drop); pump(0.35)
                try marker(name)
            }
        } else {
        // Coordinates from the reviewed 2200x1560 native raster, at 2 pixels/point.
        // Panel cells: black interior x1706...1797,1832...1923,1957...2048,2083...2174 at y400.
        var frame=try show("select-two")
        try click(point(frame,1001.25,200));try click(point(frame,1064.25,200),flags:.maskCommand)
        report["calibration"]=["windowPoints":[1100,780],"panelCentersX":[875.75,938.75,1001.25,1064.25],"panelY":200,"toolbarA":[20.75,78]]
        try marker("selected-two");frame=try show("drag-two-left")
        try drag(point(frame,1001.25,200),point(frame,875.75,200))
        try marker("moved-left");frame=try show("drag-selection-to-empty")
        // Locate C and D in the actual persisted order, read-only, supplied by the outer runner.
        let checkpointData=try Data(contentsOf:out.appending(path:"live-checkpoints.json"))
        let checkpoints=try JSONSerialization.jsonObject(with:checkpointData) as! [String:Any]
        guard let checkpoint=checkpoints["moved-left"] as? [String:Any],let order=checkpoint["order"] as? [String],
              let c=order.firstIndex(of:"Curve C"),let d=order.firstIndex(of:"Curve D"),order.count==4 else { throw CheckFailure.failed("Missing actual post-drag order") }
        let centers:[CGFloat]=[875.75,938.75,1001.25,1064.25]
        try click(point(frame,centers[c],200));try click(point(frame,centers[d],200),flags:.maskCommand)
        try drag(point(frame,centers[c],200),point(frame,1000,400))
        try marker("moved-end");frame=try show("toolbar-copy-to-panel")
        let endData=try Data(contentsOf:out.appending(path:"live-checkpoints.json"))
        let endCheck=try JSONSerialization.jsonObject(with:endData) as! [String:Any]
        guard let endState=endCheck["moved-end"] as? [String:Any],let endOrder=endState["order"] as? [String],
              let a=endOrder.firstIndex(of:"Curve A"),let b=endOrder.firstIndex(of:"Curve B") else { throw CheckFailure.failed("Missing actual move-to-end order") }
        let stripCenters:[CGFloat]=[20.75,37.75,54.75,71.75]
        report["panelToToolbar"] = panelToToolbar
        if panelToToolbar {
            try drag(point(frame,centers[a],200),point(frame,stripCenters[b],78))
        } else {
            try drag(point(frame,stripCenters[a],78),point(frame,centers[b],200))
        }
        pump(0.5)
        let sheet=find(window,role:kAXSheetRole)
        let prompt=sheet.flatMap{find($0,name:"Curve name?")}
        report["toolbarNamingPromptObserved"]=prompt != nil
        if let sheet, prompt != nil,let field=find(sheet,role:kAXTextFieldRole),let okay=find(sheet,role:kAXButtonRole,name:"OK") {
            try require(frontmost(),"Foreground lost before name entry")
            try require(AXUIElementSetAttributeValue(field,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"Cannot focus private name field")
            pump(0.15)
            func fieldOwnsInput() -> Bool {
                guard frontmost(), let focused=element(value(app,kAXFocusedUIElementAttribute)) else { return false }
                return CFEqual(focused,field)
            }
            try require(fieldOwnsInput(),"Private name field did not acquire actual text focus")
            func sendKey(_ code: CGKeyCode, flags: CGEventFlags = [], unicode: String? = nil) throws {
                for down in [true,false] {
                    try require(fieldOwnsInput(),"Text focus changed before name input")
                    guard let key=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else { throw CheckFailure.failed("Cannot create name key") }
                    key.flags=flags
                    if let unicode {
                        let units=Array(unicode.utf16)
                        units.withUnsafeBufferPointer { key.keyboardSetUnicodeString(stringLength:units.count,unicodeString:$0.baseAddress!) }
                    }
                    key.post(tap:.cghidEventTap);pump(0.03)
                }
            }
            try sendKey(0,flags:.maskCommand)
            try sendKey(0,unicode:"Imported Curve")
            for _ in 0..<20 where value(field,kAXValueAttribute) as? String != "Imported Curve" { pump(0.05) }
            try require(value(field,kAXValueAttribute) as? String == "Imported Curve","Private name text did not match after actual key events")
            // Let SwiftUI consume the editing notification before invoking its button.
            pump(0.2)
            report["nameInputMethod"]="Focused CG Unicode keys with AX readback and editing settle"
            report["enteredName"]=value(field,kAXValueAttribute) as? String
            try require(frontmost(),"Foreground lost before name acceptance")
            try require(AXUIElementPerformAction(okay,kAXPressAction as CFString) == .success,"Cannot accept private library name")
        }
        try marker("toolbar-copy")
        }
        report["inputCompleted"]=true
        while target?.isTerminated==false && Date()<deadline { pump() }
        try require(target?.isTerminated==true,"Private app did not exit")
        report["passed"]=true
    } catch {
        report["error"]=String(describing:error)
        if target?.isTerminated==false { kill(pid,SIGTERM);for _ in 0..<30 where target?.isTerminated==false { pump() } }
    }
}
