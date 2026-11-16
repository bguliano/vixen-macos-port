// Test-only normal-app Open/Save panel driver. No harness, model hooks, clipboard,
// guessed coordinates, or actions on another process. Unrecognized AX UI stops.
import AppKit
import ApplicationServices
import CryptoKit
import Foundation

enum Failure: Error { case check(String) }
func require(_ ok: Bool, _ message: String) throws { if !ok { throw Failure.check(message) } }
func output(_ value: [String: Any]) { if let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), let text = String(data: data, encoding: .utf8) { print(text) } }
let arguments = CommandLine.arguments
if arguments.count == 2 && arguments[1] == "--desktop-state" {
    let front = NSWorkspace.shared.frontmostApplication
    output(["pid": front.map { Int($0.processIdentifier) } ?? -1, "bundle": front?.bundleIdentifier ?? "", "executable": front?.executableURL?.path ?? "",
            "accessibility": AXIsProcessTrusted(), "eventPosting": CGPreflightPostEventAccess(),
            "vixenProcesses": NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier?.hasPrefix("com.vixenlights.") == true }.map { ["pid": Int($0.processIdentifier), "bundle": $0.bundleIdentifier ?? ""] }])
    exit(0)
}
if arguments.count == 3 && arguments[1] == "--restore", let previous = Int32(arguments[2]) {
    let restored = NSRunningApplication(processIdentifier: previous)?.activate(options: []) ?? false
    output(["restored": restored]); exit(restored ? 0 : 1)
}
guard arguments.count == 2 else { fatalError("One run-config JSON path is required, or --desktop-state/--restore PID") }
let config = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: arguments[1]))) as! [String: Any]
let pid = pid_t(config["pid"] as! Int)
var target = NSRunningApplication(processIdentifier: pid)
let expectedExecutable = URL(fileURLWithPath: config["executable"] as! String).standardizedFileURL
let expectedBundle = config["bundleIdentifier"] as! String
let source = URL(fileURLWithPath: config["openFile"] as! String).standardizedFileURL
let saved = URL(fileURLWithPath: config["saveFile"] as! String).standardizedFileURL
let out = URL(fileURLWithPath: config["out"] as! String).standardizedFileURL
let previousPID = pid_t(config["previousPID"] as! Int)
let deadline = Date().addingTimeInterval(75)
let app = AXUIElementCreateApplication(pid)
var report: [String: Any] = ["passed": false, "targetPID": Int(pid), "previousPID": Int(previousPID), "mode": "normal-native-panels", "steps": [], "focus": []]
var steps: [[String: Any]] = [], focus: [[String: Any]] = []
func pump(_ seconds: Double = 0.1) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func attr(_ owner: AXUIElement, _ name: String) -> CFTypeRef? {
    guard Date() < deadline else { return nil }
    AXUIElementSetMessagingTimeout(owner, 0.3)
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(owner, name as CFString, &value) == .success ? value : nil
}
func text(_ owner: AXUIElement, _ name: String) -> String { attr(owner, name) as? String ?? "" }
func ax(_ value: CFTypeRef?) -> AXUIElement? { guard let value, CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }; return (value as! AXUIElement) }
func children(_ owner: AXUIElement, _ name: String = kAXChildrenAttribute) -> [AXUIElement] { attr(owner, name) as? [AXUIElement] ?? [] }
func ownerPID(_ owner: AXUIElement) -> pid_t { var value: pid_t = -1; return AXUIElementGetPid(owner, &value) == .success ? value : -1 }
func owned(_ owner: AXUIElement) -> Bool { ownerPID(owner) == pid }
func foreground() -> Bool { target?.isTerminated == false && target?.isActive == true && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid }
func digest(_ url: URL) throws -> String { SHA256.hash(data: try Data(contentsOf: url)).map { String(format: "%02x", $0) }.joined() }
func wait(_ description: String, seconds: Double = 12, _ ready: () throws -> Bool) throws {
    let until = min(deadline, Date().addingTimeInterval(seconds))
    while Date() < until {
        try require(target?.isTerminated == false, "owned app exited while waiting for \(description)")
        if try ready() { return }; pump()
    }
    throw Failure.check("timeout waiting for \(description)")
}
func nodes(_ root: AXUIElement, depth: Int = 0, budget: inout Int) -> [AXUIElement] {
    guard depth < 20, budget > 0, Date() < deadline else { return [] }
    budget -= 1
    // Unknown/out-of-process AX descendants are recorded but never traversed.
    var result = [root]
    if owned(root) { for child in children(root) { result += nodes(child, depth: depth+1, budget: &budget) } }
    return result
}
func tree(_ root: AXUIElement) -> [AXUIElement] { var budget = 350; return nodes(root, budget: &budget) }
func windows() -> [AXUIElement] { children(app, kAXWindowsAttribute).filter(owned) }
func sheets() -> [AXUIElement] {
    var result: [AXUIElement] = []
    for window in windows() { for node in tree(window) where text(node, kAXRoleAttribute) == kAXSheetRole && !result.contains(where: { CFEqual($0, node) }) { result.append(node) } }
    return result
}
func focusedWindow() -> AXUIElement? { ax(attr(app, kAXFocusedWindowAttribute)) }
func focusedElement() -> AXUIElement? { ax(attr(app, kAXFocusedUIElementAttribute)) }
func recordFocus(_ stage: String) throws {
    let key = focusedWindow(), main = ax(attr(app, kAXMainWindowAttribute))
    focus.append(["stage": stage, "frontmostPID": NSWorkspace.shared.frontmostApplication.map { Int($0.processIdentifier) } ?? -1,
                  "keyTitle": key.map { text($0, kAXTitleAttribute) } ?? "", "keyPID": key.map { Int(ownerPID($0)) } ?? -1,
                  "mainTitle": main.map { text($0, kAXTitleAttribute) } ?? "", "mainPID": main.map { Int(ownerPID($0)) } ?? -1])
    try require(foreground() && key.map(owned) == true && main.map(owned) == true, "focus left the owned app before \(stage)")
}
func key(_ code: CGKeyCode, flags: CGEventFlags = [], stage: String) throws {
    try recordFocus(stage)
    for down in [true, false] {
        try require(foreground() && focusedWindow().map(owned) == true, "focus changed before key event")
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { throw Failure.check("cannot construct key event") }
        event.flags = flags; event.post(tap: .cghidEventTap)
    }
    pump(0.15)
}
func label(_ owner: AXUIElement) -> [String] { [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute].map { text(owner, $0) } }
func buttons(_ root: AXUIElement, named name: String) -> [AXUIElement] { tree(root).filter { owned($0) && text($0, kAXRoleAttribute) == kAXButtonRole && label($0).contains(name) } }
func uniqueButton(_ panel: AXUIElement, _ name: String) throws -> AXUIElement {
    let found = buttons(panel, named: name)
    try require(found.count == 1, "expected exactly one observed \(name) button, found \(found.count)")
    try require(attr(found[0], kAXEnabledAttribute) as? Bool == true, "\(name) button is disabled")
    return found[0]
}
func press(_ item: AXUIElement, stage: String) throws {
    try recordFocus(stage); try require(owned(item), "AX action target belongs to another process")
    try require(AXUIElementPerformAction(item, kAXPressAction as CFString) == .success, "AX press failed at \(stage)")
    pump(0.2)
}
func snapshot(_ name: String) throws {
    let rows = windows().flatMap { tree($0) }.map { node in
        ["pid": Int(ownerPID(node)), "role": text(node, kAXRoleAttribute), "title": text(node, kAXTitleAttribute), "description": text(node, kAXDescriptionAttribute), "value": text(node, kAXValueAttribute), "document": text(node, kAXDocumentAttribute), "edited": attr(node, kAXEditedAttribute) as? Bool as Any? ?? NSNull()] as [String: Any]
    }
    try JSONSerialization.data(withJSONObject: rows, options: [.prettyPrinted, .sortedKeys]).write(to: out.appending(path: name+".ax.json"))
}
func menuSnapshot(_ name: String) throws -> [AXUIElement] {
    guard let menu = ax(attr(app, kAXMenuBarAttribute)), owned(menu) else { throw Failure.check("owned application menu is unavailable") }
    let items = tree(menu)
    let rows: [[String: Any]] = items.map { node in
        ["pid": Int(ownerPID(node)), "role": text(node, kAXRoleAttribute), "title": text(node, kAXTitleAttribute),
         "enabled": attr(node, kAXEnabledAttribute) as? Bool ?? false,
         "commandCharacter": text(node, kAXMenuItemCmdCharAttribute),
         "commandModifiers": attr(node, kAXMenuItemCmdModifiersAttribute) as? Int ?? -1]
    }
    try JSONSerialization.data(withJSONObject: rows, options: [.prettyPrinted, .sortedKeys]).write(to: out.appending(path: name+".menu.json"))
    return items
}
func panel(_ action: String) throws -> AXUIElement {
    var observed: AXUIElement?
    try wait("native \(action) panel") {
        try require(foreground(), "foreground changed while waiting for native panel")
        let candidates = sheets().filter { buttons($0, named: action).count == 1 && buttons($0, named: "Cancel").count == 1 }
        if candidates.count == 1 { observed = candidates[0]; return true }
        return false
    }
    guard let observed else { throw Failure.check("native panel disappeared") }
    try require(owned(observed), "native panel AX owner is not the private app; needs separate audited XPC ownership support")
    return observed
}
func noSheets() throws { try wait("native sheet dismissed") { foreground() && sheets().isEmpty } }
func document(_ url: URL) throws -> AXUIElement {
    var result: AXUIElement?
    try wait("sequence window \(url.lastPathComponent)", seconds: 20) {
        try require(foreground(), "foreground changed waiting for sequence")
        guard sheets().isEmpty, let key = focusedWindow(), owned(key), (text(key, kAXTitleAttribute) == url.deletingPathExtension().lastPathComponent || text(key, kAXTitleAttribute).hasPrefix(url.deletingPathExtension().lastPathComponent + " – ")),
              let represented = URL(string: text(key, kAXDocumentAttribute)), represented.standardizedFileURL.path.precomposedStringWithCanonicalMapping == url.path.precomposedStringWithCanonicalMapping else { return false }
        result = key; return true
    }
    return result!
}
func administration() throws -> AXUIElement {
    var result: AXUIElement?
    try wait("ready Administration window", seconds: 20) {
        guard foreground(), sheets().isEmpty, let key = focusedWindow(), text(key, kAXTitleAttribute).contains("Vixen Administration") else { return false }
        let open = tree(key).filter { owned($0) && label($0).contains("Open Sequence...") && attr($0, kAXEnabledAttribute) as? Bool == true }
        guard open.count == 1 else { return false }; result = key; return true
    }
    return result!
}
func goTo(_ path: String, within nativePanel: AXUIElement, stage: String) throws {
    try key(5, flags: [.maskCommand, .maskShift], stage: stage+"-go-to")
    var field: AXUIElement?
    try wait("owned Go To text input") {
        guard foreground(), let value = focusedElement(), owned(value), [kAXTextFieldRole, kAXComboBoxRole].contains(text(value, kAXRoleAttribute)) else { return false }
        // It must be visibly descended from the existing native file panel.
        guard tree(nativePanel).contains(where: { CFEqual($0, value) }) else { return false }
        field = value; return true
    }
    guard let field else { throw Failure.check("missing Go To field") }
    try require(AXUIElementSetAttributeValue(field, kAXValueAttribute as CFString, path as CFString) == .success, "cannot set owned Go To field")
    try require(text(field, kAXValueAttribute) == path, "Go To text did not match private path")
    try key(36, stage: stage+"-go-to-confirm")
    try wait("Go To dismissed") { foreground() && !(focusedElement().map { CFEqual($0, field) } ?? false) }
}
func open(_ url: URL, stage: String) throws {
    _ = try administration()
    try key(31, flags: .maskCommand, stage: stage+"-open-command")
    let chooser = try panel("Open"); try snapshot(stage+"-panel")
    try goTo(url.path, within: chooser, stage: stage)
    try press(uniqueButton(chooser, "Open"), stage: stage+"-accept")
    _ = try document(url)
    steps.append(["stage": stage, "openedDocument": url.path])
}
func disk(_ stage: String) throws {
    let checksum = try digest(source)
    try require(checksum == config["originalSequenceSha256"] as? String, "original sequence changed at \(stage)")
    var row: [String: Any] = ["stage": stage, "originalSha256": checksum, "savedExists": FileManager.default.fileExists(atPath: saved.path)]
    if row["savedExists"] as? Bool == true { row["savedSha256"] = try digest(saved) }
    steps.append(row)
}

do {
    defer {
        report["steps"] = steps; report["focus"] = focus; report["targetExited"] = target?.isTerminated ?? true
        let front = NSWorkspace.shared.frontmostApplication?.processIdentifier
        if previousPID > 0 && previousPID != pid && (front == pid || front == previousPID) {
            _ = NSRunningApplication(processIdentifier: previousPID)?.activate(options: [])
            for _ in 0..<20 where NSWorkspace.shared.frontmostApplication?.processIdentifier != previousPID { pump(0.05) }
        }
        report["restoredPrevious"] = previousPID > 0 && NSWorkspace.shared.frontmostApplication?.processIdentifier == previousPID
        if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) { try? data.write(to: out.appending(path: "foreground.json")) }
    }
    do {
        // Direct spawn may precede Launch Services registration. Observe the
        // owned identity within a fixed limit before activation or input.
        let registrationDeadline = min(deadline, Date().addingTimeInterval(10))
        while target?.executableURL == nil && Date() < registrationDeadline {
            pump(0.1); target = NSRunningApplication(processIdentifier: pid)
        }
        try require(target?.executableURL?.standardizedFileURL == expectedExecutable && target?.bundleIdentifier == expectedBundle, "private PID/executable/bundle mismatch")
        try require(expectedBundle.hasPrefix("com.vixenlights.verification."), "ordinary bundle identity refused")
        try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(), "AX/CG permissions unavailable; no request or input sent")
        try require(!FileManager.default.fileExists(atPath: saved.path), "save destination already exists")
        try disk("initial")
        let currentFront = NSWorkspace.shared.frontmostApplication
        try require(!(currentFront?.bundleIdentifier ?? "").lowercased().contains("securityagent"), "system authorization is foreground; no activation/input sent")
        try require(target?.activate(options: [.activateIgnoringOtherApps]) == true, "owned app activation request refused")
        try wait("owned app activation", seconds: 10) { foreground() }
        _ = try administration()
        try key(31, flags: .maskCommand, stage: "open-cancel-command")
        let cancelledOpen = try panel("Open"); try snapshot("open-cancel-panel")
        try press(uniqueButton(cancelledOpen, "Cancel"), stage: "open-cancel")
        try noSheets(); _ = try administration(); try disk("open-cancelled")
        try open(source, stage: "source-opened")
        try key(1, flags: [.maskCommand, .maskAlternate], stage: "save-as-cancel-command")
        let cancelledSave = try panel("Save"); try snapshot("save-cancel-panel")
        try press(uniqueButton(cancelledSave, "Cancel"), stage: "save-as-cancel")
        try noSheets(); _ = try document(source); try disk("save-as-cancelled")
        try require(!FileManager.default.fileExists(atPath: saved.path), "cancel created destination")
        try key(1, flags: [.maskCommand, .maskAlternate], stage: "save-as-command")
        let savePanel = try panel("Save")
        let originalNames = [source.lastPathComponent, source.deletingPathExtension().lastPathComponent]
        let fields = tree(savePanel).filter { owned($0) && text($0, kAXRoleAttribute) == kAXTextFieldRole && originalNames.contains(text($0, kAXValueAttribute)) }
        try require(fields.count == 1, "cannot uniquely identify native Save As filename by observed original name")
        // The observed native panel treats a Go To path without a trailing slash
        // as a proposed file: it moved to the parent and replaced the filename
        // with "Saved sequences é.tim". Explicitly navigate to the directory,
        // then verify its observed Where control before changing any name.
        let directory = saved.deletingLastPathComponent()
        try require(FileManager.default.fileExists(atPath: directory.path), "private destination directory is missing")
        try goTo(directory.path + "/", within: savePanel, stage: "save-destination")
        let locations = tree(savePanel).filter { owned($0) && text($0, kAXRoleAttribute) == kAXPopUpButtonRole && label($0).contains("Where:") }
        try require(locations.count == 1 && text(locations[0], kAXValueAttribute) == directory.lastPathComponent, "native Save As directory does not match the private destination")
        try require(tree(savePanel).contains(where: { CFEqual($0, fields[0]) }), "observed Save As filename field was replaced during navigation")
        try recordFocus("save-filename")
        try require(AXUIElementSetAttributeValue(fields[0], kAXValueAttribute as CFString, saved.lastPathComponent as CFString) == .success && text(fields[0], kAXValueAttribute) == saved.lastPathComponent, "native Save As filename did not accept exact Unicode name")
        // Commit the field editor before the native Save button reads its value.
        try key(48, stage: "save-filename-commit")
        try require(text(fields[0], kAXValueAttribute) == saved.lastPathComponent, "native Save As filename changed on commit")
        try snapshot("save-confirm-panel")
        try press(uniqueButton(savePanel, "Save"), stage: "save-confirm")
        _ = try document(saved)
        try wait("saved file publication") { FileManager.default.fileExists(atPath: saved.path) }
        try disk("saved-live")
        try key(13, flags: .maskCommand, stage: "close-saved-window")
        _ = try administration()
        try open(saved, stage: "saved-reopened")
        try disk("reopened-live")
        try snapshot("before-quit")
        let items = try menuSnapshot("before-quit")
        // The owner uses Vorssaint's hold-to-quit safeguard. A tapped Command-Q
        // is intentionally intercepted on this host; test the normal menu action.
        report["commandQQuitTested"] = false
        report["commandQScope"] = "Owner-confirmed Vorssaint hold-to-quit setting; shortcut behavior is outside this file-panel check"
        try require(sheets().isEmpty, "unexpected owned sheet before Quit; preserve it without further actions")
        let quits = items.filter { owned($0) && text($0, kAXRoleAttribute) == kAXMenuItemRole && text($0, kAXTitleAttribute) == "Quit Vixen" && text($0, kAXMenuItemCmdCharAttribute).lowercased() == "q" }
        try require(quits.count == 1 && attr(quits[0], kAXEnabledAttribute) as? Bool == true, "no unique enabled observed Quit Vixen menu item with Q equivalent")
        try recordFocus("normal-quit-menu")
        let status = AXUIElementPerformAction(quits[0], kAXPressAction as CFString)
        report["quitMenuAXStatus"] = status.rawValue
        try require(status == .success, "observed Quit menu AXPress did not report success")
        let quitDeadline = min(deadline, Date().addingTimeInterval(15))
        while target?.isTerminated == false && Date() < quitDeadline { pump() }
        report["quitMenuSucceeded"] = target?.isTerminated == true
        try require(target?.isTerminated == true, "normal Quit did not exit")
        report["passed"] = true
    } catch {
        report["error"] = String(describing: error)
        // Bounded read-only diagnosis, then the runner cleans up only its child.
        try? snapshot("failure")
    }
}
exit(report["passed"] as? Bool == true ? 0 : 1)
