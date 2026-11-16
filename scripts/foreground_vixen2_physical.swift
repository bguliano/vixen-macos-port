// Normal-app Vixen2 verification: actual OS drags, native picker controls, no UI_TEST hooks.
// Reuses the reviewed registration/ownership/cleanup policies from the Mapper and native-panel drivers.
import AppKit
import ApplicationServices
import Foundation

enum Stop: Error { case failed(String) }
func need(_ value: Bool, _ text: String) throws { if !value { throw Stop.failed(text) } }
func now() -> Double { ProcessInfo.processInfo.systemUptime }
func pump(_ seconds: Double = 0.05) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func write(_ value: [String: Any], _ url: URL) throws { try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys]).write(to: url, options: .atomic) }
func identity(_ app: NSRunningApplication?) -> [String: Any] { ["pid":Int(app?.processIdentifier ?? -1), "bundleIdentifier":app?.bundleIdentifier ?? "", "executable":app?.executableURL?.standardizedFileURL.path ?? ""] }
func physical() -> (Set<CGKeyCode>, Set<CGMouseButton.RawValue>, CGEventFlags) {
    let keys=Set((0..<128).map{CGKeyCode($0)}.filter{CGEventSource.keyState(.combinedSessionState,key:$0)})
    let buttons=Set([CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}.map(\.rawValue))
    let flags=CGEventSource.flagsState(.combinedSessionState).intersection([.maskCommand,.maskShift,.maskControl,.maskAlternate,.maskSecondaryFn])
    return (keys,buttons,flags)
}
func clearPhysical() -> Bool { let p=physical();return p.0.isEmpty && p.1.isEmpty && p.2.isEmpty }
func converterSignature(_ labels:Set<String>,path:String)->Bool { labels.contains("Channel Maps:") && labels.contains("Vixen 2 Sequence:") && labels.contains("Create New Map") && labels.contains(path) }
func hitOwnership(status:Int,owner:Int,expectedOwner:Int,rootReached:Bool)->Bool { status==0 && owner==expectedOwner && rootReached }
func mappingSignature(_ labels:Set<String>)->Bool { ["Vixen 2 channel mappings","Vixen 2 destination elements","Set Import Color"].allSatisfy{labels.contains($0)} }
func idleRestore(time:Double,limit:Double,identity:Bool,front:Bool,pointer:Bool,clear:Bool,lost:Bool) -> Bool { time<limit && identity && front && pointer && clear && !lost }
if CommandLine.arguments.count==2 && CommandLine.arguments[1]=="--guard-checks" {
    let restoreCases:[(String,Bool)]=[("idle restore",idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:true,lost:false)),("expired",!idleRestore(time:2,limit:2,identity:true,front:true,pointer:true,clear:true,lost:false)),("replaced prior",!idleRestore(time:1,limit:2,identity:false,front:true,pointer:true,clear:true,lost:false)),("foreign foreground",!idleRestore(time:1,limit:2,identity:true,front:false,pointer:true,clear:true,lost:false)),("pointer interference",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:false,clear:true,lost:false)),("held input",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:false,lost:false)),("ownership lost",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:true,lost:true))]
    let signatures:[(String,Bool)]=[("converter exact path",converterSignature(["Channel Maps:","Vixen 2 Sequence:","Create New Map","/private/fixture.vix"],path:"/private/fixture.vix")),("converter wrong path refused",!converterSignature(["Channel Maps:","Vixen 2 Sequence:","Create New Map","/other.vix"],path:"/private/fixture.vix")),("converter incomplete controls refused",!converterSignature(["Create New Map","/private/fixture.vix"],path:"/private/fixture.vix")),("mapper signature",mappingSignature(["Vixen 2 channel mappings","Vixen 2 destination elements","Set Import Color"])),("mapper missing destination refused",!mappingSignature(["Vixen 2 channel mappings","Set Import Color"]))]
    func record(_ owner:Int,_ number:Int,_ layer:Int,_ alpha:Double=1,_ x:Double=10)->[String:Any] { [kCGWindowOwnerPID as String:owner,kCGWindowNumber as String:number,kCGWindowLayer as String:layer,kCGWindowAlpha as String:alpha,kCGWindowBounds as String: CGRect(x:x,y:10,width:100,height:100).dictionaryRepresentation] }
    let panel=record(11,101,3),underlying=record(22,202,0),point=CGPoint(x:60,y:60),frame=CGRect(x:10,y:10,width:100,height:100)
    let nativeChecks:[(String,Bool)]=[("floating panel exact frame included",matchingNative([panel,underlying],owner:11,frame:frame).count==1 && topNative([panel,underlying],at:point)?[kCGWindowNumber as String] as? Int==101),("foreign higher window remains blocking",topNative([record(22,303,8),panel],at:point)?[kCGWindowOwnerPID as String] as? Int==22),("wrong frame refused",matchingNative([record(11,101,3,1,200)],owner:11,frame:frame).isEmpty),("transparent native window ignored",topNative([record(22,303,8,0),panel],at:point)?[kCGWindowNumber as String] as? Int==101)]
    let hitChecks:[(String,Bool)]=[("actual root hit accepted",hitOwnership(status:0,owner:11,expectedOwner:11,rootReached:true)),("foreign actual hit refused",!hitOwnership(status:0,owner:22,expectedOwner:11,rootReached:true)),("same-app wrong root refused",!hitOwnership(status:0,owner:11,expectedOwner:11,rootReached:false)),("AX hit failure refused",!hitOwnership(status:-25204,owner:11,expectedOwner:11,rootReached:true))]
    let cases=restoreCases+signatures+nativeChecks+hitChecks
    let result:[String:Any]=["passed":cases.allSatisfy{$0.1},"nativeOperations":0,"checks":cases.map{["name":$0.0,"passed":$0.1] as [String:Any]}]
    print(String(data:try JSONSerialization.data(withJSONObject:result,options:.sortedKeys),encoding:.utf8)!);exit(cases.allSatisfy{$0.1} ? 0:1)
}
if CommandLine.arguments.count==2 && CommandLine.arguments[1]=="--desktop" {
    let point=CGEvent(source:nil)?.location
    let result:[String:Any]=["application":identity(NSWorkspace.shared.frontmostApplication),"pointer":point.map{[$0.x,$0.y]} ?? [],"accessibility":AXIsProcessTrusted(),"eventPosting":CGPreflightPostEventAccess(),"noPhysicalInputHeld":clearPhysical()]
    print(String(data:try JSONSerialization.data(withJSONObject:result,options:.sortedKeys),encoding:.utf8)!);exit(0)
}
guard CommandLine.arguments.count==2 else{exit(2)}
let cfg=try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1]))) as! [String:Any]
let pid=pid_t(cfg["pid"] as! Int), out=URL(fileURLWithPath:cfg["out"] as! String)
let executable=cfg["executable"] as! String,bundle=cfg["bundleIdentifier"] as! String,previous=cfg["previousApplication"] as! [String:Any]
let discovery=cfg["discovery"] as? Bool == true
let oldPointer=cfg["pointer"] as! [Double], began=now(),inputEnd=began+(discovery ? 60:120),cleanupEnd=began+(discovery ? 75:135)
let app=AXUIElementCreateApplication(pid),systemAX=AXUIElementCreateSystemWide()
var currentRoot:AXUIElement?, companionPanel:AXUIElement?, companionOwner:AXUIElement?, currentNativeID:Int?
var mouseHeld=false,heldKeys=Set<CGKeyCode>(),downs=0,ups=0,keyDowns=0,keyUps=0,ownershipLost=false,cleanupReading=false
var lastPoint=oldPointer.count==2 ? CGPoint(x:oldPointer[0],y:oldPointer[1]):.zero
var events=[[String:Any]](),observations=[[String:Any]](),report:[String:Any]=["passed":false,"mode":"normal-app-no-harness","pid":Int(pid)]
func limit()->Double{cleanupReading ? cleanupEnd:inputEnd}
func owned(_ e:AXUIElement)->Bool{var p:pid_t = -1;return AXUIElementGetPid(e,&p) == .success && p==pid}
func attr(_ e:AXUIElement,_ name:String)->CFTypeRef?{guard now()<limit(),owned(e) else{return nil};AXUIElementSetMessagingTimeout(e,0.15);var v:CFTypeRef?;return AXUIElementCopyAttributeValue(e,name as CFString,&v) == .success ? v:nil}
func ax(_ v:CFTypeRef?)->AXUIElement?{guard let v,CFGetTypeID(v)==AXUIElementGetTypeID() else{return nil};return (v as! AXUIElement)}
func str(_ e:AXUIElement,_ name:String)->String{attr(e,name) as? String ?? ""}
func children(_ e:AXUIElement,_ key:String=kAXChildrenAttribute)->[AXUIElement]{attr(e,key) as? [AXUIElement] ?? []}
func same(_ a:AXUIElement?,_ b:AXUIElement?)->Bool{guard let a,let b else{return false};return CFEqual(a,b)}
func labels(_ e:AXUIElement)->[String]{[kAXTitleAttribute,kAXDescriptionAttribute,kAXValueAttribute].map{str(e,$0)}}
func tree(_ root:AXUIElement,prune:Bool=false)->[AXUIElement]{var result=[AXUIElement]();func walk(_ e:AXUIElement,_ d:Int){guard now()<limit(),d<18,result.count<550,owned(e) else{return};result.append(e);if prune && ["AXTable","AXOutline","AXList","AXBrowser"].contains(str(e,kAXRoleAttribute)){return};for c in children(e){walk(c,d+1)}};walk(root,0);return result}
func rect(_ e:AXUIElement)throws->CGRect{guard let p=attr(e,kAXPositionAttribute),let s=attr(e,kAXSizeAttribute),CFGetTypeID(p)==AXValueGetTypeID(),CFGetTypeID(s)==AXValueGetTypeID() else{throw Stop.failed("AX frame unavailable")};var q=CGPoint.zero,z=CGSize.zero;try need(AXValueGetValue(p as! AXValue,.cgPoint,&q) && AXValueGetValue(s as! AXValue,.cgSize,&z),"AX frame invalid");try need([q.x,q.y,z.width,z.height].allSatisfy(\.isFinite) && z.width>0 && z.height>0,"AX frame empty");return CGRect(origin:q,size:z)}
func targetIdentity()->Bool{guard let a=NSRunningApplication(processIdentifier:pid) else{return false};return !a.isTerminated && a.executableURL?.standardizedFileURL.path==executable && a.bundleIdentifier==bundle}
func priorIdentity()->Bool{guard let n=previous["pid"] as? Int,let a=NSRunningApplication(processIdentifier:pid_t(n)) else{return false};return !a.isTerminated && a.executableURL?.standardizedFileURL.path==previous["executable"] as? String && a.bundleIdentifier==previous["bundleIdentifier"] as? String}
func foreground()->Bool{targetIdentity() && NSRunningApplication(processIdentifier:pid)?.isActive==true && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid}
func priorOrTarget()->Bool{let p=NSWorkspace.shared.frontmostApplication?.processIdentifier;return (p==pid && targetIdentity()) || (priorIdentity() && p==pid_t(previous["pid"] as! Int))}
func pointerStable()->Bool{guard let p=CGEvent(source:nil)?.location else{return false};return hypot(p.x-lastPoint.x,p.y-lastPoint.y)<=3}
func ownFlags()->CGEventFlags{var f=CGEventFlags();if heldKeys.contains(55){f.insert(.maskCommand)};if heldKeys.contains(56){f.insert(.maskShift)};return f}
func physicalOnlyOurs()->Bool{let p=physical();return p.0.isSubset(of:heldKeys) && p.1.allSatisfy{mouseHeld && $0==CGMouseButton.left.rawValue} && p.2.subtracting(ownFlags()).isEmpty}
func guardApp()throws{try need(now()<limit() && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Deadline or cooperative abort");let good=foreground() && pointerStable() && physicalOnlyOurs();if !good{ownershipLost=true};try need(good,"Foreground identity, cursor, or physical input changed");try need(now()<limit(),"Deadline during ownership read")}
func ancestry(_ e:AXUIElement)->[AXUIElement]{var a=[e],n=e;for _ in 0..<12{guard let p=ax(attr(n,kAXParentAttribute)),owned(p) else{break};if ["AXWindow","AXSheet"].contains(str(p,kAXRoleAttribute)){a.append(p)};n=p};return a}
func focusOwned()->Bool{guard let root=currentRoot,let key=ax(attr(app,kAXFocusedWindowAttribute)),let main=ax(attr(app,kAXMainWindowAttribute)) else{return false};let descendants=tree(root,prune:true).filter{["AXWindow","AXSheet"].contains(str($0,kAXRoleAttribute))};let allowed=ancestry(root)+descendants+(companionPanel.map{[$0]} ?? [])+(companionOwner.map{ancestry($0)} ?? []);return allowed.contains{same($0,key)} && allowed.contains{same($0,main)}}
func nativeWindows()->[[String:Any]] { CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] ?? [] }
func nativeFrame(_ record:[String:Any])->CGRect? {
    guard let b=record[kCGWindowBounds as String] as? [String:Any],let r=CGRect(dictionaryRepresentation:b as CFDictionary),[r.minX,r.minY,r.width,r.height].allSatisfy(\.isFinite),r.width>0,r.height>0 else{return nil};return r
}
func sameFrame(_ a:CGRect,_ b:CGRect)->Bool { abs(a.minX-b.minX)<1 && abs(a.minY-b.minY)<1 && abs(a.width-b.width)<1 && abs(a.height-b.height)<1 }
func matchingNative(_ records:[[String:Any]],owner:Int,frame:CGRect)->[[String:Any]] { records.filter{($0[kCGWindowOwnerPID as String] as? Int)==owner && ($0[kCGWindowAlpha as String] as? Double ?? 0)>0 && nativeFrame($0).map{sameFrame($0,frame)}==true} }
func topNative(_ records:[[String:Any]],at point:CGPoint)->[String:Any]? {
    records.first { ($0[kCGWindowAlpha as String] as? Double ?? 0)>0 && nativeFrame($0)?.contains(point)==true }
}
func nativeSummary(_ record:[String:Any])->[String:Any] {
    ["pid":record[kCGWindowOwnerPID as String] ?? NSNull(),"window":record[kCGWindowNumber as String] ?? NSNull(),"owner":record[kCGWindowOwnerName as String] ?? NSNull(),"title":record[kCGWindowName as String] ?? NSNull(),"layer":record[kCGWindowLayer as String] ?? NSNull(),"alpha":record[kCGWindowAlpha as String] ?? NSNull(),"bounds":record[kCGWindowBounds as String] ?? NSNull()]
}
func actualHit(_ point:CGPoint,root:AXUIElement)->(Bool,[String:Any]) {
    guard now()<limit() else{return (false,["error":"hit deadline expired"])}
    AXUIElementSetMessagingTimeout(systemAX,0.15)
    var hit:AXUIElement?
    let status=AXUIElementCopyElementAtPosition(systemAX,Float(point.x),Float(point.y),&hit)
    var evidence:[String:Any]=["point":[point.x,point.y],"status":Int(status.rawValue),"reachedExactRoot":false]
    guard status == .success,let hit,now()<limit() else{return (false,evidence)}
    var owner:pid_t = -1
    let pidStatus=AXUIElementGetPid(hit,&owner);evidence["pidStatus"]=Int(pidStatus.rawValue);evidence["ownerPID"]=Int(owner)
    guard pidStatus == .success,owner==pid else{return (false,evidence)}
    let window=ax(attr(hit,kAXWindowAttribute))
    var reached=same(window,root),chain=[[String:Any]](),node:AXUIElement?=hit
    evidence["windowIsExactRoot"]=reached
    if !reached {
        for _ in 0..<14 {
            guard let current=node,now()<limit(),owned(current) else{break}
            let role=str(current,kAXRoleAttribute)
            chain.append(["role":role,"exactRoot":same(current,root)])
            if same(current,root){reached=true;break}
            if role==kAXApplicationRole{break}
            node=ax(attr(current,kAXParentAttribute))
        }
    }
    evidence["ancestry"]=chain;evidence["reachedExactRoot"]=reached
    return (hitOwnership(status:Int(status.rawValue),owner:Int(owner),expectedOwner:Int(pid),rootReached:reached) && now()<limit(),evidence)
}
func nativeAt(_ point:CGPoint)->Int? {
    guard let root=currentRoot,let number=currentNativeID,let bounds=try? rect(root),bounds.contains(point) else{return nil}
    let matches=matchingNative(nativeWindows(),owner:Int(pid),frame:bounds)
    guard matches.count==1,matches[0][kCGWindowNumber as String] as? Int==number,actualHit(point,root:root).0,now()<limit() else{return nil}
    return number
}

func guardInput(_ point:CGPoint?=nil)throws{try guardApp();let focus=focusOwned();try need(now()<limit(),"Deadline after AX focus");if !focus{ownershipLost=true};try need(focus,"Exact observed window hierarchy lost focus");try guardApp();if let point{try need(nativeAt(point)==currentNativeID && now()<limit(),"Posted point is not in exact native window")}}
func allWindows()->[AXUIElement]{var found=[AXUIElement]();for w in children(app,kAXWindowsAttribute){for e in tree(w,prune:true) where ["AXWindow","AXSheet"].contains(str(e,kAXRoleAttribute)) && !found.contains(where:{same($0,e)}){found.append(e)}};return found}
func wait(_ name:String,_ seconds:Double=5,_ condition:()throws->Bool)throws{let end=min(inputEnd,now()+seconds);while now()<end{try guardApp();if try condition(){return};pump()};throw Stop.failed("Timed out: "+name)}
func findWindow(_ name:String,_ predicate:(AXUIElement)->Bool)throws->AXUIElement {
    var result:AXUIElement?, lastSummary=""
    do { try wait(name,10) {
        let windows=allWindows()
        let summary=windows.map{str($0,kAXRoleAttribute)+":"+str($0,kAXTitleAttribute)}.joined(separator:" | ")
        if summary != lastSummary { observations.append(["stage":"wait-window","name":name,"windows":summary]);lastSummary=summary }
        let candidates=windows.filter(predicate)
        let found=candidates.filter{parent in !candidates.contains{child in !same(parent,child) && ancestry(child).dropFirst().contains{same($0,parent)}}}
        try need(found.count<=1,"Ambiguous "+name);result=found.first;return result != nil
    }} catch {
        if foreground() && pointerStable() && clearPhysical() && now()<inputEnd {
            for (i,w) in allWindows().prefix(8).enumerated(){try? dump("failed-window-\(i)",w)}
        }
        throw error
    }
    return result!
}
func use(_ root:AXUIElement)throws {
    try guardApp();currentRoot=root;currentNativeID=nil
    let end=min(inputEnd,now()+1.2)
    repeat {
        try guardApp();let r=try rect(root),records=nativeWindows(),center=CGPoint(x:r.midX,y:r.midY)
        let matches=matchingNative(records,owner:Int(pid),frame:r)
        let top=topNative(records,at:center),hit=actualHit(center,root:root)
        observations.append(["stage":"root-native-observation","title":str(root,kAXTitleAttribute),"role":str(root,kAXRoleAttribute),"frame":[r.minX,r.minY,r.width,r.height],"matchingNativeWindows":matches.map(nativeSummary),"topAtCenter":top.map(nativeSummary) as Any? ?? NSNull(),"actualAXHit":hit.1])
        try need(matches.count<=1,"Ambiguous exact AX/native root geometry")
        if let match=matches.first,let number=match[kCGWindowNumber as String] as? Int,hit.0 {
            try guardApp();try need(sameFrame(try rect(root),r) && now()<inputEnd,"Root moved during native binding")
            currentNativeID=number
            observations.append(["stage":"root","title":str(root,kAXTitleAttribute),"role":str(root,kAXRoleAttribute),"frame":[r.minX,r.minY,r.width,r.height],"cgWindow":number,"cgLayer":match[kCGWindowLayer as String] ?? NSNull()])
            try guardInput();return
        }
        pump()
    } while now()<end
    throw Stop.failed("Exact native root geometry or actual AX hit ownership did not match; see root-native-observation")
}
func unique(_ root:AXUIElement,_ role:String,_ name:String)throws->AXUIElement{let found=tree(root).filter{str($0,kAXRoleAttribute)==role && labels($0).contains(name)};try need(found.count==1,"Expected unique "+name+"; found \(found.count)");return found[0]}
@discardableResult func action(_ control:AXUIElement,_ verb:String=kAXPressAction,_ stage:String)throws->AXError{try guardInput();try need(owned(control) && attr(control,kAXEnabledAttribute) as? Bool != false,"Disabled or foreign control");var names:CFArray?;AXUIElementCopyActionNames(control,&names);try need((names as? [String] ?? []).contains(verb),"Native control lacks "+verb);try guardInput();let status=AXUIElementPerformAction(control,verb as CFString);observations.append(["stage":stage,"action":verb,"status":status.rawValue]);try need(status == .success || status == .cannotComplete,"Native action failed: \(status.rawValue)");pump(0.1);return status}
func press(_ root:AXUIElement,_ name:String,_ stage:String)throws{try action(unique(root,"AXButton",name),kAXPressAction,stage)}
func set(_ field:AXUIElement,_ value:String,_ stage:String,confirm:Bool=false)throws{
    try guardInput();try need(AXUIElementSetAttributeValue(field,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"Cannot focus field "+stage);pump()
    guard let fresh=ax(attr(app,kAXFocusedUIElementAttribute)),let root=currentRoot else{throw Stop.failed("No focused native field")}
    try need(["AXTextField","AXComboBox"].contains(str(fresh,kAXRoleAttribute)) && tree(root).contains{same($0,fresh)},"Focused field left observed root")
    try guardInput();try need(AXUIElementSetAttributeValue(fresh,kAXValueAttribute as CFString,value as CFString) == .success && str(fresh,kAXValueAttribute)==value,"Field value not accepted "+stage)
    if confirm{guard let latest=ax(attr(app,kAXFocusedUIElementAttribute)) else{throw Stop.failed("Focused field disappeared")};try need(tree(root).contains{same($0,latest)} && str(latest,kAXValueAttribute)==value,"Changed field readback/owner mismatch");try action(latest,kAXConfirmAction,stage)}
    observations.append(["stage":stage,"value":value])
}
func key(_ code:CGKeyCode,_ down:Bool)throws{try guardInput();guard let e=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else{throw Stop.failed("Cannot create key")};if down{try need(!heldKeys.contains(code),"Duplicate key down")}else{try need(heldKeys.contains(code),"Unpaired key up")};var after=heldKeys;if down{after.insert(code)}else{after.remove(code)};var flags=CGEventFlags();if after.contains(55){flags.insert(.maskCommand)};if after.contains(56){flags.insert(.maskShift)};e.flags=flags;try guardInput();if down{heldKeys.insert(code);keyDowns+=1};e.post(tap:.cghidEventTap);if !down{heldKeys.remove(code);keyUps+=1};events.append(["key":Int(code),"down":down]);pump(0.05)}
func chord(_ codes:[CGKeyCode])throws{for c in codes{try key(c,true)};for c in codes.reversed(){try key(c,false)}}
func mouse(_ kind:CGEventType,_ point:CGPoint)throws{try guardInput(point);guard let e=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else{throw Stop.failed("Cannot create mouse")};e.setIntegerValueField(.mouseEventClickState,value:1);try guardInput(point);if kind == .leftMouseDown{try need(!mouseHeld,"Duplicate mouse down");mouseHeld=true;downs+=1};if kind == .leftMouseUp{try need(mouseHeld,"Unpaired mouse up")};e.post(tap:.cghidEventTap);lastPoint=point;if kind == .leftMouseUp{mouseHeld=false;ups+=1};events.append(["kind":kind.rawValue,"point":[point.x,point.y],"window":currentNativeID ?? -1])}
func click(_ point:CGPoint)throws{try mouse(.mouseMoved,point);pump();try mouse(.leftMouseDown,point);pump();try mouse(.leftMouseUp,point);pump(0.12)}
func container(_ root:AXUIElement,_ name:String)throws->AXUIElement{let found=tree(root).filter{["AXTable","AXOutline"].contains(str($0,kAXRoleAttribute)) && labels($0).contains(name)};try need(found.count==1,"Missing unique container "+name);return found[0]}
func row(_ owner:AXUIElement,_ name:String)throws->AXUIElement{let fields=tree(owner).filter{labels($0).contains(name) && str($0,kAXRoleAttribute)=="AXStaticText"};try need(fields.count==1,"Missing unique row text "+name);var n=fields[0];for _ in 0..<6{if str(n,kAXRoleAttribute)=="AXRow"{return n};guard let p=ax(attr(n,kAXParentAttribute)) else{break};n=p};throw Stop.failed("No AX row for "+name)}
func rowPoint(_ owner:AXUIElement,_ name:String)throws->CGPoint{let r=try row(owner,name);let fields=tree(r).filter{str($0,kAXRoleAttribute)=="AXStaticText" && labels($0).contains(name)};try need(fields.count==1,"Ambiguous observed row field");let b=try rect(fields[0]);return CGPoint(x:b.midX,y:b.midY)}
func rowTexts(_ owner:AXUIElement,_ name:String)throws->[String]{tree(try row(owner,name)).filter{str($0,kAXRoleAttribute)=="AXStaticText"}.flatMap(labels).filter{!$0.isEmpty}}
func drag(_ from:CGPoint,_ to:CGPoint,_ stage:String)throws{try mouse(.mouseMoved,from);pump();try mouse(.leftMouseDown,from);pump(0.18);for i in 1...22{let f=CGFloat(i)/22;try mouse(.leftMouseDragged,CGPoint(x:from.x+(to.x-from.x)*f,y:from.y+(to.y-from.y)*f));pump(0.025)};try mouse(.leftMouseUp,to);pump(0.2);observations.append(["stage":stage,"dragCompleted":true])}
func dump(_ stage:String,_ root:AXUIElement)throws{let rows=tree(root).map{e->[String:Any] in ["role":str(e,kAXRoleAttribute),"title":str(e,kAXTitleAttribute),"description":str(e,kAXDescriptionAttribute),"value":str(e,kAXValueAttribute),"frame":(try? rect(e)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? [],"enabled":attr(e,kAXEnabledAttribute) as? Bool as Any? ?? NSNull()]};try write(["stage":stage,"rows":rows],out.appendingPathComponent(stage+".ax.json"))}
func checkpoint(_ name:String)throws{try guardApp();try write(["request":name],out.appendingPathComponent("checkpoint-"+name+".request"));let file=out.appendingPathComponent("checkpoint-"+name+".json");try wait("saved checkpoint "+name,3){FileManager.default.fileExists(atPath:file.path)};let proof=try JSONSerialization.jsonObject(with:Data(contentsOf:file)) as! [String:Any];try need(proof["passed"] as? Bool==true,"Saved checkpoint mismatch "+name)}
func closePanel(_ panel:AXUIElement)throws{try use(panel);let close=try unique(panel,"AXButton","close button");try action(close,kAXPressAction,"close-import-color");try wait("color panel hidden"){!allWindows().contains{same($0,panel)}};companionPanel=nil}
func rgb(_ panel:AXUIElement,_ hex:String)throws{
    try use(panel)
    var nodes=tree(panel,prune:true)
    if !nodes.contains(where:{str($0,kAXRoleAttribute)=="AXPopUpButton"}){let sliders=nodes.filter{str($0,kAXRoleAttribute)=="AXButton" && str($0,kAXDescriptionAttribute)=="Color Sliders"};try need(sliders.count==1,"No Color Sliders button");try action(sliders[0],kAXPressAction,"color-sliders");try wait("color slider popup"){tree(panel,prune:true).contains{str($0,kAXRoleAttribute)=="AXPopUpButton"}}}
    nodes=tree(panel,prune:true);let popups=nodes.filter{str($0,kAXRoleAttribute)=="AXPopUpButton"};try need(popups.count==1,"Nonunique color slider popup")
    if str(popups[0],kAXValueAttribute) != "RGB Sliders"{try action(popups[0],kAXPressAction,"color-mode-popup");var item:AXUIElement?;try wait("RGB mode item"){let matches=tree(panel,prune:true).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="RGB Sliders"};try need(matches.count<=1,"Ambiguous RGB menu item");item=matches.first;return item != nil};try action(item!,kAXPressAction,"color-rgb-mode")}
    var field:AXUIElement?;var cancelled=false
    try wait("settled exact red RGB field",5){let a=tree(panel,prune:true),menus=a.filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="RGB Sliders"};let p=a.filter{str($0,kAXRoleAttribute)=="AXPopUpButton" && str($0,kAXValueAttribute)=="RGB Sliders"};if !cancelled && p.count==1 && menus.count==1{try action(menus[0],kAXCancelAction,"close-rgb-menu");cancelled=true;return false};let f=a.filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)=="FF0000"};try need(f.count<=1,"Ambiguous RGB hex field");field=f.first;return p.count==1 && menus.isEmpty && field != nil}
    try set(field!,hex,"color-hex",confirm:true)
    try wait("color hex readback"){tree(panel,prune:true).contains{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)==hex}}
    try dump("color-"+hex,panel)
}
func cleanup(){cleanupReading=true;var errors=[String]();func releaseSafe(_ point:CGPoint?=nil)->Bool{guard now()<cleanupEnd,!ownershipLost,foreground(),pointerStable(),physicalOnlyOurs(),focusOwned() else{return false};return foreground() && pointerStable() && physicalOnlyOurs() && (point.map{nativeAt($0)==currentNativeID} ?? true) && now()<cleanupEnd}
    if mouseHeld,releaseSafe(lastPoint),let e=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left),releaseSafe(lastPoint),now()<cleanupEnd{e.post(tap:.cghidEventTap);mouseHeld=false;ups+=1;pump()}
    for code in heldKeys.sorted().reversed() where releaseSafe(){if let e=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:false),releaseSafe(),now()<cleanupEnd{e.flags=[];e.post(tap:.cghidEventTap);heldKeys.remove(code);keyUps+=1;pump()}}
    let clear=clearPhysical() && !mouseHeld && heldKeys.isEmpty && downs==ups && keyDowns==keyUps
    func restore()->Bool{let front=NSWorkspace.shared.frontmostApplication?.processIdentifier;let yes=idleRestore(time:now(),limit:cleanupEnd,identity:priorIdentity(),front:front==pid || front==pid_t(previous["pid"] as! Int),pointer:pointerStable(),clear:clearPhysical(),lost:ownershipLost);return yes && now()<cleanupEnd}
    if clear,oldPointer.count==2,restore(){if restore() && now()<cleanupEnd{report["restoredPointer"]=CGWarpMouseCursorPosition(CGPoint(x:oldPointer[0],y:oldPointer[1])) == .success;lastPoint=CGPoint(x:oldPointer[0],y:oldPointer[1]);pump()};if restore() && now()<cleanupEnd{_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};for _ in 0..<20 where now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}else{errors.append("Restoration withheld by ownership/input guard")}
    let finalPhysical=physical();let finalClear=clear && finalPhysical.0.isEmpty && finalPhysical.1.isEmpty && finalPhysical.2.isEmpty
    report["physicalAfterRestoration"]=["keys":finalPhysical.0.sorted().map(Int.init),"buttons":finalPhysical.1.sorted(),"modifiers":finalPhysical.2.rawValue]
    report["restoredPrevious"]=priorIdentity() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);report["noInputHeld"]=finalClear;report["cleanupErrors"]=errors;report["targetExited"]=NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true;report["mouseDowns"]=downs;report["mouseUps"]=ups;report["keyDowns"]=keyDowns;report["keyUps"]=keyUps;report["events"]=events;report["observations"]=observations;report["elapsedSeconds"]=now()-began
    if !finalClear || !errors.isEmpty || report["restoredPrevious"] as? Bool != true || report["restoredPointer"] as? Bool != true{report["passed"]=false};try? write(report,out.appendingPathComponent("foreground.json"))
}
do{defer{cleanup()};do{
    try need(oldPointer.count==2 && oldPointer.allSatisfy(\.isFinite) && clearPhysical(),"Initial pointer/input state invalid")
    try need(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Native permissions unavailable; no request")
    let registration=min(inputEnd,now()+8)
    while now()<registration{let current=NSWorkspace.shared.frontmostApplication?.processIdentifier;try need(pointerStable() && clearPhysical() && (current==pid || (priorIdentity() && current==pid_t(previous["pid"] as! Int))),"Desktop changed during registration");if targetIdentity() && NSRunningApplication(processIdentifier:pid)?.isFinishedLaunching==true{break};pump()}
    try need(targetIdentity() && bundle.hasPrefix("com.vixenlights.verification."),"Private app registration failed")
    try need(priorOrTarget() && pointerStable() && clearPhysical(),"Activation ownership changed");try need(NSRunningApplication(processIdentifier:pid)?.activate(options:[])==true,"Activation refused");for _ in 0..<20 where !foreground(){pump()};try guardApp()
    let admin=try findWindow("ready Administration"){str($0,kAXTitleAttribute).contains("Vixen Administration") && tree($0,prune:true).contains{labels($0).contains("Open Sequence...") && attr($0,kAXEnabledAttribute) as? Bool==true}}
    try use(admin);try dump("administration-discovery",admin);try press(admin,"Open Sequence...","open-sequence")
    let chooser=try findWindow("Open chooser"){str($0,kAXRoleAttribute)=="AXSheet" && tree($0,prune:true).contains{str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains("Open")}}
    try use(chooser);try dump("open-chooser-discovery",chooser);try chord([55,56,5]);var goto:AXUIElement?
    try wait("Go To field"){guard let f=ax(attr(app,kAXFocusedUIElementAttribute)),["AXTextField","AXComboBox"].contains(str(f,kAXRoleAttribute)),tree(chooser).contains(where:{same($0,f)}) else{return false};goto=f;return true}
    try set(goto!,cfg["openFile"] as! String,"open-private-vix");try chord([36]);try wait("Go To dismissed"){!same(ax(attr(app,kAXFocusedUIElementAttribute)),goto)};try dump("open-selection-discovery",chooser);try press(chooser,"Open","accept-open")
    let converter=try findWindow("Vixen2 Converter"){converterSignature(Set(tree($0,prune:true).flatMap(labels)),path:cfg["openFile"] as! String)};try use(converter);try dump("converter-discovery",converter);try press(converter,"Create New Map","create-map")
    let mapping=try findWindow("Vixen2 Channel Mapping"){mappingSignature(Set(tree($0,prune:true).flatMap(labels)))};try use(mapping)
    if discovery {
        try dump("mapper-discovery",mapping)
        try press(mapping,"Set Import Color","discovery-open-color")
        let panel=try findWindow("Set Import Color discovery"){str($0,kAXTitleAttribute)=="Set Import Color"}
        companionPanel=panel;companionOwner=mapping;try use(panel);try dump("color-discovery",panel)
        try closePanel(panel);companionOwner=nil
        try use(mapping);try press(mapping,"Cancel","discovery-cancel-map")
        try wait("discovery map closed"){!allWindows().contains{same($0,mapping)}}
        try use(converter);try press(converter,"Cancel","discovery-cancel-converter")
        try wait("discovery converter closed"){!allWindows().contains{same($0,converter)}}
        try wait("Administration focus restored"){same(ax(attr(app,kAXFocusedWindowAttribute)),admin)}
        try use(admin);guard let menu=ax(attr(app,kAXMenuBarAttribute)) else{throw Stop.failed("No app menu")}
        let quit=tree(menu).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="Quit Vixen"}
        try need(quit.count==1,"No unique Quit Vixen menu")
        try action(quit[0],kAXPressAction,"discovery-quit-menu")
        let end=min(inputEnd,now()+8)
        while NSRunningApplication(processIdentifier:pid)?.isTerminated==false && now()<end {try need(priorOrTarget() && pointerStable(),"Desktop changed during normal Quit");pump()}
        try need(NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true,"Normal discovery Quit did not terminate")
        report["normalQuitMenu"]=true;report["inputCompleted"]=true;report["discoveryOnly"]=true;report["passed"]=true
    } else {
    let fields=tree(mapping,prune:true).filter{str($0,kAXRoleAttribute)=="AXTextField"};try need(fields.count==1,"Mapping name field nonunique");try set(fields[0],cfg["mapName"] as! String,"map-name")
    let table=try container(mapping,"Vixen 2 channel mappings"),source=try container(mapping,"Vixen 2 destination elements")
    for (dest,channel) in [("V2 Fixture 1","Channel 1"),("V2 Fixture 2","Channel 2")]{try drag(rowPoint(source,dest),rowPoint(table,channel),"map-"+channel);try wait("mapped "+channel){try rowTexts(table,channel).contains(dest)}}
    try drag(rowPoint(source,"V2 Fixture 3"),rowPoint(table,"Channel 1"),"overwrite-cancel-drag")
    let alert=try findWindow("overwrite confirmation"){tree($0,prune:true).contains{labels($0).contains("You are about to over write existing items.  Do you wish to continue?")}};try use(alert);try press(alert,"Cancel","overwrite-cancel");try wait("overwrite alert dismissed"){!allWindows().contains{same($0,alert)}};try use(mapping);try need(try rowTexts(table,"Channel 1").contains("V2 Fixture 1"),"Overwrite Cancel changed target")
    try click(rowPoint(table,"Channel 1"));try press(mapping,"Set Import Color","start-row1-color")
    let panel=try findWindow("Set Import Color panel"){str($0,kAXTitleAttribute)=="Set Import Color"};companionPanel=panel;companionOwner=mapping;try rgb(panel,"00FF00")
    try use(mapping);try click(rowPoint(table,"Channel 2"));try dump("selection-changed-with-panel-open",mapping);try need(allWindows().contains{same($0,panel)},"Color panel closed before selection changed");try closePanel(panel)
    try use(mapping);try press(mapping,"Set Import Color","start-row2-unchanged-red");let red=try findWindow("red color panel"){str($0,kAXTitleAttribute)=="Set Import Color"};companionPanel=red;companionOwner=mapping;try closePanel(red)
    try use(mapping);try dump("before-map-accept",mapping);try press(mapping,"OK","accept-map");try wait("map accepted"){!allWindows().contains{same($0,mapping)}};try use(converter);try checkpoint("accepted")
    let saved=tree(converter).filter{str($0,kAXRoleAttribute)=="AXStaticText" && labels($0).contains(cfg["mapName"] as! String)};try need(saved.count==1,"Saved map list entry nonunique");let b=try rect(saved[0]);try click(CGPoint(x:b.midX,y:b.midY));try wait("Edit Selected Map enabled"){tree(converter).contains{str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains("Edit Selected Map") && attr($0,kAXEnabledAttribute) as? Bool==true}};try press(converter,"Edit Selected Map","reopen-map")
    let reopened=try findWindow("reopened mapping"){str($0,kAXTitleAttribute)=="Vixen 2.x Channel Mapping"};try use(reopened);try dump("reopened-map",reopened);let rows=try container(reopened,"Vixen 2 channel mappings");try need(try rowTexts(rows,"Channel 1").contains("V2 Fixture 1"),"Saved target missing on reopen");try click(rowPoint(rows,"Channel 1"));try press(reopened,"Set Import Color","start-cancelled-color");let pending=try findWindow("cancel pending color"){str($0,kAXTitleAttribute)=="Set Import Color"};companionPanel=pending;companionOwner=reopened;try rgb(pending,"0000FF");try use(reopened);try press(reopened,"Cancel","cancel-map-with-pending-color");try wait("map cancelled"){!allWindows().contains{same($0,reopened)}};if allWindows().contains(where:{same($0,pending)}){try closePanel(pending)};companionPanel=nil;try use(converter);try checkpoint("cancelled")
    try press(converter,"Cancel","cancel-converter");try wait("converter closed"){!allWindows().contains{same($0,converter)}}
    currentRoot=admin;companionPanel=nil;try guardInput();guard let menu=ax(attr(app,kAXMenuBarAttribute)) else{throw Stop.failed("No app menu")};let quit=tree(menu).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="Quit Vixen"};try need(quit.count==1,"No unique Quit Vixen menu");try action(quit[0],kAXPressAction,"quit-menu")
    let end=min(inputEnd,now()+8);while NSRunningApplication(processIdentifier:pid)?.isTerminated==false && now()<end{try need(priorOrTarget() && pointerStable(),"Desktop changed during normal Quit");pump()};try need(NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true,"Normal Quit did not terminate")
    report["normalQuitMenu"]=true;report["inputCompleted"]=true;report["passed"]=true
    }
}catch{report["error"]=String(describing:error)}}
exit(report["passed"] as? Bool==true ? 0:1)
