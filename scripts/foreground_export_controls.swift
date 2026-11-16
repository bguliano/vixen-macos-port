// Test-only foreground driver. Input is limited to one owned Vixen PID/window.
import AppKit
import ApplicationServices
import Foundation

enum Failure: Error { case check(String) }
func require(_ condition: Bool, _ message: String) throws { if !condition { throw Failure.check(message) } }
func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    if Date() >= deadline { return nil }
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}
func element(_ value: CFTypeRef?) -> AXUIElement? {
    guard let value, CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }
    return (value as! AXUIElement)
}
func text(_ value: CFTypeRef?) -> String { value as? String ?? "" }
func elements(_ owner: AXUIElement, _ name: String) -> [AXUIElement] { attribute(owner, name) as? [AXUIElement] ?? [] }
func rect(_ owner: AXUIElement) throws -> CGRect {
    guard let p = attribute(owner, kAXPositionAttribute), let s = attribute(owner, kAXSizeAttribute),
          CFGetTypeID(p) == AXValueGetTypeID(), CFGetTypeID(s) == AXValueGetTypeID() else { throw Failure.check("missing AX bounds") }
    var point = CGPoint.zero, size = CGSize.zero
    try require(AXValueGetValue(p as! AXValue, .cgPoint, &point) && AXValueGetValue(s as! AXValue, .cgSize, &size), "invalid AX bounds")
    return CGRect(origin: point, size: size)
}
func table(in view: AXUIElement, depth: Int = 0) -> AXUIElement? {
    guard depth < 20 else { return nil }
    if [kAXTableRole, kAXOutlineRole].contains(text(attribute(view, kAXRoleAttribute))), elements(view, kAXRowsAttribute).count == 2 { return view }
    for child in elements(view, kAXChildrenAttribute) { if let found = table(in: child, depth: depth+1) { return found } }
    return nil
}

let args = CommandLine.arguments
if args.count == 3, args[1] == "--restore", let previousPID = Int32(args[2]) {
    let restored = NSRunningApplication(processIdentifier: previousPID)?.activate(options: [.activateIgnoringOtherApps]) ?? false
    print("{\"restored\":\(restored)}")
    exit(restored ? 0 : 1)
}
guard args.count == 4, let rawPID = Int32(args[1]) else { fatalError("PID, output folder, expected executable required") }
let pid = pid_t(rawPID), out = URL(fileURLWithPath: args[2]), expectedExecutable = URL(fileURLWithPath: args[3]).standardizedFileURL
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let deadline = Date().addingTimeInterval(70)
var report: [String: Any] = ["passed": false, "targetPID": Int(pid), "previousPID": previous.map { Int($0.processIdentifier) } ?? -1,
                           "previousBundle": previous?.bundleIdentifier ?? "", "focus": []]
var focusRecords: [[String: Any]] = []
var mouseHeld = false
var lastPoint = CGPoint.zero
let target = NSRunningApplication(processIdentifier: pid)
if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) {
    try? data.write(to: out.appending(path: "foreground-start.json"))
}
func pump(_ seconds: Double = 0.1) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func waitFor(_ name: String) throws {
    while !FileManager.default.fileExists(atPath: out.appending(path: "ui/\(name).png").path) {
        try require(Date() < deadline && target?.isTerminated == false, "timeout/exit waiting for \(name)")
        pump()
    }
    // The harness moves ordinary windows offscreen after each completed step.
    // Let that post-snapshot work finish before raising the private window.
    pump(0.5)
}
func foreground() -> Bool { NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && target?.isActive == true }
func ownsScreenPoint(_ point: CGPoint) -> Bool {
    guard let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements], kCGNullWindowID) as? [[String:Any]] else { return false }
    for window in windows {
        guard (window[kCGWindowLayer as String] as? Int) == 0,
              (window[kCGWindowAlpha as String] as? Double ?? 0) > 0,
              let dictionary = window[kCGWindowBounds as String] as? [String:Any],
              let bounds = CGRect(dictionaryRepresentation: dictionary as CFDictionary), bounds.contains(point) else { continue }
        return (window[kCGWindowOwnerPID as String] as? Int) == Int(pid)
    }
    return false
}
func mouse(_ kind: CGEventType, _ point: CGPoint) throws {
    try require(foreground(), "foreground changed before mouse event")
    try require(ownsScreenPoint(point), "private window does not own drag point")
    guard let event = CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left) else { throw Failure.check("cannot create mouse event") }
    event.setIntegerValueField(.mouseEventClickState, value: 1)
    event.post(tap: .cghidEventTap)
    lastPoint = point
    if kind == .leftMouseDown { mouseHeld = true }
    if kind == .leftMouseUp { mouseHeld = false }
}
func key(_ code: CGKeyCode, _ flags: CGEventFlags = []) throws {
    try require(foreground(), "foreground changed before key event")
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { throw Failure.check("cannot create key event") }
        event.flags = flags
        event.post(tap: .cghidEventTap)
    }
    pump(0.25)
}
func verifyFocus(_ stage: String, app: AXUIElement, table: AXUIElement) throws {
    let keyWindow = element(attribute(app, kAXFocusedWindowAttribute))
    let mainWindow = element(attribute(app, kAXMainWindowAttribute))
    let focused = element(attribute(app, kAXFocusedUIElementAttribute))
    let record: [String: Any] = ["stage": stage, "frontmostPID": NSWorkspace.shared.frontmostApplication.map { Int($0.processIdentifier) } ?? -1,
        "frontmostBundle": NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "",
        "active": target?.isActive ?? false, "keyWindow": keyWindow.map { text(attribute($0, kAXTitleAttribute)) } ?? "",
        "mainWindow": mainWindow.map { text(attribute($0, kAXTitleAttribute)) } ?? "",
        "focusedRole": focused.map { text(attribute($0, kAXRoleAttribute)) } ?? "",
        "tableFocused": focused.map { CFEqual($0, table) } ?? false,
        "selectedRows": elements(table, kAXSelectedRowsAttribute).count]
    focusRecords.append(record)
    try require(foreground(), "target is not foreground")
    try require(record["keyWindow"] as? String == "Export Wizard" && record["mainWindow"] as? String == "Export Wizard", "wizard is not both key and main")
    try require(record["tableFocused"] as? Bool == true, "controller table is not the focused AX element")
}

do {
    defer {
        if mouseHeld, let event = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: lastPoint, mouseButton: .left) { event.post(tap: .cghidEventTap) }
        if let oldPointer { CGWarpMouseCursorPosition(oldPointer) }
        let restored = previous?.activate(options: [.activateIgnoringOtherApps]) ?? true
        report["restoredPrevious"] = restored
        report["focus"] = focusRecords
        report["targetExited"] = target?.isTerminated ?? true
        if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: out.appending(path: "foreground.json"))
        }
    }
    do {
        try require(target?.executableURL?.standardizedFileURL == expectedExecutable, "PID executable mismatch")
        try require(AXIsProcessTrusted(), "Accessibility permission unavailable; no input sent")
        try require(CGPreflightPostEventAccess(), "CG event posting permission unavailable; no input sent")
        try waitFor("ready")
        try require(target?.activate(options: [.activateIgnoringOtherApps]) == true, "activation failed")
        for _ in 0..<30 where !foreground() { pump() }
        try require(foreground(), "activation did not make owned PID foreground")
        let app = AXUIElementCreateApplication(pid)
        AXUIElementSetMessagingTimeout(app, 2)
        guard let window = elements(app, kAXWindowsAttribute).first(where: { text(attribute($0,kAXTitleAttribute)) == "Export Wizard" }) else { throw Failure.check("no Export Wizard AX window") }
        guard let screen = NSScreen.screens.first else { throw Failure.check("no display") }
        var position = CGPoint(x: screen.visibleFrame.minX+30, y: screen.frame.maxY-screen.visibleFrame.maxY+30)
        guard let value = AXValueCreate(.cgPoint, &position) else { throw Failure.check("cannot encode position") }
        func showWizard() throws {
            try require(foreground(), "foreground changed before private window raise")
            try require(AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, value) == .success, "cannot place private wizard onscreen")
            _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
            _ = AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
            pump(0.2)
        }
        try showWizard()
        guard let controllers = table(in: window) else { throw Failure.check("no two-row controller table") }
        try require(AXUIElementSetAttributeValue(controllers, kAXFocusedAttribute as CFString, kCFBooleanTrue) == .success, "cannot focus controller table")
        pump(0.2)
        try verifyFocus("before-command-a", app: app, table: controllers)
        try require(elements(controllers,kAXSelectedRowsAttribute).count == 1, "expected one initial selected row")
        try key(0, .maskCommand)
        try waitFor("command-a")
        try showWizard()
        try verifyFocus("before-space-off", app: app, table: controllers)
        try require(elements(controllers,kAXSelectedRowsAttribute).count == 2, "Command-A did not select both rows")
        try key(49)
        try waitFor("space-off")
        try showWizard()
        try verifyFocus("before-space-on", app: app, table: controllers)
        try key(49)
        try waitFor("space-on")
        try showWizard()
        try verifyFocus("before-drag", app: app, table: controllers)
        try key(53) // Clear highlighted rows; with selection it must not close the wizard.
        try require(elements(controllers,kAXSelectedRowsAttribute).isEmpty, "Escape did not clear selection")
        try verifyFocus("after-selection-clear", app: app, table: controllers)
        let rows = elements(controllers,kAXRowsAttribute), tableBounds = try rect(controllers)
        try require(rows.count == 2, "controller row count changed")
        let first = try rect(rows[0]), second = try rect(rows[1])
        let start = CGPoint(x: tableBounds.maxX-160, y: second.midY)
        let end = CGPoint(x: start.x, y: first.minY+1)
        try require(tableBounds.contains(start) && tableBounds.contains(end), "drag points outside controller table")
        report["drag"] = ["from": [start.x,start.y], "to": [end.x,end.y]]
        try mouse(.leftMouseDown,start); pump(0.15)
        for i in 1...20 {
            let t = CGFloat(i)/20
            try mouse(.leftMouseDragged,CGPoint(x:start.x+(end.x-start.x)*t,y:start.y+(end.y-start.y)*t)); pump(0.035)
        }
        try mouse(.leftMouseUp,end); pump(0.3)
        try waitFor("reordered")
        report["inputCompleted"] = true
        while target?.isTerminated == false && Date() < deadline { pump() }
        try require(target?.isTerminated == true, "private app did not exit before cleanup deadline")
        report["passed"] = true
    } catch { report["error"] = String(describing:error) }
}
