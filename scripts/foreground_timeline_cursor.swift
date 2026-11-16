// Three actual timeline hover cursors. No button/key events or model mutations.
// Exact-window/desktop guards derived from frozen Prop03; raster decoding from family helper.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit
import ImageIO
import zlib

enum Failure:Error{case failed(String)}
func require(_ ok:Bool,_ why:String)throws{if !ok{throw Failure.failed(why)}}
func sha(_ p:String)throws->String{SHA256.hash(data:try Data(contentsOf:URL(fileURLWithPath:p))).map{String(format:"%02x",$0)}.joined()}
func physicalAllowed(keys:Set<Int>,buttons:Set<Int>,flags:UInt64)->Bool{keys.isEmpty && buttons.isEmpty && flags==0}
func markerBudget(_ age:Double)->Double? {guard age.isFinite && age>=0 && age<4 else{return nil};return 12-age}
func terminalReturnAllowed(resultExists:Bool,exactPrevious:Bool,clearAndStable:Bool,elapsed:Double)->Bool {resultExists && exactPrevious && clearAndStable && elapsed>=0 && elapsed<=0.5}
func terminalTransition(resultExists:Bool,exactPrevious:Bool,clearAndStable:Bool,elapsed:Double,lost:inout Bool)->Bool {let allowed=terminalReturnAllowed(resultExists:resultExists,exactPrevious:exactPrevious,clearAndStable:clearAndStable,elapsed:elapsed);if !allowed{lost=true};return allowed}
func sha256(_ data: Data) -> String { SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined() }
func normalizedRaster(_ path: URL, evidence: inout [String: Any]) throws -> (Int, Int, [UInt8]) {
    let data = try Data(contentsOf: path)
    evidence["sourceImageSha256"] = sha256(data)
    try require(data.count <= 64 * 1024 * 1024 && data.count >= 33 && Array(data.prefix(8)) == [137,80,78,71,13,10,26,10], "Calibration must be a bounded PNG")
    func be32(_ at: Int) -> Int { data[at..<at+4].reduce(0) { ($0 << 8) | Int($1) } }
    var offset = 8, icc: Data?, srgb = false, sawEnd = false, profileName: String?
    while offset + 12 <= data.count {
        let count = be32(offset)
        try require(count <= data.count-offset-12, "Truncated PNG chunk")
        let type = String(bytes: data[offset+4..<offset+8], encoding: .ascii) ?? ""
        let body = data.subdata(in: offset+8..<offset+8+count)
        let checksum = data.subdata(in: offset+4..<offset+8+count).withUnsafeBytes { crc32(0, $0.bindMemory(to: Bytef.self).baseAddress, uInt(count+4)) }
        try require(Int(checksum) == be32(offset+8+count), "Invalid PNG chunk checksum")
        if offset == 8 {
            try require(type == "IHDR" && count == 13 && body[8] == 8 && body[9] == 6, "Expected PNG RGBA8 samples")
        }
        if type == "sRGB" {
            try require(!srgb && icc == nil && count == 1 && body[0] <= 3, "Invalid/conflicting PNG sRGB declaration")
            srgb = true
        }
        if type == "iCCP" {
            try require(icc == nil && !srgb, "Duplicate/conflicting PNG ICC profile")
            guard let nul = body.firstIndex(of: 0), nul > 0, nul <= 79, nul+2 < body.count, body[nul+1] == 0 else { throw Failure.failed("Invalid PNG ICC encoding") }
            profileName = String(data: body.prefix(nul), encoding: .isoLatin1)
            let compressed = body.subdata(in: nul+2..<body.count)
            var decoded = [UInt8](repeating: 0, count: 1024*1024), length = uLongf(1024*1024)
            let status = decoded.withUnsafeMutableBytes { dest in compressed.withUnsafeBytes { src in
                uncompress(dest.bindMemory(to: Bytef.self).baseAddress, &length, src.bindMemory(to: Bytef.self).baseAddress, uLong(compressed.count))
            } }
            try require(status == Z_OK && length >= 128, "Invalid or oversized compressed ICC profile")
            icc = Data(decoded.prefix(Int(length)))
            let p = icc!
            let declared = p.prefix(4).reduce(0) { ($0 << 8) | Int($1) }
            try require(declared == p.count && String(data:p[16..<20],encoding:.ascii) == "RGB " && String(data:p[36..<40],encoding:.ascii) == "acsp", "Malformed/non-RGB ICC profile")
        }
        offset += count+12
        if type == "IEND" { try require(count == 0 && offset == data.count, "Invalid PNG ending"); sawEnd = true; break }
    }
    try require(sawEnd && (icc != nil || srgb), "Missing explicit PNG color profile")
    guard let destination = CGColorSpace(name: CGColorSpace.sRGB),
          let input = icc.flatMap({ CGColorSpace(iccData: $0 as CFData) }) ?? (srgb ? destination : nil), input.model == .rgb,
          let source = CGImageSourceCreateWithData(data as CFData, nil), CGImageSourceGetCount(source) == 1,
          let decoded = CGImageSourceCreateImageAtIndex(source, 0, nil),
          let image = decoded.copy(colorSpace: input) else { throw Failure.failed("Cannot decode declared RGB profile/image") }
    let decodedSpaceName: String
    if let cs=decoded.colorSpace, let name=cs.name { decodedSpaceName=name as String } else { decodedSpaceName="unnamed" }
    let profileByteCount: Int = icc?.count ?? 0
    let profileSHA: Any = icc.map(sha256) as Any? ?? NSNull()
    let embeddedName: String = profileName ?? "sRGB"
    evidence["sourceColorProfile"] = ["declaration": icc == nil ? "PNG sRGB" : "PNG iCCP", "embeddedName":embeddedName, "iccSha256":profileSHA, "iccBytes":profileByteCount, "decodedSpaceName":decodedSpaceName, "normalization":"ImageIO decode, declared source profile, relative-colorimetric CoreGraphics to explicit sRGB; RGBA8 premultiplied-last big-endian; no resampling"]
    let scale = Double(image.width)/1100
    evidence["observedRasterScale"] = scale
    try require([1.0,2.0].contains(scale) && Double(image.height) == 780*scale, "Unexpected calibration raster size")
    try require(image.bitsPerComponent == 8 && image.bitsPerPixel == 32, "Unexpected screenshot sample format")
    var bytes = [UInt8](repeating:0, count:image.width*image.height*4)
    try bytes.withUnsafeMutableBytes { storage in
        guard let context = CGContext(data:storage.baseAddress, width:image.width, height:image.height, bitsPerComponent:8, bytesPerRow:image.width*4, space:destination, bitmapInfo:CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue) else { throw Failure.failed("Cannot create fixed sRGB context") }
        context.setRenderingIntent(.relativeColorimetric); context.interpolationQuality = .none
        context.setBlendMode(.copy)
        context.draw(image, in:CGRect(x:0,y:0,width:image.width,height:image.height))
    }
    return (image.width,image.height,bytes)
}

func checkedRaster(_ path:URL,_ cal:[String:Any],_ proof:inout [String:Any])throws {
 let(width,_,bytes)=try normalizedRaster(path,evidence:&proof),scale=Double(width)/1100
 try require(cal["scale"] as? Int==Int(scale),"No observed reference for this backing scale")
 let samples=cal["samples"] as! [[Int]];try require(samples.count==200*32,"Complete reviewed effect rectangle required")
 var maximum=0
 for s in samples {try require(s.count==6 && (308..<508).contains(s[0]) && (172..<204).contains(s[1]),"Unexpected raster region");let at=(Int(Double(s[1])*scale)*width+Int(Double(s[0])*scale))*4;try require(Int(bytes[at+3])==s[5],"Effect alpha changed");for k in 0..<3{maximum=max(maximum,abs(Int(bytes[at+k])-s[k+2]))}}
 proof["samples"]=samples.count;proof["maximumRGBDifference"]=maximum;try require(maximum<=3,"Actual effect geometry/raster differs from retained layout")
}
let args=CommandLine.arguments
if args.count==2 && args[1]=="--pure-checks" {
 var lost=false;let valid=terminalTransition(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.2,lost:&lost) && !lost;let rejected = !terminalTransition(resultExists:true,exactPrevious:false,clearAndStable:true,elapsed:0.2,lost:&lost) && lost;_ = terminalTransition(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.2,lost:&lost)
 let checks=[valid,rejected,lost,physicalAllowed(keys:[],buttons:[],flags:0),!physicalAllowed(keys:[55],buttons:[],flags:0),!physicalAllowed(keys:[],buttons:[0],flags:0),!physicalAllowed(keys:[],buttons:[],flags:1),markerBudget(0)==12,markerBudget(3.9) != nil,markerBudget(-0.1)==nil,markerBudget(4)==nil,markerBudget(.nan)==nil,terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:false,exactPrevious:true,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:false,clearAndStable:true,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:false,elapsed:0.2),!terminalReturnAllowed(resultExists:true,exactPrevious:true,clearAndStable:true,elapsed:0.51)]
 precondition(checks.allSatisfy{$0});print("{\"passed\":true,\"checks\":17,\"nativeOperations\":0}");exit(0)
}
if args.count==4 && args[1]=="--make-calibration" {
 var proof=[String:Any]();let(w,_,pixels)=try normalizedRaster(URL(fileURLWithPath:args[2]),evidence:&proof),scale=w/1100;var samples=[[Int]]()
 for y in 172..<204{for x in 308..<508{let at=(y*scale*w+x*scale)*4;samples.append([x,y,Int(pixels[at]),Int(pixels[at+1]),Int(pixels[at+2]),Int(pixels[at+3])])}}
 let cal:[String:Any]=["scale":scale,"samples":samples,"reference":args[2],"referenceSha256":try sha(args[2]),"windowSize":[1100,780],"points":[[311,188],[505,188],[408,188]],"captureNames":["01-front-resize","02-back-resize","03-body-arrow"],"expectedCursors":["horizontal-resize","horizontal-resize","arrow"],"normalization":proof]
 try JSONSerialization.data(withJSONObject:cal,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[3]));exit(0)
}
if args.count==5 && args[1]=="--check-calibration" {
 let cal=try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:args[3]))) as! [String:Any];var proof:[String:Any]=["passed":false,"nativeOperations":0]
 do{try checkedRaster(URL(fileURLWithPath:args[2]),cal,&proof);proof["passed"]=true}catch{proof["error"]=String(describing:error)}
 try JSONSerialization.data(withJSONObject:proof,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[4]));exit(proof["passed"] as? Bool==true ? 0:1)
}
func identity(_ a:NSRunningApplication?)->[String:Any]{["pid":Int(a?.processIdentifier ?? -1),"bundle":a?.bundleIdentifier ?? "","executable":a?.executableURL?.standardizedFileURL.path ?? ""]}
if args.count==3 && args[1]=="--desktop"{let m:[String:Any]=["previous":identity(NSWorkspace.shared.frontmostApplication),"accessibility":AXIsProcessTrusted(),"eventPosting":CGPreflightPostEventAccess(),"screenCapture":CGPreflightScreenCaptureAccess(),"permissionRequested":false,"inputPosted":false];try JSONSerialization.data(withJSONObject:m,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[2]));exit(0)}
guard args.count==4,let pid=Int32(args[1]) else{fatalError("pid,out,executable")}
let out=URL(fileURLWithPath:args[2]),executable=args[3]
let prepared=try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("prepared.json"))) as! [String:Any]
let calibration=try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("calibration.json"))) as! [String:Any]
let bundle=prepared["bundleIdentifier"] as! String,previous=(try JSONSerialization.jsonObject(with:Data(contentsOf:out.appendingPathComponent("desktop-before.json"))) as! [String:Any])["previous"] as! [String:Any]
let began=ProcessInfo.processInfo.systemUptime,terminalEnd=began+26,cleanupEnd=began+28
var inputEnd=began+20,releaseEnd=cleanupEnd
let oldPointer=CGEvent(source:nil)?.location,app=AXUIElementCreateApplication(pid),system=AXUIElementCreateSystemWide()
var windows=[String:AXUIElement](),frames=[String:CGRect](),ids=[String:Int]()
var posted=false,activated=false,ownershipLost=false,lastPoint:CGPoint?
var events=[[String:Any]](),captures=[[String:Any]](),observations=[[String:Any]](),report:[String:Any]=["passed":false,"scope":"Timeline F65 hover cursors only:3mouseMoved/3small captures,0buttons/0keys,unchanged effect/profile. Actual appearance requires visual inspection."]
func now()->Double{ProcessInfo.processInfo.systemUptime}
func pump(_ t:Double=0.025){RunLoop.current.run(until:Date().addingTimeInterval(t))}
func target()->NSRunningApplication?{NSRunningApplication(processIdentifier:pid)}
func ownedProcess()->Bool{let t=target();return t?.isTerminated==false && t?.executableURL?.standardizedFileURL.path==executable && t?.bundleIdentifier==bundle}
func foreground()->Bool{ownedProcess() && target()?.isActive==true && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid}
func previousMatches()->Bool{let t=NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int));return t?.isTerminated==false && t?.bundleIdentifier==previous["bundle"] as? String && t?.executableURL?.standardizedFileURL.path==previous["executable"] as? String}
func priorOrTarget()->Bool{let t=NSWorkspace.shared.frontmostApplication;return t?.processIdentifier==pid ? ownedProcess():previousMatches() && t?.processIdentifier==pid_t(previous["pid"] as! Int)}
func pointerStable()->Bool{guard let p=CGEvent(source:nil)?.location,let e=lastPoint ?? oldPointer else{return false};return hypot(p.x-e.x,p.y-e.y)<1}
func physical()->Bool{physicalAllowed(keys:Set((0..<128).filter{CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0))}),buttons:Set([CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}.map{Int($0.rawValue)}),flags:CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand,.maskSecondaryFn]).rawValue)}
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
func guardApp()throws{try require(now()<inputEnd && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Input cutoff/cooperative abort");let ok = !ownershipLost && foreground() && pointerStable() && physical();if !ok{ownershipLost=true};try require(ok && now()<inputEnd,"Private process/pointer/input ownership lost")}
func hit(_ point:CGPoint,_ label:String,cleanup:Bool=false)->Bool{
 let end=cleanup ? releaseEnd:inputEnd;guard now()<end,let w=windows[label],let f=frames[label],f.contains(point),let id=ids[label],(try? frame(w,cleanup:cleanup))==f else{return false}
 guard native().filter({($0[kCGWindowOwnerPID as String] as? Int)==Int(pid) && ($0[kCGWindowNumber as String] as? Int)==id && nativeFrame($0)==f && ($0[kCGWindowAlpha as String] as? Double ?? 0)>0}).count==1 else{return false}
 AXUIElementSetMessagingTimeout(system,0.1);var found:AXUIElement?;guard AXUIElementCopyElementAtPosition(system,Float(point.x),Float(point.y),&found) == .success,let found,owned(found),now()<end else{return false}
 if let owner=ax(attr(found,kAXWindowAttribute,cleanup:cleanup)),CFEqual(owner,w){return now()<end}
 var node:AXUIElement?=found
 for _ in 0..<14{guard let n=node,now()<end,owned(n) else{return false};if CFEqual(n,w){return true};node=ax(attr(n,kAXParentAttribute,cleanup:cleanup))};return false
}

func move(_ p:CGPoint)throws {
 try guardApp();try require(keyMain(["editor"]) && hit(p,"editor"),"Exact timeline hover target lost")
 guard let e=CGEvent(mouseEventSource:nil,mouseType:.mouseMoved,mouseCursorPosition:p,mouseButton:.left) else{throw Failure.failed("Mouse allocation")};e.flags=[]
 try guardApp();try require(hit(p,"editor"),"Immediate native target changed");try guardApp();e.post(tap:.cghidEventTap);posted=true;lastPoint=p
 events.append(["kind":CGEventType.mouseMoved.rawValue,"point":[p.x,p.y],"window":ids["editor"]!,"elapsedSeconds":now()-began]);pump(0.2);try guardApp()
}
func capture(_ name:String,_ p:CGPoint)throws {
 try guardApp();try require(CGPreflightScreenCaptureAccess(),"Screen recording unavailable; no request")
 let f=frames["editor"]!,crop=CGRect(x:p.x-36,y:p.y-24,width:80,height:56).integral
 try require(f.contains(crop),"Crop escaped exact private window")
 for p in [CGPoint(x:crop.minX+1,y:crop.minY+1),CGPoint(x:crop.maxX-1,y:crop.minY+1),CGPoint(x:crop.minX+1,y:crop.maxY-1),CGPoint(x:crop.maxX-1,y:crop.maxY-1),p]{try require(hit(p,"editor"),"Crop no longer owned")}
 let destination=out.appendingPathComponent(name+".png");try require(!FileManager.default.fileExists(atPath:destination.path),"Capture already exists")
 let child=Process();child.executableURL=URL(fileURLWithPath:"/usr/sbin/screencapture");child.arguments=["-x","-C","-R\(Int(crop.minX)),\(Int(crop.minY)),80,56",destination.path];child.standardOutput=FileHandle.nullDevice;child.standardError=FileHandle.nullDevice
 try guardApp();try require(inputEnd-now()>2,"Insufficient complete capture budget");try child.run();report["lastCapturePID"]=child.processIdentifier
 let end=min(inputEnd,now()+1.5);while child.isRunning && now()<end{pump(0.015)}
 if child.isRunning{child.terminate();let soft=min(cleanupEnd,now()+0.15);while child.isRunning && now()<soft{pump(0.01)};if child.isRunning{kill(child.processIdentifier,SIGKILL);let hard=min(cleanupEnd,now()+0.15);while child.isRunning && now()<hard{pump(0.01)}}}
 report["lastCaptureExited"] = !child.isRunning;try require(!child.isRunning && child.terminationStatus==0,"Bounded capture failed");try guardApp();try require(hit(p,"editor"),"Capture ownership changed");try guardApp()
 guard let source=CGImageSourceCreateWithURL(destination as CFURL,nil),let image=CGImageSourceCreateImageAtIndex(source,0,nil) else{throw Failure.failed("Capture unreadable")}
 let scale=Double(image.width)/80;try require([1.0,2.0].contains(scale) && Double(image.height)==56*scale,"Capture backing geometry changed")
 captures.append(["name":name,"path":destination.path,"sha256":try sha(destination.path),"cropCG":[crop.minX,crop.minY,crop.width,crop.height],"pixelSize":[image.width,image.height],"cursorCG":[p.x,p.y],"window":ids["editor"]!,"cursorRequested":true,"arguments":child.arguments!,"elapsedSeconds":now()-began])
}
func publish(){if let d=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]){try? d.write(to:out.appendingPathComponent("foreground.json"),options:.atomic)}}
do{
 defer{
  func restoreSafe()->Bool{!ownershipLost && physical() && pointerStable() && priorOrTarget() && previousMatches() && now()<cleanupEnd}
  if activated && restoreSafe(){if posted,let oldPointer,restoreSafe(){report["restoredPointer"]=CGWarpMouseCursorPosition(oldPointer) == .success;lastPoint=oldPointer};if restoreSafe(){_ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[])};while now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != pid_t(previous["pid"] as! Int){pump()}}
  report["noInputHeld"]=physical();report["mouseDowns"]=0;report["mouseUps"]=0;report["keyEvents"]=0;report["events"]=events;report["captures"]=captures;report["observations"]=observations;report["windowIDs"]=ids;report["frames"]=frames.mapValues{[$0.minX,$0.minY,$0.width,$0.height]};report["ownershipLost"]=ownershipLost;report["restoredPrevious"]=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);report["restoredPointer"] = !posted || (lastPoint==oldPointer && pointerStable());report["elapsedSeconds"]=now()-began
  report["cleanupCompleted"]=physical() && !ownershipLost && now()<cleanupEnd
  if report["cleanupCompleted"] as? Bool != true || report["restoredPrevious"] as? Bool != true || report["restoredPointer"] as? Bool != true{report["passed"]=false};publish()
 }
 do{
  try require(try sha(executable)==prepared["appSha256"] as? String && bundle.hasPrefix("com.vixenlights.verification."),"Private executable pin")
  try require(oldPointer != nil && previousMatches() && physical() && pointerStable() && AXIsProcessTrusted() && CGPreflightPostEventAccess() && CGPreflightScreenCaptureAccess(),"Initial ownership/permissions; no permission request")
  let marker=out.appendingPathComponent("ui/ready.png")
  while !FileManager.default.fileExists(atPath:marker.path){let clear=priorOrTarget() && physical() && pointerStable();if !clear{ownershipLost=true};try require(now()<inputEnd && clear && !FileManager.default.fileExists(atPath:out.appendingPathComponent("abort-request.json").path),"Startup cutoff/ownership");pump()}
  let stamp=try FileManager.default.attributesOfItem(atPath:marker.path)[.modificationDate] as! Date,wall=Date(),mono=now(),age=wall.timeIntervalSince(stamp)
  guard let budget=markerBudget(age) else{throw Failure.failed("Unsafe marker age")};try require(abs(Date().timeIntervalSince(wall)-(now()-mono))<0.05,"Clock changed")
  inputEnd=min(inputEnd,mono+budget);releaseEnd=min(cleanupEnd,inputEnd+4);report["inputCutoffSeconds"]=inputEnd-began;report["snapshotAgeSeconds"]=age
  try require(inputEnd-now()>8 && ownedProcess() && priorOrTarget() && physical() && pointerStable(),"Insufficient interaction window or identity")
  try require(try sha(calibration["reference"] as! String)==calibration["referenceSha256"] as? String,"Retained reference changed");var proof=[String:Any]();try checkedRaster(marker,calibration,&proof);report["rasterEvidence"]=proof
  let before=try Data(contentsOf:out.appendingPathComponent("ui/timeline-state.json"));try before.write(to:out.appendingPathComponent("before-timeline-state.json"));report["beforeTimelineSha256"]=sha256(before)
  let all=attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? [],found=all.filter{attr($0,kAXTitleAttribute) as? String=="Editor Color – profile"};try require(found.count==1,"Unique exact editor window unavailable");windows["editor"]=found[0]
  guard let screen=NSScreen.screens.first else{throw Failure.failed("No display")};let area=screen.visibleFrame,screenTop=screen.frame.maxY
  try require(area.width>=1140 && area.height>=820,"Display cannot contain calibrated editor")
  try require(ownedProcess() && priorOrTarget() && physical() && pointerStable() && now()<inputEnd,"Desktop changed before activation")
  try require(target()?.activate(options:[])==true,"Activation refused");activated=true;while !foreground() && now()<inputEnd{pump()};try guardApp()
  let expected=CGRect(x:area.minX+20,y:screenTop-area.maxY+20,width:1100,height:780);var p=expected.origin
  try guardApp();try require(AXUIElementSetAttributeValue(found[0],kAXPositionAttribute as CFString,AXValueCreate(.cgPoint,&p)!) == .success,"Private window position refused")
  if !keyMain(["editor"]){try guardApp();report["raiseStatus"]=AXUIElementPerformAction(found[0],kAXRaiseAction as CFString).rawValue}
  try bind("editor");let settle=min(inputEnd,now()+1);while !keyMain(["editor"]) && now()<settle{try guardApp();pump()}
  try require(frames["editor"]==expected && keyMain(["editor"]),"Exact editor frame/key/main unavailable")
  let names=calibration["captureNames"] as! [String],points=calibration["points"] as! [[Double]]
  try require(names.count==3 && points.count==3,"Expected exactly3 hover points")
  for i in 0..<3{let point=CGPoint(x:expected.minX+points[i][0],y:expected.minY+points[i][1]);try move(point);try capture(names[i],point)}
  try require(events.count==3 && captures.count==3,"Expected only3 moves/captures");report["inputCompleted"]=true
  var returnedAt:Double?
  while ownedProcess() && now()<terminalEnd{
   let stable=pointerStable() && physical();if !stable{ownershipLost=true;throw Failure.failed("Input changed while waiting for normal host exit")}
   if !foreground(){let moment=now(),elapsed=moment-(returnedAt ?? moment),exists=FileManager.default.fileExists(atPath:out.appendingPathComponent("ui/result.json").path),prior=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int);try require(terminalTransition(resultExists:exists,exactPrevious:prior,clearAndStable:stable,elapsed:elapsed,lost:&ownershipLost),"Unexpected foreground during normal exit");if returnedAt==nil{returnedAt=moment}}
   pump()
  }
  try require(!ownedProcess(),"Private harness did not exit before deadline");if let returnedAt{try require(now()-returnedAt<=0.5,"Normal exit transition exceeded bound")};report["passed"]=true
 }catch{report["error"]=String(describing:error)}
}
exit(report["passed"] as? Bool==true ? 0:1)
