// Actual Preview linker drops and double-click. Existing harness only opens/saves/reads state.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit

enum Failure:Error{case failed(String)}
func require(_ ok:Bool,_ why:String)throws{if !ok{throw Failure.failed(why)}}
func sha(_ p:String)throws->String{SHA256.hash(data:try Data(contentsOf:URL(fileURLWithPath:p))).map{String(format:"%02x",$0)}.joined()}
func physicalAllowed(keys:Set<Int>,buttons:Set<Int>,flags:UInt64,held:Bool,pending:Bool)->Bool{keys.isEmpty && flags==0 && buttons.allSatisfy{ $0==0 && (held || pending) }}
func terminalReturnAllowed(resultExists:Bool,exactPrevious:Bool,clearAndStable:Bool,elapsed:Double)->Bool {resultExists && exactPrevious && clearAndStable && elapsed>=0 && elapsed<=0.5}
func ownershipObservation(_ observed:Bool,_ lost:inout Bool)->Bool{if !observed{lost=true};return !lost}
let args=CommandLine.arguments
if args.count==2 && args[1]=="--pure-checks"{
 let checks=[physicalAllowed(keys:[],buttons:[],flags:0,held:false,pending:false),physicalAllowed(keys:[],buttons:[0],flags:0,held:true,pending:false),physicalAllowed(keys:[],buttons:[0],flags:0,held:false,pending:true),!physicalAllowed(keys:[55],buttons:[],flags:0,held:true,pending:false),!physicalAllowed(keys:[],buttons:[1],flags:0,held:true,pending:true),!physicalAllowed(keys:[],buttons:[],flags:1,held:true,pending:true),!physicalAllowed(keys:[],buttons:[0],flags:0,held:false,pending:false),terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:false,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:false,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:false,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.501)]
 var lost=false;let ownershipChecks=[ownershipObservation(true,&lost),!ownershipObservation(false,&lost),!ownershipObservation(true,&lost)]
 precondition((checks+ownershipChecks).allSatisfy{$0});print("{\"passed\":true,\"checks\":15,\"nativeOperations\":0}");exit(0)
}
func identity(_ a:NSRunningApplication?)->[String:Any]{["pid":Int(a?.processIdentifier ?? -1),"bundle":a?.bundleIdentifier ?? "","executable":a?.executableURL?.standardizedFileURL.path ?? ""]}
if args.count==3 && args[1]=="--desktop"{let m:[String:Any]=["previous":identity(NSWorkspace.shared.frontmostApplication),"accessibility":AXIsProcessTrusted(),"eventPosting":CGPreflightPostEventAccess(),"permissionRequested":false,"inputPosted":false];try JSONSerialization.data(withJSONObject:m,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[2]));exit(0)}
guard args.count==4,let pid=Int32(args[1]) else{fatalError("pid,out,executable")}
let out=URL(fileURLWithPath:args[2]),executable=args[3]
let prepared=try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("prepared.json"))) as! [String:Any]
let bundle=prepared["bundleIdentifier"] as! String,previous=(try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("desktop-before.json"))) as! [String:Any])["previous"] as! [String:Any]
let began=ProcessInfo.processInfo.systemUptime,terminalEnd=began+62,cleanupEnd=began+65
let overallInputEnd=began+55
var inputEnd=overallInputEnd,releaseEnd=cleanupEnd
let oldPointer=CGEvent(source:nil)?.location,app=AXUIElementCreateApplication(pid),system=AXUIElementCreateSystemWide()
var windows=[String:AXUIElement](),frames=[String:CGRect](),ids=[String:Int](),points=[String:CGPoint](),nativeRoots=[String:AXUIElement](),hostFrames=[String:CGRect]()
var posted=false,activated=false,ownershipLost=false,held=false,pendingUp=false,lastPoint:CGPoint?,downs=0,ups=0
var firstClickDown:Double?
var events=[[String:Any]](),observations=[[String:Any]](),report:[String:Any]=["passed":false,"scope":"Two actual Preview linker drops (valid lower half and blank rejection), then actual target selection/source double-click and Cancel. No assignment/drop/double-click hooks or key events."]
func now()->Double{ProcessInfo.processInfo.systemUptime}
func pump(_ t:Double=0.025){RunLoop.current.run(until:Date().addingTimeInterval(t))}
func target()->NSRunningApplication?{NSRunningApplication(processIdentifier:pid)}
func ownedProcess()->Bool{let t=target();return t?.isTerminated==false && t?.executableURL?.standardizedFileURL.path==executable && t?.bundleIdentifier==bundle}
func foreground()->Bool{ownedProcess() && target()?.isActive==true && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid}
func previousMatches()->Bool{let t=NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int));return t?.isTerminated==false && t?.bundleIdentifier==previous["bundle"] as? String && t?.executableURL?.standardizedFileURL.path==previous["executable"] as? String}
func priorOrTarget()->Bool{let t=NSWorkspace.shared.frontmostApplication;return t?.processIdentifier==pid ? ownedProcess():previousMatches() && t?.processIdentifier==pid_t(previous["pid"] as! Int)}
func pointerStable()->Bool{guard let p=CGEvent(source:nil)?.location,let e=lastPoint ?? oldPointer else{return false};return hypot(p.x-e.x,p.y-e.y)<1}
func physical()->Bool{physicalAllowed(keys:Set((0..<128).filter{CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0))}),buttons:Set([CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}.map{Int($0.rawValue)}),flags:CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand,.maskSecondaryFn]).rawValue,held:held,pending:pendingUp)}
func owned(_ n:AXUIElement)->Bool{var p:pid_t = -1;return AXUIElementGetPid(n,&p) == .success && p==pid}
func attr(_ n:AXUIElement,_ a:String,cleanup:Bool=false)->CFTypeRef?{guard now()<(cleanup ? releaseEnd:inputEnd),owned(n) else{return nil};AXUIElementSetMessagingTimeout(n,0.1);var v:CFTypeRef?;return AXUIElementCopyAttributeValue(n,a as CFString,&v) == .success ? v:nil}
func ax(_ v:CFTypeRef?)->AXUIElement?{guard let v,CFGetTypeID(v)==AXUIElementGetTypeID() else{return nil};return (v as! AXUIElement)}
func frame(_ n:AXUIElement,cleanup:Bool=false)throws->CGRect{guard let p=attr(n,kAXPositionAttribute,cleanup:cleanup),let s=attr(n,kAXSizeAttribute,cleanup:cleanup),CFGetTypeID(p)==AXValueGetTypeID(),CFGetTypeID(s)==AXValueGetTypeID() else{throw Failure.failed("Missing owned AX geometry")};var pt=CGPoint.zero,sz=CGSize.zero;try require(AXValueGetValue(p as! AXValue,.cgPoint,&pt) && AXValueGetValue(s as! AXValue,.cgSize,&sz),"AX geometry type");return CGRect(origin:pt,size:sz)}
func labels(_ n:AXUIElement)->[String]{[kAXTitleAttribute,kAXDescriptionAttribute,kAXValueAttribute].compactMap{attr(n,$0) as? String}}
func native()->[[String:Any]]{CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] ?? []}
func nativeFrame(_ r:[String:Any])->CGRect?{guard let d=r[kCGWindowBounds as String] as? [String:Any] else{return nil};return CGRect(dictionaryRepresentation:d as CFDictionary)}
func descendants(_ n:AXUIElement,_ prune:Bool=false)->[AXUIElement]{var result=[AXUIElement]();func visit(_ e:AXUIElement,_ d:Int){guard d<16,result.count<450,now()<inputEnd else{return};result.append(e);if prune && ["AXTable","AXOutline","AXList"].contains(attr(e,kAXRoleAttribute) as? String ?? ""){return};for c in attr(e,kAXChildrenAttribute) as? [AXUIElement] ?? []{visit(c,d+1)}};visit(n,0);return result}
func ancestors(_ n:AXUIElement,cleanup:Bool=false)->[AXUIElement]{var result=[n],node=n;for _ in 0..<14{guard let parent=ax(attr(node,kAXParentAttribute,cleanup:cleanup)),owned(parent) else{break};if ["AXWindow","AXSheet"].contains(attr(parent,kAXRoleAttribute,cleanup:cleanup) as? String ?? ""){result.append(parent)};node=parent};return result}
func sameFrame(_ a:CGRect,_ b:CGRect)->Bool{abs(a.minX-b.minX)<1 && abs(a.minY-b.minY)<1 && abs(a.width-b.width)<1 && abs(a.height-b.height)<1}
func linker()throws->AXUIElement{
 let all=(attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? []).flatMap{descendants($0,true)}.filter{["AXWindow","AXSheet"].contains(attr($0,kAXRoleAttribute) as? String ?? "")}
 var matches=[AXUIElement]();for w in all{let names=Set(descendants(w,true).flatMap{labels($0)});if ["Link Elements","Available Elements","OK","Cancel"].allSatisfy({names.contains($0)}) && !matches.contains(where:{CFEqual($0,w)}){matches.append(w)}}
 let leaf=matches.filter{w in !matches.contains{other in !CFEqual(w,other) && ancestors(other).contains{CFEqual($0,w)}}}
 try require(leaf.count==1,"Expected one exact live Link Elements sheet");return leaf[0]
}
func bind(_ label:String)throws{
 let end=min(inputEnd,now()+1.2)
 repeat{
  try guardApp();let w=try linker(),f=try frame(w),ownedRows=native().filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}
  var host:AXUIElement?,hf:CGRect?,match:[[String:Any]]=[]
  for h in ancestors(w){guard let fr=try? frame(h),fr.insetBy(dx:-1,dy:-1).contains(f) else{continue};let found=ownedRows.filter{($0[kCGWindowAlpha as String] as? Double ?? 0)>0 && nativeFrame($0).map{sameFrame($0,fr)}==true};if !found.isEmpty{host=h;hf=fr;match=found;break}}
  observations.append(["stage":"native-binding","label":label,"axFrame":[f.minX,f.minY,f.width,f.height],"matchingIDs":match.compactMap{$0[kCGWindowNumber as String] as? Int},"privateNativeWindows":ownedRows.map{r in ["id":r[kCGWindowNumber as String] ?? NSNull(),"bounds":r[kCGWindowBounds as String] ?? NSNull(),"layer":r[kCGWindowLayer as String] ?? NSNull()]}])
  try guardApp();try require(now()<end,"Native binding settle deadline")
  if match.count==1,let host,let hf{let id=match[0][kCGWindowNumber as String] as! Int;try require(ids[label]==nil || ids[label]==id,"Linker native identity changed");if let old=frames[label]{try require(sameFrame(old,f),"Linker root geometry changed")};windows[label]=w;frames[label]=f;nativeRoots[label]=host;hostFrames[label]=hf;ids[label]=id;return}
  try require(match.count<2,"Ambiguous native linker host");pump()
 }while now()<end
 throw Failure.failed("No exact native linker host after settle")
}
func keyMain(_ names:[String],cleanup:Bool=false)->Bool{guard let k=ax(attr(app,kAXFocusedWindowAttribute,cleanup:cleanup)),let m=ax(attr(app,kAXMainWindowAttribute,cleanup:cleanup)) else{return false};let allowed=names.flatMap{windows[$0].map{ancestors($0,cleanup:cleanup)} ?? []};return [k,m].allSatisfy{n in allowed.contains{CFEqual($0,n)}}}
func guardApp()throws{try require(now()<inputEnd && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Input cutoff/cooperative abort");try require(!pendingUp,"MouseUP still awaiting physical acknowledgment");let ok = !ownershipLost && foreground() && pointerStable() && physical();if !ok{ownershipLost=true};try require(ok && now()<inputEnd,"Private process/pointer/input ownership lost")}
func hit(_ point:CGPoint,_ label:String,cleanup:Bool=false)->Bool{
 let end=cleanup ? releaseEnd:inputEnd
 guard now()<end,let w=windows[label],let f=frames[label],let host=nativeRoots[label],let hf=hostFrames[label],let id=ids[label],f.contains(point),let live=try? frame(w,cleanup:cleanup),sameFrame(live,f),let liveHost=try? frame(host,cleanup:cleanup),sameFrame(liveHost,hf),ancestors(w,cleanup:cleanup).contains(where:{CFEqual($0,host)}) else{return false}
 guard native().filter({($0[kCGWindowOwnerPID as String] as? Int)==Int(pid) && ($0[kCGWindowNumber as String] as? Int)==id && nativeFrame($0).map{sameFrame($0,hf)}==true && ($0[kCGWindowAlpha as String] as? Double ?? 0)>0}).count==1 else{return false}
 AXUIElementSetMessagingTimeout(system,0.1);var found:AXUIElement?;guard AXUIElementCopyElementAtPosition(system,Float(point.x),Float(point.y),&found) == .success,let found,owned(found),now()<end else{return false}
 if let owner=ax(attr(found,kAXWindowAttribute,cleanup:cleanup)),CFEqual(owner,w){return now()<end}
 var node:AXUIElement?=found
 for _ in 0..<14{guard let n=node,now()<end,owned(n) else{return false};if CFEqual(n,w){return now()<end};node=ax(attr(n,kAXParentAttribute,cleanup:cleanup))};return false
}
func mouse(_ kind:CGEventType,_ p:CGPoint,_ label:String,_ clicks:Int64=1)throws{
 try guardApp();try require(keyMain([label]) && hit(p,label),"Exact input window/point changed")
 try require((try? frame(windows[label]!)).map{sameFrame($0,frames[label]!)}==true,"Window moved during pointer input")
 guard let e=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:p,mouseButton:.left) else{throw Failure.failed("Mouse allocation")};e.flags=[];e.setIntegerValueField(.mouseEventClickState,value:clicks)
 try guardApp();try require(hit(p,label),"Immediate actual hit changed");try guardApp()
 let postedAt=now()
 if kind == .leftMouseDown{
  try require(!held,"Duplicate down")
  if clicks==2{guard let first=firstClickDown else{throw Failure.failed("Missing first click down")};let gap=postedAt-first;try require(gap>=0 && gap<NSEvent.doubleClickInterval,"Actual second down exceeded native double-click interval");report["doubleClickInterval"]=NSEvent.doubleClickInterval;report["doubleClickActualGap"]=gap;report["doubleClickDownTimesSeconds"]=[first-began,postedAt-began]}else{firstClickDown=postedAt}
  held=true;downs+=1
 };if kind == .leftMouseUp{try require(held,"Unpaired up")}
 e.post(tap:.cghidEventTap);posted=true;lastPoint=p
 if kind == .leftMouseUp{held=false;pendingUp=true;ups+=1}
 events.append(["kind":kind.rawValue,"point":[p.x,p.y],"label":label,"window":ids[label]!,"clickState":clicks,"elapsedSeconds":postedAt-began]);pump(0.025)
}
func releaseSafe()->Bool{let ok = !ownershipLost && foreground() && pointerStable() && physical();if !ok{ownershipLost=true};return ok && now()<releaseEnd}
func acknowledge()throws{let end=min(releaseEnd,now()+0.6);var absent:Double?;while pendingUp && now()<end{try require(releaseSafe(),"Release acknowledgment ownership changed");if !CGEventSource.buttonState(.combinedSessionState,button:.left){if let t=absent,now()-t>=0.025{pendingUp=false}else if absent==nil{absent=now()}}else{absent=nil};if pendingUp{pump(0.015)}};try require(!pendingUp && now()<end,"MouseUP not acknowledged")}
func fieldPoint(_ owner:AXUIElement,_ name:String)throws->CGPoint{let fields=descendants(owner).filter{attr($0,kAXRoleAttribute) as? String=="AXStaticText" && labels($0).contains(name)};try require(fields.count==1,"Expected unique native text "+name);let f=try frame(fields[0]);try require(f.width>5 && f.height>5 && (try frame(owner)).insetBy(dx:-1,dy:-1).contains(f),"Text outside native container");return CGPoint(x:f.midX,y:f.midY)}
func controls(_ label:String)throws->(AXUIElement,AXUIElement){try bind(label);let all=descendants(windows[label]!),outlines=all.filter{attr($0,kAXRoleAttribute) as? String=="AXOutline" && labels($0).contains("Available Elements")},tables=all.filter{attr($0,kAXRoleAttribute) as? String=="AXTable"};try require(outlines.count==1 && tables.count==1,"Expected native Available Elements outline and one linked table");return(outlines[0],tables[0])}
func linked(_ label:String,_ expected:String,_ stage:String)throws->(CGPoint,CGPoint){let c=try controls(label),rs=attr(c.1,kAXRowsAttribute) as? [AXUIElement] ?? [];try require(rs.count==1,"Standard string must have one linked row");let rf=try frame(rs[0]),tf=try frame(c.1),names=descendants(rs[0]).flatMap{labels($0)};observations.append(["stage":stage,"linkedRowLabels":names,"rowFrame":[rf.minX,rf.minY,rf.width,rf.height],"tableFrame":[tf.minX,tf.minY,tf.width,tf.height]]);try require(names.contains("1") && names.contains(expected) && !names.contains(expected=="Fixture One" ? "Fixture Two":"Fixture One"),"Exact linked row differs at "+stage);let cell=try fieldPoint(c.1,expected);let lower=CGPoint(x:cell.x,y:rf.minY+rf.height*0.75),blank=CGPoint(x:cell.x,y:tf.maxY-20);try require(tf.contains(lower) && rf.contains(lower) && tf.contains(blank) && blank.y>rf.maxY+30,"Need real lower-row and blank-table positions");return(lower,blank)}
func drag(_ label:String,_ source:String,_ end:CGPoint,_ stage:String)throws{let c=try controls(label),start=try fieldPoint(c.0,source),expected=prepared["elementIDs"] as! [String:String],board=NSPasteboard(name:.drag),before=board.changeCount;try mouse(.mouseMoved,start,label);pump(0.08);try mouse(.leftMouseDown,start,label);pump(0.08);for i in 1...6{try mouse(.leftMouseDragged,CGPoint(x:start.x+CGFloat(i)*3,y:start.y),label)};let payload=board.pasteboardItems?.compactMap{$0.string(forType:.init("com.vixenlights.outline-items"))} ?? [];observations.append(["stage":stage+"-payload","changeCountBefore":before,"changeCountDuring":board.changeCount,"payload":payload,"source":source,"start":[start.x,start.y],"end":[end.x,end.y]]);try require(board.changeCount != before && payload.count==1 && payload[0].split(separator:"/").last.map(String.init)==expected[source],"Real source drag payload missing or wrong");let elbow=CGPoint(x:start.x+18,y:start.y);for i in 1...24{let t=CGFloat(i)/24;try mouse(.leftMouseDragged,CGPoint(x:elbow.x+(end.x-elbow.x)*t,y:elbow.y+(end.y-elbow.y)*t),label)};try mouse(.leftMouseUp,end,label);try acknowledge();pump(0.15)}
func click(_ label:String,_ point:CGPoint,_ count:Int64=1)throws{try mouse(.mouseMoved,point,label,count);try mouse(.leftMouseDown,point,label,count);try mouse(.leftMouseUp,point,label,count);try acknowledge()}
func marker(_ name:String,_ budget:Double)throws{
 inputEnd=overallInputEnd;releaseEnd=cleanupEnd;let path=out.appendingPathComponent("ui/"+name+".png")
 func observe()throws{try require(now()<inputEnd && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Marker deadline/abort");let good=priorOrTarget() && physical() && pointerStable();try require(ownershipObservation(good,&ownershipLost),"Marker observed identity/pointer/input interference")}
 while !FileManager.default.fileExists(atPath:path.path){try observe();pump()};try observe()
 let stamp=try FileManager.default.attributesOfItem(atPath:path.path)[.modificationDate] as! Date,wall=Date(),mono=now(),age=wall.timeIntervalSince(stamp)
 try require(age.isFinite && age>=0 && age<4 && abs(Date().timeIntervalSince(wall)-(now()-mono))<0.05,"Unsafe ready marker age/clock")
 inputEnd=min(overallInputEnd,mono+budget-age);releaseEnd=min(cleanupEnd,inputEnd+4);observations.append(["stage":"marker-"+name,"ageSeconds":age,"inputCutoffSeconds":inputEnd-began,"releaseCutoffSeconds":releaseEnd-began]);try require(inputEnd-now()>budget-4,"Insufficient input reserve")
}
func dump(_ label:String)throws{let rows=descendants(windows[label]!).map{n in ["role":attr(n,kAXRoleAttribute) as? String ?? "","labels":labels(n),"frame":(try? frame(n)).map{[$0.minX,$0.minY,$0.width,$0.height]} ?? []] as [String:Any]};try JSONSerialization.data(withJSONObject:rows,options:[.prettyPrinted,.sortedKeys]).write(to:out.appendingPathComponent(label+".ax.json"))}
func publish(){if let d=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]){try? d.write(to:out.appendingPathComponent("foreground.json"),options:.atomic)}}
do{
 defer{
  if held,releaseSafe(),let p=CGEvent(source:nil)?.location,windows.keys.contains(where:{hit(p,$0,cleanup:true)}),let e=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:p,mouseButton:.left),releaseSafe(){e.flags=[];e.post(tap:.cghidEventTap);held=false;pendingUp=true;ups+=1}
  if pendingUp{try? acknowledge()}
  func restoreSafe()->Bool{guard now()<cleanupEnd else{return false};let clear=physical(),stable=pointerStable(),identity=priorOrTarget() && previousMatches();guard ownershipObservation(clear && stable && identity,&ownershipLost) else{return false};return !held && !pendingUp && now()<cleanupEnd}
  if activated && restoreSafe(){if posted,let oldPointer,restoreSafe(){report["restoredPointer"]=CGWarpMouseCursorPosition(oldPointer) == .success;lastPoint=oldPointer};if restoreSafe(){_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};while now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}
  let clear = !held && !pendingUp && downs==ups && physical()
  report["noInputHeld"]=clear;report["pendingMouseUp"]=pendingUp;report["mouseDowns"]=downs;report["mouseUps"]=ups;report["keyEvents"]=0;report["events"]=events;report["observations"]=observations;report["windowIDs"]=ids;report["ownershipLost"]=ownershipLost;report["restoredPrevious"]=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);report["restoredPointer"] = !posted || (lastPoint==oldPointer && pointerStable());report["elapsedSeconds"]=now()-began
  report["cleanupCompleted"]=clear && !ownershipLost && now()<cleanupEnd
  if report["cleanupCompleted"] as? Bool != true || report["restoredPrevious"] as? Bool != true || report["restoredPointer"] as? Bool != true{report["passed"]=false};publish()
 }
 do{
  try require(try sha(executable)==prepared["appSha256"] as? String && bundle.hasPrefix("com.vixenlights.verification."),"Private executable pin")
  try require(oldPointer != nil && previousMatches() && physical() && pointerStable() && AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Initial ownership/permissions")
  try marker("drop-ready",12)
  try require(ownedProcess() && priorOrTarget() && physical() && pointerStable(),"Startup identity/input")
  try require(target()?.activate(options:[])==true,"Activation refused");activated=true;while !foreground() && now()<inputEnd{pump()};try guardApp()
  try bind("drop");try dump("drop");let first=try linked("drop","Fixture Two","initial")
  try drag("drop","Fixture One",first.0,"valid-lower-half")
  let after=try linked("drop","Fixture One","after-valid")
  try drag("drop","Fixture Two",after.1,"blank")
  _ = try linked("drop","Fixture One","after-blank")
  let store=out.appendingPathComponent("profile/SystemData/ModuleStore.xml"),config=out.appendingPathComponent("profile/SystemData/SystemConfig.xml"),hashes=prepared["profileHashes"] as! [String:String]
  try require(try sha(store.path)==hashes["SystemData/ModuleStore.xml"] && sha(config.path)==hashes["SystemData/SystemConfig.xml"],"Private source persisted before OK+Save")
  report["beforeSaveUnchanged"]=true;try Data("complete".utf8).write(to:out.appendingPathComponent("drop-complete"),options:.atomic)
  try marker("cancel-ready",8);try guardApp();try bind("cancel");try dump("cancel")
  let saved=try sha(store.path);report["savedStoreSha256"]=saved;let original=try linked("cancel","Fixture One","reopened-saved")
  try click("cancel",original.0);let c=try controls("cancel"),selected=attr(c.1,kAXSelectedRowsAttribute) as? [AXUIElement] ?? []
  try require(selected.count==1,"Physical target-row selection missing");let row=try fieldPoint(c.0,"Fixture Two")
  observations.append(["stage":"double-click-target-selection","selectedRows":selected.count,"targetPoint":[original.0.x,original.0.y],"sourcePoint":[row.x,row.y],"source":"Fixture Two","sourceID":(prepared["elementIDs"] as! [String:String])["Fixture Two"]!])
  try mouse(.mouseMoved,row,"cancel");try mouse(.leftMouseDown,row,"cancel",1);try mouse(.leftMouseUp,row,"cancel",1);try acknowledge()
  try mouse(.leftMouseDown,row,"cancel",2);try mouse(.leftMouseUp,row,"cancel",2);try acknowledge();pump(0.1)
  _ = try linked("cancel","Fixture Two","unsaved-double-click")
  try require(try sha(store.path)==saved,"Double-click committed before Cancel");try Data("complete".utf8).write(to:out.appendingPathComponent("cancel-complete"),options:.atomic)
  inputEnd=overallInputEnd;releaseEnd=cleanupEnd
  var returnedAt:Double?
  while ownedProcess() && now()<terminalEnd{
   let stable=pointerStable() && physical() && !held && !pendingUp
   if !stable{ownershipLost=true;throw Failure.failed("Input changed while waiting for normal host exit")}
   if !foreground(){
    let moment=now(),elapsed=moment-(returnedAt ?? moment),resultExists=FileManager.default.fileExists(atPath:out.appendingPathComponent("ui/result.json").path)
    let exactPrevious=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int)
    let allowed=terminalReturnAllowed(resultExists:resultExists,exactPrevious:exactPrevious,clearAndStable:stable,elapsed:elapsed)
    report["normalExitTransition"]=["resultExists":resultExists,"exactPrevious":exactPrevious,"clearAndStable":stable,"elapsedSeconds":elapsed,"allowed":allowed,"inputPostedDuringTransition":false]
    if !allowed{ownershipLost=true;throw Failure.failed("Unexpected foreground or normal exit grace expired")}
    if returnedAt==nil{returnedAt=moment}
   }
   pump()
  }
  try require(!ownedProcess(),"Owned harness did not terminate before deadline")
  if let returnedAt{let elapsed=now()-returnedAt;report["normalExitTransitionFinalSeconds"]=elapsed;try require(elapsed<=0.5,"Normal host exit exceeded transition bound")}
  try require(try sha(out.appendingPathComponent("profile/SystemData/ModuleStore.xml").path)==report["savedStoreSha256"] as? String,"Cancel/reopen changed saved bytes");report["cancelStoreUnchanged"]=true
  report["passed"]=true
 }catch{report["error"]=String(describing:error)}
}
exit(report["passed"] as? Bool==true ? 0:1)
