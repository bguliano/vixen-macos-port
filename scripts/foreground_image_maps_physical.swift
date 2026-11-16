// Physical Image Maps rename/reselect/double-click. Guard core copied from reviewed Package v11.
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
// NSOpenPanel/NSSavePanel use an owned sheet or the exact standalone title
// declared by PackageWizardModel. A window with arbitrary Open/Cancel is rejected.
func chooserRoleAllowed(role:String,title:String,expectedTitle:String)->Bool {
    role=="AXSheet" || (role=="AXWindow" && !expectedTitle.isEmpty && title==expectedTitle)
}
func chooserRoleChecks()->[(String,Bool)] {
    [("observed sheet chooser",chooserRoleAllowed(role:"AXSheet",title:"",expectedTitle:"Sequence Package File")),
     ("observed standalone package chooser",chooserRoleAllowed(role:"AXWindow",title:"Sequence Package File",expectedTitle:"Sequence Package File")),
     ("source-defined standalone map chooser",chooserRoleAllowed(role:"AXWindow",title:"Profile Map File",expectedTitle:"Profile Map File")),
     ("unrelated window refused",!chooserRoleAllowed(role:"AXWindow",title:"Vixen Administration",expectedTitle:"Sequence Package File")),
     ("other file chooser refused",!chooserRoleAllowed(role:"AXWindow",title:"Profile Map File",expectedTitle:"Sequence Package File")),
     ("empty standalone title refused",!chooserRoleAllowed(role:"AXWindow",title:"",expectedTitle:"")),
     ("nonwindow role refused",!chooserRoleAllowed(role:"AXGroup",title:"Sequence Package File",expectedTitle:"Sequence Package File"))]
}
func converterSignature(_ labels:Set<String>,path:String)->Bool { labels.contains("Channel Maps:") && labels.contains("Vixen 2 Sequence:") && labels.contains("Create New Map") && labels.contains(path) }
func hitOwnership(status:Int,owner:Int,expectedOwner:Int,rootReached:Bool)->Bool { status==0 && owner==expectedOwner && rootReached }
func mappingSignature(_ labels:Set<String>)->Bool { ["Vixen 2 channel mappings","Vixen 2 destination elements","Set Import Color"].allSatisfy{labels.contains($0)} }
// Only completion of an already posted chord may cross its native focus change.
func ownedKeyUpAllowed(time:Double,deadline:Double,tracked:Bool,leased:Bool,identity:Bool,pointer:Bool,physical:Bool,interference:Bool)->Bool {
    time<deadline && tracked && leased && identity && pointer && physical && !interference
}
// A posted keyUP is asynchronous. Retain that exact owned key until its key bit
// and modifier bit have both stayed absent across two samples at least25ms apart.
// This allowance never authorizes another keyDOWN or an AX/mouse action.
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

func idleRestore(time:Double,limit:Double,identity:Bool,front:Bool,pointer:Bool,clear:Bool,lost:Bool) -> Bool { time<limit && identity && front && pointer && clear && !lost }
if CommandLine.arguments.count==2 && CommandLine.arguments[1]=="--guard-checks" {
    let restoreCases:[(String,Bool)]=[("idle restore",idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:true,lost:false)),("expired",!idleRestore(time:2,limit:2,identity:true,front:true,pointer:true,clear:true,lost:false)),("replaced prior",!idleRestore(time:1,limit:2,identity:false,front:true,pointer:true,clear:true,lost:false)),("foreign foreground",!idleRestore(time:1,limit:2,identity:true,front:false,pointer:true,clear:true,lost:false)),("pointer interference",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:false,clear:true,lost:false)),("held input",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:false,lost:false)),("ownership lost",!idleRestore(time:1,limit:2,identity:true,front:true,pointer:true,clear:true,lost:true))]
    let signatures:[(String,Bool)]=[("converter exact path",converterSignature(["Channel Maps:","Vixen 2 Sequence:","Create New Map","/private/fixture.vix"],path:"/private/fixture.vix")),("converter wrong path refused",!converterSignature(["Channel Maps:","Vixen 2 Sequence:","Create New Map","/other.vix"],path:"/private/fixture.vix")),("converter incomplete controls refused",!converterSignature(["Create New Map","/private/fixture.vix"],path:"/private/fixture.vix")),("mapper signature",mappingSignature(["Vixen 2 channel mappings","Vixen 2 destination elements","Set Import Color"])),("mapper missing destination refused",!mappingSignature(["Vixen 2 channel mappings","Set Import Color"]))]
    func record(_ owner:Int,_ number:Int,_ layer:Int,_ alpha:Double=1,_ x:Double=10)->[String:Any] { [kCGWindowOwnerPID as String:owner,kCGWindowNumber as String:number,kCGWindowLayer as String:layer,kCGWindowAlpha as String:alpha,kCGWindowBounds as String: CGRect(x:x,y:10,width:100,height:100).dictionaryRepresentation] }
    let panel=record(11,101,3),underlying=record(22,202,0),point=CGPoint(x:60,y:60),frame=CGRect(x:10,y:10,width:100,height:100)
    let nativeChecks:[(String,Bool)]=[("floating panel exact frame included",matchingNative([panel,underlying],owner:11,frame:frame).count==1 && topNative([panel,underlying],at:point)?[kCGWindowNumber as String] as? Int==101),("foreign higher window remains blocking",topNative([record(22,303,8),panel],at:point)?[kCGWindowOwnerPID as String] as? Int==22),("wrong frame refused",matchingNative([record(11,101,3,1,200)],owner:11,frame:frame).isEmpty),("transparent native window ignored",topNative([record(22,303,8,0),panel],at:point)?[kCGWindowNumber as String] as? Int==101)]
    let hitChecks:[(String,Bool)]=[("actual root hit accepted",hitOwnership(status:0,owner:11,expectedOwner:11,rootReached:true)),("foreign actual hit refused",!hitOwnership(status:0,owner:22,expectedOwner:11,rootReached:true)),("same-app wrong root refused",!hitOwnership(status:0,owner:11,expectedOwner:11,rootReached:false)),("AX hit failure refused",!hitOwnership(status:-25204,owner:11,expectedOwner:11,rootReached:true))]
    let containmentChecks:[(String,Bool)]=[("same native root contained",contained(frame,frame)),("actual inner sheet contained",contained(CGRect(x:20,y:20,width:30,height:30),frame)),("escaping sheet refused",!contained(CGRect(x:0,y:20,width:30,height:30),frame)),("empty root refused",!contained(CGRect(x:20,y:20,width:0,height:30),frame))]
    let releaseChecks:[(String,Bool)]=[
        ("owned chord keyUP",ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:true,identity:true,pointer:true,physical:true,interference:false)),
        ("untracked keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:false,leased:true,identity:true,pointer:true,physical:true,interference:false)),
        ("unleased keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:false,identity:true,pointer:true,physical:true,interference:false)),
        ("expired keyUP refused",!ownedKeyUpAllowed(time:2,deadline:2,tracked:true,leased:true,identity:true,pointer:true,physical:true,interference:false)),
        ("foreign identity keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:true,identity:false,pointer:true,physical:true,interference:false)),
        ("moved pointer keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:true,identity:true,pointer:false,physical:true,interference:false)),
        ("unowned physical keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:true,identity:true,pointer:true,physical:false,interference:false)),
        ("latched interference keyUP refused",!ownedKeyUpAllowed(time:1,deadline:2,tracked:true,leased:true,identity:true,pointer:true,physical:true,interference:true))]
    let cases=restoreCases+signatures+nativeChecks+hitChecks+containmentChecks+releaseChecks+keyUpAcknowledgementChecks()+chooserRoleChecks()
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
try need(cfg["workflow"] as? String == "image-maps-physical","Explicit Image Maps workflow required")
let oldPointer=cfg["pointer"] as! [Double], began=now(),initialInputEnd=began+40,cleanupEnd=began+48
var inputEnd=initialInputEnd
let app=AXUIElementCreateApplication(pid),systemAX=AXUIElementCreateSystemWide()
var currentRoot:AXUIElement?, currentNativeRoot:AXUIElement?, companionPanel:AXUIElement?, companionOwner:AXUIElement?, currentNativeID:Int?
var pendingMouseUp=false
var doubleClickEnd:Double?
var mouseHeld=false,heldKeys=Set<CGKeyCode>(),downs=0,ups=0,keyDowns=0,keyUps=0,ownershipLost=false,cleanupReading=false
var lastPoint=oldPointer.count==2 ? CGPoint(x:oldPointer[0],y:oldPointer[1]):.zero
var keyUpAcks=KeyUpAcknowledgements()
var inputFocusLost=false,releaseLeaseKeys=Set<CGKeyCode>(),releaseLeaseOrder=[CGKeyCode](),releaseLeaseUntil:Double=0
var events=[[String:Any]](),observations=[[String:Any]](),report:[String:Any]=["passed":false,"mode":"physical-controls-harness-setup-and-readonly-probes","pid":Int(pid)]
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
func physicalOnlyOurs()->Bool{let p=physical();return keyUpAcks.observe(time:now(),held:heldKeys,keys:p.0,buttons:p.1,flags:p.2,mouseHeld:mouseHeld || pendingMouseUp)}
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

func guardInput(_ point:CGPoint?=nil)throws{try need(!pendingMouseUp,"Posted mouseUP must be acknowledged before new input");try need(keyUpAcks.pending.isEmpty,"Posted keyUPs must be acknowledged before new input");try need(!inputFocusLost,"Original window focus was lost; no new input permitted");try guardApp();let focus=focusOwned();try need(now()<limit(),"Deadline after AX focus");if !focus{inputFocusLost=true;observations.append(["stage":"input-focus-lost","keyWindow":ax(attr(app,kAXFocusedWindowAttribute)).map{["role":str($0,kAXRoleAttribute),"title":str($0,kAXTitleAttribute)]} ?? [:],"mainWindow":ax(attr(app,kAXMainWindowAttribute)).map{["role":str($0,kAXRoleAttribute),"title":str($0,kAXTitleAttribute)]} ?? [:]])};try need(focus,"Exact observed window hierarchy lost focus");try guardApp();if let point{try need(nativeAt(point)==currentNativeID && now()<limit(),"Posted point is not in exact native window")};try guardApp();try need(now()<limit(),"Deadline after actual hit traversal")}
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
    try need(same(fresh,field) && ["AXTextField","AXComboBox"].contains(str(fresh,kAXRoleAttribute)) && tree(root).contains{same($0,fresh)},"Focused field differs from the exact observed field")
    try guardInput();try need(AXUIElementSetAttributeValue(fresh,kAXValueAttribute as CFString,value as CFString) == .success && str(fresh,kAXValueAttribute)==value,"Field value not accepted "+stage)
    if confirm{guard let latest=ax(attr(app,kAXFocusedUIElementAttribute)) else{throw Stop.failed("Focused field disappeared")};try need(tree(root).contains{same($0,latest)} && str(latest,kAXValueAttribute)==value,"Changed field readback/owner mismatch");try action(latest,kAXConfirmAction,stage)}
    observations.append(["stage":stage,"value":value])
}
func ownedReleaseDeadline()->Double { min(limit(),cleanupReading ? cleanupEnd:releaseLeaseUntil) }
func canReleaseOwnedKey(_ code:CGKeyCode)->Bool {
    let identity=foreground(),pointer=pointerStable(),physical=physicalOnlyOurs()
    if !identity || !pointer || !physical{ownershipLost=true}
    return ownedKeyUpAllowed(time:now(),deadline:ownedReleaseDeadline(),tracked:heldKeys.contains(code),leased:releaseLeaseKeys.contains(code),identity:identity,pointer:pointer,physical:physical,interference:ownershipLost)
}
func awaitKeyUpAcknowledgements()throws {
    let settleEnd=min(ownedReleaseDeadline(),now()+0.75)
    while now()<settleEnd {
        let identity=foreground(),pointer=pointerStable(),physical=physicalOnlyOurs()
        if !identity || !pointer || !physical{ownershipLost=true}
        observations.append(["stage":"key-up-acknowledgement","pending":keyUpAcks.pending.sorted().map(Int.init),"cleanup":cleanupReading,"identity":identity,"pointer":pointer,"physicalOwned":physical])
        try need(!ownershipLost && identity && pointer && physical && now()<ownedReleaseDeadline(),"KeyUP acknowledgement ownership/deadline guard")
        if keyUpAcks.pending.isEmpty {
            if heldKeys.isEmpty{releaseLeaseKeys=[];releaseLeaseOrder=[];releaseLeaseUntil=0}
            return
        }
        pump(0.02)
    }
    throw Stop.failed("Posted keyUP acknowledgement timed out; no new input permitted")
}
func postOwnedKeyUp(_ code:CGKeyCode)throws {
    try need(canReleaseOwnedKey(code),"Owned chord keyUP withheld by identity/input/deadline guard")
    guard let e=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:false) else{throw Stop.failed("Cannot create owned keyUP")}
    var remaining=heldKeys;remaining.remove(code);var flags=CGEventFlags()
    if remaining.contains(55){flags.insert(.maskCommand)};if remaining.contains(56){flags.insert(.maskShift)};e.flags=flags
    try need(canReleaseOwnedKey(code) && now()<ownedReleaseDeadline(),"Owned keyUP guard changed before post")
    e.post(tap:.cghidEventTap);let tracked=keyUpAcks.posted(code,held:heldKeys,leased:releaseLeaseKeys);heldKeys.remove(code);keyUps+=1;events.append(["key":Int(code),"down":false]);try need(tracked,"Posted keyUP lost original tracking")
    observations.append(["stage":"owned-key-release","key":Int(code),"remainingKeys":heldKeys.sorted().map(Int.init),"cleanup":cleanupReading,"flags":flags.rawValue])
    try awaitKeyUpAcknowledgements()
}
func key(_ code:CGKeyCode,_ down:Bool)throws {
    if !down{try postOwnedKeyUp(code);return}
    try guardInput();try need(releaseLeaseKeys.contains(code) && now()<releaseLeaseUntil,"KeyDOWN lacks original guarded chord lease")
    guard let e=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:true) else{throw Stop.failed("Cannot create key")}
    try need(!heldKeys.contains(code),"Duplicate key down");var after=heldKeys;after.insert(code);var flags=CGEventFlags()
    if after.contains(55){flags.insert(.maskCommand)};if after.contains(56){flags.insert(.maskShift)};e.flags=flags
    try guardInput();try need(now()<releaseLeaseUntil,"Chord keyDOWN deadline expired")
    heldKeys.insert(code);keyDowns+=1;e.post(tap:.cghidEventTap);events.append(["key":Int(code),"down":true]);pump(0.05)
}
func chord(_ codes:[CGKeyCode])throws {
    try guardInput();try need(heldKeys.isEmpty && keyUpAcks.pending.isEmpty && Set(codes).count==codes.count && !codes.isEmpty,"Chord requires empty held state and distinct keys")
    releaseLeaseKeys=Set(codes);releaseLeaseOrder=codes;releaseLeaseUntil=min(inputEnd,now()+6)
    for c in codes{try key(c,true)}
    for c in codes.reversed(){try key(c,false)}
}

func mouse(_ kind:CGEventType,_ point:CGPoint,_ count:Int64=1)throws{try guardInput(point);guard let e=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else{throw Stop.failed("Cannot create mouse")};e.setIntegerValueField(.mouseEventClickState,value:count);try guardInput(point);if kind == .leftMouseDown,count==2,let end=doubleClickEnd{try need(now()<end,"Double-click interval expired before second DOWN; no extra click")};if kind == .leftMouseDown{try need(!mouseHeld,"Duplicate mouse down");mouseHeld=true;downs+=1};if kind == .leftMouseUp{try need(mouseHeld,"Unpaired mouse up")};e.post(tap:.cghidEventTap);lastPoint=point;if kind == .leftMouseUp{mouseHeld=false;pendingMouseUp=true;ups+=1};events.append(["kind":kind.rawValue,"point":[point.x,point.y],"window":currentNativeID ?? -1,"clickCount":count,"elapsedSeconds":now()-began]);if kind == .leftMouseUp{try acknowledgeMouseUp()}}
// Retained25ms stable absence policy from foreground_prop_cross_window.swift.
// Pending UP is owned only for acknowledgement; it never authorizes a new DOWN.
func acknowledgeMouseUp()throws {
    let end=min(limit(),now()+0.6);var absent:Double?
    while pendingMouseUp && now()<end {
        let good=foreground() && pointerStable() && physicalOnlyOurs()
        if !good{ownershipLost=true}
        try need(good && !ownershipLost && now()<end,"MouseUP acknowledgement ownership changed")
        if !CGEventSource.buttonState(.combinedSessionState,button:.left) {
            if let t=absent,now()-t>=0.025{pendingMouseUp=false}else if absent==nil{absent=now()}
        }else{absent=nil}
        if pendingMouseUp{pump(0.015)}
    }
    try need(!pendingMouseUp && now()<end,"MouseUP acknowledgement timed out")
}
func click(_ point:CGPoint)throws{try mouse(.mouseMoved,point);pump();try mouse(.leftMouseDown,point);pump();try mouse(.leftMouseUp,point);pump(0.12)}
func dump(_ stage:String,_ root:AXUIElement)throws {
    let nodes=tree(root)
    let rows=nodes.map{e->[String:Any] in ["role":str(e,kAXRoleAttribute),"title":str(e,kAXTitleAttribute),"description":str(e,kAXDescriptionAttribute),"value":str(e,kAXValueAttribute),"frame":(try? rect(e)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? [],"enabled":attr(e,kAXEnabledAttribute) as? Bool as Any? ?? NSNull()]}
    let fields=nodes.filter{str($0,kAXRoleAttribute)=="AXTextField"}.map{e->[String:Any] in
        let title=ax(attr(e,kAXTitleUIElementAttribute)),parent=ax(attr(e,kAXParentAttribute))
        return ["value":str(e,kAXValueAttribute),"description":str(e,kAXDescriptionAttribute),"identifier":str(e,kAXIdentifierAttribute),"subrole":str(e,kAXSubroleAttribute),"titleUIRole":title.map{str($0,kAXRoleAttribute)} ?? "","titleUILabels":title.map(labels) ?? [],"parentRole":parent.map{str($0,kAXRoleAttribute)} ?? "","parentChildren":parent.map{children($0).map{["role":str($0,kAXRoleAttribute),"labels":labels($0)] as [String:Any]}} ?? []]
    }
    try write(["stage":stage,"rows":rows,"fields":fields],out.appendingPathComponent(stage+".ax.json"))
}

func mapsSignature(_ root:AXUIElement)->Bool {
    let nodes=tree(root), text=Set(nodes.flatMap(labels))
    return ["Image Maps","Name","Notes","New","Edit","Remove","Clone","OK"].allSatisfy{text.contains($0)} && nodes.contains{["AXTable","AXOutline"].contains(str($0,kAXRoleAttribute))}
}
func editorSignature(_ root:AXUIElement)->Bool {
    let text=Set(tree(root).flatMap(labels))
    return ["Image Map","Renamed Map","Cancel","OK"].allSatisfy{text.contains($0)} && tree(root).contains{str($0,kAXRoleAttribute)=="AXTextArea"}
}
func center(_ element:AXUIElement)throws->CGPoint {let r=try rect(element);return CGPoint(x:r.midX,y:r.midY)}
func leafText(_ root:AXUIElement,_ text:String)throws->AXUIElement {
    let matches=tree(root).filter{str($0,kAXRoleAttribute)=="AXStaticText" && labels($0).contains(text)}
    try need(matches.count==1,"Expected unique actual table text "+text+"; got \(matches.count)")
    let e=matches[0];var p:AXUIElement?=e;var inTable=false
    for _ in 0..<12{guard let n=p else{break};if ["AXTable","AXOutline"].contains(str(n,kAXRoleAttribute)){inTable=true;break};if same(n,root){break};p=ax(attr(n,kAXParentAttribute))}
    try need(inTable,"Text is outside actual table hierarchy");return e
}
func buttons(_ root:AXUIElement,_ expected:[Bool],_ stage:String)throws {
    let names=["New","Edit","Remove","Clone"]
    var actual=[Bool]()
    try wait(stage,2){
        actual=try names.map{try attr(unique(root,"AXButton",$0),kAXEnabledAttribute) as? Bool == true}
        return actual==expected
    }
    observations.append(["stage":stage,"buttons":actual]);try dump(stage,root)
}
func inlineField(_ root:AXUIElement)throws->AXUIElement {
    var found=[AXUIElement]()
    try wait("physical inline rename field",2){found=tree(root).filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)=="Rename Me"};return found.count==1}
    guard let focus=ax(attr(app,kAXFocusedUIElementAttribute)) else{throw Stop.failed("No actual rename focus")}
    try need(same(focus,found[0]),"Inline rename field lacks actual focus");return found[0]
}
func slowPause()throws {let started=now(),end=now()+0.75;while now()<end{try guardInput();pump(0.04)};observations.append(["stage":"selected-name-delay","seconds":now()-started])}
func doubleClick(_ point:CGPoint)throws {
    try mouse(.mouseMoved,point)
    let interval=NSEvent.doubleClickInterval
    try need(interval.isFinite && interval>0 && interval<=2,"Unexpected system double-click interval")
    try mouse(.leftMouseDown,point,1)
    let first=events.last!["elapsedSeconds"] as! Double
    doubleClickEnd=min(inputEnd,began+first+interval)
    defer{doubleClickEnd=nil}
    pump(0.015);try mouse(.leftMouseUp,point,1)
    try mouse(.leftMouseDown,point,2)
    let second=events.last!["elapsedSeconds"] as! Double
    pump(0.015);try mouse(.leftMouseUp,point,2)
    // UP remains permitted under ordinary ownership/deadline guards even if
    // the recognizer interval expires after its paired DOWN has been posted.
    let elapsed=second-first
    try need(elapsed>0 && elapsed<interval,"Physical double-click down interval was exceeded")
    report["doubleClickTiming"]=["maximumSeconds":interval,"elapsedSeconds":elapsed,"firstDownSeconds":first,"secondDownSeconds":second,"point":[point.x,point.y]]
    pump(0.12)
}

func originalStore(_ stage:String)throws {
    let a=try Data(contentsOf:URL(fileURLWithPath:cfg["moduleStore"] as! String)),b=try Data(contentsOf:out.appendingPathComponent("initial-ModuleStore.xml"))
    try need(a==b,"Saved module data changed before root accept: "+stage)
    observations.append(["stage":stage,"moduleBytesUnchanged":true])
}
func cleanup(){cleanupReading=true;var errors=[String]();func releaseSafe(_ point:CGPoint?=nil)->Bool{guard now()<cleanupEnd,!ownershipLost,foreground(),pointerStable(),physicalOnlyOurs(),focusOwned() else{return false};guard point.map({nativeAt($0)==currentNativeID}) ?? true else{return false};return foreground() && pointerStable() && physicalOnlyOurs() && now()<cleanupEnd}
    if mouseHeld,releaseSafe(lastPoint),let e=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left),releaseSafe(lastPoint),now()<cleanupEnd{e.post(tap:.cghidEventTap);mouseHeld=false;pendingMouseUp=true;ups+=1;pump()}
    if pendingMouseUp{do{try acknowledgeMouseUp()}catch{errors.append("Pending mouseUP acknowledgement: "+String(describing:error))}}
    for code in releaseLeaseOrder.reversed() where heldKeys.contains(code) {
        do{try postOwnedKeyUp(code)}catch{errors.append("Owned key release: "+String(describing:error))}
    }
    if !keyUpAcks.pending.isEmpty {do{try awaitKeyUpAcknowledgements()}catch{errors.append("Pending keyUP acknowledgement: "+String(describing:error))}}
    let clear=clearPhysical() && !mouseHeld && !pendingMouseUp && heldKeys.isEmpty && keyUpAcks.pending.isEmpty && downs==ups && keyDowns==keyUps
    func restore()->Bool{if !priorOrTarget() || !pointerStable() || !clearPhysical(){ownershipLost=true};let front=NSWorkspace.shared.frontmostApplication?.processIdentifier;let yes=idleRestore(time:now(),limit:cleanupEnd,identity:priorIdentity(),front:front==pid || front==pid_t(previous["pid"] as! Int),pointer:pointerStable(),clear:clearPhysical(),lost:ownershipLost);return yes && now()<cleanupEnd}
    if clear,oldPointer.count==2,restore(){if restore() && now()<cleanupEnd{report["restoredPointer"]=CGWarpMouseCursorPosition(CGPoint(x:oldPointer[0],y:oldPointer[1])) == .success;lastPoint=CGPoint(x:oldPointer[0],y:oldPointer[1]);pump()};if restore() && now()<cleanupEnd{_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};for _ in 0..<20 where now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}else{errors.append("Restoration withheld by ownership/input guard")}
    let finalPhysical=physical();let finalClear=clear && finalPhysical.0.isEmpty && finalPhysical.1.isEmpty && finalPhysical.2.isEmpty
    report["ownershipLost"]=ownershipLost;report["cleanupCompleted"]=true;report["pendingMouseUp"]=pendingMouseUp
    report["pendingKeyUpAcknowledgements"]=keyUpAcks.pending.sorted().map(Int.init)
    report["physicalAfterRestoration"]=["keys":finalPhysical.0.sorted().map(Int.init),"buttons":finalPhysical.1.sorted(),"modifiers":finalPhysical.2.rawValue]
    report["restoredPrevious"]=priorIdentity() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);report["noInputHeld"]=finalClear;report["cleanupErrors"]=errors;report["targetExited"]=NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true;report["mouseDowns"]=downs;report["mouseUps"]=ups;report["keyDowns"]=keyDowns;report["keyUps"]=keyUps;report["events"]=events;report["observations"]=observations;report["elapsedSeconds"]=now()-began
    if !finalClear || !errors.isEmpty || report["restoredPrevious"] as? Bool != true || report["restoredPointer"] as? Bool != true{report["passed"]=false};try? write(report,out.appendingPathComponent("foreground.json"))
}

// Only the original registered app may be raised. Foreign activity is latched,
// including during startup and final cleanup; an eventual return never clears it.
func desktopUnchanged(_ allowTarget:Bool=true)->Bool {
    let yes=(allowTarget ? priorOrTarget():priorIdentity()) && pointerStable() && clearPhysical()
    if !yes{ownershipLost=true};return yes && !ownershipLost
}
do{defer{cleanup()};do{
    try need(oldPointer.count==2 && oldPointer.allSatisfy(\.isFinite) && clearPhysical(),"Initial pointer/input state invalid")
    try need(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Native permissions unavailable; no request")
    let registration=min(inputEnd,now()+8)
    while now()<registration{try need(desktopUnchanged(),"Desktop changed during registration");if targetIdentity() && NSRunningApplication(processIdentifier:pid)?.isFinishedLaunching==true{break};pump()}
    try need(targetIdentity() && bundle.hasPrefix("com.vixenlights.verification."),"Private app registration failed")
    try need(desktopUnchanged(),"Activation ownership changed");try need(NSRunningApplication(processIdentifier:pid)?.activate(options:[])==true,"Activation refused");for _ in 0..<20 where !foreground(){pump()};try guardApp()
    let marker=out.appendingPathComponent("ui/physical-ready.png")
    try wait("harness prepared Image Maps",15){FileManager.default.fileExists(atPath:marker.path)}
    let wall=Date().timeIntervalSince1970,mono=now(),attributes=try FileManager.default.attributesOfItem(atPath:marker.path)
    guard let modified=attributes[.modificationDate] as? Date else{throw Stop.failed("Ready marker has no timestamp")}
    let age=wall-modified.timeIntervalSince1970
    try need(age.isFinite && age>=0 && age<10 && abs((Date().timeIntervalSince1970-wall)-(now()-mono))<0.25,"Ready marker time invalid")
    inputEnd=min(inputEnd,mono+35-age);try need(inputEnd-now()>15,"Insufficient interaction reserve before harness resumes")
    report["inputCutoffSeconds"]=inputEnd-began;report["markerAgeSeconds"]=age
    let parents=children(app,kAXWindowsAttribute).filter{str($0,kAXTitleAttribute).contains("Image Maps Verification")}
    try need(parents.count==1,"Expected one private sequence parent")
    try guardApp();var position=CGPoint(x:100,y:80)
    try need(AXUIElementSetAttributeValue(parents[0],kAXPositionAttribute as CFString,AXValueCreate(.cgPoint,&position)!) == .success,"Private parent position refused")
    try guardApp();report["raiseStatus"]=AXUIElementPerformAction(parents[0],kAXRaiseAction as CFString).rawValue;pump(0.2)
    var root=try findWindow("Image Maps root",mapsSignature);try use(root);let anchor=try anchorRoot(root)
    func refresh(_ stage:String)throws {root=try reacquireRoot(anchor,stage,mapsSignature)}
    try buttons(root,[true,false,false,false],"initial-only-new")
    try click(center(leafText(root,"Rename Me")));try refresh("selected-first")
    try buttons(root,[true,true,true,true],"first-physical-selection");try slowPause()
    try click(center(leafText(root,"Rename Me")));try refresh("rename-cancel-start")
    try buttons(root,[false,false,false,false],"inline-cancel-disabled");try set(inlineField(root),"Discarded Rename","pending-rename-cancel")
    try chord([53]);try refresh("rename-cancelled");try buttons(root,[true,false,false,false],"cancel-only-new")
    _=try leafText(root,"Rename Me");try originalStore("cancel-keeps-saved-bytes")
    try click(center(leafText(root,"Rename Me")));try refresh("same-row-reselection")
    try buttons(root,[true,true,true,true],"same-row-after-cancel");try slowPause()
    try click(center(leafText(root,"Rename Me")));try refresh("rename-accept-start")
    try buttons(root,[false,false,false,false],"inline-accept-disabled");try set(inlineField(root),"Renamed Map","pending-rename-accept")
    try chord([36]);try refresh("rename-committed");try buttons(root,[true,false,false,false],"commit-only-new")
    _=try leafText(root,"Renamed Map")
    try click(center(leafText(root,"Renamed Map")));try refresh("renamed-same-row-reselection")
    try buttons(root,[true,true,true,true],"same-row-after-commit")
    try slowPause();try doubleClick(center(leafText(root,"Renamed Map")))
    let editor=try findWindow("exact Renamed Map editor",editorSignature);try use(editor);let editorAnchor=try anchorRoot(editor)
    try dump("double-click-editor",editor)
    let names=tree(editor).filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXValueAttribute)=="Renamed Map"}
    let notes=tree(editor).filter{str($0,kAXRoleAttribute)=="AXTextArea" && str($0,kAXValueAttribute)=="Original Rename Me"}
    try need(names.count==1 && notes.count==1,"Double click opened wrong editor name/notes")
    try guardInput();try need(AXUIElementSetAttributeValue(notes[0],kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"Cannot focus exact native notes")
    guard let focused=ax(attr(app,kAXFocusedUIElementAttribute)) else{throw Stop.failed("No native notes focus")}
    try need(same(focused,notes[0]),"Actual notes focus differs");try guardInput()
    try need(AXUIElementSetAttributeValue(focused,kAXValueAttribute as CFString,"Discarded editor notes" as CFString) == .success && str(focused,kAXValueAttribute)=="Discarded editor notes","Native notes edit not accepted")
    observations.append(["stage":"pending-editor-notes","value":"Discarded editor notes","name":"Renamed Map"])
    let freshEditor=try reacquireRoot(editorAnchor,"notes-mutated",{editorSignature($0)})
    try press(freshEditor,"Cancel","editor-cancel");try refresh("editor-cancelled")
    _=try leafText(root,"Original Rename Me");try originalStore("editor-cancel-keeps-saved-bytes")
    observations.append(["stage":"cancelled-editor-notes","value":"Original Rename Me","name":"Renamed Map"])
    try press(root,"OK","root-accept")
    try wait("root accepted and dismissed",3){!allWindows().contains(where:mapsSignature)}
    report["inputCompleted"]=true;report["workflow"]="image-maps-physical";report["passed"]=true
}catch{report["error"]=String(describing:error)}}
exit(report["passed"] as? Bool==true ? 0:1)
