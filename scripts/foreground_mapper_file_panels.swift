// Owned-window Element Mapper file panels. Adapted from foreground_native_panels.swift;
// this separate helper adds strict identities/deadlines and never changes the frozen helper.
import AppKit
import ApplicationServices
import CryptoKit
import Foundation

enum Stop: Error { case failed(String) }
func need(_ ok: Bool, _ text: String) throws { if !ok { throw Stop.failed(text) } }
func hash(_ path: String) throws -> String { SHA256.hash(data: try Data(contentsOf: URL(fileURLWithPath: path))).map { String(format: "%02x", $0) }.joined() }
func canonical(_ value: String) -> String { value.precomposedStringWithCanonicalMapping }
func application(_ app: NSRunningApplication?) -> [String: Any] {
    ["pid": Int(app?.processIdentifier ?? -1), "bundleIdentifier": app?.bundleIdentifier ?? "", "executable": app?.executableURL?.standardizedFileURL.path ?? ""]
}
func emit(_ value: [String: Any]) { if let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), let text = String(data: data, encoding: .utf8) { print(text) } }
if CommandLine.arguments.count == 2 && CommandLine.arguments[1] == "--desktop" {
    emit(["application": application(NSWorkspace.shared.frontmostApplication), "accessibility": AXIsProcessTrusted(), "eventPosting": CGPreflightPostEventAccess(),
          "vixenProcesses": NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier?.hasPrefix("com.vixenlights.") == true }.map(application)])
    exit(0)
}
let cfg = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))) as! [String: Any]
let pid = pid_t(cfg["pid"] as! Int), out = URL(fileURLWithPath: cfg["out"] as! String)
let previous = cfg["previousApplication"] as! [String: Any]
let deadline = Date(timeIntervalSince1970: cfg["inputDeadlineEpoch"] as! Double)
let cleanupDeadline = Date(timeIntervalSince1970: cfg["cleanupDeadlineEpoch"] as! Double)
let root = AXUIElementCreateApplication(pid)
var target: NSRunningApplication?, held: (CGKeyCode, CGEventFlags)?
var report: [String: Any] = ["passed": false, "pid": Int(pid)], steps: [[String: Any]] = []
func pump(_ duration: Double = 0.05) { RunLoop.current.run(until: min(deadline, Date().addingTimeInterval(duration))) }
func owned(_ node: AXUIElement) -> Bool { var owner: pid_t = -1; return AXUIElementGetPid(node, &owner) == .success && owner == pid }
func attr(_ node: AXUIElement, _ key: String) -> CFTypeRef? {
    guard Date() < deadline, owned(node) else { return nil }
    AXUIElementSetMessagingTimeout(node, 0.2); var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(node, key as CFString, &value) == .success ? value : nil
}
func text(_ node: AXUIElement, _ key: String) -> String { attr(node, key) as? String ?? "" }
func kids(_ node: AXUIElement, _ key: String = kAXChildrenAttribute) -> [AXUIElement] { attr(node, key) as? [AXUIElement] ?? [] }
func ax(_ value: CFTypeRef?) -> AXUIElement? { guard let value, CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }; return (value as! AXUIElement) }
func guardInput(_ node: AXUIElement? = nil) throws {
    let current = NSRunningApplication(processIdentifier: pid)
    report["lastGuard"] = ["beforeDeadline": Date() < deadline, "target": application(current),
                           "active": current?.isActive ?? false, "finishedLaunching": current?.isFinishedLaunching ?? false,
                           "foreground": application(NSWorkspace.shared.frontmostApplication)]
    try need(Date() < deadline && current?.isTerminated == false && current?.executableURL?.standardizedFileURL.path == cfg["executable"] as? String &&
             current?.bundleIdentifier == cfg["bundleIdentifier"] as? String && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && current?.isActive == true,
             "input deadline, identity or foreground guard")
    if let node { try need(owned(node), "foreign AX control") }
    try need(Date() < deadline, "input deadline expired")
}
func tree(_ node: AXUIElement) -> [AXUIElement] {
    var result: [AXUIElement] = []
    func visit(_ value: AXUIElement, _ depth: Int) {
        guard Date() < deadline, owned(value), depth < 14, result.count < 300 else { return }
        result.append(value)
        if ["AXTable", "AXOutline", "AXBrowser"].contains(text(value, kAXRoleAttribute)) { return }
        for child in kids(value) { visit(child, depth+1) }
    }
    visit(node, 0); return result
}
func windows() -> [AXUIElement] { kids(root, kAXWindowsAttribute).filter(owned) }
func sheets() -> [AXUIElement] {
    var result: [AXUIElement] = []
    for window in windows() { for node in tree(window) where text(node, kAXRoleAttribute) == "AXSheet" && !result.contains(where: { CFEqual($0,node) }) { result.append(node) } }
    return result
}
func wait(_ stage: String, seconds: Double = 5, _ condition: () throws -> Bool) throws {
    let end = min(deadline, Date().addingTimeInterval(seconds))
    while Date() < end { try guardInput(); if try condition() { return }; pump() }
    throw Stop.failed("timeout: \(stage)")
}
func mapper(_ name: String) throws -> AXUIElement {
    var found: AXUIElement?
    try wait("mapper \(name)") {
        let matches = windows().filter { canonical(text($0,kAXTitleAttribute)) == canonical("Element Mapper - " + name) }
        try need(matches.count <= 1, "duplicate mapper window")
        if matches.count == 1 && sheets().isEmpty { found = matches[0]; return true }; return false
    }
    return found!
}
func labels(_ node: AXUIElement) -> [String] { [kAXTitleAttribute,kAXDescriptionAttribute,kAXHelpAttribute].map { text(node,$0) } }
func buttons(_ node: AXUIElement, _ name: String) -> [AXUIElement] { tree(node).filter { text($0,kAXRoleAttribute) == "AXButton" && labels($0).contains(name) } }
func press(_ node: AXUIElement, _ stage: String) throws {
    try guardInput(node); try need(attr(node,kAXEnabledAttribute) as? Bool == true,"disabled control: \(stage)")
    var actions: CFArray?; AXUIElementCopyActionNames(node,&actions)
    try need((actions as? [String] ?? []).contains(kAXPressAction),"control has no observed AXPress: \(stage)")
    try guardInput(node)
    let status=AXUIElementPerformAction(node,kAXPressAction as CFString)
    if status != .success && stage == "mapper-cancel" {
        let until=min(deadline,Date().addingTimeInterval(1))
        while target?.isTerminated==false && Date()<until { pump() }
        let data=try? Data(contentsOf:out.appendingPathComponent("events.json"))
        let value=data.flatMap { try? JSONSerialization.jsonObject(with:$0) as? [String:Any] }
        let events=value?["events"] as? [[String:Any]] ?? []
        let completed=events.filter {$0["event"] as? String=="mapper-completed"}
        try need(target?.isTerminated==true && completed.count==1 && completed[0]["cancelled"] as? Bool==true &&
                 events.suffix(2).compactMap {$0["event"] as? String}==["shutdown-complete","terminated"] &&
                 !events.contains(where:{$0["event"] as? String=="watchdog"}),"terminal Cancel lacks normal owned shutdown proof")
        report["terminalCancelAXStatus"]=status.rawValue
    } else { try need(status == .success,"AXPress: \(stage)") }
    pump(0.1)
}
func button(_ owner: AXUIElement, _ name: String) throws -> AXUIElement {
    let found = buttons(owner,name); try need(found.count == 1,"nonunique \(name) button"); return found[0]
}
func menu(_ name: String, mapName: String) throws {
    let window = try mapper(mapName)
    let file = tree(window).filter { ["AXButton","AXMenuButton","AXPopUpButton"].contains(text($0,kAXRoleAttribute)) && labels($0).contains("File") }
    try need(file.count == 1,"unique observed File menu unavailable"); try press(file[0],"File")
    var item: AXUIElement?
    try wait("observed menu \(name)") {
        let found = tree(window).filter { text($0,kAXRoleAttribute) == "AXMenuItem" && text($0,kAXTitleAttribute) == name }
        try need(found.count <= 1,"duplicate menu item"); item=found.first; return item != nil
    }
    try press(item!,name)
}
func panel(_ action: String) throws -> AXUIElement {
    var found: AXUIElement?
    try wait("native \(action) panel") {
        let matches = sheets().filter { buttons($0,action).count == 1 && buttons($0,"Cancel").count == 1 }
        try need(matches.count <= 1,"multiple native panels"); found=matches.first; return found != nil
    }
    return found!
}
func key(_ code: CGKeyCode, flags: CGEventFlags = []) throws {
    try guardInput(); try need(ax(attr(root,kAXFocusedWindowAttribute)).map(owned) == true,"focused window not owned")
    for down in [true,false] {
        try guardInput(); try need(ax(attr(root,kAXFocusedWindowAttribute)).map(owned) == true,"key target changed")
        try guardInput() // AX focus lookup may consume its bounded messaging timeout.
        guard let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else { throw Stop.failed("CG event unavailable") }
        event.flags=flags;try guardInput();if down { held=(code,flags) }; event.post(tap:.cghidEventTap); if !down { held=nil }
    }
    pump(0.1)
}
func set(_ node: AXUIElement, _ value: String) throws {
    try guardInput(node); try need(AXUIElementSetAttributeValue(node,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,"field focus failed")
    try need(ax(attr(root,kAXFocusedUIElementAttribute)).map { CFEqual($0,node) } == true,"observed field did not gain focus")
    try guardInput(node); try need(AXUIElementSetAttributeValue(node,kAXValueAttribute as CFString,value as CFString) == .success && canonical(text(node,kAXValueAttribute)) == canonical(value),"exact field value failed")
}
func goTo(_ path: String, panel: AXUIElement) throws {
    try key(5,flags:[.maskCommand,.maskShift]); var field: AXUIElement?
    try wait("native Go To field") {
        guard let node=ax(attr(root,kAXFocusedUIElementAttribute)), ["AXTextField","AXComboBox"].contains(text(node,kAXRoleAttribute)),tree(panel).contains(where:{CFEqual($0,node)}) else { return false }
        field=node; return true
    }
    try set(field!,path); try key(36)
    try wait("Go To dismissal") { !(ax(attr(root,kAXFocusedUIElementAttribute)).map { CFEqual($0,field!) } ?? false) }
}
func disk(_ stage: String) throws {
    try need(try hash(cfg["sourceMap"] as! String) == cfg["sourceMapSha256"] as? String,"source map changed")
    let path=cfg["savedMap"] as! String,exists=FileManager.default.fileExists(atPath:path)
    var step:[String:Any]=["stage":stage,"savedExists":exists]
    if exists { step["savedSha256"]=try hash(path) }
    let wanted = ["saved-live","saved-reopened"].contains(stage) ? path : cfg["sourceMap"] as! String
    var state: [String: Any]?
    try wait("read-only mapper state after \(stage)", seconds: 2) {
        guard let data=try? Data(contentsOf:out.appendingPathComponent("events.json")),
              let contents=try? JSONSerialization.jsonObject(with:data) as? [String:Any],
              let events=contents["events"] as? [[String:Any]],
              let latest=events.last(where:{$0["event"] as? String=="mapper-state"}),
              let actual=latest["path"] as? String, actual.precomposedStringWithCanonicalMapping==wanted.precomposedStringWithCanonicalMapping,
              latest["modified"] as? Bool==false else { return false }
        state=latest;return true
    }
    step["observedMapPath"]=state?["path"];step["observedMapName"]=state?["name"];step["observedModified"]=state?["modified"]
    steps.append(step)
}
func snapshot(_ name: String) {
    let rows=windows().flatMap(tree).map { node in ["role":text(node,kAXRoleAttribute),"title":text(node,kAXTitleAttribute),"description":text(node,kAXDescriptionAttribute),"value":text(node,kAXValueAttribute)] }
    if let data=try? JSONSerialization.data(withJSONObject:rows,options:[.prettyPrinted,.sortedKeys]) { try? data.write(to:out.appendingPathComponent(name+".ax.json")) }
}
do {
    defer {
        if let (code,flags)=held, Date()<cleanupDeadline {
            let current=NSRunningApplication(processIdentifier:pid)
            if current?.executableURL?.standardizedFileURL.path==cfg["executable"] as? String && current?.bundleIdentifier==cfg["bundleIdentifier"] as? String &&
                current?.isActive==true && NSWorkspace.shared.frontmostApplication?.processIdentifier==pid {
                AXUIElementSetMessagingTimeout(root,0.2);var focused:CFTypeRef?
                let status=AXUIElementCopyAttributeValue(root,kAXFocusedWindowAttribute as CFString,&focused)
                if status == .success, ax(focused).map(owned)==true, Date()<cleanupDeadline,
                   current?.isActive==true, NSWorkspace.shared.frontmostApplication?.processIdentifier==pid,
                   let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:false) {
                    event.flags=flags;event.post(tap:.cghidEventTap);held=nil
                }
            }
        }
        report["inputReleaseWithheld"]=held != nil
        let front=NSWorkspace.shared.frontmostApplication,old=NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))
        if held==nil && Date()<cleanupDeadline && (front?.processIdentifier==pid || front?.processIdentifier==old?.processIdentifier) &&
            old?.executableURL?.standardizedFileURL.path==previous["executable"] as? String && old?.bundleIdentifier==previous["bundleIdentifier"] as? String &&
            !(old?.bundleIdentifier ?? "").lowercased().contains("securityagent") {
            if front?.processIdentifier != old?.processIdentifier { _=old?.activate(options:[]) }
            let until=min(cleanupDeadline,Date().addingTimeInterval(2))
            while NSWorkspace.shared.frontmostApplication?.processIdentifier != old?.processIdentifier && Date()<until { RunLoop.current.run(until:Date().addingTimeInterval(0.05)) }
        }
        report["steps"]=steps;report["targetExited"]=target?.isTerminated ?? true;report["noInputHeld"]=held==nil
        report["restoredPrevious"]=NSWorkspace.shared.frontmostApplication?.processIdentifier==pid_t(previous["pid"] as! Int)
        if let data=try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]) { try? data.write(to:out.appendingPathComponent("foreground.json"),options:.atomic) }
    }
    do {
        let until=min(deadline,Date().addingTimeInterval(8))
        repeat { target=NSRunningApplication(processIdentifier:pid);if target?.executableURL != nil { break };pump() } while Date()<until
        try need(target?.executableURL?.standardizedFileURL.path==cfg["executable"] as? String && target?.bundleIdentifier==cfg["bundleIdentifier"] as? String &&
                 (target?.bundleIdentifier ?? "").hasPrefix("com.vixenlights.verification."),"owned app identity mismatch")
        try need(try hash(cfg["executable"] as! String)==cfg["executableSha256"] as? String,"owned executable changed")
        try need(AXIsProcessTrusted() && CGPreflightPostEventAccess(),"permissions unavailable; no prompt requested")
        let front=NSWorkspace.shared.frontmostApplication
        try need(!(front?.bundleIdentifier ?? "").lowercased().contains("securityagent") &&
                 (front?.processIdentifier==pid || (front?.processIdentifier==pid_t(previous["pid"] as! Int) && front?.executableURL?.standardizedFileURL.path==previous["executable"] as? String && front?.bundleIdentifier==previous["bundleIdentifier"] as? String)),"new foreground owner; activation refused")
        // A registered NSRunningApplication can report active before its launch
        // callback and WindowServer foreground state settle. Observe readiness
        // without sending input or repeatedly activating an unfinished process.
        let readyUntil=min(deadline,Date().addingTimeInterval(8))
        var ready=false
        while Date()<readyUntil {
            let current=NSRunningApplication(processIdentifier:pid), front=NSWorkspace.shared.frontmostApplication
            try need(current?.isTerminated==false && current?.executableURL?.standardizedFileURL.path==cfg["executable"] as? String &&
                     current?.bundleIdentifier==cfg["bundleIdentifier"] as? String,"owned app changed during startup")
            try need(front?.processIdentifier==pid || (front?.processIdentifier==pid_t(previous["pid"] as! Int) &&
                     front?.executableURL?.standardizedFileURL.path==previous["executable"] as? String && front?.bundleIdentifier==previous["bundleIdentifier"] as? String),
                     "new foreground owner during startup; activation refused")
            if let data=try? Data(contentsOf:out.appendingPathComponent("events.json")),
               let value=try? JSONSerialization.jsonObject(with:data) as? [String:Any],
               value["pid"] as? Int==Int(pid), let events=value["events"] as? [[String:Any]] {
                try need(!events.contains(where:{["startup-error","watchdog","terminated"].contains($0["event"] as? String ?? "")}),"host failed during startup")
                ready=current?.isFinishedLaunching==true && events.contains(where:{$0["event"] as? String=="ready"})
                if ready { break }
            }
            pump()
        }
        try need(ready,"host launch readiness timeout")
        try need(Date()<deadline && target?.activate(options:[])==true,"activation refused")
        let activeUntil=min(deadline,Date().addingTimeInterval(5))
        while Date()<activeUntil {
            let current=NSRunningApplication(processIdentifier:pid), front=NSWorkspace.shared.frontmostApplication
            try need(current?.isTerminated==false && current?.executableURL?.standardizedFileURL.path==cfg["executable"] as? String &&
                     current?.bundleIdentifier==cfg["bundleIdentifier"] as? String,"owned app changed during activation")
            try need(front?.processIdentifier==pid || (front?.processIdentifier==pid_t(previous["pid"] as! Int) &&
                     front?.executableURL?.standardizedFileURL.path==previous["executable"] as? String && front?.bundleIdentifier==previous["bundleIdentifier"] as? String),
                     "new foreground owner during activation")
            if current?.isActive==true && front?.processIdentifier==pid { break }
            pump()
        }
        try guardInput()
        let source=URL(fileURLWithPath:cfg["sourceMap"] as! String),saved=URL(fileURLWithPath:cfg["savedMap"] as! String)
        let sourceName=source.deletingPathExtension().lastPathComponent,savedName=saved.deletingPathExtension().lastPathComponent
        _=try mapper(sourceName);try disk("initial")
        try menu("Open Map…",mapName:sourceName);let openCancel=try panel("Open");try press(button(openCancel,"Cancel"),"open-cancel");_=try mapper(sourceName);try disk("open-cancelled")
        try menu("Save Map As…",mapName:sourceName);let saveCancel=try panel("Save");try press(button(saveCancel,"Cancel"),"save-cancel");_=try mapper(sourceName);try disk("save-cancelled")
        try menu("Save Map As…",mapName:sourceName);let chooser=try panel("Save")
        let directory=saved.deletingLastPathComponent();try goTo(directory.path+"/",panel:chooser)
        let whereControl=tree(chooser).filter { text($0,kAXRoleAttribute)=="AXPopUpButton" && labels($0).contains("Where:") }
        try need(whereControl.count==1 && canonical(text(whereControl[0],kAXValueAttribute))==canonical(directory.lastPathComponent),"save directory mismatch")
        let fields=tree(chooser).filter { text($0,kAXRoleAttribute)=="AXTextField" && [sourceName,source.lastPathComponent].map(canonical).contains(canonical(text($0,kAXValueAttribute))) }
        try need(fields.count==1,"unique observed save filename unavailable");try set(fields[0],saved.lastPathComponent);try key(48)
        try need(canonical(text(fields[0],kAXValueAttribute))==canonical(saved.lastPathComponent),"filename changed on commit");snapshot("save-ready")
        try press(button(chooser,"Save"),"save-accept");_=try mapper(savedName);try wait("saved file") { FileManager.default.fileExists(atPath:saved.path) };try disk("saved-live")
        for (url,currentName,stage) in [(source,savedName,"original-reopened"),(saved,sourceName,"saved-reopened")] {
            try menu("Open Map…",mapName:currentName);let open=try panel("Open");try goTo(url.path,panel:open);try press(button(open,"Open"),stage)
            _=try mapper(url.deletingPathExtension().lastPathComponent);try disk(stage)
        }
        let final=try mapper(savedName);try press(button(final,"Cancel"),"mapper-cancel")
        let exitUntil=min(deadline,Date().addingTimeInterval(5));while target?.isTerminated==false && Date()<exitUntil { pump() }
        try need(target?.isTerminated==true,"owned host did not exit after mapper Cancel");report["passed"]=true
    } catch { report["error"]=String(describing:error);snapshot("failure") }
}
exit(report["passed"] as? Bool==true ? 0:1)
