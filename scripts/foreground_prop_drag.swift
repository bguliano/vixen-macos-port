// Bounded owned-process WindowServer drag driver. No application callbacks.
import AppKit
import ApplicationServices
import Foundation

enum CheckFailure: Error { case failed(String) }
func require(_ condition: Bool, _ message: String) throws { if !condition { throw CheckFailure.failed(message) } }
let args=CommandLine.arguments
guard args.count==4,let pid=Int32(args[1]) else { fatalError("PID, output directory, exact private executable required") }
let out=URL(fileURLWithPath:args[2]),executable=URL(fileURLWithPath:args[3]).standardizedFileURL
let previous=NSWorkspace.shared.frontmostApplication,oldPointer=CGEvent(source:nil)?.location
let deadline=Date().addingTimeInterval(135)
var target=NSRunningApplication(processIdentifier:pid)
var held=false,lastPoint=CGPoint.zero
var report:[String:Any]=["passed":false,"targetPID":pid,"previousPID":previous?.processIdentifier ?? -1,"scope":"Owned AX-calibrated CG drags; no selection/drop callbacks"]
var drags:[[String:Any]]=[],pointerChecks:[[String:Any]]=[],focus:[[String:Any]]=[]
func pump(_ seconds:Double=0.1) { RunLoop.current.run(until:Date().addingTimeInterval(seconds)) }
func value(_ e:AXUIElement,_ name:String)->CFTypeRef? {
    guard Date()<deadline else{return nil}
    var owner:pid_t=0
    guard AXUIElementGetPid(e,&owner) == .success,owner==pid else{return nil}
    AXUIElementSetMessagingTimeout(e,0.3)
    var v:CFTypeRef?
    return AXUIElementCopyAttributeValue(e,name as CFString,&v) == .success ? v:nil
}
func child(_ v:CFTypeRef?)->AXUIElement? {
    guard let v,CFGetTypeID(v)==AXUIElementGetTypeID() else{return nil};return (v as! AXUIElement)
}
func children(_ e:AXUIElement)->[AXUIElement] { value(e,kAXChildrenAttribute) as? [AXUIElement] ?? [] }
func labels(_ e:AXUIElement)->[String] { [kAXTitleAttribute,kAXValueAttribute,kAXDescriptionAttribute].compactMap{value(e,$0) as? String}.filter{!$0.isEmpty} }
func rect(_ e:AXUIElement)throws->CGRect {
    guard let p=value(e,kAXPositionAttribute),let s=value(e,kAXSizeAttribute),CFGetTypeID(p)==AXValueGetTypeID(),CFGetTypeID(s)==AXValueGetTypeID() else{throw CheckFailure.failed("AX geometry missing")}
    var point=CGPoint.zero,size=CGSize.zero
    try require(AXValueGetValue(p as! AXValue,.cgPoint,&point) && AXValueGetValue(s as! AXValue,.cgSize,&size),"Invalid AX geometry")
    return CGRect(origin:point,size:size)
}
struct Node { let element:AXUIElement;let role:String;let labels:[String];let frame:CGRect;let rowFrame:CGRect? }
func scan(_ e:AXUIElement,_ depth:Int=0,_ row:CGRect?=nil,_ budget:inout Int)->[Node] {
    guard depth<26 && budget>0 && Date()<deadline else{return []};budget-=1
    let role=value(e,kAXRoleAttribute) as? String ?? "",frame=(try? rect(e)) ?? .zero
    let rowFrame=role==kAXRowRole ? frame:row
    var result=[Node(element:e,role:role,labels:labels(e),frame:frame,rowFrame:rowFrame)]
    for c in children(e){result += scan(c,depth+1,rowFrame,&budget)};return result
}
func frontmost()->Bool { NSWorkspace.shared.frontmostApplication?.processIdentifier==pid && target?.isActive==true }
func owns(_ point:CGPoint)->Bool {
    guard let list=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] else{return false}
    for item in list {
        guard item[kCGWindowLayer as String] as? Int==0,(item[kCGWindowAlpha as String] as? Double ?? 0)>0,
              let raw=item[kCGWindowBounds as String] as? [String:Any],let frame=CGRect(dictionaryRepresentation:raw as CFDictionary),frame.contains(point) else{continue}
        return item[kCGWindowOwnerPID as String] as? Int==Int(pid)
    };return false
}
func mouse(_ type:CGEventType,_ point:CGPoint,_ flags:CGEventFlags=[])throws {
    try require(Date()<deadline && frontmost(),"Deadline or foreground ownership lost")
    try require(owns(point),"Owned window does not cover pointer point")
    guard let event=CGEvent(mouseEventSource:nil,mouseType:type,mouseCursorPosition:point,mouseButton:.left) else{throw CheckFailure.failed("CG event creation failed")}
    event.flags=flags;event.setIntegerValueField(.mouseEventClickState,value:1);event.post(tap:.cghidEventTap);lastPoint=point
    if type == .leftMouseDown{held=true};if type == .leftMouseUp{held=false}
}
func settle(_ point:CGPoint)throws {
    try mouse(.mouseMoved,point);pump(0.18)
    guard let observed=CGEvent(source:nil)?.location else{throw CheckFailure.failed("Pointer readback unavailable")}
    try require(hypot(observed.x-point.x,observed.y-point.y)<1,"Pointer failed to settle")
    pointerChecks.append(["requested":[point.x,point.y],"observed":[observed.x,observed.y]])
}
func click(_ point:CGPoint,_ flags:CGEventFlags=[])throws {
    try settle(point);try mouse(.leftMouseDown,point,flags);pump(0.08);try mouse(.leftMouseUp,point,flags);pump(0.22)
}
func drag(_ start:CGPoint,_ end:CGPoint,_ flags:CGEventFlags=[])throws {
    try settle(start);try mouse(.leftMouseDown,start,flags);pump(0.15)
    // First move horizontally beyond native drag threshold, then approach the target.
    let elbow=CGPoint(x:start.x+18,y:start.y)
    for i in 1...6{let t=CGFloat(i)/6;try mouse(.leftMouseDragged,CGPoint(x:start.x+18*t,y:start.y),flags);pump(0.035)}
    for i in 1...24{let t=CGFloat(i)/24;try mouse(.leftMouseDragged,CGPoint(x:elbow.x+(end.x-elbow.x)*t,y:elbow.y+(end.y-elbow.y)*t),flags);pump(0.035)}
    pump(0.35);try mouse(.leftMouseUp,end,flags);pump(0.55)
}
func marker(_ name:String)throws {
    while !FileManager.default.fileExists(atPath:out.appending(path:"ui/\(name).png").path) {
        try require(Date()<deadline && target?.isTerminated==false,"Timeout/exit awaiting \(name)");pump()
    };pump(0.5)
}
func publish(){
    report["drags"]=drags;report["pointerChecks"]=pointerChecks;report["focus"]=focus
    if let data=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]){try? data.write(to:out.appending(path:"foreground.json"))}
}
func selected(_ panel:AXUIElement)->[String] {
    let rows=value(panel,kAXSelectedRowsAttribute) as? [AXUIElement] ?? []
    var names:Set<String>=[]
    for row in rows{var budget=80;for n in scan(row,0,nil,&budget){for s in n.labels where s.hasPrefix("Pixel "){names.insert(s)}}}
    return names.sorted()
}

do {
    defer {
        if held,frontmost(),let event=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left){event.post(tap:.cghidEventTap)}
        let current=NSWorkspace.shared.frontmostApplication
        if !(current?.bundleIdentifier ?? "").lowercased().contains("securityagent"),current?.processIdentifier==pid || current?.processIdentifier==previous?.processIdentifier {
            if let oldPointer{report["restoredPointer"]=CGWarpMouseCursorPosition(oldPointer) == .success}
            _=previous?.activate(options:[.activateIgnoringOtherApps]);for _ in 0..<15 where NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier{pump()}
        }else{report["restoredPointer"]=false;report["restoreSkippedUnexpectedForeground"]=true}
        report["restoredPrevious"]=NSWorkspace.shared.frontmostApplication?.processIdentifier==previous?.processIdentifier
        report["targetExited"]=target?.isTerminated ?? true;publish()
    }
    do {
        for _ in 0..<100 where target==nil || target?.executableURL==nil{pump();target=NSRunningApplication(processIdentifier:pid)}
        try require(target?.executableURL?.standardizedFileURL==executable,"Owned executable mismatch")
        try require((target?.bundleIdentifier ?? "").hasPrefix("com.vixenlights.verification."),"Non-private bundle")
        try require(!(previous?.bundleIdentifier ?? "").lowercased().contains("securityagent"),"Authorization is foreground; no input")
        try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Existing AX/CG access unavailable; no prompt requested")
        let prepared=try JSONSerialization.jsonObject(with:Data(contentsOf:out.appending(path:"prepared.json"))) as! [String:Any]
        let cases=prepared["cases"] as! [[String:Any]]
        let app=AXUIElementCreateApplication(pid);AXUIElementSetMessagingTimeout(app,0.3)
        try marker(cases[0]["id"] as! String + "-ready")
        try require(target?.activate(options:[.activateIgnoringOtherApps])==true,"Private activation failed")
        for _ in 0..<25 where !frontmost(){pump()};try require(frontmost(),"Private app not foreground")
        guard let screen=NSScreen.screens.first else{throw CheckFailure.failed("No display")}
        for test in cases {
            let id=test["id"] as! String,tab=test["tab"] as! String
            try marker(id+"-ready")
            try require(frontmost(),"Foreground lost before \(id)")
            guard let window=(value(app,kAXWindowsAttribute) as? [AXUIElement])?.first(where:{labels($0).contains(where:{$0.contains("Prop Drag Verification")})}) else{throw CheckFailure.failed("No private prop AX window")}
            var position=CGPoint(x:screen.visibleFrame.minX+20,y:screen.frame.maxY-screen.visibleFrame.maxY+20)
            guard let v=AXValueCreate(.cgPoint,&position) else{throw CheckFailure.failed("Position encoding")}
            try require(AXUIElementSetAttributeValue(window,kAXPositionAttribute as CFString,v) == .success,"Cannot place owned prop window")
            _=AXUIElementPerformAction(window,kAXRaiseAction as CFString);_=AXUIElementSetAttributeValue(window,kAXMainAttribute as CFString,kCFBooleanTrue);pump(0.35)
            let key=child(value(app,kAXFocusedWindowAttribute)),main=child(value(app,kAXMainWindowAttribute))
            try require(key.map{CFEqual($0,window)}==true && main.map{CFEqual($0,window)}==true,"Private prop not key/main")
            let windowFrame=try rect(window)
            try require(windowFrame.width<=screen.visibleFrame.width-20 && windowFrame.height<=screen.visibleFrame.height-20,"Private prop exceeds visible display")
            func frameSettled()->Bool {
                guard let records=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] else{return false}
                return records.contains { item in
                    guard item[kCGWindowOwnerPID as String] as? Int==Int(pid),item[kCGWindowLayer as String] as? Int==0,
                          let raw=item[kCGWindowBounds as String] as? [String:Any],let r=CGRect(dictionaryRepresentation:raw as CFDictionary) else{return false}
                    return abs(r.minX-windowFrame.minX)<1 && abs(r.minY-windowFrame.minY)<1 && abs(r.width-windowFrame.width)<1 && abs(r.height-windowFrame.height)<1
                }
            }
            for _ in 0..<15 where !frameSettled(){pump(0.05)}
            try require(frameSettled(),"WindowServer frame did not settle at AX geometry")
            focus.append(["case":id,"frontmostPID":pid,"key":labels(window),"frame":[windowFrame.minX,windowFrame.minY,windowFrame.width,windowFrame.height]])
            var budget=1200;let all=scan(window,0,nil,&budget)
            let role=tab=="layout" ? kAXOutlineRole:kAXTableRole
            let panels=all.filter{$0.role==role && $0.frame.width>80 && $0.frame.height>50}
            guard panels.count==1 else{report["axCandidates"]=all.map{["role":$0.role,"labels":$0.labels,"frame":[$0.frame.minX,$0.frame.minY,$0.frame.width,$0.frame.height]]};throw CheckFailure.failed("Expected one \(role), got \(panels.count)")}
            let panel=panels[0];budget=500
            // Native outline/table rows may be exposed through AXRows rather
            // than the ordinary view-children hierarchy.
            let nativeRows=value(panel.element,kAXRowsAttribute) as? [AXUIElement] ?? []
            let nodes=nativeRows.isEmpty ? scan(panel.element,0,nil,&budget) : nativeRows.flatMap{scan($0,0,nil,&budget)}
            func row(_ name:String)throws->Node {
                let matches=nodes.filter{$0.labels.contains(name) && $0.frame.width>15 && $0.frame.height>5 && $0.frame.height<50}
                guard let found=matches.min(by:{$0.frame.width*$0.frame.height<$1.frame.width*$1.frame.height}) else{report["axCandidates"]=nodes.map{["role":$0.role,"labels":$0.labels,"frame":[$0.frame.minX,$0.frame.minY,$0.frame.width,$0.frame.height]]};throw CheckFailure.failed("No visible row \(name)")}
                return found
            }
            func point(_ node:Node)->CGPoint{CGPoint(x:node.frame.midX,y:node.frame.midY)}
            let selections=test["select"] as! [String]
            for (i,name) in selections.enumerated(){try click(point(try row(name)),i==0 ? []:.maskCommand)}
            let selectedBefore=selected(panel.element)
            if !selections.isEmpty{try require(selectedBefore==selections.sorted(),"Physical selection mismatch \(id): \(selectedBefore)")}
            let source=try row(test["source"] as! String),destination=try row(test["target"] as! String)
            let start=point(source);var end=point(destination)
            if let edge=test["edge"] as? String {let r=destination.rowFrame ?? destination.frame;end.y=edge=="before" ? r.minY+1:r.maxY+2}
            try require(panel.frame.contains(start) && panel.frame.contains(end),"Calibrated points outside panel")
            try drag(start,end,(test["option"] as? Bool)==true ? .maskAlternate:[])
            drags.append(["case":id,"source":test["source"]!,"target":test["target"]!,"start":[start.x,start.y],"end":[end.x,end.y],"selectedBefore":selectedBefore,"selectedAfter":selected(panel.element),"option":test["option"] as? Bool ?? false]);publish()
            try marker(id+"-after")
        }
        while target?.isTerminated==false && Date()<deadline{pump()}
        try require(target?.isTerminated==true,"Owned harness did not exit")
        report["passed"]=true
    }catch{report["error"]=String(describing:error)}
}
exit((report["passed"] as? Bool)==true ? 0:1)
