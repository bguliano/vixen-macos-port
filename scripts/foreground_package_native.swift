// Normal-app package file-panel workflow. Guard core retained from reviewed Vixen2 full helper v6.
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
try need(cfg["workflow"] as? String == "package-file-panels","Explicit package workflow required")
let oldPointer=cfg["pointer"] as! [Double], began=now(),inputEnd=began+240,cleanupEnd=began+255
let app=AXUIElementCreateApplication(pid),systemAX=AXUIElementCreateSystemWide()
var currentRoot:AXUIElement?, currentNativeRoot:AXUIElement?, companionPanel:AXUIElement?, companionOwner:AXUIElement?, currentNativeID:Int?
var mouseHeld=false,heldKeys=Set<CGKeyCode>(),downs=0,ups=0,keyDowns=0,keyUps=0,ownershipLost=false,cleanupReading=false
var lastPoint=oldPointer.count==2 ? CGPoint(x:oldPointer[0],y:oldPointer[1]):.zero
var keyUpAcks=KeyUpAcknowledgements()
var inputFocusLost=false,releaseLeaseKeys=Set<CGKeyCode>(),releaseLeaseOrder=[CGKeyCode](),releaseLeaseUntil:Double=0
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
func physicalOnlyOurs()->Bool{let p=physical();return keyUpAcks.observe(time:now(),held:heldKeys,keys:p.0,buttons:p.1,flags:p.2,mouseHeld:mouseHeld)}
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

func guardInput(_ point:CGPoint?=nil)throws{try need(keyUpAcks.pending.isEmpty,"Posted keyUPs must be acknowledged before new input");try need(!inputFocusLost,"Original window focus was lost; no new input permitted");try guardApp();let focus=focusOwned();try need(now()<limit(),"Deadline after AX focus");if !focus{inputFocusLost=true;observations.append(["stage":"input-focus-lost","keyWindow":ax(attr(app,kAXFocusedWindowAttribute)).map{["role":str($0,kAXRoleAttribute),"title":str($0,kAXTitleAttribute)]} ?? [:],"mainWindow":ax(attr(app,kAXMainWindowAttribute)).map{["role":str($0,kAXRoleAttribute),"title":str($0,kAXTitleAttribute)]} ?? [:]])};try need(focus,"Exact observed window hierarchy lost focus");try guardApp();if let point{try need(nativeAt(point)==currentNativeID && now()<limit(),"Posted point is not in exact native window")};try guardApp();try need(now()<limit(),"Deadline after actual hit traversal")}
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

func mouse(_ kind:CGEventType,_ point:CGPoint)throws{try guardInput(point);guard let e=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else{throw Stop.failed("Cannot create mouse")};e.setIntegerValueField(.mouseEventClickState,value:1);try guardInput(point);if kind == .leftMouseDown{try need(!mouseHeld,"Duplicate mouse down");mouseHeld=true;downs+=1};if kind == .leftMouseUp{try need(mouseHeld,"Unpaired mouse up")};e.post(tap:.cghidEventTap);lastPoint=point;if kind == .leftMouseUp{mouseHeld=false;ups+=1};events.append(["kind":kind.rawValue,"point":[point.x,point.y],"window":currentNativeID ?? -1])}
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
func checkpoint(_ name:String)throws{try guardApp();try write(["checkpoint":name],out.appendingPathComponent("checkpoint-"+name+".request"));let file=out.appendingPathComponent("checkpoint-"+name+".json");try wait("saved checkpoint "+name,3){FileManager.default.fileExists(atPath:file.path)};let proof=try JSONSerialization.jsonObject(with:Data(contentsOf:file)) as! [String:Any];try need(proof["passed"] as? Bool==true,"Saved checkpoint mismatch "+name)}
final class BoundWindow {
    var root:AXUIElement
    let anchor:ObservedRootAnchor
    let signature:(AXUIElement)->Bool
    init(_ root:AXUIElement,_ signature:@escaping (AXUIElement)->Bool)throws {
        self.root=root;self.signature=signature;try need(signature(root),"Initial workflow window signature mismatch");try use(root);anchor=try anchorRoot(root)
    }
    @discardableResult func refresh(_ stage:String)throws->AXUIElement {root=try reacquireRoot(anchor,stage,signature);return root}
}
func samePath(_ a:String,_ b:String)->Bool { a.precomposedStringWithCanonicalMapping==b.precomposedStringWithCanonicalMapping }
func menu(_ name:String,_ stage:String)throws {
    guard let menu=ax(attr(app,kAXMenuBarAttribute)) else{throw Stop.failed("Missing app menu")}
    let matches=tree(menu).filter{str($0,kAXRoleAttribute)=="AXMenuItem" && str($0,kAXTitleAttribute)==name}
    try need(matches.count==1,"Missing unique menu item "+name);try action(matches[0],kAXPressAction,stage)
}
func panel(_ actionName:String,_ stage:String,_ expectedTitle:String)throws->AXUIElement {
    let p=try findWindow(stage){candidate in chooserRoleAllowed(role:str(candidate,kAXRoleAttribute),title:str(candidate,kAXTitleAttribute),expectedTitle:expectedTitle) && [actionName,"Cancel"].allSatisfy { name in tree(candidate,prune:true).contains { str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains(name) } }}
    try use(p);try dump(stage,p);return p
}
func disappeared(_ root:AXUIElement,_ stage:String)throws {try wait(stage){!allWindows().contains{same($0,root)}}}
func goToFocusSnapshot(_ stage:String)throws {
    func row(_ e:AXUIElement)->[String:Any] { ["role":str(e,kAXRoleAttribute),"title":str(e,kAXTitleAttribute),"description":str(e,kAXDescriptionAttribute),"identifier":str(e,kAXIdentifierAttribute),"value":str(e,kAXValueAttribute),"frame":(try? rect(e)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? []] }
    let focused=ax(attr(app,kAXFocusedUIElementAttribute))
    let data:[String:Any] = ["stage":stage,"focusedField":focused.map(row) ?? [:],"focusedWindow":ax(attr(app,kAXFocusedWindowAttribute)).map(row) ?? [:],"mainWindow":ax(attr(app,kAXMainWindowAttribute)).map(row) ?? [:],"focusedFieldWindowChain":focused.map{nativeHosts($0).map(row)} ?? [],"windows":allWindows().map(row),"privateNativeWindows":nativeWindows().filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}.map(nativeSummary)]
    observations.append(data);try write(data,out.appendingPathComponent(stage+".json"))
    if let root=focused.flatMap({nativeHosts($0).first}) {try dump(stage+"-root",root)}
}
@discardableResult func goTo(_ panel:AXUIElement,_ path:String,_ stage:String)throws->AXUIElement {
    try use(panel);let anchor=try anchorRoot(panel)
    let buttons=tree(panel,prune:true).filter{str($0,kAXRoleAttribute)=="AXButton"}.flatMap(labels)
    let operation=buttons.contains("Save") ? "Save":"Open"
    try need(buttons.contains(operation) && buttons.contains("Cancel"),"Go To requires an observed private file chooser")
    let observedRole=str(panel,kAXRoleAttribute),observedTitle=str(panel,kAXTitleAttribute)
    let chooserSignature:(AXUIElement)->Bool={root in str(root,kAXRoleAttribute)==observedRole && str(root,kAXTitleAttribute)==observedTitle && tree(root,prune:true).contains{str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains(operation)} && tree(root,prune:true).contains{str($0,kAXRoleAttribute)=="AXButton" && labels($0).contains("Cancel")}}
    try chord([55,56,5]);try goToFocusSnapshot(stage+"-goto-opened")
    var field:AXUIElement?
    try wait(stage+" focused Go To control",3) {
        guard let f=ax(attr(app,kAXFocusedUIElementAttribute)),owned(f),["AXTextField","AXComboBox"].contains(str(f,kAXRoleAttribute)),str(f,kAXIdentifierAttribute) != "saveAsNameTextField",str(f,kAXDescriptionAttribute) != "tag editor" else{return false}
        if let label=ax(attr(f,kAXTitleUIElementAttribute)),labels(label).contains("Save As:"){return false}
        field=f;return true
    }
    try goToFocusSnapshot(stage+"-goto-before-binding")
    let hosts=nativeHosts(field!),records=nativeWindows()
    let original=hosts.filter{host in guard let frame=try? rect(host) else{return false};return sameFrame(frame,anchor.hostFrame) && matchingNative(records,owner:Int(pid),frame:frame).contains{($0[kCGWindowNumber as String] as? Int)==anchor.nativeID}}
    try need(original.count==1,"Observed Go To control does not descend from the original exact native chooser host")
    guard let root=hosts.first else{throw Stop.failed("Observed Go To control has no owned window/sheet ancestor")}
    try use(root)
    guard let fresh=ax(attr(app,kAXFocusedUIElementAttribute)),same(fresh,field),nativeHosts(fresh).contains(where:{same($0,root)}) else{throw Stop.failed("Go To field changed while binding its native owner")}
    try set(fresh,path,stage+"-path");try chord([36])
    try wait(stage+" Go To dismissed"){!same(ax(attr(app,kAXFocusedUIElementAttribute)),field)}
    return try reacquireRoot(anchor,stage+"-chooser-return",chooserSignature)
}

func chooseOpen(_ wizard:BoundWindow,_ button:String,_ path:String,_ stage:String,_ expectedTitle:String)throws {
    try wizard.refresh("package-owner");try press(wizard.root,button,stage+"-browse")
    var chooser=try panel("Open",stage+"-open",expectedTitle);chooser=try goTo(chooser,path,stage+"-path");try dump(stage+"-selected",chooser)
    try press(chooser,"Open",stage+"-accept");try disappeared(chooser,stage+"-dismissed");try wizard.refresh("package-owner")
    try wait(stage+"-exact-selected-path"){tree(wizard.root).contains{labels($0).contains{samePath($0,path)}}}
    let raw=Set(tree(wizard.root).flatMap(labels).filter{samePath($0,path)});try need(raw.count==1,"Selected path readback ambiguous");observations.append(["stage":stage,"selectedPath":path,"observedSelectedPath":raw.first!])
}
func cancelChooser(_ wizard:BoundWindow,_ button:String,_ actionName:String,_ stage:String,_ expectedTitle:String)throws {
    try wizard.refresh("package-owner");try press(wizard.root,button,stage+"-browse");let chooser=try panel(actionName,stage+"-panel",expectedTitle)
    try press(chooser,"Cancel",stage+"-cancel");try disappeared(chooser,stage+"-dismissed");try wizard.refresh("package-owner");try dump(stage+"-after",wizard.root)
    observations.append(["stage":stage,"panelCancelled":true])
}
func sourceChooser(_ wizard:BoundWindow,_ cancel:Bool)throws {
    try wizard.refresh("package-owner");let buttons=tree(wizard.root).filter{str($0,kAXRoleAttribute)=="AXButton" && (labels($0).contains("Explore Sequence folder") || str($0,kAXHelpAttribute)=="Explore Sequence folder")}
    try need(buttons.count==1,"No unique observed source chooser button");try action(buttons[0],kAXPressAction,cancel ? "source-cancel-browse":"source-accept-browse")
    var chooser=try panel("Open",cancel ? "source-cancel-panel":"source-accept-panel","Select Sequences")
    if cancel {try press(chooser,"Cancel","source-chooser-cancel")} else {chooser=try goTo(chooser,cfg["sourceSequence"] as! String,"source-file-path");try press(chooser,"Open","source-chooser-accept")}
    try disappeared(chooser,"source chooser dismissed");try wizard.refresh("package-owner")
    if cancel {try need(attr(try unique(wizard.root,"AXButton","Next >>"),kAXEnabledAttribute) as? Bool==false,"Cancel unexpectedly populated source list")}
    else {try wait("exact source list row"){tree(wizard.root).contains{labels($0).contains{samePath($0,cfg["sourceSequence"] as! String)}}}}
    observations.append(["stage":cancel ? "source-chooser-cancelled":"source-chooser-accepted"])
}
// Select the actual native Save As field through its observed label relation,
// never through an assumed original value or extension visibility setting.
func saveNameField(_ panel:AXUIElement,_ stage:String)throws->AXUIElement {
    let nodes=tree(panel,prune:true)
    let labelsFound=nodes.filter{str($0,kAXRoleAttribute)=="AXStaticText" && labels($0).contains("Save As:")}
    try need(labelsFound.count==1,"Native Save As label is not unique")
    let label=labelsFound[0]
    var related=nodes.filter{str($0,kAXRoleAttribute)=="AXTextField" && same(ax(attr($0,kAXTitleUIElementAttribute)),label)}
    if let parent=ax(attr(label,kAXParentAttribute)),owned(parent) {
        let siblings=children(parent)
        if let index=siblings.firstIndex(where:{same($0,label)}),index+1<siblings.count {
            let next=siblings[index+1]
            if str(next,kAXRoleAttribute)=="AXTextField" && !related.contains(where:{same($0,next)}) {related.append(next)}
        }
    }
    if related.isEmpty {
        // Retained native compact/expanded panel evidence identifies the only
        // unlabeled non-tag text field beside Save As; fail if not unique.
        related=nodes.filter{str($0,kAXRoleAttribute)=="AXTextField" && str($0,kAXDescriptionAttribute).isEmpty && str($0,kAXSubroleAttribute) != "AXSearchField"}
    }
    try need(related.count==1,"No unique native Save As label-linked or non-tag text field")
    let field=related[0]
    try need(owned(field) && (same(ax(attr(field,kAXWindowAttribute)),panel) || ancestry(field).contains{same($0,panel)}) && (try rect(panel)).contains(try rect(field)),"Save filename field left exact panel")
    var settable:DarwinBoolean=false
    try need(AXUIElementIsAttributeSettable(field,kAXValueAttribute as CFString,&settable) == .success && settable.boolValue,"Native Save As value is not editable")
    observations.append(["stage":stage+"-filename-field","observedValue":str(field,kAXValueAttribute),"label":"Save As:","identifier":str(field,kAXIdentifierAttribute)])
    return field
}
func chooseSave(_ previous:AXUIElement,_ path:String,_ stage:String)throws {
    let destination=URL(fileURLWithPath:path),directory=destination.deletingLastPathComponent()
    try dump(stage+"-save-initial",previous);try use(previous)
    _ = try saveNameField(previous,stage+"-initial")
    try need(FileManager.default.fileExists(atPath:directory.path),"Private Save directory missing")
    let chooser=try goTo(previous,directory.path+"/",stage+"-directory")
    try dump(stage+"-save-directory",chooser)
    let locations=tree(chooser).filter{str($0,kAXRoleAttribute)=="AXPopUpButton" && labels($0).contains("Where:")}
    try need(locations.count==1 && samePath(str(locations[0],kAXValueAttribute),directory.lastPathComponent),"Native Save directory mismatch")
    let field=try saveNameField(chooser,stage+"-after-directory")
    try set(field,destination.lastPathComponent,stage+"-filename");try chord([48])
    let fresh=try saveNameField(chooser,stage+"-committed")
    try need(samePath(str(fresh,kAXValueAttribute),destination.lastPathComponent),"Save filename changed on commit")
    observations.append(["stage":stage+"-destination","directory":str(locations[0],kAXValueAttribute),"filename":str(fresh,kAXValueAttribute)])
    try dump(stage+"-before-save",chooser);try press(chooser,"Save",stage+"-save");try disappeared(chooser,stage+"-dismissed")
    observations.append(["stage":stage,"savedPath":path])
}

func startWizard(_ admin:BoundWindow,_ importing:Bool)throws->BoundWindow {
    try admin.refresh("administration-before-package-menu")
    try menu(importing ? "Import Sequence Package":"Export Sequence Package",importing ? "open-import":"open-export")
    let title=importing ? "Sequence Package Import":"Sequence Package Export"
    let signature:(AXUIElement)->Bool={ str($0,kAXTitleAttribute)==title && tree($0,prune:true).contains{ button in str(button,kAXRoleAttribute)=="AXButton" && [str(button,kAXTitleAttribute),str(button,kAXDescriptionAttribute)].contains{["Next >>","Finish"].contains($0)} } }
    let root=try findWindow(title,signature)
    return try BoundWindow(root,signature)
}

func next(_ wizard:BoundWindow,_ stage:String)throws {try wizard.refresh("package-owner");try press(wizard.root,"Next >>",stage);try wizard.refresh("package-owner")}
func finish(_ wizard:BoundWindow,_ text:String,_ count:String,_ stage:String)throws {
    try wizard.refresh("package-owner");try wait(stage+" completed",20){let values=Set(tree(wizard.root,prune:true).flatMap(labels));return values.contains(text) && values.contains(count)}
    try dump(stage+"-complete",wizard.root);observations.append(["stage":stage,"status":text,"count":count]);try press(wizard.root,"Finish",stage+"-finish");try disappeared(wizard.root,stage+"-closed")
}
func importRun(_ admin:BoundWindow,_ cancel:Bool)throws {
    let wizard=try startWizard(admin,true)
    if cancel {try cancelChooser(wizard,"Browse Sequence Package File","Open","package-open-cancel","Sequence Package File")}
    try chooseOpen(wizard,"Browse Sequence Package File",cfg["packageOutput"] as! String,"package-open-accept","Sequence Package File")
    if cancel {try cancelChooser(wizard,"Browse Profile Map File","Open","map-open-cancel","Profile Map File")}
    try chooseOpen(wizard,"Browse Profile Map File",cfg["mapFile"] as! String,"map-open-accept","Profile Map File")
    try next(wizard,"import-next-sequences");try need(tree(wizard.root).contains{labels($0).contains{samePath($0,"Panel Source Café")}},"Expected only private import sequence missing");try dump(cancel ? "cancel-import-sequences":"accepted-import-sequences",wizard.root)
    try next(wizard,"import-next-summary");try wizard.refresh("import-owner-before-start");try press(wizard.root,"Next >>","import-start")
    // Start opens the collision alert; bind that actual child directly. The
    // original wizard is deliberately reacquired only after its Save closes.
    let warning=try findWindow("sequence collision warning"){tree($0,prune:true).contains{labels($0).contains{samePath($0,"The sequence Panel Source Café already exists. Overwrite?")}}}
    try use(warning);try press(warning,"Choose Another Name","import-choose-another-name")
    let save=try panel("Save",cancel ? "cancel-import-save":"accepted-import-save","Timed Sequence")
    if cancel {try press(save,"Cancel","import-save-cancel");try disappeared(save,"cancelled import save dismissed")}
    else {try chooseSave(save,cfg["importedSequence"] as! String,"import-native-save")}
    try finish(wizard,"Package Import Complete",cancel ? "0 sequence(s) completed; 1 skipped.":"1 sequence(s) completed; 0 skipped.",cancel ? "cancelled-import":"imported")
    try admin.refresh("administration-after-package");try checkpoint(cancel ? "cancelled-import":"imported")
}

func cleanup(){cleanupReading=true;var errors=[String]();func releaseSafe(_ point:CGPoint?=nil)->Bool{guard now()<cleanupEnd,!ownershipLost,foreground(),pointerStable(),physicalOnlyOurs(),focusOwned() else{return false};guard point.map({nativeAt($0)==currentNativeID}) ?? true else{return false};return foreground() && pointerStable() && physicalOnlyOurs() && now()<cleanupEnd}
    if mouseHeld,releaseSafe(lastPoint),let e=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left),releaseSafe(lastPoint),now()<cleanupEnd{e.post(tap:.cghidEventTap);mouseHeld=false;ups+=1;pump()}
    for code in releaseLeaseOrder.reversed() where heldKeys.contains(code) {
        do{try postOwnedKeyUp(code)}catch{errors.append("Owned key release: "+String(describing:error))}
    }
    if !keyUpAcks.pending.isEmpty {do{try awaitKeyUpAcknowledgements()}catch{errors.append("Pending keyUP acknowledgement: "+String(describing:error))}}
    let clear=clearPhysical() && !mouseHeld && heldKeys.isEmpty && keyUpAcks.pending.isEmpty && downs==ups && keyDowns==keyUps
    func restore()->Bool{let front=NSWorkspace.shared.frontmostApplication?.processIdentifier;let yes=idleRestore(time:now(),limit:cleanupEnd,identity:priorIdentity(),front:front==pid || front==pid_t(previous["pid"] as! Int),pointer:pointerStable(),clear:clearPhysical(),lost:ownershipLost);return yes && now()<cleanupEnd}
    if clear,oldPointer.count==2,restore(){if restore() && now()<cleanupEnd{report["restoredPointer"]=CGWarpMouseCursorPosition(CGPoint(x:oldPointer[0],y:oldPointer[1])) == .success;lastPoint=CGPoint(x:oldPointer[0],y:oldPointer[1]);pump()};if restore() && now()<cleanupEnd{_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};for _ in 0..<20 where now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}else{errors.append("Restoration withheld by ownership/input guard")}
    let finalPhysical=physical();let finalClear=clear && finalPhysical.0.isEmpty && finalPhysical.1.isEmpty && finalPhysical.2.isEmpty
    report["pendingKeyUpAcknowledgements"]=keyUpAcks.pending.sorted().map(Int.init)
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
    let adminRoot=try findWindow("ready Administration"){str($0,kAXTitleAttribute).contains("Vixen Administration") && tree($0,prune:true).contains{labels($0).contains("Open Sequence...") && attr($0,kAXEnabledAttribute) as? Bool==true}}
    let admin=try BoundWindow(adminRoot){str($0,kAXTitleAttribute).contains("Vixen Administration") && tree($0,prune:true).contains{labels($0).contains("Open Sequence...") && attr($0,kAXEnabledAttribute) as? Bool==true}}
    let wizard=try startWizard(admin,false)
    try sourceChooser(wizard,true);try sourceChooser(wizard,false);try next(wizard,"export-next-output")
    let defaults=Set(tree(wizard.root).flatMap(labels).filter{$0.hasPrefix(cfg["profileExportDirectory"] as! String) && $0.hasSuffix(".vpkg")});try need(defaults.count==1,"Cannot identify native default export path")
    let initialOutput=defaults.first!;observations.append(["stage":"initial-export-output","path":initialOutput])
    try cancelChooser(wizard,"Browse Sequence Export Location","Save","package-save-cancel","Sequence Export Location")
    try need(tree(wizard.root).flatMap(labels).contains{samePath($0,initialOutput)},"Save Cancel changed output selection")
    try press(wizard.root,"Browse Sequence Export Location","package-save-accept-browse")
    let save=try panel("Save","package-save-accept-panel","Sequence Export Location")
    try chooseSave(save,cfg["packageOutput"] as! String,"package-native-save")
    try next(wizard,"export-next-summary");try next(wizard,"export-start")
    try finish(wizard,"Package Export Complete","1 sequence(s) completed; 0 skipped.","exported")
    try admin.refresh("administration-after-package");try checkpoint("exported")
    try importRun(admin,true);try importRun(admin,false)
    try admin.refresh("administration-after-package");try menu("Quit Vixen","quit-menu")
    let end=min(inputEnd,now()+8);while NSRunningApplication(processIdentifier:pid)?.isTerminated==false && now()<end{try need(priorOrTarget() && pointerStable(),"Desktop changed during normal Quit");pump()}
    try need(NSRunningApplication(processIdentifier:pid)?.isTerminated ?? true,"Normal Quit did not terminate")
    report["normalQuitMenu"]=true;report["inputCompleted"]=true;report["workflow"]="package-file-panels";report["passed"]=true
}catch{report["error"]=String(describing:error)}}
exit(report["passed"] as? Bool==true ? 0:1)
