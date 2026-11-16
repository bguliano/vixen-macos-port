// Test-only actual input for one private Vixen PID; no production mutation hooks.
// The companion harness expands only row 0, then emits name-ready/name-returned.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit

enum CheckFailure: Error { case failed(String) }
func require(_ ok: Bool, _ message: String) throws {
    if !ok { throw CheckFailure.failed(message) }
}
guard CommandLine.arguments.count == 4, let rawPID = Int32(CommandLine.arguments[1]), rawPID > 0 else {
    fputs("PID, output folder, expected executable required\n", stderr); exit(2)
}
let pid = pid_t(rawPID)
let out = URL(fileURLWithPath: CommandLine.arguments[2]).standardizedFileURL
let executable = URL(fileURLWithPath: CommandLine.arguments[3]).standardizedFileURL.resolvingSymlinksInPath()
let started = Date(), deadline = started.addingTimeInterval(30), cleanupDeadline = started.addingTimeInterval(35)
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let wantedName = "Layer Draft Retained", initialName = "Zebra Layer", windowName = "Layer Drag Verification"
var target = NSRunningApplication(processIdentifier: pid)
var verifiedBundle: String?, activated = false, mouseHeld = false, heldKey: CGKeyCode?
var ownedAXApp: AXUIElement?, ownedAXWindow: AXUIElement?, ownedFrame: CGRect?
var inputGuardingWindow = false, pointerInterference = false
var postedKeysDown = 0, postedKeysUp = 0, postedButtonsDown = 0, postedButtonsUp = 0
var pointerInputPosted = false, keyboardInputPosted = false, terminationRequested = false
var lastPoint = CGPoint.zero, phase = "identity", workPassed = false
var observations = [[String: Any]](), pointerChecks = [[String: Any]]()
var report: [String: Any] = ["passed": false, "targetPID": Int(pid), "expectedExecutable": executable.path,
    "previousPID": previous.map { Int($0.processIdentifier) } ?? -1,
    "previousBundle": previous?.bundleIdentifier ?? "", "inputDeadlineSeconds": 30, "cleanupDeadlineSeconds": 35,
    "scope": "Actual CG pointer/keyboard input and external AX readback; layer model and saved XML assertions belong to the companion runner."]

func write(_ object: [String: Any], _ name: String) throws {
    try JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys])
        .write(to: out.appending(path: name), options: .atomic)
}
func pump(_ seconds: Double = 0.05) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func frontmost() -> Bool {
    NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && target?.isActive == true
}
func ownedExecutable(_ app: NSRunningApplication?) -> Bool {
    app?.executableURL?.standardizedFileURL.resolvingSymlinksInPath() == executable
        && app?.bundleIdentifier == verifiedBundle && verifiedBundle != nil
}
func value(_ node: AXUIElement, _ name: String) -> CFTypeRef? {
    guard Date() < deadline else { return nil }
    var owner: pid_t = -1, result: CFTypeRef?
    guard AXUIElementGetPid(node, &owner) == .success && owner == pid else { return nil }
    AXUIElementSetMessagingTimeout(node, 0.2)
    return AXUIElementCopyAttributeValue(node, name as CFString, &result) == .success ? result : nil
}
func element(_ object: CFTypeRef?) -> AXUIElement? {
    guard let object, CFGetTypeID(object) == AXUIElementGetTypeID() else { return nil }
    return (object as! AXUIElement)
}
func children(_ node: AXUIElement) -> [AXUIElement] { value(node, kAXChildrenAttribute) as? [AXUIElement] ?? [] }
func bounds(_ node: AXUIElement) throws -> CGRect {
    guard let p = value(node, kAXPositionAttribute), let s = value(node, kAXSizeAttribute),
          CFGetTypeID(p) == AXValueGetTypeID(), CFGetTypeID(s) == AXValueGetTypeID() else {
        throw CheckFailure.failed("AX geometry unavailable")
    }
    var point = CGPoint.zero, size = CGSize.zero
    try require(AXValueGetValue(p as! AXValue, .cgPoint, &point) && AXValueGetValue(s as! AXValue, .cgSize, &size), "Invalid AX geometry")
    try require([point.x, point.y, size.width, size.height].allSatisfy(\.isFinite) && size.width > 0 && size.height > 0, "Empty/nonfinite AX geometry")
    return CGRect(origin: point, size: size)
}
func rectangle(_ r: CGRect) -> [Double] { [r.minX, r.minY, r.width, r.height] }
func inputGuard(_ node: AXUIElement? = nil) throws {
    try require(Date() < deadline && frontmost() && ownedExecutable(target), "Deadline or owned foreground identity lost")
    if inputGuardingWindow, let app = ownedAXApp, let window = ownedAXWindow {
        try require(windowOwnsInput(app, window), "Exact private key/main window lost")
    }
    if let node {
        var owner: pid_t = -1
        try require(AXUIElementGetPid(node, &owner) == .success && owner == pid, "AX target PID differs")
    }
    try require(Date() < deadline, "Input deadline reached during ownership checks")
}
func topWindowOwns(_ point: CGPoint, frame: CGRect? = nil) -> Bool {
    guard let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { return false }
    for window in windows {
        guard (window[kCGWindowLayer as String] as? Int) == 0,
              (window[kCGWindowAlpha as String] as? Double ?? 0) > 0,
              let dictionary = window[kCGWindowBounds as String] as? [String: Any],
              let r = CGRect(dictionaryRepresentation: dictionary as CFDictionary), r.contains(point) else { continue }
        guard (window[kCGWindowOwnerPID as String] as? Int) == Int(pid) else { return false }
        return frame.map { abs(r.minX-$0.minX) < 1 && abs(r.minY-$0.minY) < 1 && abs(r.width-$0.width) < 1 && abs(r.height-$0.height) < 1 } ?? true
    }
    return false
}
func mouse(_ kind: CGEventType, _ point: CGPoint, frame: CGRect) throws {
    try inputGuard()
    try require(topWindowOwns(point, frame: frame), "Exact private window does not own pointer point")
    guard let event = CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left) else {
        throw CheckFailure.failed("Cannot create CG pointer event")
    }
    event.setIntegerValueField(.mouseEventClickState, value: 1)
    try inputGuard()
    try require(topWindowOwns(point, frame: frame), "Exact pointer window changed immediately before event")
    try require(Date() < deadline, "Input deadline reached immediately before pointer post")
    event.post(tap: .cghidEventTap); pointerInputPosted = true; lastPoint = point
    if kind == .leftMouseDown { mouseHeld = true; postedButtonsDown += 1 }
    if kind == .leftMouseUp { mouseHeld = false; postedButtonsUp += 1 }
}
func click(_ point: CGPoint, frame: CGRect) throws {
    if pointerInputPosted, let actual = CGEvent(source: nil)?.location,
       hypot(actual.x-lastPoint.x, actual.y-lastPoint.y) > 1 {
        pointerInterference = true
        throw CheckFailure.failed("Pointer moved independently; further input withheld")
    }
    try mouse(.mouseMoved, point, frame: frame); pump(0.12)
    guard let actual = CGEvent(source: nil)?.location else { throw CheckFailure.failed("Cannot read pointer position") }
    try require(hypot(actual.x-point.x, actual.y-point.y) < 1, "Pointer did not settle")
    pointerChecks.append(["requested": [point.x, point.y], "observed": [actual.x, actual.y]])
    try mouse(.leftMouseDown, point, frame: frame); pump(0.05)
    try mouse(.leftMouseUp, point, frame: frame); pump(0.15)
}
func marker(_ name: String) throws {
    while !FileManager.default.fileExists(atPath: out.appending(path: "ui/\(name).png").path) {
        try require(Date() < deadline && target?.isTerminated == false, "Timeout/target exit awaiting \(name)"); pump()
    }
    pump(0.35) // The harness hides the window immediately after each snapshot.
}
func outerScrollAreas(_ node: AXUIElement, depth: Int = 0, budget: inout Int) -> [AXUIElement] {
    guard depth < 14 && budget > 0 && Date() < deadline else { return [] }
    budget -= 1
    if value(node, kAXRoleAttribute) as? String == kAXScrollAreaRole { return [node] }
    var result = [AXUIElement]()
    for child in children(node) { result += outerScrollAreas(child, depth: depth+1, budget: &budget) }
    return result
}
func fields(_ node: AXUIElement, depth: Int = 0, budget: inout Int) -> [AXUIElement] {
    guard depth < 24 && budget > 0 && Date() < deadline else { return [] }
    budget -= 1
    var result = [AXUIElement]()
    if value(node, kAXRoleAttribute) as? String == kAXTextFieldRole,
       [kAXTitleAttribute, kAXDescriptionAttribute].contains(where: { value(node, $0) as? String == "Layer Name" }) {
        result.append(node)
    }
    for child in children(node) { result += fields(child, depth: depth+1, budget: &budget) }
    return result
}
func nameField(_ window: AXUIElement, frame: CGRect, stage: String) throws -> AXUIElement {
    var budget = 180
    let panels = outerScrollAreas(window, budget: &budget).filter {
        guard let r = try? bounds($0) else { return false }
        return r.minX > frame.midX && r.width > 100 && r.height > 100 && frame.contains(r)
    }
    try require(budget > 0, "AX panel traversal budget exhausted")
    var candidates = [AXUIElement]()
    for panel in panels {
        var fieldBudget = 350
        for field in fields(panel, budget: &fieldBudget) {
            if !candidates.contains(where: { CFEqual($0, field) }) { candidates.append(field) }
        }
        try require(fieldBudget > 0, "AX field traversal budget exhausted")
    }
    observations.append(["stage": stage+"-lookup", "candidateCount": candidates.count,
        "candidates": candidates.map { ["title": value($0, kAXTitleAttribute) as? String ?? "", "description": value($0, kAXDescriptionAttribute) as? String ?? "", "value": value($0, kAXValueAttribute) as? String ?? "", "bounds": (try? bounds($0)).map(rectangle) ?? []] as [String: Any] }])
    try require(candidates.count == 1, "Expected exactly one Layer Name AXTextField in right Layers subtree")
    let field = candidates[0], r = try bounds(field)
    try require(frame.contains(r) && r.width > 30 && r.height >= 10 && r.height < 60, "Layer field is not visibly bounded")
    try require(value(field, kAXEnabledAttribute) as? Bool == true, "Layer name field is disabled")
    return field
}
func windowOwnsInput(_ app: AXUIElement, _ window: AXUIElement) -> Bool {
    guard frontmost(), let key = element(value(app, kAXFocusedWindowAttribute)), let main = element(value(app, kAXMainWindowAttribute)) else { return false }
    return CFEqual(key, window) && CFEqual(main, window)
}
func fieldOwnsInput(_ app: AXUIElement, _ field: AXUIElement) -> Bool {
    guard frontmost(), let focused = element(value(app, kAXFocusedUIElementAttribute)) else { return false }
    return CFEqual(focused, field)
}
func key(_ code: CGKeyCode, app: AXUIElement, window: AXUIElement, field: AXUIElement, flags: CGEventFlags = [], unicode: String? = nil, endsEditing: Bool = false) throws {
    for down in [true, false] {
        try inputGuard(field)
        try require(windowOwnsInput(app, window) && ((!down && endsEditing) || fieldOwnsInput(app, field)), "Key/main or actual field focus changed before key event")
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { throw CheckFailure.failed("Cannot construct CG key") }
        event.flags = flags
        if let unicode {
            let units = Array(unicode.utf16)
            units.withUnsafeBufferPointer { event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: $0.baseAddress!) }
        }
        try inputGuard(field)
        try require(Date() < deadline, "Input deadline reached immediately before key post")
        event.post(tap: .cghidEventTap); keyboardInputPosted = true; heldKey = down ? code : nil
        if down { postedKeysDown += 1 } else { postedKeysUp += 1 }; pump(0.04)
    }
}
func cleanupWindowOwned() -> Bool {
    guard Date() < cleanupDeadline, frontmost(), ownedExecutable(target),
          let app = ownedAXApp, let window = ownedAXWindow else { return false }
    func attribute(_ name: String) -> AXUIElement? {
        guard Date() < cleanupDeadline else { return nil }
        var object: CFTypeRef?
        AXUIElementSetMessagingTimeout(app, 0.1)
        guard AXUIElementCopyAttributeValue(app, name as CFString, &object) == .success else { return nil }
        return element(object)
    }
    guard let key = attribute(kAXFocusedWindowAttribute), let main = attribute(kAXMainWindowAttribute) else { return false }
    return Date() < cleanupDeadline && CFEqual(key, window) && CFEqual(main, window)
}
func releaseOwnedInput() {
    // Release only our tracked inputs, to our exact window, at the current pointer.
    if mouseHeld, cleanupWindowOwned(), let point = CGEvent(source: nil)?.location,
       topWindowOwns(point, frame: ownedFrame),
       let event = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left),
       cleanupWindowOwned(), Date() < cleanupDeadline {
        event.post(tap: .cghidEventTap); mouseHeld = false; postedButtonsUp += 1
    }
    if let code = heldKey, cleanupWindowOwned(),
       let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false), cleanupWindowOwned(), Date() < cleanupDeadline {
        event.flags = []; event.post(tap: .cghidEventTap); heldKey = nil; postedKeysUp += 1
    }
}

try? write(report, "foreground-start.json")
do {
    for _ in 0..<50 where target?.executableURL == nil && Date() < deadline { pump(0.1); target = NSRunningApplication(processIdentifier: pid) }
    try require(target?.executableURL?.standardizedFileURL.resolvingSymlinksInPath() == executable, "Owned PID executable mismatch")
    try require((target?.bundleIdentifier ?? "").hasPrefix("com.vixenlights.verification."), "Non-private Cocoa domain")
    verifiedBundle = target?.bundleIdentifier
    report["verifiedBundleIdentifier"] = verifiedBundle
    report["observedExecutable"] = target?.executableURL?.standardizedFileURL.resolvingSymlinksInPath().path
    let executableData = try Data(contentsOf: executable)
    report["executableSha256"] = SHA256.hash(data: executableData).map { String(format: "%02x", $0) }.joined()
    try require(previous != nil && previous?.processIdentifier != pid && oldPointer != nil, "Previous app/pointer identity unavailable")
    try require(!(previous?.bundleIdentifier ?? "").lowercased().contains("securityagent"), "Authorization window is foreground")
    try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(), "Existing AX/CG permissions unavailable; no prompts requested")
    try require(!CGEventSource.buttonState(.combinedSessionState, button: .left) &&
        CGEvent(source: nil)?.flags.intersection([.maskShift, .maskControl, .maskAlternate, .maskCommand]).isEmpty == true,
        "Physical button/modifier already held; input withheld")
    phase = "ready-marker"; try marker("name-ready")
    let current = NSWorkspace.shared.frontmostApplication
    try require(current?.processIdentifier == previous?.processIdentifier && current?.bundleIdentifier == previous?.bundleIdentifier && current?.executableURL == previous?.executableURL, "Foreground changed before activation")
    phase = "activation"
    try require(Date() < deadline && target?.activate(options: []) == true, "Activation failed or deadline reached")
    activated = true
    for _ in 0..<25 where !frontmost() && Date() < deadline { pump(0.05) }
    try inputGuard()
    let app = AXUIElementCreateApplication(pid)
    AXUIElementSetMessagingTimeout(app, 0.2)
    let windows = (value(app, kAXWindowsAttribute) as? [AXUIElement] ?? []).filter {
        (value($0, kAXTitleAttribute) as? String ?? "").contains(windowName)
    }
    try require(windows.count == 1, "Private sequence AX window title is not unique")
    let window = windows[0]
    ownedAXApp = app; ownedAXWindow = window
    guard let screen = NSScreen.screens.first else { throw CheckFailure.failed("No screen") }
    let visible = CGRect(x: screen.visibleFrame.minX, y: screen.frame.maxY-screen.visibleFrame.maxY,
                         width: screen.visibleFrame.width, height: screen.visibleFrame.height)
    var position = CGPoint(x: visible.minX+20, y: visible.minY+20)
    guard let positionValue = AXValueCreate(.cgPoint, &position) else { throw CheckFailure.failed("Position encoding failed") }
    func show(_ stage: String) throws -> CGRect {
        inputGuardingWindow = false
        try inputGuard(window)
        try require(AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, positionValue) == .success, "Cannot place private sequence onscreen")
        try inputGuard(window)
        let alreadyOwnsInput = windowOwnsInput(app, window)
        observations.append(["stage": stage+"-initial-ownership", "keyMainVerified": alreadyOwnsInput])
        // AX can reject a redundant Raise or return after an action took effect.
        // Only the observed exact focused/main window and native frame permit input.
        if !alreadyOwnsInput {
            try inputGuard(window)
            let raised = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
            observations.append(["stage": stage+"-raise", "actionStatus": raised.rawValue])
            try inputGuard(window)
            if !windowOwnsInput(app, window) {
                try inputGuard(window)
                let madeMain = AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
                observations.append(["stage": stage+"-make-main", "actionStatus": madeMain.rawValue])
            }
        }
        let settleUntil = min(deadline, Date().addingTimeInterval(1.2))
        while Date() < settleUntil {
            try inputGuard(window)
            let keyMain = windowOwnsInput(app, window)
            let frame = try bounds(window)
            let contained = visible.contains(frame)
            let nativeFrame = contained && topWindowOwns(CGPoint(x: frame.midX, y: frame.midY), frame: frame)
            observations.append(["stage": stage+"-settling", "keyMainVerified": keyMain,
                "frame": rectangle(frame), "contained": contained, "nativeFrameVerified": nativeFrame,
                "secondsBeforeInputDeadline": deadline.timeIntervalSinceNow])
            try inputGuard(window)
            if keyMain && nativeFrame && Date() < settleUntil {
                inputGuardingWindow = true; ownedFrame = frame
                try inputGuard(window)
                observations.append(["stage": stage, "frontmostPID": Int(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1),
                    "keyMainVerified": true, "windowTitle": value(window, kAXTitleAttribute) as? String ?? "", "frame": rectangle(frame)])
                return frame
            }
            pump(0.05)
        }
        throw CheckFailure.failed("Private sequence did not acquire exact key/main and native frame within bounded settling")
    }
    phase = "type-draft"
    var frame = try show("name-ready")
    var field = try nameField(window, frame: frame, stage: "initial")
    try require(value(field, kAXValueAttribute) as? String == initialName, "Initial field is not Zebra Layer")
    var r = try bounds(field)
    try click(CGPoint(x: r.midX, y: r.midY), frame: frame)
    try require(fieldOwnsInput(app, field), "Pointer click did not focus the exact layer field")
    try key(0, app: app, window: window, field: field, flags: .maskCommand)
    try key(0, app: app, window: window, field: field, unicode: wantedName)
    for _ in 0..<12 where value(field, kAXValueAttribute) as? String != wantedName && Date() < deadline { pump(0.05) }
    try require(value(field, kAXValueAttribute) as? String == wantedName, "Typed layer draft failed AX readback")
    try inputGuard(field)
    let typed: [String: Any] = ["passed": true, "targetPID": Int(pid), "value": wantedName, "actualFieldFocus": fieldOwnsInput(app, field), "elapsedSeconds": Date().timeIntervalSince(started), "method": "CG pointer, Command-A, Unicode key events; AX readback"]
    try require(typed["actualFieldFocus"] as? Bool == true, "Text focus lost after typing")
    try write(typed, "driver-typed.json"); report["typed"] = typed
    phase = "returned-marker"; try marker("name-returned")
    phase = "verify-retained-draft"
    frame = try show("name-returned")
    field = try nameField(window, frame: frame, stage: "returned")
    let retained = value(field, kAXValueAttribute) as? String ?? ""
    report["returnedValueBeforeInput"] = retained
    try require(retained == wantedName, "Draft changed across tab-away/back before any second-stage input")
    r = try bounds(field)
    try click(CGPoint(x: r.midX, y: r.midY), frame: frame)
    try require(fieldOwnsInput(app, field), "Returned layer field did not acquire actual focus")
    try key(36, app: app, window: window, field: field, endsEditing: true)
    let committed: [String: Any] = ["passed": true, "targetPID": Int(pid), "valueBeforeInput": retained,
        "retyped": false, "returnKeyPosted": true, "elapsedSeconds": Date().timeIntervalSince(started)]
    try write(committed, "driver-committed.json"); report["committed"] = committed
    report["inputCompleted"] = true; phase = "owned-normal-exit"
    while target?.isTerminated == false && Date() < deadline { pump() }
    try require(target?.isTerminated == true, "Private app did not exit within 30 seconds")
    report["normalTerminationObserved"] = true
    workPassed = true
} catch {
    report["error"] = String(describing: error); report["failurePhase"] = phase
    releaseOwnedInput()
    let current = NSRunningApplication(processIdentifier: pid)
    if current?.isTerminated == false, ownedExecutable(current), Date() < cleanupDeadline {
        terminationRequested = kill(pid, SIGTERM) == 0
        for _ in 0..<30 where target?.isTerminated == false && Date() < cleanupDeadline { pump(0.05) }
    }
}
releaseOwnedInput()
func cleanupOwnsForeground() -> Bool {
    let current = NSWorkspace.shared.frontmostApplication
    guard Date() < cleanupDeadline, !(current?.bundleIdentifier ?? "").lowercased().contains("securityagent") else { return false }
    return current?.processIdentifier == pid || (current?.processIdentifier == previous?.processIdentifier && current?.bundleIdentifier == previous?.bundleIdentifier && current?.executableURL == previous?.executableURL)
}
if activated && cleanupOwnsForeground() {
    if pointerInputPosted, let actual = CGEvent(source: nil)?.location,
       hypot(actual.x-lastPoint.x, actual.y-lastPoint.y) > 1 { pointerInterference = true }
    if pointerInputPosted, !pointerInterference, let oldPointer, cleanupOwnsForeground() {
        report["restoredPointer"] = CGWarpMouseCursorPosition(oldPointer) == .success
    }
    if cleanupOwnsForeground() { _ = previous?.activate(options: []) }
    for _ in 0..<20 where Date() < cleanupDeadline && NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier { pump(0.05) }
}
let finalApplication = NSWorkspace.shared.frontmostApplication
let restored = finalApplication?.processIdentifier == previous?.processIdentifier &&
    finalApplication?.bundleIdentifier == previous?.bundleIdentifier && finalApplication?.executableURL == previous?.executableURL
report["restoredPrevious"] = restored
report["targetExited"] = target?.isTerminated ?? true
report["targetTerminationRequested"] = terminationRequested
report["heldButtonAfterCleanup"] = mouseHeld
report["heldKeyAfterCleanup"] = heldKey != nil
report["pointerInputPosted"] = pointerInputPosted
report["keyboardInputPosted"] = keyboardInputPosted
report["activated"] = activated
report["observations"] = observations
report["pointerChecks"] = pointerChecks
report["elapsedSeconds"] = Date().timeIntervalSince(started)
let finalMouse = CGEventSource.buttonState(.combinedSessionState, button: .left)
let finalModifiers = CGEvent(source: nil)?.flags.intersection([.maskShift, .maskControl, .maskAlternate, .maskCommand])
let noInputHeld = !mouseHeld && heldKey == nil && !finalMouse && finalModifiers?.isEmpty == true
    && postedKeysDown == postedKeysUp && postedButtonsDown == postedButtonsUp
report["noInputHeld"] = noInputHeld
report["postedKeysDown"] = postedKeysDown; report["postedKeysUp"] = postedKeysUp
report["postedButtonsDown"] = postedButtonsDown; report["postedButtonsUp"] = postedButtonsUp
report["finalOSLeftButton"] = finalMouse
report["finalOSModifierFlags"] = finalModifiers.map { Int($0.rawValue) } ?? -1
report["pointerInterference"] = pointerInterference
let passed = workPassed && noInputHeld && !pointerInterference && restored && !mouseHeld && heldKey == nil && (target?.isTerminated ?? true)
    && (!pointerInputPosted || report["restoredPointer"] as? Bool == true) && !terminationRequested
report["passed"] = passed
do { try write(report, "foreground.json") } catch { fputs("Cannot write final foreground report\n", stderr); exit(1) }
exit(passed ? 0 : 1)
