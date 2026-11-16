// Test-only WindowServer input for one private Vixen process. No app callbacks.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit
import ImageIO
import zlib

enum CheckFailure: Error { case failed(String) }
func require(_ ok: Bool, _ message: String) throws { if !ok { throw CheckFailure.failed(message) } }
// The PNG snapshot may carry either explicit sRGB or the current display ICC.
// Normalize through that declared profile to one fixed sRGB RGBA8 destination.
// This function also backs --check-calibration, which never initializes desktop APIs.
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
            guard let nul = body.firstIndex(of: 0), nul > 0, nul <= 79, nul+2 < body.count, body[nul+1] == 0 else { throw CheckFailure.failed("Invalid PNG ICC encoding") }
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
          let image = decoded.copy(colorSpace: input) else { throw CheckFailure.failed("Cannot decode declared RGB profile/image") }
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
        guard let context = CGContext(data:storage.baseAddress, width:image.width, height:image.height, bitsPerComponent:8, bytesPerRow:image.width*4, space:destination, bitmapInfo:CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue) else { throw CheckFailure.failed("Cannot create fixed sRGB context") }
        context.setRenderingIntent(.relativeColorimetric); context.interpolationQuality = .none
        context.setBlendMode(.copy)
        context.draw(image, in:CGRect(x:0,y:0,width:image.width,height:image.height))
    }
    return (image.width,image.height,bytes)
}
func checkedRaster(_ path:URL, calibration:[String:Any], evidence:inout [String:Any]) throws {
    let (width,_,bytes)=try normalizedRaster(path,evidence:&evidence)
    let scale=Double(width)/1100
    let selected:[String:Any]
    if let variants=calibration["scaleReferences"] as? [String:[String:Any]] {
        try require(Set(variants.keys)==Set(["1","2"]),"Expected both separately observed backing-scale references")
        guard let variant=variants[String(Int(scale))] else{throw CheckFailure.failed("No observed reference for current backing scale")}
        try require(variant["scale"] as? Int==Int(scale),"Reference backing scale differs")
        selected=variant
        evidence["referenceImageSha256"]=variant["referenceImageSha256"]
        evidence["referenceBackingScale"]=Int(scale)
    } else {selected=calibration}
    let expected=selected["samples"] as! [[Int]]
    try require(expected.count==72*18+248*33,"Expected complete reviewed toolbar and editor Curve regions")
    var maximum=0
    for sample in expected {
        try require(sample.count==6,"Invalid raster sample")
        let x=Int((Double(sample[0])*scale).rounded()),y=Int((Double(sample[1])*scale).rounded())
        let at=(y*width+x)*4
        try require(bytes[at+3]==sample[5],"Toolbar alpha differs")
        for k in 0..<3 {maximum=max(maximum,abs(Int(bytes[at+k])-sample[k+2]))}
    }
    evidence["toolbarSamples"]=expected.count;evidence["maximumRGBDifference"]=maximum
    try require(maximum<=3,"Reviewed toolbar pixels do not match current normalized raster")
}

let args=CommandLine.arguments
if args.count==4 && args[1]=="--make-calibration" {
    let path=args[2];var proof=[String:Any]();let(width,_,bytes)=try normalizedRaster(URL(fileURLWithPath:path),evidence:&proof)
    let scale=Double(width)/1100;var samples=[[Int]]()
    for region in [(10,72,72,18),(844,272,248,33)] {
        for y in region.1..<(region.1+region.3) {for x in region.0..<(region.0+region.2) {
            let at=(Int((Double(y)*scale).rounded())*width+Int((Double(x)*scale).rounded()))*4
            samples.append([x,y,Int(bytes[at]),Int(bytes[at+1]),Int(bytes[at+2]),Int(bytes[at+3])])
        }}
    }
    let window:[String:Any]=["label":"editor","title":"Editor Library – profile","referenceImage":path,"referenceImageSha256":sha256(try Data(contentsOf:URL(fileURLWithPath:path))),"samples":samples,"proof":proof]
    let calibration:[String:Any]=["schema":1,"windows":[window],"windowSize":[1100,780],"sourcePoint":[970.0,299.0],"destinationPoint":[37.75,81.0],"curveRegion":[844,272,248,33],"toolbarRegion":[10,72,72,18],"scope":"Observed current Pulse inline Curve three-point graph and four Curve mini-cells. Complete graph/toolbar pixels in normalized sRGB, RGB rounding tolerance3, exact alpha and geometry. No desktop initialization/input."]
    try JSONSerialization.data(withJSONObject:calibration,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[3]));exit(0)
}
if args.count==5 && args[1]=="--check-calibration" {
    let calibration=try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:args[3]))) as! [String:Any]
    var evidence:[String:Any]=["passed":false,"inputPosted":false]
    do {try checkedRaster(URL(fileURLWithPath:args[2]),calibration:calibration,evidence:&evidence);evidence["passed"]=true}
    catch {evidence["error"]=String(describing:error)}
    try JSONSerialization.data(withJSONObject:evidence,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[4]));exit(evidence["passed"] as? Bool == true ? 0:1)
}
if args.count==3 && args[1]=="--preflight" {
    let evidence:[String:Any]=["accessibilityTrusted":AXIsProcessTrusted(),"eventPostingPreflight":CGPreflightPostEventAccess(),"permissionRequested":false,"inputPosted":false]
    try JSONSerialization.data(withJSONObject:evidence,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[2]));exit(0)
}
guard args.count==5,let pid=Int32(args[1]) else {fatalError("PID, output directory, exact executable, calibration required")}
let out=URL(fileURLWithPath:args[2]),executable=URL(fileURLWithPath:args[3]).standardizedFileURL
let calibration=try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:args[4]))) as! [String:Any]
let expectedBundle=calibration["bundleIdentifier"] as! String
let previous=NSWorkspace.shared.frontmostApplication,oldPointer=CGEvent(source:nil)?.location
let began=ProcessInfo.processInfo.systemUptime,terminalEnd=began+40,cleanupEnd=began+48
var inputEnd=terminalEnd,releaseEnd=cleanupEnd
var target=NSRunningApplication(processIdentifier:pid)
let app=AXUIElementCreateApplication(pid),systemAX=AXUIElementCreateSystemWide()
AXUIElementSetMessagingTimeout(app,0.15)
var report:[String:Any]=["passed":false,"targetPID":pid,"inputDeadlineSeconds":40,"cleanupDeadlineSeconds":48,"scope":"Actual inline editor Curve to same-window toolbar copy; exact AX/native window ID; no drag or mutation hooks"]
var observations=[[String:Any]](),events=[[String:Any]]()
var ownedNameSheet:AXUIElement?,ownedNameField:AXUIElement?
var ownedWindows=[String:AXUIElement](),windowIDs=[String:Int](),frames=[String:CGRect]()
var activated=false,ownershipLost=false,pointerPosted=false,keyboardPosted=false,mouseHeld=false
var heldKeys=Set<CGKeyCode>(),mouseDowns=0,mouseUps=0,keyDowns=0,keyUps=0,lastPoint=CGPoint.zero
func now()->Double {ProcessInfo.processInfo.systemUptime}
func pump(_ seconds:Double=0.05){RunLoop.current.run(until:Date().addingTimeInterval(seconds))}
func owned(_ node:AXUIElement)->Bool {var owner:pid_t = -1;return AXUIElementGetPid(node,&owner) == .success && owner==pid}
func attr(_ node:AXUIElement,_ name:String,cleanup:Bool=false)->CFTypeRef? {
    guard now()<(cleanup ? cleanupEnd:inputEnd),owned(node) else{return nil}
    AXUIElementSetMessagingTimeout(node,0.15);var result:CFTypeRef?
    return AXUIElementCopyAttributeValue(node,name as CFString,&result) == .success ? result:nil
}
func ax(_ value:CFTypeRef?)->AXUIElement? {guard let value,CFGetTypeID(value)==AXUIElementGetTypeID() else{return nil};return (value as! AXUIElement)}
func title(_ node:AXUIElement)->String {attr(node,kAXTitleAttribute) as? String ?? ""}
func rect(_ node:AXUIElement,cleanup:Bool=false)throws->CGRect {
    guard let p=attr(node,kAXPositionAttribute,cleanup:cleanup),let s=attr(node,kAXSizeAttribute,cleanup:cleanup),CFGetTypeID(p)==AXValueGetTypeID(),CFGetTypeID(s)==AXValueGetTypeID() else{throw CheckFailure.failed("Missing AX frame")}
    var point=CGPoint.zero,size=CGSize.zero
    try require(AXValueGetValue(p as! AXValue,.cgPoint,&point) && AXValueGetValue(s as! AXValue,.cgSize,&size),"Invalid AX frame")
    return CGRect(origin:point,size:size)
}
func sameFrame(_ a:CGRect,_ b:CGRect)->Bool {abs(a.minX-b.minX)<1 && abs(a.minY-b.minY)<1 && abs(a.width-b.width)<1 && abs(a.height-b.height)<1}
func identity()->Bool {
    guard let current=NSRunningApplication(processIdentifier:pid) else{return false};target=current
    return !current.isTerminated && current.executableURL?.standardizedFileURL==executable && current.bundleIdentifier==expectedBundle
}
func previousMatches()->Bool {
    guard let previous,let current=NSRunningApplication(processIdentifier:previous.processIdentifier) else{return false}
    return !current.isTerminated && current.bundleIdentifier==previous.bundleIdentifier && current.executableURL?.standardizedFileURL==previous.executableURL?.standardizedFileURL
}
func priorOrTarget()->Bool {
    let current=NSWorkspace.shared.frontmostApplication
    if (current?.bundleIdentifier ?? "").lowercased().contains("securityagent"){return false}
    if current?.processIdentifier==pid{return identity()}
    return previousMatches() && current?.processIdentifier==previous?.processIdentifier
}
func foreground()->Bool {identity() && target?.isActive==true && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid}
func pointerStable()->Bool {guard let p=CGEvent(source:nil)?.location,let initial=oldPointer else{return false};let expected=pointerPosted ? lastPoint:initial;return hypot(p.x-expected.x,p.y-expected.y)<=3}
func physicalOnlyOurs()->Bool {
    let keys=Set((0..<128).map{CGKeyCode($0)}.filter{CGEventSource.keyState(.combinedSessionState,key:$0)})
    let buttons=[CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}
    let flags=CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand])
    return keys.isSubset(of:heldKeys) && buttons.allSatisfy{mouseHeld && $0 == .left} && flags.subtracting(heldKeys.contains(55) ? .maskCommand:[]).isEmpty
}
func restoreSafe()->Bool {
    guard activated && !ownershipLost && now()<cleanupEnd && !mouseHeld && heldKeys.isEmpty && mouseDowns==mouseUps && keyDowns==keyUps else{return false}
    return previousMatches() && priorOrTarget() && pointerStable() && physicalOnlyOurs() && now()<cleanupEnd
}
func guardApp(_ node:AXUIElement?=nil)throws {
    let ok=now()<inputEnd && foreground() && pointerStable() && physicalOnlyOurs() && (node.map(owned) ?? true)
    if !ok{ownershipLost=true};try require(ok,"Deadline, app identity, foreground or pointer changed")
    try require(now()<inputEnd,"Deadline during app observation")
}
func nativeWindows()->[[String:Any]] {CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] ?? []}
func nativeFrame(_ row:[String:Any])->CGRect? {guard let value=row[kCGWindowBounds as String] as? [String:Any] else{return nil};return CGRect(dictionaryRepresentation:value as CFDictionary)}
// CG rectangles include input-transparent overlays. Require the actual AX hit
// to belong to this exact observed root as well as its stable native ID/frame.
func actualHit(_ point:CGPoint,root:AXUIElement,cleanup:Bool=false)->Bool {
    let deadline=cleanup ? releaseEnd:inputEnd
    guard now()<deadline else{return false}
    AXUIElementSetMessagingTimeout(systemAX,0.15)
    var hit:AXUIElement?
    let status=AXUIElementCopyElementAtPosition(systemAX,Float(point.x),Float(point.y),&hit)
    var evidence:[String:Any]=["point":[point.x,point.y],"status":status.rawValue,"cleanup":cleanup,"reachedExactRoot":false]
    defer {report["lastActualHit"]=evidence}
    guard status == .success,let hit,now()<deadline else{return false}
    var owner:pid_t = -1
    let pidStatus=AXUIElementGetPid(hit,&owner);evidence["pidStatus"]=pidStatus.rawValue;evidence["ownerPID"]=owner
    guard pidStatus == .success,owner==pid else{return false}
    let window=ax(attr(hit,kAXWindowAttribute,cleanup:cleanup))
    var reached=window.map{CFEqual($0,root)} ?? false,node:AXUIElement?=hit
    evidence["windowIsExactRoot"]=reached
    if !reached {
        for _ in 0..<14 {
            guard let current=node,now()<deadline,owned(current) else{break}
            if CFEqual(current,root){reached=true;break}
            if attr(current,kAXRoleAttribute,cleanup:cleanup) as? String==kAXApplicationRole{break}
            node=ax(attr(current,kAXParentAttribute,cleanup:cleanup))
        }
    }
    evidence["reachedExactRoot"]=reached
    return reached && now()<deadline
}
func topWindow(_ point:CGPoint,cleanup:Bool=false)->Int? {
    let deadline=cleanup ? releaseEnd:inputEnd
    guard now()<deadline else{return nil}
    let records=nativeWindows()
    for (label,root) in ownedWindows {
        guard let number=windowIDs[label],let expected=frames[label],expected.contains(point),
              let current=try? rect(root,cleanup:cleanup),sameFrame(current,expected) else{continue}
        let matches=records.filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid) && ($0[kCGWindowAlpha as String] as? Double ?? 0)>0 && nativeFrame($0).map{sameFrame($0,expected)}==true}
        guard matches.count==1,matches[0][kCGWindowNumber as String] as? Int==number,
              actualHit(point,root:root,cleanup:cleanup),now()<deadline else{continue}
        return number
    }
    return nil
}
func keyAndMain(_ labels:[String],cleanup:Bool=false)->Bool {
    guard let key=ax(attr(app,kAXFocusedWindowAttribute,cleanup:cleanup)),let main=ax(attr(app,kAXMainWindowAttribute,cleanup:cleanup)) else{return false}
    return [key,main].allSatisfy {node in labels.contains {ownedWindows[$0].map{CFEqual($0,node)}==true}}
}
func observeFrame(_ label:String)throws->Bool {
    let bounds=try rect(ownedWindows[label]!),records=nativeWindows()
    let matches=records.filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid) && ($0[kCGWindowLayer as String] as? Int)==0 && nativeFrame($0).map{sameFrame($0,bounds)}==true}
    observations.append(["stage":"frame-"+label,"ax":[bounds.minX,bounds.minY,bounds.width,bounds.height],"matches":matches.count,"native":records.filter{($0[kCGWindowOwnerPID as String] as? Int)==Int(pid)}])
    try require(matches.count<=1,"Ambiguous exact native window")
    guard let number=matches.first?[kCGWindowNumber as String] as? Int else{return false}
    try require(windowIDs[label]==nil || windowIDs[label]==number,"Native window ID changed")
    windowIDs[label]=number;frames[label]=bounds;return true
}
func raise(_ label:String)throws {
    guard let window=ownedWindows[label] else{throw CheckFailure.failed("Missing exact window")}
    try guardApp(window)
    if !keyAndMain([label]) {
        let status=AXUIElementPerformAction(window,kAXRaiseAction as CFString)
        observations.append(["stage":"raise-"+label,"status":status.rawValue])
        try guardApp(window)
        if !keyAndMain([label]){_ = AXUIElementSetAttributeValue(window,kAXMainAttribute as CFString,kCFBooleanTrue)}
    }
    let end=min(inputEnd,now()+1.2)
    while now()<end {
        try guardApp(window)
        if keyAndMain([label]),try observeFrame(label),let frame=frames[label] {
            let requiredPoint=CGPoint(x:frame.minX+970,y:frame.minY+299)
            let actual=topWindow(requiredPoint)
            observations.append(["stage":"native-front-"+label,"requiredPoint":[requiredPoint.x,requiredPoint.y],"expectedWindow":windowIDs[label] ?? -1,"actualWindow":actual ?? -1])
            try guardApp(window)
            if actual==windowIDs[label] && now()<inputEnd{return}
        }
        pump()
    }
    throw CheckFailure.failed("Exact key/main/native frame/front point did not settle: "+label)
}
func perPoint(_ point:CGPoint,_ label:String,dragging:Bool)throws {
    try guardApp()
    let labels=dragging ? ["editor"]:[label]
    let ok=keyAndMain(labels) && topWindow(point)==windowIDs[label] && frames[label]?.contains(point)==true
    if !ok{ownershipLost=true};try require(ok,"Unexpected exact window at posted point: "+label)
    for name in ["editor"] {try require(sameFrame(try rect(ownedWindows[name]!),frames[name]!),"Owned window moved during input")}
    try require(now()<inputEnd,"Deadline after point/window observations")
}
func mouse(_ kind:CGEventType,_ point:CGPoint,_ label:String,command:Bool=false)throws {
    try perPoint(point,label,dragging:mouseHeld)
    guard let event=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else{throw CheckFailure.failed("Cannot create mouse event")}
    event.flags=command ? .maskCommand:[];event.setIntegerValueField(.mouseEventClickState,value:1)
    try require(now()<inputEnd,"Deadline before mouse post")
    if kind == .leftMouseDown{try require(!mouseHeld,"Duplicate mouse down")}
    if kind == .leftMouseUp{try require(mouseHeld,"Unpaired mouse up")}
    try guardApp();try require(topWindow(point)==windowIDs[label],"Immediate mouse-post window changed");try guardApp();try require(now()<inputEnd,"Immediate mouse-post deadline changed")
    if kind == .leftMouseDown{mouseHeld=true;mouseDowns+=1}
    event.post(tap:.cghidEventTap);pointerPosted=true;lastPoint=point
    if kind == .leftMouseUp{mouseHeld=false;mouseUps+=1}
    events.append(["kind":kind.rawValue,"point":[point.x,point.y],"window":windowIDs[label] ?? -1,"label":label,"command":command])
}
func checkpoint(_ name:String)throws {
    try guardApp()
    try Data("ready".utf8).write(to:out.appending(path:"checkpoint-\(name).request"),options:.atomic)
    let ack=out.appending(path:"checkpoint-\(name).json"),end=min(inputEnd,now()+2)
    while !FileManager.default.fileExists(atPath:ack.path) && now()<end {try guardApp();pump()}
    let data=try Data(contentsOf:ack),value=try JSONSerialization.jsonObject(with:data) as! [String:Any]
    try require(value["passed"] as? Bool==true && value["whileAlive"] as? Bool==true,"Saved checkpoint not accepted: "+name)
}
func keyboard(_ code:CGKeyCode,_ down:Bool,flags:CGEventFlags=[],unicode:String?=nil,check:()->Bool)throws {
    try guardApp();try require(check(),"Exact keyboard field/window ownership changed")
    guard let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else{throw CheckFailure.failed("Cannot create key")}
    event.flags=flags
    if let unicode{let units=Array(unicode.utf16);units.withUnsafeBufferPointer{event.keyboardSetUnicodeString(stringLength:units.count,unicodeString:$0.baseAddress!)}}
    try guardApp();try require(now()<inputEnd,"Deadline before key post")
    if down{try require(!heldKeys.contains(code),"Duplicate key down");heldKeys.insert(code);keyDowns+=1}else{try require(heldKeys.contains(code),"Unpaired key up")}
    event.post(tap:.cghidEventTap);keyboardPosted=true
    if !down{heldKeys.remove(code);keyUps+=1};events.append(["key":Int(code),"down":down,"command":flags.contains(.maskCommand)]);pump(0.04)
}
func drag()throws {
    try raise("editor")
    let frame=frames["editor"]!,a=calibration["sourcePoint"] as! [Double],b=calibration["destinationPoint"] as! [Double]
    let start=CGPoint(x:frame.minX+a[0],y:frame.minY+a[1]),end=CGPoint(x:frame.minX+b[0],y:frame.minY+b[1])
    try mouse(.mouseMoved,start,"editor");pump(0.15)
    try require(pointerStable(),"Source pointer did not settle")
    try mouse(.leftMouseDown,start,"editor");pump(0.12)
    // The press is in the observed graph background, away from all three handles;
    // unmodified motion therefore starts the production native value drag.
    for i in 1...28 {
        let t=CGFloat(i)/28,point=CGPoint(x:start.x+(end.x-start.x)*t,y:start.y+(end.y-start.y)*t)
        try mouse(.leftMouseDragged,point,"editor");pump(0.035)
    }
    try mouse(.leftMouseUp,end,"editor");pump(0.2)
}
func tree(_ root:AXUIElement)->[AXUIElement] {
    var values=[AXUIElement]()
    func visit(_ node:AXUIElement,_ depth:Int){guard depth<16,values.count<350,now()<inputEnd else{return};values.append(node)
        if ["AXTable","AXOutline","AXBrowser"].contains(attr(node,kAXRoleAttribute) as? String ?? ""){return}
        for child in attr(node,kAXChildrenAttribute) as? [AXUIElement] ?? []{visit(child,depth+1)}}
    visit(root,0);return values
}
func labels(_ node:AXUIElement)->[String]{[kAXTitleAttribute,kAXDescriptionAttribute,kAXValueAttribute].map{attr(node,$0) as? String ?? ""}}
func nameCopy()throws {
    let destination=ownedWindows["editor"]!,end=min(inputEnd,now()+2)
    var found:AXUIElement?
    while now()<end {
        try guardApp();let matches=tree(destination).filter{attr($0,kAXRoleAttribute) as? String==kAXSheetRole}
        try require(matches.count<=1,"Multiple destination sheets")
        if let sheet=matches.first,tree(sheet).contains(where:{labels($0).contains("Curve name?")}){found=sheet;break};pump()
    }
    guard let sheet=found else{throw CheckFailure.failed("Editor Curve copy has no Curve name sheet")}
    ownedNameSheet=sheet
    if !nameWindowOwned() {
        try guardApp(destination);_ = AXUIElementPerformAction(destination,kAXRaiseAction as CFString)
        try guardApp(destination);_ = AXUIElementSetAttributeValue(destination,kAXMainAttribute as CFString,kCFBooleanTrue)
    }
    let focusEnd=min(inputEnd,now()+1.2)
    while now()<focusEnd && !nameWindowOwned(){try guardApp();pump()}
    try require(nameWindowOwned(),"Exact destination or attached sheet does not own naming focus")
    let nodes=tree(sheet),fields=nodes.filter{attr($0,kAXRoleAttribute) as? String==kAXTextFieldRole},buttons=nodes.filter{attr($0,kAXRoleAttribute) as? String==kAXButtonRole && labels($0).contains("OK")}
    try require(fields.count==1 && buttons.count==1,"Nonunique native name controls")
    var field=fields[0];let button=buttons[0];ownedNameField=field
    try guardApp(field);try require(AXUIElementSetAttributeValue(field,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"Cannot focus destination name field")
    pump(0.1)
    func focus()->Bool {nameWindowOwned() && ax(attr(app,kAXFocusedUIElementAttribute)).map{CFEqual($0,field)}==true}
    try keyboard(55,true,flags:.maskCommand,check:focus)
    try keyboard(0,true,flags:.maskCommand,check:focus);try keyboard(0,false,flags:.maskCommand,check:focus)
    try keyboard(55,false,check:focus)
    try keyboard(0,true,unicode:"Editor Curve Copy",check:focus);try keyboard(0,false,unicode:"Editor Curve Copy",check:focus)
    let settle=min(inputEnd,now()+1)
    while now()<settle && attr(field,kAXValueAttribute) as? String != "Editor Curve Copy"{try guardApp();pump()}
    try require(attr(field,kAXValueAttribute) as? String=="Editor Curve Copy","Actual Unicode name readback mismatch");pump(0.2)
    report["nameFocusIdentity"]=["focusedRole":ax(attr(app,kAXFocusedWindowAttribute)).flatMap{attr($0,kAXRoleAttribute) as? String} ?? "", "mainTitle":ax(attr(app,kAXMainWindowAttribute)).map(title) ?? "", "fieldReadback":attr(field,kAXValueAttribute) as? String ?? ""]
    // SwiftUI may reconstruct accessibility button objects after @State text
    // changes. Re-observe the exact attached sheet and its enabled OK control.
    try guardApp();try require(nameWindowOwned(),"Name sheet ownership changed after typing")
    let freshNodes=tree(sheet)
    let freshFields=freshNodes.filter{attr($0,kAXRoleAttribute) as? String==kAXTextFieldRole}
    try require(freshFields.count==1 && attr(freshFields[0],kAXValueAttribute) as? String=="Editor Curve Copy","Live name field changed before fresh OK lookup")
    field=freshFields[0];ownedNameField=field
    let freshButtons=freshNodes.filter{attr($0,kAXRoleAttribute) as? String==kAXButtonRole && labels($0).contains("OK")}
    try require(freshButtons.count==1,"No unique fresh name OK button")
    let fresh=freshButtons[0]
    try guardApp(fresh);try require(attr(fresh,kAXEnabledAttribute) as? Bool==true,"Fresh native name OK disabled")
    var actionNames:CFArray?;let actionsStatus=AXUIElementCopyActionNames(fresh,&actionNames)
    report["nameOKObservation"]=["sameAsBeforeTyping":CFEqual(fresh,button),"oldButtonRole":attr(button,kAXRoleAttribute) as? String ?? "unavailable","freshRole":attr(fresh,kAXRoleAttribute) as? String ?? "", "actionsStatus":actionsStatus.rawValue,"actions":actionNames as? [String] ?? []]
    try require(actionsStatus == .success && (actionNames as? [String] ?? []).contains(kAXPressAction),"Fresh name OK does not advertise AXPress")
    try guardApp(fresh);try require(focus() && attr(field,kAXValueAttribute) as? String=="Editor Curve Copy" && now()<inputEnd,"Name acceptance ownership/value/deadline")
    try guardApp(fresh)
    let pressStatus=AXUIElementPerformAction(fresh,kAXPressAction as CFString)
    report["nameOKPressStatus"]=pressStatus.rawValue
    let gone=min(inputEnd,now()+1.5)
    while now()<gone && tree(destination).contains(where:{attr($0,kAXRoleAttribute) as? String==kAXSheetRole}){try guardApp();pump()}
    try require(!tree(destination).contains(where:{attr($0,kAXRoleAttribute) as? String==kAXSheetRole}),"Name sheet remained after fresh OK, AX status=\(pressStatus.rawValue)")
    report["nativeNameSheetObserved"]=true;report["enteredName"]="Editor Curve Copy"
    report["nameSheetDismissed"]=true
    // The caller's copied checkpoint additionally requires all five exact saved
    // entries. A non-success status is never accepted without that state proof.

}
func nameWindowOwned(cleanup:Bool=false)->Bool {
    guard let destination=ownedWindows["editor"],let sheet=ownedNameSheet,
          let main=ax(attr(app,kAXMainWindowAttribute,cleanup:cleanup)),CFEqual(main,destination),
          let key=ax(attr(app,kAXFocusedWindowAttribute,cleanup:cleanup)),(CFEqual(key,destination) || CFEqual(key,sheet)),
          let bounds=try? rect(sheet,cleanup:cleanup),let parent=frames["editor"],bounds.width>0,bounds.height>0,parent.insetBy(dx:-1,dy:-1).contains(bounds) else{return false}
    // Follow the observed sheet's actual AX parent chain rather than walking the
    // unrelated timeline subtree during a held key's guarded cleanup.
    var node:AXUIElement?=sheet
    for _ in 0..<10 {guard let current=node else{return false};if CFEqual(current,destination){return true};node=ax(attr(current,kAXParentAttribute,cleanup:cleanup))}
    return false
}
func releaseOwnedInput(){
    func cleanupOwned()->Bool {
        guard now()<releaseEnd && !ownershipLost && foreground() && pointerStable() && physicalOnlyOurs() else{return false}
        let exact=keyAndMain(["editor"],cleanup:true) || nameWindowOwned(cleanup:true)
        return exact && now()<releaseEnd && foreground() && pointerStable() && physicalOnlyOurs()
    }
    if mouseHeld,cleanupOwned(),let point=CGEvent(source:nil)?.location,windowIDs.values.contains(where:{$0==topWindow(point,cleanup:true)}),let event=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:point,mouseButton:.left),cleanupOwned(),now()<releaseEnd{
        event.flags=heldKeys.contains(55) ? .maskCommand:[]
        if now()<releaseEnd{event.post(tap:.cghidEventTap);mouseHeld=false;mouseUps+=1}
    }
    for code in heldKeys.sorted() where cleanupOwned(){
        if let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:false),cleanupOwned(){
            event.flags=[]
            if now()<releaseEnd{event.post(tap:.cghidEventTap);heldKeys.remove(code);keyUps+=1}
        }
    }
}
func publish(){if let data=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]){try? data.write(to:out.appending(path:"foreground.json"),options:.atomic)}}
do {
    defer {
        releaseOwnedInput();pump(0.08)
        var pressedKeys=(0..<128).filter{CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0))}
        var pressedButtons=[CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}
        var flags=CGEventSource.flagsState(.combinedSessionState)
        var noInputHeld = !mouseHeld && heldKeys.isEmpty && mouseDowns==mouseUps && keyDowns==keyUps && pressedKeys.isEmpty && pressedButtons.isEmpty && flags.intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand]).isEmpty
        if noInputHeld && restoreSafe(){
            if pointerPosted,let oldPointer,restoreSafe(),now()<cleanupEnd{report["restoredPointer"]=CGWarpMouseCursorPosition(oldPointer) == .success;lastPoint=oldPointer}
            if restoreSafe(),now()<cleanupEnd{_ = previous?.activate(options:[])}
            for _ in 0..<15 where now()<cleanupEnd && NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier{pump()}
        }
        pressedKeys=(0..<128).filter{CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0))}
        pressedButtons=[CGMouseButton.left,.right,.center].filter{CGEventSource.buttonState(.combinedSessionState,button:$0)}
        flags=CGEventSource.flagsState(.combinedSessionState)
        noInputHeld = noInputHeld && pressedKeys.isEmpty && pressedButtons.isEmpty && flags.intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand]).isEmpty
        report["restoredPrevious"]=previousMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier==previous?.processIdentifier
        report["cleanupCompleted"] = !ownershipLost && noInputHeld && now()<cleanupEnd
        report["inputOwnershipMaintained"] = !ownershipLost;report["noInputHeld"]=noInputHeld
        report["heldButtonAfterCleanup"]=mouseHeld;report["heldKeyAfterCleanup"] = !heldKeys.isEmpty
        report["observedPressedKeyCodesAfterCleanup"]=pressedKeys;report["observedPressedButtonsAfterCleanup"]=pressedButtons.map(\.rawValue);report["observedModifierFlagsAfterCleanup"]=flags.rawValue
        report["mouseDownCount"]=mouseDowns;report["mouseUpCount"]=mouseUps;report["keyDownCount"]=keyDowns;report["keyUpCount"]=keyUps
        report["pointerInputPosted"]=pointerPosted;report["keyboardInputPosted"]=keyboardPosted;report["activated"]=activated
        report["targetExited"]=target?.isTerminated ?? true;report["events"]=events;report["observations"]=observations;report["windowIDs"]=windowIDs
        report["elapsedSeconds"]=now()-began
        if report["cleanupCompleted"] as? Bool != true || report["restoredPrevious"] as? Bool != true || (pointerPosted && report["restoredPointer"] as? Bool != true){report["passed"]=false}
        publish()
    }
    do {
        let registered=min(inputEnd,now()+8)
        while now()<registered{try require(priorOrTarget() && pointerStable() && physicalOnlyOurs(),"Desktop/input changed during app registration");target=NSRunningApplication(processIdentifier:pid);if identity() && target?.isFinishedLaunching==true{break};pump()}
        try require(identity() && expectedBundle.hasPrefix("com.vixenlights.verification."),"Owned private launch identity mismatch")
        try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Existing permissions unavailable; no prompt")
        for name in ["ready"]{
            while !FileManager.default.fileExists(atPath:out.appending(path:"ui/\(name).png").path){try require(now()<inputEnd && priorOrTarget() && identity() && pointerStable() && physicalOnlyOurs(),"Startup marker deadline/ownership");pump()}
        };pump(0.3)
        // The harness's fixed24s input window begins immediately after this
        // snapshot. Convert its observed age once to a monotonic cutoff, leaving
        // 8s before harness activation/quit and6s for owned input release.
        let stamp=try FileManager.default.attributesOfItem(atPath:out.appending(path:"ui/ready.png").path)[.modificationDate] as! Date
        let wallBefore=Date(),monoBefore=now(),age=wallBefore.timeIntervalSince(stamp)
        try require(age>=0 && age<10,"Snapshot age is unsafe for fixed harness wait")
        let wallAfter=Date(),monoAfter=now()
        try require(abs(wallAfter.timeIntervalSince(wallBefore)-(monoAfter-monoBefore))<0.05,"Clock changed during wait-window conversion")
        inputEnd=min(terminalEnd,monoBefore+16-age);releaseEnd=min(cleanupEnd,inputEnd+6)
        try require(now()+8<inputEnd,"Insufficient real input window remains")
        report["interactionEndSeconds"]=inputEnd-began;report["releaseEndSeconds"]=releaseEnd-began;report["snapshotAgeSeconds"]=age;report["harnessResumeReserveSeconds"]=8
        try require(oldPointer != nil,"Initial pointer unavailable; restoration cannot be proven")
        let cals=calibration["windows"] as! [[String:Any]]
        var pixelEvidence=[[String:Any]]()
        for cal in cals{
            let label=cal["label"] as! String,path=out.appending(path:"ui/ready.png")
            try require(sha256(try Data(contentsOf:URL(fileURLWithPath:cal["referenceImage"] as! String)))==cal["referenceImageSha256"] as? String,"Retained reference changed")
            if let variants=cal["scaleReferences"] as? [String:[String:Any]] {
                for variant in variants.values {
                    try require(sha256(try Data(contentsOf:URL(fileURLWithPath:variant["referenceImage"] as! String)))==variant["referenceImageSha256"] as? String,"Retained backing-scale reference changed")
                }
            }
            var evidence:[String:Any]=["label":label];try checkedRaster(path,calibration:cal,evidence:&evidence);pixelEvidence.append(evidence)
            let matches=(attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? []).filter{title($0)==cal["title"] as? String}
            try require(matches.count==1,"Exact unique sequence AX window unavailable: "+label);ownedWindows[label]=matches[0]
        }
        report["rasterEvidence"]=pixelEvidence
        try require(ownedWindows.count==1,"Expected one exact editor window")
        guard let screen=NSScreen.screens.first else{throw CheckFailure.failed("No screen")}
        let visible=screen.visibleFrame,full=screen.frame
        try require(visible.width>=1140 && visible.height>=820,"Screen cannot contain editor calibration")
        let top=full.maxY-visible.maxY+20,left=visible.minX+20
        let planned=["editor":CGRect(x:left,y:top,width:1100,height:780)]
        report["screen"]=["frame":[full.minX,full.minY,full.width,full.height],"visible":[visible.minX,visible.minY,visible.width,visible.height]]
        try require((0..<128).allSatisfy{!CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0))} && [CGMouseButton.left,.right,.center].allSatisfy{!CGEventSource.buttonState(.combinedSessionState,button:$0)} && CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand]).isEmpty,"Input already held")
        try require(now()<inputEnd && priorOrTarget() && pointerStable() && physicalOnlyOurs(),"Desktop/input changed before activation")
        try require(target?.activate(options:[])==true,"Activation failed");activated=true
        for _ in 0..<20 where !foreground(){pump()};try guardApp()
        for label in ["editor"]{
            var position=planned[label]!.origin
            try guardApp(ownedWindows[label]);try require(AXUIElementSetAttributeValue(ownedWindows[label]!,kAXPositionAttribute as CFString,AXValueCreate(.cgPoint,&position)!) == .success,"Cannot place exact owned window")
            try raise(label);try require(sameFrame(frames[label]!,planned[label]!),"Window differs from screen-relative calibration")
        }
        try require(windowIDs.count==1,"Expected one exact native window ID")
        try checkpoint("ready")
        try drag();try nameCopy();try checkpoint("copied")
        report["inputCompleted"]=true
        while identity() && now()<terminalEnd{try require(priorOrTarget(),"Foreground changed while awaiting owned shutdown");pump()}
        try require(target?.isTerminated==true,"Private app did not exit before deadline");report["passed"]=true
    } catch {report["error"]=String(describing:error);releaseOwnedInput();if identity(){kill(pid,SIGTERM);let end=min(cleanupEnd,now()+2);while target?.isTerminated==false && now()<end{pump()}}}
}

exit(report["passed"] as? Bool == true ? 0:1)
