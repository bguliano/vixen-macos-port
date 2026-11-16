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
func checkedRaster(_ path: URL, calibration: [String: Any], evidence: inout [String: Any]) throws {
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
    guard let centers=calibration["panelCentersX"] as? [Double], let strip=calibration["toolbarCentersX"] as? [Double], let panelY=calibration["panelY"] as? Double, let toolbarY=calibration["toolbarY"] as? Double, centers.count==4,strip.count==4 else { throw CheckFailure.failed("Invalid reviewed calibration coordinates") }
    let reference = URL(fileURLWithPath:calibration["referenceImage"] as! String)
    try require(sha256(try Data(contentsOf:reference)) == calibration["referenceImageSha256"] as? String, "Reviewed layout image changed")
    // The retained Display-ICC snapshot round-trip differs by at most two codes.
    // Three codes allow one extra rounding code; alpha and geometry remain exact,
    // and the four color boxes remain separated by at least 249 codes/channel.
    let tolerance = 3, expected = [[255,0,0,255],[0,255,0,255],[0,0,255,255],[255,255,0,255]]
    evidence["rgbQuantizationTolerance"] = tolerance
    var samples = [[String: Any]]()
    for i in 0..<4 {
        for (role,x,y) in [("panel",centers[i],panelY),("toolbar",strip[i],toolbarY)] {
            let px=Int((x*scale).rounded()), py=Int((y*scale).rounded())
            try require(px>=0 && py>=0 && px<image.width && py<image.height, "Calibration point outside screenshot")
            let at=(py*image.width+px)*4, rgba=Array(bytes[at..<at+4]).map(Int.init)
            let matches=rgba[3]==255 && (0..<3).allSatisfy { abs(rgba[$0]-expected[i][$0]) <= tolerance }
            samples.append(["role":role,"index":i,"point":[x,y],"pixel":[px,py],"normalizedSRGBA8":rgba,"expectedSRGBA8":expected[i],"matches":matches])
            evidence["initialPixelSamples"] = samples
            try require(matches, "Reviewed cell coordinates do not match normalized sRGB raster: \(role) \(i), observed \(rgba), expected \(expected[i]), RGB tolerance \(tolerance)")
        }
    }
}

let args = CommandLine.arguments
if args.count == 5 && args[1] == "--check-calibration" {
    var evidence: [String: Any] = ["passed":false, "scope":"Same runtime raster guard only; no desktop initialization, app launch, AX access or input"]
    do {
        let calibration = try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:args[3]))) as! [String:Any]
        try checkedRaster(URL(fileURLWithPath:args[2]), calibration:calibration, evidence:&evidence)
        evidence["passed"] = true
    } catch { evidence["error"] = String(describing:error) }
    try JSONSerialization.data(withJSONObject:evidence,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:args[4]))
    exit(evidence["passed"] as? Bool == true ? 0 : 1)
}
if args.count == 3 && args[1] == "--preflight" {
    let result: [String: Any] = ["accessibilityTrusted": AXIsProcessTrusted(),
        "eventPostingPreflight": CGPreflightPostEventAccess(), "permissionRequested": false,
        "inputPosted": false, "scope": "Read-only permission preflight for this exact helper binary; runtime checks repeat before activation/input."]
    try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
        .write(to: URL(fileURLWithPath: args[2]))
    exit(0)
}
guard args.count == 5, let pid = Int32(args[1]) else { fatalError("PID, output folder, exact executable, calibration required") }
let calibrationURL = URL(fileURLWithPath: args[4])
let calibration = try JSONSerialization.jsonObject(with: Data(contentsOf: calibrationURL)) as! [String: Any]
let kind = calibration["kind"] as! String
precondition(["color", "gradient"].contains(kind))
let familyTitle = kind == "color" ? "Color" : "Gradient"
let centers: [CGFloat] = (calibration["panelCentersX"] as! [Double]).map { CGFloat($0) }
let stripCenters: [CGFloat] = (calibration["toolbarCentersX"] as! [Double]).map { CGFloat($0) }
let panelY = CGFloat(calibration["panelY"] as! Double)
let toolbarY = CGFloat(calibration["toolbarY"] as! Double)
let empty = calibration["emptyPoint"] as! [Double]
precondition(centers.count == 4 && stripCenters.count == 4 && empty.count == 2)
let panelToToolbar = false
let out = URL(fileURLWithPath: args[2])
let executable = URL(fileURLWithPath: args[3]).standardizedFileURL
var target = NSRunningApplication(processIdentifier: pid)
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let deadline = Date().addingTimeInterval(40)
let cleanupDeadline = deadline.addingTimeInterval(8)
var report: [String:Any] = ["passed":false,"targetPID":pid,"previousPID":previous?.processIdentifier ?? -1,
                          "scope":"Actual Color/Gradient OS pointer drags from reviewed layout; no drag callbacks or mutation hooks", "kind":kind, "inputDeadlineSeconds":40]
var observations: [[String:Any]] = []
var pointerChecks: [[String:Any]] = []
var held = false
var heldKey: CGKeyCode?
var verifiedBundleIdentifier: String?
var activated = false
var pointerInputPosted = false
var keyboardInputPosted = false
var failurePhase = "identity_preinput"
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
func inputGuard(_ node: AXUIElement? = nil) throws {
    try require(Date() < deadline && frontmost(), "Input deadline or foreground ownership lost")
    if let node {
        var owner: pid_t = -1
        try require(AXUIElementGetPid(node, &owner) == .success && owner == pid, "AX mutation target is not owned")
    }
}
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
    try require(Date() < deadline, "Hard40-second input deadline")
    try require(frontmost(),"Foreground ownership lost before input")
    try require(owns(point),"Private window does not own the pointer point")
    guard let event=CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:point,mouseButton:.left) else { throw CheckFailure.failed("Cannot construct CG mouse event") }
    event.flags=flags;event.setIntegerValueField(.mouseEventClickState,value:1)
    try inputGuard()
    event.post(tap:.cghidEventTap)
    pointerInputPosted = true
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
func releaseOwnedInput() {
    if held, Date() < cleanupDeadline, frontmost(), owns(lastPoint),
       let event=CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:lastPoint,mouseButton:.left),
       Date() < cleanupDeadline, frontmost() {
        event.post(tap:.cghidEventTap); held=false
    }
    if let keyCode=heldKey, Date() < cleanupDeadline, frontmost(),
       let event=CGEvent(keyboardEventSource:nil,virtualKey:keyCode,keyDown:false),
       Date() < cleanupDeadline, frontmost() {
        event.flags=[];event.post(tap:.cghidEventTap);heldKey=nil
    }
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
        report["cleanupDeadlineSeconds"] = 48
        releaseOwnedInput()
        func cleanupOwnsForeground() -> Bool {
            let current=NSWorkspace.shared.frontmostApplication
            guard Date() < cleanupDeadline, !(current?.bundleIdentifier ?? "").lowercased().contains("securityagent") else { return false }
            return current?.processIdentifier == pid || (current?.processIdentifier == previous?.processIdentifier && current?.bundleIdentifier == previous?.bundleIdentifier && current?.executableURL == previous?.executableURL)
        }
        if activated && cleanupOwnsForeground() {
            if pointerInputPosted, let oldPointer, cleanupOwnsForeground() { CGWarpMouseCursorPosition(oldPointer) }
            if cleanupOwnsForeground() { _ = previous?.activate(options:[]) }
            for _ in 0..<20 where Date() < cleanupDeadline && NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier { pump(0.05) }
            report["restoredPrevious"] = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        } else { report["restoredPrevious"]=NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier }
        report["targetExited"]=target?.isTerminated ?? true
        report["pointerInputPosted"]=pointerInputPosted
        report["keyboardInputPosted"]=keyboardInputPosted
        report["activated"]=activated
        report["heldButtonAfterCleanup"]=held
        report["heldKeyAfterCleanup"]=heldKey != nil
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
        verifiedBundleIdentifier=target?.bundleIdentifier
        try require(!(previous?.bundleIdentifier ?? "").lowercased().contains("securityagent"),"Authorization window is foreground; no input sent")
        try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"Existing AX/CG permissions unavailable; no prompts requested")
        try marker("ready")
        failurePhase = "calibration_preinput"
        do {
            try checkedRaster(out.appending(path:"ui/ready.png"), calibration:calibration, evidence:&report)
        } catch CheckFailure.failed(let message) {
            report["failureKind"] = "calibration_rejected"
            throw CheckFailure.failed(message)
        }
        failurePhase = "foreground_activation"
        let current = NSWorkspace.shared.frontmostApplication
        try require(!(current?.bundleIdentifier ?? "").lowercased().contains("securityagent"), "System authorization became foreground")
        try require(current?.processIdentifier == pid || (current?.processIdentifier == previous?.processIdentifier && current?.bundleIdentifier == previous?.bundleIdentifier && current?.executableURL == previous?.executableURL), "Another app became foreground before activation")
        try require(Date() < deadline && target?.activate(options:[])==true,"Input deadline or activation failed")
        activated = true
        for _ in 0..<25 where !frontmost() { pump() }
        try require(frontmost(),"Private app did not become foreground")
        failurePhase = "foreground_input"
        let app=AXUIElementCreateApplication(pid);AXUIElementSetMessagingTimeout(app,1)
        let windows=(value(app,kAXWindowsAttribute) as? [AXUIElement] ?? []).filter { label($0) == calibration["observedWindowTitle"] as? String }
        try require(windows.count == 1, "Reviewed private AX window title is not unique")
        let window=windows[0]
        guard let screen=NSScreen.screens.first else { throw CheckFailure.failed("No screen") }
        let width=screen.visibleFrame.width,height=screen.visibleFrame.height
        try require(width>=1140 && height>=820,"Screen cannot contain calibrated private window")
        var position=CGPoint(x:screen.visibleFrame.minX+20,y:screen.frame.maxY-screen.visibleFrame.maxY+20)
        guard let positionValue=AXValueCreate(.cgPoint,&position) else { throw CheckFailure.failed("Position encoding failed") }
        func show(_ stage: String) throws -> CGRect {
            try require(Date() < deadline && frontmost(),"Input deadline or foreground ownership lost before \(stage)")
            try inputGuard(window)
            try require(AXUIElementSetAttributeValue(window,kAXPositionAttribute as CFString,positionValue) == .success,"Cannot place private window")
            try inputGuard(window)
            _=AXUIElementPerformAction(window,kAXRaiseAction as CFString)
            try inputGuard(window)
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
        // Coordinates measured from the retained actual Color/Gradient layouts.
        // Initial raster samples below must match before activation or any input.
        var frame=try show("select-two")
        try click(point(frame,centers[2],panelY));try click(point(frame,centers[3],panelY),flags:.maskCommand)
        report["calibration"]=calibration
        try marker("selected-two");frame=try show("drag-two-left")
        try drag(point(frame,centers[2],panelY),point(frame,centers[0],panelY))
        try marker("moved-left");frame=try show("drag-selection-to-empty")
        // Locate C and D in the actual persisted order, read-only, supplied by the outer runner.
        let checkpointData=try Data(contentsOf:out.appending(path:"live-checkpoints.json"))
        let checkpoints=try JSONSerialization.jsonObject(with:checkpointData) as! [String:Any]
        guard let checkpoint=checkpoints["moved-left"] as? [String:Any],let order=checkpoint["order"] as? [String],
              let c=order.firstIndex(of:"\(familyTitle) C"),let d=order.firstIndex(of:"\(familyTitle) D"),order.count==4 else { throw CheckFailure.failed("Missing actual post-drag order") }
        try click(point(frame,centers[c],panelY));try click(point(frame,centers[d],panelY),flags:.maskCommand)
        try drag(point(frame,centers[c],panelY),point(frame,CGFloat(empty[0]),CGFloat(empty[1])))
        try marker("moved-end");frame=try show("toolbar-copy-to-panel")
        let endData=try Data(contentsOf:out.appending(path:"live-checkpoints.json"))
        let endCheck=try JSONSerialization.jsonObject(with:endData) as! [String:Any]
        guard let endState=endCheck["moved-end"] as? [String:Any],let endOrder=endState["order"] as? [String],
              let a=endOrder.firstIndex(of:"\(familyTitle) A"),let b=endOrder.firstIndex(of:"\(familyTitle) B") else { throw CheckFailure.failed("Missing actual move-to-end order") }
        report["panelToToolbar"] = panelToToolbar
        if panelToToolbar {
            try drag(point(frame,centers[a],panelY),point(frame,stripCenters[b],toolbarY))
        } else {
            try drag(point(frame,stripCenters[a],toolbarY),point(frame,centers[b],panelY))
        }
        pump(0.5)
        let sheet=find(window,role:kAXSheetRole)
        let prompt=sheet.flatMap{find($0,name:"\(familyTitle) name?")}
        report["toolbarNamingPromptObserved"]=prompt != nil
        try require(kind == "color" ? sheet == nil : prompt != nil, "Unexpected naming-sheet state for family")
        if let sheet, prompt != nil,let field=find(sheet,role:kAXTextFieldRole),let okay=find(sheet,role:kAXButtonRole,name:"OK") {
            try require(frontmost(),"Foreground lost before name entry")
            try inputGuard(field)
            try require(AXUIElementSetAttributeValue(field,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"Cannot focus private name field")
            pump(0.15)
            func fieldOwnsInput() -> Bool {
                guard frontmost(), let focused=element(value(app,kAXFocusedUIElementAttribute)) else { return false }
                return CFEqual(focused,field)
            }
            try require(fieldOwnsInput(),"Private name field did not acquire actual text focus")
            func sendKey(_ code: CGKeyCode, flags: CGEventFlags = [], unicode: String? = nil) throws {
                for down in [true,false] {
                    try require(Date() < deadline && fieldOwnsInput(),"Input deadline or text focus changed before name input")
                    guard let key=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else { throw CheckFailure.failed("Cannot create name key") }
                    key.flags=flags
                    if let unicode {
                        let units=Array(unicode.utf16)
                        units.withUnsafeBufferPointer { key.keyboardSetUnicodeString(stringLength:units.count,unicodeString:$0.baseAddress!) }
                    }
                    try inputGuard(field)
                    try require(fieldOwnsInput() && Date() < deadline, "Name field lost ownership before key post")
                    key.post(tap:.cghidEventTap);keyboardInputPosted=true;heldKey=down ? code : nil;pump(0.03)
                }
            }
            try sendKey(0,flags:.maskCommand)
            try sendKey(0,unicode:"Imported \(familyTitle)")
            for _ in 0..<20 where value(field,kAXValueAttribute) as? String != "Imported \(familyTitle)" { pump(0.05) }
            try require(value(field,kAXValueAttribute) as? String == "Imported \(familyTitle)","Private name text did not match after actual key events")
            // Let SwiftUI consume the editing notification before invoking its button.
            pump(0.2)
            report["nameInputMethod"]="Focused CG Unicode keys with AX readback and editing settle"
            report["enteredName"]=value(field,kAXValueAttribute) as? String
            try require(Date() < deadline && frontmost(),"Input deadline or foreground lost before name acceptance")
            try inputGuard(okay)
            try require(AXUIElementPerformAction(okay,kAXPressAction as CFString) == .success,"Cannot accept private library name")
        }
        try marker("toolbar-copy")
        report["inputCompleted"]=true
        failurePhase = "owned_exit_wait"
        while target?.isTerminated==false && Date()<deadline { pump() }
        try require(target?.isTerminated==true,"Private app did not exit")
        report["passed"]=true
    } catch {
        report["error"]=String(describing:error)
        report["failurePhase"]=failurePhase
        // Only a calibration failure before activation/input is eligible for
        // independent-stage continuation; the parent must also prove cleanup.
        report["benignPreinputCalibrationFailure"] = failurePhase == "calibration_preinput" && !activated && !pointerInputPosted && !keyboardInputPosted
        releaseOwnedInput()
        let currentTarget=NSRunningApplication(processIdentifier:pid)
        if let verifiedBundleIdentifier, currentTarget?.isTerminated==false,
           currentTarget?.executableURL?.standardizedFileURL==executable,
           currentTarget?.bundleIdentifier==verifiedBundleIdentifier {
            kill(pid,SIGTERM)
            for _ in 0..<30 where target?.isTerminated==false && Date() < cleanupDeadline { pump() }
        }
    }
}
