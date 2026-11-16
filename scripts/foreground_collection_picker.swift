// Normal-app collection Picker ownership verification; native AX menus/color controls, no hooks.
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
try need(cfg["workflow"] as? String == "collection-picker","Explicit collection-picker workflow required")
let oldPointer=cfg["pointer"] as! [Double], began=now(),inputEnd=began+90,cleanupEnd=began+105
let app=AXUIElementCreateApplication(pid),systemAX=AXUIElementCreateSystemWide()
var currentRoot:AXUIElement?, companionPanel:AXUIElement?, companionOwner:AXUIElement?, currentNativeRoot:AXUIElement?, currentNativeID:Int?
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
func nativeHosts(_ root:AXUIElement)->[AXUIElement] {
    var result=[AXUIElement](),seen=[AXUIElement](),node:AXUIElement?=root
    for _ in 0..<14 {
        guard let item=node,now()<limit(),owned(item),!seen.contains(where:{same($0,item)}) else{break}
        seen.append(item);let role=str(item,kAXRoleAttribute)
        if role==kAXApplicationRole{break}
        if ["AXWindow","AXSheet"].contains(role){result.append(item)}
        node=ax(attr(item,kAXParentAttribute))
    }
    return result
}
func contained(_ root:CGRect,_ host:CGRect)->Bool {
    [root.minX,root.minY,root.width,root.height,host.minX,host.minY,host.width,host.height].allSatisfy(\.isFinite) && root.width>0 && root.height>0 && host.contains(root)
}
func nativeAt(_ point:CGPoint)->Int? {
    guard let root=currentRoot,let host=currentNativeRoot,let number=currentNativeID,
          let bounds=try? rect(root),let hostBounds=try? rect(host),bounds.contains(point),contained(bounds,hostBounds),
          nativeHosts(root).contains(where:{same($0,host)}) else{return nil}
    let matches=matchingNative(nativeWindows(),owner:Int(pid),frame:hostBounds)
    guard matches.count==1,matches[0][kCGWindowNumber as String] as? Int==number,actualHit(point,root:root).0,
          let freshRoot=try? rect(root),let freshHost=try? rect(host),sameFrame(freshRoot,bounds),sameFrame(freshHost,hostBounds),
          nativeHosts(root).contains(where:{same($0,host)}),now()<limit() else{return nil}
    return number
}

func guardInput(_ point:CGPoint?=nil)throws{try guardApp();let focus=focusOwned();try need(now()<limit(),"Deadline after AX focus");if !focus{ownershipLost=true};try need(focus,"Exact observed window hierarchy lost focus");try guardApp();if let point{try need(nativeAt(point)==currentNativeID && now()<limit(),"Posted point is not in exact native window")};try guardApp();try need(now()<limit(),"Deadline after actual hit traversal")}
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
    try guardApp();currentRoot=root;currentNativeRoot=nil;currentNativeID=nil
    let end=min(inputEnd,now()+1.2)
    repeat {
        try guardApp();let r=try rect(root),records=nativeWindows(),center=CGPoint(x:r.midX,y:r.midY)
        let hosts=nativeHosts(root);var selected:(AXUIElement,CGRect,[String:Any])?,hostObservations=[[String:Any]]()
        for host in hosts {
            guard let h=try? rect(host) else{continue}
            let matches=matchingNative(records,owner:Int(pid),frame:h)
            hostObservations.append(["role":str(host,kAXRoleAttribute),"title":str(host,kAXTitleAttribute),"exactRoot":same(root,host),"frame":[h.minX,h.minY,h.width,h.height],"containsRoot":contained(r,h),"matches":matches.map(nativeSummary)])
            try need(matches.count<=1,"Ambiguous exact AX/native host geometry")
            if selected==nil,contained(r,h),let match=matches.first {selected=(host,h,match)}
        }
        let hit=actualHit(center,root:root)
        observations.append(["stage":"root-native-observation","title":str(root,kAXTitleAttribute),"role":str(root,kAXRoleAttribute),"frame":[r.minX,r.minY,r.width,r.height],"nativeHostAncestry":hostObservations,"privateNativeWindows":records.filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}.map(nativeSummary),"actualAXHit":hit.1])
        if let (host,h,match)=selected,let number=match[kCGWindowNumber as String] as? Int,hit.0 {
            try guardApp();let freshRoot=try rect(root),freshHost=try rect(host)
            try need(sameFrame(freshRoot,r) && sameFrame(freshHost,h) && contained(freshRoot,freshHost) && nativeHosts(root).contains{same($0,host)} && now()<inputEnd,"Root/host moved or actual ancestry changed during binding")
            let freshNative=matchingNative(nativeWindows(),owner:Int(pid),frame:freshHost)
            try need(freshNative.count==1 && freshNative[0][kCGWindowNumber as String] as? Int==number && now()<inputEnd,"Native host identity changed during binding")
            currentNativeRoot=host;currentNativeID=number
            observations.append(["stage":"root","title":str(root,kAXTitleAttribute),"role":str(root,kAXRoleAttribute),"frame":[r.minX,r.minY,r.width,r.height],"nativeHostIsRoot":same(root,host),"nativeHostFrame":[h.minX,h.minY,h.width,h.height],"cgWindow":number,"cgLayer":match[kCGWindowLayer as String] ?? NSNull()])
            try guardInput();return
        }
        pump()
    } while now()<end
    throw Stop.failed("Exact native root/ancestor geometry or actual AX hit ownership did not match; see root-native-observation")
}

// Read-only identity token: a view refresh may replace AX wrappers, but it must
// not replace or move the original native owner window.
struct ObservedRootAnchor {
    let oldRoot: AXUIElement
    let nativeID: Int
    let rootFrame: CGRect
    let hostFrame: CGRect
}
func anchorRoot(_ root:AXUIElement)throws->ObservedRootAnchor {
    try need(same(currentRoot,root),"Root anchor is not the current observed root")
    guard let host=currentNativeRoot,let number=currentNativeID else{throw Stop.failed("Root has no bound native host")}
    let r=try rect(root),h=try rect(host)
    try guardInput(CGPoint(x:r.midX,y:r.midY))
    try need(currentNativeID==number && sameFrame(try rect(root),r) && sameFrame(try rect(host),h),"Owner moved while recording root anchor")
    try guardApp()
    return ObservedRootAnchor(oldRoot:root,nativeID:number,rootFrame:r,hostFrame:h)
}
func reacquireRoot(_ anchor:ObservedRootAnchor,_ stage:String,_ signature:(AXUIElement)->Bool)throws->AXUIElement {
    let end=min(inputEnd,now()+2)
    var lastWindows=[[String:Any]](),lastNative=[[String:Any]]()
    var succeeded=false
    defer {
        if !succeeded {
            observations.append(["stage":"root-reacquire-failed","transition":stage,"expectedNativeID":anchor.nativeID,"expectedRootFrame":[anchor.rootFrame.minX,anchor.rootFrame.minY,anchor.rootFrame.width,anchor.rootFrame.height],"expectedHostFrame":[anchor.hostFrame.minX,anchor.hostFrame.minY,anchor.hostFrame.width,anchor.hostFrame.height],"actualAXWindows":lastWindows,"actualPrivateNativeWindows":lastNative])
        }
    }
    repeat {
        try guardApp()
        let records=nativeWindows(),windows=allWindows()
        lastNative=records.filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}.map(nativeSummary)
        var candidates=[AXUIElement]()
        lastWindows=windows.map { root in
            let frame=try? rect(root),matches=frame.map{sameFrame($0,anchor.rootFrame)}==true && signature(root)
            if matches{candidates.append(root)}
            return ["role":str(root,kAXRoleAttribute),"title":str(root,kAXTitleAttribute),"frame":frame.map{[$0.minX,$0.minY,$0.width,$0.height]} ?? [],"matchesOriginalFrameAndSignature":matches,"sameAXRoot":same(root,anchor.oldRoot)]
        }
        let hosts=matchingNative(records,owner:Int(pid),frame:anchor.hostFrame)
        try need(hosts.count<=1 && candidates.count<=1,"Ambiguous original owner during "+stage)
        if hosts.count==1,hosts[0][kCGWindowNumber as String] as? Int==anchor.nativeID,let root=candidates.first {
            try need(now()<end,"Owner reacquisition deadline before binding")
            try use(root)
            guard let host=currentNativeRoot else{throw Stop.failed("Reacquired root lost native host")}
            try need(currentNativeID==anchor.nativeID && sameFrame(try rect(root),anchor.rootFrame) && sameFrame(try rect(host),anchor.hostFrame),"Reacquired owner changed native identity/frame")
            if same(companionOwner,anchor.oldRoot){companionOwner=root}
            try guardInput()
            try need(now()<end,"Owner reacquisition deadline after binding")
            observations.append(["stage":"root-reacquired","transition":stage,"sameAXRoot":same(root,anchor.oldRoot),"cgWindow":anchor.nativeID,"frame":[anchor.rootFrame.minX,anchor.rootFrame.minY,anchor.rootFrame.width,anchor.rootFrame.height]])
            succeeded=true
            return root
        }
        pump()
    } while now()<end
    throw Stop.failed("Original native owner did not reappear after "+stage+"; see root-reacquire-failed inventory")
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
func dump(_ stage:String,_ root:AXUIElement)throws{let rows=tree(root).map{e->[String:Any] in ["role":str(e,kAXRoleAttribute),"title":str(e,kAXTitleAttribute),"description":str(e,kAXDescriptionAttribute),"value":str(e,kAXValueAttribute),"frame":(try? rect(e)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? [],"enabled":attr(e,kAXEnabledAttribute) as? Bool as Any? ?? NSNull()]};try write(["stage":stage,"rows":rows],out.appendingPathComponent(stage+".ax.json"))}
func checkpoint(_ name:String)throws{try guardApp();try write(["checkpoint":name],out.appendingPathComponent("checkpoint-"+name+".request"));let file=out.appendingPathComponent("checkpoint-"+name+".json");try wait("saved checkpoint "+name,3){FileManager.default.fileExists(atPath:file.path)};let proof=try JSONSerialization.jsonObject(with:Data(contentsOf:file)) as! [String:Any];try need(proof["passed"] as? Bool==true,"Saved checkpoint mismatch "+name)}
func closePanel(_ panel:AXUIElement)throws {
    try use(panel)
    let direct=ax(attr(panel,kAXCloseButtonAttribute))
    let candidates=tree(panel,prune:true).filter{str($0,kAXRoleAttribute)=="AXButton" && str($0,kAXSubroleAttribute)=="AXCloseButton"}
    guard let close=direct ?? (candidates.count==1 ? candidates[0]:nil) else{throw Stop.failed("No exact native panel close button")}
    try need(owned(close) && str(close,kAXRoleAttribute)=="AXButton" && tree(panel,prune:true).contains{same($0,close)},"Panel close button relationship changed")
    observations.append(["stage":"native-panel-close-control","lookup":direct == nil ? "AXCloseButton subrole":"AXCloseButton attribute"])
    try action(close,kAXPressAction,"close-import-color")
    try wait("color panel hidden"){!allWindows().contains{same($0,panel)}}
    companionPanel=nil
}
func activeColorWell(_ panel:AXUIElement)throws->AXUIElement {
    let candidates=tree(panel,prune:true).filter { well in
        guard str(well,kAXRoleAttribute)=="AXColorWell" else{return false}
        var node:AXUIElement?=well
        for _ in 0..<14 {
            guard let current=node,owned(current),now()<limit() else{return false}
            if same(current,panel){return true}
            if ["AXList","AXScrollArea"].contains(str(current,kAXRoleAttribute)){return false}
            node=ax(attr(current,kAXParentAttribute))
        }
        return false
    }
    try need(candidates.count==1,"No unique active Color panel well outside favorite swatches")
    return candidates[0]
}
// Matches the already accepted system_panel_driver set-rgb route: a native
// floating panel may use a field editor without exposing that same AX element
// as the application's global focused item. Bind the actual field by its exact
// panel/root, role, unique six-digit value and supported native Confirm action.
func rgbField(_ panel:AXUIElement,_ value:String)throws->AXUIElement {
    try need(same(currentRoot,panel),"RGB field left exact active panel root")
    let nodes=tree(panel,prune:true),popups=nodes.filter{str($0,kAXRoleAttribute)=="AXPopUpButton"}
    try need(popups.count==1 && str(popups[0],kAXValueAttribute)=="RGB Sliders" && !nodes.contains{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="RGB Sliders"},"RGB context changed or menu remained open")
    let fields=nodes.filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)==value}
    try need(fields.count==1,"RGB field selector is not unique for \(value)")
    let field=fields[0],window=ax(attr(field,kAXWindowAttribute))
    try need(owned(field) && (same(window,panel) || ancestry(field).contains{same($0,panel)}),"RGB field not related to exact panel")
    let fieldFrame=try rect(field),panelFrame=try rect(panel)
    try need(panelFrame.contains(fieldFrame),"RGB field frame left exact panel")
    try guardInput();return field
}
func focusedFieldEvidence(_ field:AXUIElement,_ panel:AXUIElement,_ stage:String) {
    var evidence:[String:Any]=["stage":stage,"fieldRole":str(field,kAXRoleAttribute),"fieldValue":str(field,kAXValueAttribute),"fieldFrame":(try? rect(field)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? [],"fieldFocused":attr(field,kAXFocusedAttribute) as Any? ?? NSNull()]
    if let focused=ax(attr(app,kAXFocusedUIElementAttribute)) {
        var owner:pid_t = -1;let status=AXUIElementGetPid(focused,&owner)
        evidence["globalFocusPIDStatus"]=status.rawValue;evidence["globalFocusPID"]=owner;evidence["sameAsObservedField"]=same(focused,field)
        if owned(focused) {
            evidence["globalFocusRole"]=str(focused,kAXRoleAttribute);evidence["globalFocusValue"]=str(focused,kAXValueAttribute)
            evidence["globalFocusWindowIsPanel"]=same(ax(attr(focused,kAXWindowAttribute)),panel)
            evidence["globalFocusAncestry"]=ancestry(focused).map{["role":str($0,kAXRoleAttribute),"title":str($0,kAXTitleAttribute),"isExactPanel":same($0,panel)] as [String:Any]}
        }
    } else {evidence["globalFocusMissing"]=true}
    observations.append(evidence)
}
func setRGB(_ panel:AXUIElement,_ expectedStart:String,_ value:String)throws {
    let field=try rgbField(panel,expectedStart)
    focusedFieldEvidence(field,panel,"rgb-before-focus")
    try guardInput();let status=AXUIElementSetAttributeValue(field,kAXFocusedAttribute as CFString,kCFBooleanTrue)
    observations.append(["stage":"rgb-focus-request","status":status.rawValue]);try need(status == .success,"Cannot focus exact RGB field")
    var fresh:AXUIElement?
    try wait("fresh exact RGB field after focus",1) {
        guard let candidate=try? rgbField(panel,expectedStart) else{return false}
        fresh=candidate;return true
    }
    focusedFieldEvidence(fresh!,panel,"rgb-after-focus")
    try guardInput();try need(AXUIElementSetAttributeValue(fresh!,kAXValueAttribute as CFString,value as CFString) == .success,"Native RGB value was not accepted")
    var changed:AXUIElement?
    try wait("fresh RGB value readback",1) {
        guard let candidate=try? rgbField(panel,value) else{return false}
        changed=candidate;return true
    }
    focusedFieldEvidence(changed!,panel,"rgb-before-confirm")
    try action(try rgbField(panel,value),kAXConfirmAction,"color-hex")
    _ = try rgbField(panel,value)
    observations.append(["stage":"color-hex","value":value])
}
func rgb(_ panel:AXUIElement,_ hex:String,expectedStart:String="FF0000")throws{
    try use(panel)
    var nodes=tree(panel,prune:true)
    if !nodes.contains(where:{str($0,kAXRoleAttribute)=="AXPopUpButton"}){let sliders=nodes.filter{str($0,kAXRoleAttribute)=="AXButton" && str($0,kAXDescriptionAttribute)=="Color Sliders"};try need(sliders.count==1,"No Color Sliders button");try action(sliders[0],kAXPressAction,"color-sliders");try wait("color slider popup"){tree(panel,prune:true).contains{str($0,kAXRoleAttribute)=="AXPopUpButton"}}}
    nodes=tree(panel,prune:true);let popups=nodes.filter{str($0,kAXRoleAttribute)=="AXPopUpButton"};try need(popups.count==1,"Nonunique color slider popup")
    if str(popups[0],kAXValueAttribute) != "RGB Sliders"{try action(popups[0],kAXPressAction,"color-mode-popup");var item:AXUIElement?;try wait("RGB mode item"){let matches=tree(panel,prune:true).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="RGB Sliders"};try need(matches.count<=1,"Ambiguous RGB menu item");item=matches.first;return item != nil};try action(item!,kAXPressAction,"color-rgb-mode")}
    var field:AXUIElement?;var cancelled=false
    try wait("settled expected RGB field",5){let a=tree(panel,prune:true),menus=a.filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="RGB Sliders"};let p=a.filter{str($0,kAXRoleAttribute)=="AXPopUpButton" && str($0,kAXValueAttribute)=="RGB Sliders"};if !cancelled && p.count==1 && menus.count==1{try action(menus[0],kAXCancelAction,"close-rgb-menu");cancelled=true;return false};let f=a.filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)==expectedStart};try need(f.count<=1,"Ambiguous RGB hex field");field=f.first;return p.count==1 && menus.isEmpty && field != nil}
    // Vixen2MapModel.color deliberately begins every chooser at red, including existing mapped rows.
    observations.append(["stage":"picker-start-hex","expected":expectedStart,"observed":str(field!,kAXValueAttribute)])
    try setRGB(panel,expectedStart,hex)
    try wait("color hex readback"){tree(panel,prune:true).contains{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)==hex}}
    try dump("color-"+hex,panel)
}
func cleanup(){cleanupReading=true;var errors=[String]();func releaseSafe(_ point:CGPoint?=nil)->Bool{guard now()<cleanupEnd,!ownershipLost,foreground(),pointerStable(),physicalOnlyOurs(),focusOwned() else{return false};guard point.map({nativeAt($0)==currentNativeID}) ?? true else{return false};return foreground() && pointerStable() && physicalOnlyOurs() && now()<cleanupEnd}
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
func collectionSignature(_ root:AXUIElement)->Bool {
    let names=Set(tree(root,prune:true).flatMap(labels))
    return names.contains("Color Collection Editor") && names.contains("Import Collection") && names.contains("Add Color")
}
func picker(_ root:AXUIElement)throws->AXUIElement {
    let choices=tree(root,prune:true).filter{str($0,kAXRoleAttribute)=="AXPopUpButton"}
    try need(choices.count==1,"Collection Picker is not unique")
    return choices[0]
}
func chooseCollection(_ root:AXUIElement,_ name:String,_ stage:String)throws->AXUIElement {
    try use(root)
    let anchor=try anchorRoot(root)
    let popup=try picker(root),before=str(popup,kAXValueAttribute)
    try action(popup,kAXPressAction,stage+"-open-picker")
    var item:AXUIElement?
    try wait(stage+" menu item",3) {
        let matches=tree(root,prune:true).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)==name}
        try need(matches.count<=1,"Collection menu item ambiguous")
        item=matches.first;return item != nil
    }
    try action(item!,kAXPressAction,stage+"-select-native-item")
    let fresh=try reacquireRoot(anchor,stage+"-picker-change",collectionSignature)
    try wait(stage+" selected readback",3){str(try picker(fresh),kAXValueAttribute)==name}
    observations.append(["stage":stage,"before":before,"selected":str(try picker(fresh),kAXValueAttribute),"nativeMenuItem":name])
    try dump(stage,fresh);return fresh
}
func openCollections(_ sequence:AXUIElement,_ stage:String)throws->AXUIElement {
    try use(sequence)
    guard let menu=ax(attr(app,kAXMenuBarAttribute)) else{throw Stop.failed("No native menu bar")}
    let items=tree(menu).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="Color Collection Editor"}
    try need(items.count==1,"Color Collection Editor command not unique")
    try action(items[0],kAXPressAction,stage+"-menu-command")
    let sheet=try findWindow(stage+" sheet"){collectionSignature($0)}
    try use(sheet);try dump(stage,sheet);return sheet
}
do{defer{cleanup()};do{
    try need(oldPointer.count==2 && oldPointer.allSatisfy(\.isFinite) && clearPhysical(),"Initial pointer/input state invalid")
    try need(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Native permissions unavailable; no request")
    let registration=min(inputEnd,now()+8)
    while now()<registration{let current=NSWorkspace.shared.frontmostApplication?.processIdentifier;try need(pointerStable() && clearPhysical() && (current==pid || (priorIdentity() && current==pid_t(previous["pid"] as! Int))),"Desktop changed during registration");if targetIdentity() && NSRunningApplication(processIdentifier:pid)?.isFinishedLaunching==true{break};pump()}
    try need(targetIdentity() && bundle.hasPrefix("com.vixenlights.verification."),"Private app registration failed")
    try need(priorOrTarget() && pointerStable() && clearPhysical(),"Activation ownership changed");try need(NSRunningApplication(processIdentifier:pid)?.activate(options:[])==true,"Activation refused");for _ in 0..<20 where !foreground(){pump()};try guardApp()
    let admin=try findWindow("ready Administration"){str($0,kAXTitleAttribute).contains("Vixen Administration") && tree($0,prune:true).contains{labels($0).contains("Open Sequence...") && attr($0,kAXEnabledAttribute) as? Bool==true}}
    try use(admin);try press(admin,"Open Sequence...","open-sequence")
    let chooser=try findWindow("Open chooser"){str($0,kAXRoleAttribute)=="AXSheet" && tree($0,prune:true).contains{str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains("Open")}}
    try use(chooser);try chord([55,56,5]);var goto:AXUIElement?
    try wait("Go To field"){guard let f=ax(attr(app,kAXFocusedUIElementAttribute)),["AXTextField","AXComboBox"].contains(str(f,kAXRoleAttribute)),tree(chooser).contains(where:{same($0,f)}) else{return false};goto=f;return true}
    try set(goto!,cfg["openFile"] as! String,"open-private-sequence");try chord([36]);try wait("Go To dismissed"){!same(ax(attr(app,kAXFocusedUIElementAttribute)),goto)};try dump("open-selection",chooser);try press(chooser,"Open","accept-open")
    var sequence=try findWindow("exact sequence window"){str($0,kAXRoleAttribute)=="AXWindow" && str($0,kAXTitleAttribute)==cfg["sequenceTitle"] as? String}
    try use(sequence);try dump("sequence-ready",sequence)
    let sequenceAnchor=try anchorRoot(sequence)
    let sequenceSignature:(AXUIElement)->Bool = {str($0,kAXRoleAttribute)=="AXWindow" && str($0,kAXTitleAttribute)==cfg["sequenceTitle"] as? String}
    var editor=try openCollections(sequence,"collections-first")
    editor=try chooseCollection(editor,"Collection A","initial-A")
    let editorAnchor=try anchorRoot(editor)
    try press(editor,"Add Color","start-A-color")
    let panel=try findWindow("Add Color panel"){str($0,kAXTitleAttribute)=="Add Color"}
    companionPanel=panel;companionOwner=editor;try rgb(panel,"FF0000",expectedStart:"FFFFFF")
    try need(allWindows().contains{same($0,panel)},"Pending color panel disappeared before native collection switch")
    editor=try chooseCollection(editor,"Collection B","pending-red-switch-B")
    let panelVisibleAfterSwitch=allWindows().contains{same($0,panel)}
    observations.append(["stage":"panel-after-native-switch","visible":panelVisibleAfterSwitch])
    if panelVisibleAfterSwitch{try closePanel(panel)}
    companionPanel=nil;companionOwner=nil;editor=try reacquireRoot(editorAnchor,"collection-color-close",collectionSignature)
    try need(str(try picker(editor),kAXValueAttribute)=="Collection B","Closing ended picker changed selected collection")
    try press(editor,"OK","accept-collections");try wait("first collection sheet closed"){!allWindows().contains{same($0,editor)}}
    sequence=try reacquireRoot(sequenceAnchor,"collection-accepted",sequenceSignature);try checkpoint("accepted")
    var reopened=try openCollections(sequence,"collections-reopened")
    reopened=try chooseCollection(reopened,"Collection A","reopened-A")
    try press(reopened,"Add Color","start-cancelled-blue")
    let pending=try findWindow("pending Add Color panel"){str($0,kAXTitleAttribute)=="Add Color"}
    companionPanel=pending;companionOwner=reopened;try rgb(pending,"0000FF",expectedStart:"FFFFFF")
    try use(reopened);try press(reopened,"Cancel","cancel-dirty-collections")
    let warning=try findWindow("discard collection warning"){tree($0,prune:true).contains{labels($0).contains("You will loose any changes, do you wish to save them now ?")}}
    try use(warning);try dump("discard-warning",warning);try press(warning,"Don't Save","discard-pending-blue")
    try wait("discard warning and collection sheet closed"){!allWindows().contains{same($0,warning) || same($0,reopened)}}
    if allWindows().contains(where:{same($0,pending)}){companionOwner=sequence;try closePanel(pending)}
    companionPanel=nil;companionOwner=nil;sequence=try reacquireRoot(sequenceAnchor,"collection-cancelled",sequenceSignature);try checkpoint("cancelled")
    guard let menu=ax(attr(app,kAXMenuBarAttribute)) else{throw Stop.failed("No Quit menu")}
    let quit=tree(menu).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)=="Quit Vixen"};try need(quit.count==1,"Quit command not unique")
    try action(quit[0],kAXPressAction,"quit-menu")
    let end=min(inputEnd,now()+8);while NSRunningApplication(processIdentifier:pid)?.isTerminated==false && now()<end{try need(priorOrTarget() && pointerStable(),"Desktop changed during normal Quit");pump()}
    try need(NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true,"Normal Quit did not terminate")
    report["normalQuitMenu"]=true;report["inputCompleted"]=true;report["collectionPickerWorkflow"]=true;report["passed"]=true
}catch{report["error"]=String(describing:error)}}
exit(report["passed"] as? Bool==true ? 0:1)
