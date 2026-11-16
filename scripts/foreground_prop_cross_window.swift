// One real foreign-window Prop Order drop. Setup/save remain in existing harness.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit

enum Failure:Error{case failed(String)}
func require(_ ok:Bool,_ why:String)throws{if !ok{throw Failure.failed(why)}}
func sha(_ p:String)throws->String{SHA256.hash(data:try Data(contentsOf:URL(fileURLWithPath:p))).map{String(format:"%02x",$0)}.joined()}
func physicalAllowed(keys:Set<Int>,buttons:Set<Int>,flags:UInt64,held:Bool,pending:Bool)->Bool{keys.isEmpty && flags==0 && buttons.allSatisfy{ $0==0 && (held || pending) }}
func terminalReturnAllowed(resultExists:Bool,exactPrevious:Bool,clearAndStable:Bool,elapsed:Double)->Bool {resultExists && exactPrevious && clearAndStable && elapsed>=0 && elapsed<=0.5}
let args=CommandLine.arguments
if args.count==2 && args[1]=="--pure-checks"{
 let checks=[physicalAllowed(keys:[],buttons:[],flags:0,held:false,pending:false),physicalAllowed(keys:[],buttons:[0],flags:0,held:true,pending:false),physicalAllowed(keys:[],buttons:[0],flags:0,held:false,pending:true),!physicalAllowed(keys:[55],buttons:[],flags:0,held:true,pending:false),!physicalAllowed(keys:[],buttons:[1],flags:0,held:true,pending:true),!physicalAllowed(keys:[],buttons:[],flags:1,held:true,pending:true),!physicalAllowed(keys:[],buttons:[0],flags:0,held:false,pending:false),terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:false,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:false,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:false,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.501)]
 precondition(checks.allSatisfy{$0});print("{\"passed\":true,\"checks\":12,\"nativeOperations\":0}");exit(0)
}
func identity(_ a:NSRunningApplication?)->[String:Any]{["pid":Int(a?.processIdentifier ?? -1),"bundle":a?.bundleIdentifier ?? "","executable":a?.executableURL?.standardizedFileURL.path ?? ""]}
if args.count==3 && args[1]=="--desktop"{let m:[String:Any]=["previous":identity(NSWorkspace.shared.frontmostApplication),"accessibility":AXIsProcessTrusted(),"eventPosting":CGPreflightPostEventAccess(),"permissionRequested":false,"inputPosted":false];try JSONSerialization.data(withJSONObject:m,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[2]));exit(0)}
guard args.count==4,let pid=Int32(args[1]) else{fatalError("pid,out,executable")}
let out=URL(fileURLWithPath:args[2]),executable=args[3]
let prepared=try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("prepared.json"))) as! [String:Any]
let bundle=prepared["bundleIdentifier"] as! String,previous=(try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("desktop-before.json"))) as! [String:Any])["previous"] as! [String:Any]
let began=ProcessInfo.processInfo.systemUptime,terminalEnd=began+37,cleanupEnd=began+40
var inputEnd=began+28,releaseEnd=cleanupEnd
let oldPointer=CGEvent(source:nil)?.location,app=AXUIElementCreateApplication(pid),system=AXUIElementCreateSystemWide()
var windows=[String:AXUIElement](),frames=[String:CGRect](),ids=[String:Int](),points=[String:CGPoint]()
var posted=false,activated=false,ownershipLost=false,held=false,pendingUp=false,lastPoint:CGPoint?,downs=0,ups=0
var events=[[String:Any]](),observations=[[String:Any]](),report:[String:Any]=["passed":false,"scope":"One real foreign-window Prop Order drop; equal UUIDs across distinct documents. No drop/selection callbacks, keys or production edits."]
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
func bind(_ label:String)throws{
 let end=min(inputEnd,now()+1)
 repeat{
  try guardApp()
  let f=try frame(windows[label]!),ownedRows=native().filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}
  let rows=ownedRows.filter{($0[kCGWindowAlpha as String] as? Double ?? 0)>0 && nativeFrame($0)==f}
  observations.append(["stage":"native-binding","label":label,"elapsedSeconds":now()-began,"axFrame":[f.minX,f.minY,f.width,f.height],"matchingIDs":rows.compactMap{$0[kCGWindowNumber as String] as? Int},"privateNativeWindows":ownedRows.map{r in ["id":r[kCGWindowNumber as String] ?? NSNull(),"title":r[kCGWindowName as String] ?? NSNull(),"bounds":r[kCGWindowBounds as String] ?? NSNull(),"alpha":r[kCGWindowAlpha as String] ?? NSNull(),"layer":r[kCGWindowLayer as String] ?? NSNull()]}])
  try guardApp();try require(now()<end,"Exact native binding settle deadline")
  if rows.count==1{let id=rows[0][kCGWindowNumber as String] as! Int;try require(ids[label]==nil || ids[label]==id,"Native identity changed");ids[label]=id;frames[label]=f;return}
  try require(rows.count<2,"Ambiguous exact native window match")
  pump(0.025)
 }while now()<end
 throw Failure.failed("AX window has no unique exact native match after bounded settle")
}
func keyMain(_ labels:[String],cleanup:Bool=false)->Bool{guard let k=ax(attr(app,kAXFocusedWindowAttribute,cleanup:cleanup)),let m=ax(attr(app,kAXMainWindowAttribute,cleanup:cleanup)) else{return false};return [k,m].allSatisfy{n in labels.contains{windows[$0].map{CFEqual($0,n)}==true}}}
func guardApp()throws{try require(now()<inputEnd && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Input cutoff/cooperative abort");try require(!pendingUp,"MouseUP still awaiting physical acknowledgment");let ok = !ownershipLost && foreground() && pointerStable() && physical();if !ok{ownershipLost=true};try require(ok && now()<inputEnd,"Private process/pointer/input ownership lost")}
func hit(_ point:CGPoint,_ label:String,cleanup:Bool=false)->Bool{
 let end=cleanup ? releaseEnd:inputEnd;guard now()<end,let w=windows[label],let f=frames[label],f.contains(point),let id=ids[label],(try? frame(w,cleanup:cleanup))==f else{return false}
 guard native().filter({($0[kCGWindowOwnerPID as String] as? Int)==Int(pid) && ($0[kCGWindowNumber as String] as? Int)==id && nativeFrame($0)==f && ($0[kCGWindowAlpha as String] as? Double ?? 0)>0}).count==1 else{return false}
 AXUIElementSetMessagingTimeout(system,0.1);var found:AXUIElement?;guard AXUIElementCopyElementAtPosition(system,Float(point.x),Float(point.y),&found) == .success,let found,owned(found),now()<end else{return false}
 if let owner=ax(attr(found,kAXWindowAttribute,cleanup:cleanup)),CFEqual(owner,w){return now()<end}
 var node:AXUIElement?=found
 for _ in 0..<14{guard let n=node,now()<end,owned(n) else{return false};if CFEqual(n,w){return true};node=ax(attr(n,kAXParentAttribute,cleanup:cleanup))};return false
}
func mouse(_ kind:CGEventType,_ p:CGPoint,_ label:String)throws{
 try guardApp();try require(keyMain(held ? ["source","destination"]:[label]) && hit(p,label),"Exact input window/point changed")
 for l in ["source","destination"]{try require((try? frame(windows[l]!))==frames[l],"Window moved during drag")}
 guard let e=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:p,mouseButton:.left) else{throw Failure.failed("Mouse allocation")};e.flags=[];e.setIntegerValueField(.mouseEventClickState,value:1)
 try guardApp();try require(hit(p,label),"Immediate actual hit changed");try guardApp()
 if kind == .leftMouseDown{try require(!held,"Duplicate down");held=true;downs+=1};if kind == .leftMouseUp{try require(held,"Unpaired up")}
 e.post(tap:.cghidEventTap);posted=true;lastPoint=p
 if kind == .leftMouseUp{held=false;pendingUp=true;ups+=1}
 events.append(["kind":kind.rawValue,"point":[p.x,p.y],"label":label,"window":ids[label]!]);pump(0.035)
}
func releaseSafe()->Bool{let ok = !ownershipLost && foreground() && pointerStable() && physical();if !ok{ownershipLost=true};return ok && now()<releaseEnd}
func acknowledge()throws{let end=min(releaseEnd,now()+0.6);var absent:Double?;while pendingUp && now()<end{try require(releaseSafe(),"Release acknowledgment ownership changed");if !CGEventSource.buttonState(.combinedSessionState,button:.left){if let t=absent,now()-t>=0.025{pendingUp=false}else if absent==nil{absent=now()}}else{absent=nil};if pendingUp{pump(0.015)}};try require(!pendingUp && now()<end,"MouseUP not acknowledged")}
func raise(_ label:String)throws{try guardApp();let w=windows[label]!;_ = AXUIElementPerformAction(w,kAXRaiseAction as CFString);try guardApp();_ = AXUIElementSetAttributeValue(w,kAXMainAttribute as CFString,kCFBooleanTrue);let end=min(inputEnd,now()+1);while now()<end{try guardApp();if keyMain([label]),let p=points[label],hit(p,label){return};pump()};throw Failure.failed("Exact source key/main/actual hit did not settle")}
func rowPoints(_ label:String)throws{
 let w=windows[label]!;var nodes=[AXUIElement]();func visit(_ n:AXUIElement,_ depth:Int){guard depth<14,nodes.count<450,now()<inputEnd else{return};nodes.append(n);if attr(n,kAXRoleAttribute) as? String==kAXTableRole{return};for c in attr(n,kAXChildrenAttribute) as? [AXUIElement] ?? []{visit(c,depth+1)}};visit(w,0)
 let tables=nodes.filter{attr($0,kAXRoleAttribute) as? String==kAXTableRole && labels($0).contains("Light order")};try require(tables.count==1,"Expected unique Light order table")
 let table=tables[0],tableFrame=try frame(table),rows=attr(table,kAXRowsAttribute) as? [AXUIElement] ?? []
 var observed=[[String:Any]](),matched=[String:CGPoint]()
 for row in rows{let rf=try frame(row);var cells=[AXUIElement]();func cell(_ n:AXUIElement,_ d:Int){guard d<5,cells.count<40,now()<inputEnd else{return};cells.append(n);for c in attr(n,kAXChildrenAttribute) as? [AXUIElement] ?? []{cell(c,d+1)}};cell(row,0)
  let names=["Pixel 1","Pixel 2","Pixel 3"].filter{name in cells.contains{labels($0).contains(name)}};try require(names.count<=1,"Ambiguous native row name")
  if let name=names.first{let named=cells.filter{labels($0).contains(name)};let all=try named.map{($0,try frame($0))}.filter{$0.1.width>15 && $0.1.height>5 && $0.1.height<50};guard let chosen=all.min(by:{$0.1.width*$0.1.height<$1.1.width*$1.1.height}) else{throw Failure.failed("No exact named row cell")};try require(matched[name]==nil && tableFrame.contains(chosen.1),"Duplicate/outside row");matched[name]=CGPoint(x:chosen.1.midX,y:name=="Pixel 3" && label=="destination" ? rf.minY+1:chosen.1.midY);observed.append(["name":name,"rowFrame":[rf.minX,rf.minY,rf.width,rf.height],"cellFrame":[chosen.1.minX,chosen.1.minY,chosen.1.width,chosen.1.height]])}
 }
 try require(observed.compactMap{$0["name"] as? String}==["Pixel 1","Pixel 2","Pixel 3"],"Exact initial row order changed")
 points[label]=matched[label=="source" ? "Pixel 1":"Pixel 3"]!;observations.append(["stage":"rows-"+label,"rows":observed,"point":[points[label]!.x,points[label]!.y],"nativeID":ids[label]!])
}
func publish(){if let d=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]){try? d.write(to:out.appendingPathComponent("foreground.json"),options:.atomic)}}
do{
 defer{
  if held,releaseSafe(),let p=CGEvent(source:nil)?.location,["source","destination"].contains(where:{hit(p,$0,cleanup:true)}),let e=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:p,mouseButton:.left),releaseSafe(){e.flags=[];e.post(tap:.cghidEventTap);held=false;pendingUp=true;ups+=1}
  if pendingUp{try? acknowledge()}
  func restoreSafe()->Bool{!ownershipLost && !held && !pendingUp && physical() && pointerStable() && priorOrTarget() && previousMatches() && now()<cleanupEnd}
  if activated && restoreSafe(){if posted,let oldPointer,restoreSafe(){report["restoredPointer"]=CGWarpMouseCursorPosition(oldPointer) == .success;lastPoint=oldPointer};if restoreSafe(){_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};while now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}
  let clear = !held && !pendingUp && downs==ups && physical()
  report["noInputHeld"]=clear;report["pendingMouseUp"]=pendingUp;report["mouseDowns"]=downs;report["mouseUps"]=ups;report["keyEvents"]=0;report["events"]=events;report["observations"]=observations;report["windowIDs"]=ids;report["ownershipLost"]=ownershipLost;report["restoredPrevious"]=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);report["restoredPointer"] = !posted || (lastPoint==oldPointer && pointerStable());report["elapsedSeconds"]=now()-began
  report["cleanupCompleted"]=clear && !ownershipLost && now()<cleanupEnd
  if report["cleanupCompleted"] as? Bool != true || report["restoredPrevious"] as? Bool != true || report["restoredPointer"] as? Bool != true{report["passed"]=false};publish()
 }
 do{
  try require(try sha(executable)==prepared["appSha256"] as? String && bundle.hasPrefix("com.vixenlights.verification."),"Private executable pin")
  try require(oldPointer != nil && previousMatches() && physical() && pointerStable() && AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Initial ownership/permissions")
  let marker=out.appendingPathComponent("ui/destination-ready.png")
  while !FileManager.default.fileExists(atPath:marker.path){try require(now()<inputEnd && priorOrTarget() && physical() && pointerStable(),"Startup cutoff/ownership");pump()}
  let stamp=try FileManager.default.attributesOfItem(atPath:marker.path)[.modificationDate] as! Date,wall=Date(),mono=now(),age=wall.timeIntervalSince(stamp)
  try require(age>=0 && age<5 && abs(Date().timeIntervalSince(wall)-(now()-mono))<0.05,"Unsafe snapshot age/clock")
  inputEnd=min(inputEnd,mono+12-age);releaseEnd=min(cleanupEnd,inputEnd+4);report["inputCutoffSeconds"]=inputEnd-began;report["releaseCutoffSeconds"]=releaseEnd-began;report["snapshotAgeSeconds"]=age
  try require(inputEnd-now()>9 && ownedProcess() && priorOrTarget() && physical() && pointerStable(),"Insufficient interaction window or identity")
  try require(target()?.activate(options:[])==true,"Activation refused");activated=true;while !foreground() && now()<inputEnd{pump()};try guardApp()
  let all=attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? []
  for label in ["source","destination"]{let title=label=="source" ? "Source Prop Order":"Destination Prop Order";let found=all.filter{attr($0,kAXTitleAttribute) as? String==title};try require(found.count==1,"Unique labelled Prop window unavailable");windows[label]=found[0]}
  guard let screen=NSScreen.screens.first else{throw Failure.failed("No display")};let area=screen.visibleFrame,screenTop=screen.frame.maxY
  let left=area.minX+20,top=screenTop-area.maxY+25,width=min(900,area.width-360),height=min(700,area.height-50)
  try require(width>=740 && height>=560,"Display cannot expose both Order tables")
  for label in ["destination","source"]{try guardApp();let w=windows[label]!;var p=CGPoint(x:left+(label=="source" ? 320:0),y:top),s=CGSize(width:width,height:height);try require(AXUIElementSetAttributeValue(w,kAXSizeAttribute as CFString,AXValueCreate(.cgSize,&s)!) == .success,"Window size refused");try guardApp();try require(AXUIElementSetAttributeValue(w,kAXPositionAttribute as CFString,AXValueCreate(.cgPoint,&p)!) == .success,"Window position refused");pump(0.15);try bind(label);try rowPoints(label)}
  try require(ids["source"] != ids["destination"],"Distinct native windows required");try raise("destination");try raise("source")
  let start=points["source"]!,end=points["destination"]!;try require(!frames["source"]!.contains(end),"Destination target is covered by source")
  let board=NSPasteboard(name:.drag),before=board.changeCount
  try mouse(.mouseMoved,start,"source");pump(0.1);try mouse(.leftMouseDown,start,"source");pump(0.1)
  for i in 1...6{try mouse(.leftMouseDragged,CGPoint(x:start.x+CGFloat(i)*3,y:start.y),"source")}
  let payload=board.pasteboardItems?.compactMap{$0.string(forType:.init("com.vixenlights.prop-order"))} ?? []
  report["dragPayload"]=["changeCountBefore":before,"changeCountDuring":board.changeCount,"elementIDs":payload]
  try require(board.changeCount != before && payload==[prepared["sourceElementID"] as! String],"Actual native source drag payload missing/wrong")
  let elbow=CGPoint(x:start.x+18,y:start.y)
  for i in 1...24{let t=CGFloat(i)/24,p=CGPoint(x:elbow.x+(end.x-elbow.x)*t,y:elbow.y+(end.y-elbow.y)*t),label=frames["source"]!.contains(p) ? "source":"destination";try mouse(.leftMouseDragged,p,label)}
  try mouse(.leftMouseUp,end,"destination");try acknowledge();try require(downs==1 && ups==1,"One balanced drag required")
  try Data("drag-finished".utf8).write(to:out.appendingPathComponent("drag-finished.json"),options:.atomic)
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
  report["passed"]=true
 }catch{report["error"]=String(describing:error)}
}
exit(report["passed"] as? Bool==true ? 0:1)
