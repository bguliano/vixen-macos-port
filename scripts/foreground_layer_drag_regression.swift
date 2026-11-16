// Regression copy of the accepted Layer driver; gestures are unchanged.
// Adds exact per-event ownership/deadline checks and tracked cleanup evidence.
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
let args = CommandLine.arguments
guard (args.count == 4 || (args.count == 5 && args[4] == "--inspect")), let rawPID = Int32(args[1]) else { fatalError("PID, output folder, expected executable, optional --inspect required") }
let inspectOnly = args.count == 5
let pid = pid_t(rawPID), out = URL(fileURLWithPath: args[2]), expectedExecutable = URL(fileURLWithPath: args[3]).standardizedFileURL
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let deadline = Date().addingTimeInterval(inspectOnly ? 25 : 88)
let inputDeadline = ProcessInfo.processInfo.systemUptime + (inspectOnly ? 25 : 75)
let cleanupDeadline = ProcessInfo.processInfo.systemUptime + (inspectOnly ? 25 : 88)
var report: [String: Any] = ["passed": false, "targetPID": Int(pid), "previousPID": previous.map { Int($0.processIdentifier) } ?? -1,
                           "previousBundle": previous?.bundleIdentifier ?? "", "focus": [], "inspectOnly": inspectOnly]
var focusRecords: [[String: Any]] = []
var mouseHeld = false
var lastPoint = CGPoint.zero
var pointerPosted = false
var ownershipLost = false
var mouseDownCount = 0
var mouseUpCount = 0
var postedEvents: [[String: Any]] = []
var ownedWindow: AXUIElement?
var ownedWindowNumber: Int?
var frameObservations: [[String:Any]] = []
let appAX = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(appAX, 0.2)
var target = NSRunningApplication(processIdentifier: pid)
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
func previousIdentityMatches() -> Bool {
    guard let previous, let current = NSRunningApplication(processIdentifier:previous.processIdentifier) else { return false }
    return current.executableURL?.standardizedFileURL == previous.executableURL?.standardizedFileURL
        && current.bundleIdentifier == previous.bundleIdentifier && current.isTerminated == false
}
func priorOrTargetForeground() -> Bool {
    let front = NSWorkspace.shared.frontmostApplication
    if (front?.bundleIdentifier ?? "").lowercased().contains("securityagent") { return false }
    if front?.processIdentifier == pid {
        return front?.executableURL?.standardizedFileURL == expectedExecutable
            && front?.bundleIdentifier == target?.bundleIdentifier
    }
    return previousIdentityMatches() && front?.processIdentifier == previous?.processIdentifier
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
            && (window[kCGWindowNumber as String] as? Int) == ownedWindowNumber
    }
    return false
}
func exactWindowOwned() -> Bool {
    guard foreground(), target?.isTerminated == false,
          target?.executableURL?.standardizedFileURL == expectedExecutable,
          let window = ownedWindow,
          let key = element(attribute(appAX, kAXFocusedWindowAttribute)),
          let main = element(attribute(appAX, kAXMainWindowAttribute)),
          CFEqual(window, key), CFEqual(window, main) else { return false }
    return text(attribute(window, kAXTitleAttribute)).contains("Layer Drag Verification")
}
func pointerMatchesLast() -> Bool {
    guard pointerPosted else { return true }
    guard let actual = CGEvent(source: nil)?.location else { return false }
    return hypot(actual.x-lastPoint.x, actual.y-lastPoint.y) <= 3
}
func inputGuard(_ point: CGPoint) throws {
    try require(ProcessInfo.processInfo.systemUptime < inputDeadline, "absolute input deadline reached")
    let owned = exactWindowOwned() && ownsScreenPoint(point) && pointerMatchesLast()
    if !owned { ownershipLost = true }
    try require(owned, "exact foreground/key/main/window or pointer ownership changed")
    try require(ProcessInfo.processInfo.systemUptime < inputDeadline, "input deadline reached during ownership observation")
}
func mouse(_ kind: CGEventType, _ point: CGPoint) throws {
    try inputGuard(point)
    guard let event = CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left) else { throw Failure.check("cannot create mouse event") }
    event.setIntegerValueField(.mouseEventClickState, value: 1)
    try require(ProcessInfo.processInfo.systemUptime < inputDeadline, "input deadline reached before post")
    if kind == .leftMouseDown { try require(!mouseHeld, "duplicate mouse down"); mouseHeld = true; mouseDownCount += 1 }
    if kind == .leftMouseUp { try require(mouseHeld, "unpaired mouse up") }
    event.post(tap: .cghidEventTap)
    lastPoint = point; pointerPosted = true
    if kind == .leftMouseUp { mouseHeld = false; mouseUpCount += 1 }
    postedEvents.append(["kind":kind.rawValue,"x":point.x,"y":point.y,"window":ownedWindowNumber ?? -1])
}
func selectWindowNumber(_ window: AXUIElement) throws -> Bool {
    let bounds = try rect(window)
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements], kCGNullWindowID) as? [[String:Any]] ?? []
    let matches = windows.filter { candidate in
        guard (candidate[kCGWindowOwnerPID as String] as? Int) == Int(pid),
              (candidate[kCGWindowLayer as String] as? Int) == 0,
              let dictionary = candidate[kCGWindowBounds as String] as? [String:Any],
              let seen = CGRect(dictionaryRepresentation: dictionary as CFDictionary) else { return false }
        return abs(seen.minX-bounds.minX) < 1 && abs(seen.minY-bounds.minY) < 1
            && abs(seen.width-bounds.width) < 1 && abs(seen.height-bounds.height) < 1
    }
    let visibleOwned = windows.filter { ($0[kCGWindowOwnerPID as String] as? Int) == Int(pid) }
    frameObservations.append(["elapsedBeforeInputDeadline":inputDeadline-ProcessInfo.processInfo.systemUptime,
        "axBounds":[bounds.minX,bounds.minY,bounds.width,bounds.height],"matchingCount":matches.count,
        "nativeWindows":visibleOwned.map { row -> [String:Any] in
            ["number":row[kCGWindowNumber as String] ?? -1,"layer":row[kCGWindowLayer as String] ?? -1,
             "alpha":row[kCGWindowAlpha as String] ?? -1,"bounds":row[kCGWindowBounds as String] ?? [:],
             "name":row[kCGWindowName as String] ?? ""]
        }])
    try require(matches.count <= 1, "private AX window has ambiguous matching native windows")
    guard matches.count == 1 else { return false }
    let number = matches[0][kCGWindowNumber as String] as? Int
    try require(number != nil && (ownedWindowNumber == nil || ownedWindowNumber == number), "private native window identity changed")
    ownedWindowNumber = number
    return true
}

let names = ["Zebra Layer", "Alpha Layer", "Middle Layer", "Default"]
var dragRecords: [[String: Any]] = []
struct Node {
    let element: AXUIElement
    let role: String
    let labels: [String]
    let bounds: CGRect
}
func outerScrollAreas(in owner: AXUIElement, depth: Int = 0) -> [AXUIElement] {
    guard depth < 10 && Date() < deadline else { return [] }
    if text(attribute(owner, kAXRoleAttribute)) == kAXScrollAreaRole { return [owner] }
    return elements(owner, kAXChildrenAttribute).flatMap { outerScrollAreas(in: $0, depth: depth+1) }
}
func nodes(in owner: AXUIElement, depth: Int = 0, budget: inout Int) -> [Node] {
    guard depth < 28 && budget > 0 && Date() < deadline else { return [] }
    budget -= 1
    var result: [Node] = []
    if let bounds = try? rect(owner), bounds.width > 0, bounds.height > 0 {
        result.append(Node(element: owner, role: text(attribute(owner, kAXRoleAttribute)),
                           labels: [kAXTitleAttribute, kAXValueAttribute, kAXDescriptionAttribute].map { text(attribute(owner, $0)) }, bounds: bounds))
    }
    for child in elements(owner, kAXChildrenAttribute) { result += nodes(in: child, depth: depth+1, budget: &budget) }
    return result
}
func labels(in window: AXUIElement) throws -> ([String: Node], CGRect) {
    let windowBounds = try rect(window)
    // The prepared fixture docks Layers on the right. Skip the unrelated Effects
    // library and timeline AX subtrees so lookup stays bounded during each wait.
    let panels = outerScrollAreas(in: window).compactMap { element -> (AXUIElement, CGRect)? in
        guard let bounds = try? rect(element), bounds.minX > windowBounds.midX,
              bounds.width > 100, bounds.height > 200 else { return nil }
        return (element, bounds)
    }
    guard let panel = panels.max(by: { $0.1.maxX < $1.1.maxX }) else { throw Failure.check("no right-hand AX Layers panel") }
    var budget = 500
    let all = nodes(in: panel.0, budget: &budget)
    var rows: [String: Node] = [:]
    for name in names {
        // Prefer the label's bounds over a parent disclosure group. The label
        // fills the row; its centre avoids the disclosure triangle at the left.
        let matches = all.filter { $0.labels.contains(name) && $0.bounds.width > 45 && $0.bounds.height >= 10 && $0.bounds.height < 50 }
        guard let row = matches.min(by: { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }) else {
            report["axCandidates"] = all.filter { $0.labels.contains(where: { !$0.isEmpty }) }.map {
                ["role": $0.role, "labels": $0.labels, "bounds": [$0.bounds.minX,$0.bounds.minY,$0.bounds.width,$0.bounds.height]] as [String: Any]
            }
            throw Failure.check("no bounded AX layer label: \(name)")
        }
        rows[name] = row
    }
    guard let scroll = all.filter({ $0.role == kAXScrollAreaRole })
        .filter({ candidate in rows.values.allSatisfy { candidate.bounds.contains(CGPoint(x: $0.bounds.midX, y: $0.bounds.midY)) } })
        .min(by: { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }) else {
        throw Failure.check("no shared AX layer scroll area")
    }
    return (rows, scroll.bounds)
}
func verifyFocus(_ stage: String, app: AXUIElement) throws {
    let keyWindow = element(attribute(app, kAXFocusedWindowAttribute))
    let mainWindow = element(attribute(app, kAXMainWindowAttribute))
    let focused = element(attribute(app, kAXFocusedUIElementAttribute))
    let record: [String: Any] = ["stage": stage, "frontmostPID": NSWorkspace.shared.frontmostApplication.map { Int($0.processIdentifier) } ?? -1,
        "active": target?.isActive ?? false,
        "keyWindow": keyWindow.map { text(attribute($0, kAXTitleAttribute)) } ?? "",
        "mainWindow": mainWindow.map { text(attribute($0, kAXTitleAttribute)) } ?? "",
        "focusedRole": focused.map { text(attribute($0, kAXRoleAttribute)) } ?? ""]
    focusRecords.append(record)
    try require(foreground(), "target is not foreground")
    try require((record["keyWindow"] as? String)?.contains("Layer Drag Verification") == true &&
                (record["mainWindow"] as? String)?.contains("Layer Drag Verification") == true, "private sequence is not both key and main")
}

do {
    defer {
        if mouseHeld {
            if ProcessInfo.processInfo.systemUptime < cleanupDeadline,
               !ownershipLost, exactWindowOwned(), pointerMatchesLast(),
               let current = CGEvent(source:nil)?.location, ownsScreenPoint(current),
               let event = CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:current,mouseButton:.left) {
                event.post(tap:.cghidEventTap)
                mouseHeld = false; mouseUpCount += 1
                report["cleanupReleasePosted"] = true
                pump(0.05)
            } else {
                report["cleanupReleasePosted"] = false
                report["cleanupError"] = "Owned release refused after pointer/window ownership loss or deadline"
            }
        }
        if !inspectOnly {
            if !ownershipLost && !mouseHeld && ProcessInfo.processInfo.systemUptime < cleanupDeadline && priorOrTargetForeground() {
                if pointerPosted, let oldPointer, pointerMatchesLast() {
                    report["restoredPointer"] = CGWarpMouseCursorPosition(oldPointer) == .success
                }
                let currentPID = NSWorkspace.shared.frontmostApplication?.processIdentifier
                if previousIdentityMatches() && (currentPID == pid || currentPID == previous?.processIdentifier) {
                    _ = previous?.activate(options:[.activateIgnoringOtherApps])
                    for _ in 0..<10 where NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier && ProcessInfo.processInfo.systemUptime < cleanupDeadline { pump() }
                }
            }
            report["restoredPrevious"] = previousIdentityMatches() && NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        } else {
            report["foregroundUnchanged"] = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        }
        let pressedKeys = (0..<128).filter { CGEventSource.keyState(.combinedSessionState, key: CGKeyCode($0)) }
        let pressedButtons = [CGMouseButton.left, .right, .center].filter { CGEventSource.buttonState(.combinedSessionState, button: $0) }
        report["heldButtonAfterCleanup"] = mouseHeld
        report["heldKeyAfterCleanup"] = false // This helper contains no keyboard event posting.
        report["observedPressedKeyCodesAfterCleanup"] = pressedKeys
        report["observedPressedButtonsAfterCleanup"] = pressedButtons.map(\.rawValue)
        report["mouseDownCount"] = mouseDownCount
        report["mouseUpCount"] = mouseUpCount
        report["postedEvents"] = postedEvents
        report["inputOwnershipMaintained"] = !ownershipLost
        report["observedModifierFlagsAfterCleanup"] = CGEventSource.flagsState(.combinedSessionState).rawValue
        let modifiers = CGEventSource.flagsState(.combinedSessionState).intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand])
        report["noInputHeld"] = !mouseHeld && mouseDownCount == mouseUpCount && pressedKeys.isEmpty && pressedButtons.isEmpty && modifiers.isEmpty
        report["cleanupCompleted"] = !mouseHeld && !ownershipLost && ProcessInfo.processInfo.systemUptime < cleanupDeadline
        if (report["noInputHeld"] as? Bool) != true || (report["cleanupCompleted"] as? Bool) != true || (!inspectOnly && (report["restoredPrevious"] as? Bool) != true) { report["passed"] = false }
        if mouseHeld || !pressedKeys.isEmpty || !pressedButtons.isEmpty {
            report["passed"] = false
            report["cleanupError"] = "Input remained down after Layer driver cleanup"
        }
        report["nativeFrameObservations"] = frameObservations
        report["focus"] = focusRecords
        report["drags"] = dragRecords
        report["targetExited"] = target?.isTerminated ?? true
        if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: out.appending(path: "foreground.json"))
        }
    }
    do {
        let registeredUntil = min(inputDeadline, ProcessInfo.processInfo.systemUptime+8)
        while ProcessInfo.processInfo.systemUptime < registeredUntil {
            target = NSRunningApplication(processIdentifier:pid)
            try require(priorOrTargetForeground(), "foreground changed while waiting for owned app registration")
            if target?.executableURL?.standardizedFileURL == expectedExecutable && target?.isFinishedLaunching == true { break }
            try require(target?.isTerminated != true, "owned app exited during registration")
            pump(0.05)
        }
        try require(target?.executableURL?.standardizedFileURL == expectedExecutable && target?.isFinishedLaunching == true, "PID executable/launch mismatch")
        try require(AXIsProcessTrusted(), "Accessibility permission unavailable; no input sent")
        try waitFor("ready")
        let app = appAX
        guard let window = elements(app, kAXWindowsAttribute).first(where: { text(attribute($0,kAXTitleAttribute)).contains("Layer Drag Verification") }) else {
            throw Failure.check("no private sequence AX window")
        }
        ownedWindow = window
        if inspectOnly {
            let (rows, scroll) = try labels(in: window)
            report["rows"] = rows.mapValues { ["role":$0.role,"labels":$0.labels,"bounds":[$0.bounds.minX,$0.bounds.minY,$0.bounds.width,$0.bounds.height]] as [String:Any] }
            report["scroll"] = [scroll.minX,scroll.minY,scroll.width,scroll.height]
            report["passed"] = true
        } else {
        try require(CGPreflightPostEventAccess(), "CG event posting permission unavailable; no input sent")
        try require((0..<128).allSatisfy { !CGEventSource.keyState(.combinedSessionState,key:CGKeyCode($0)) }
            && [CGMouseButton.left,.right,.center].allSatisfy { !CGEventSource.buttonState(.combinedSessionState,button:$0) }, "input already held before driver start")
        let activationAllowed = priorOrTargetForeground() && ProcessInfo.processInfo.systemUptime < inputDeadline
        if !activationAllowed { ownershipLost = true }
        try require(activationAllowed, "foreground/deadline changed before activation")
        try require(target?.activate(options: [.activateIgnoringOtherApps]) == true, "activation failed")
        for _ in 0..<30 where !foreground() { pump() }
        try require(foreground(), "activation did not make owned PID foreground")
        guard let screen = NSScreen.screens.first else { throw Failure.check("no display") }
        var position = CGPoint(x: screen.visibleFrame.minX+20, y: screen.frame.maxY-screen.visibleFrame.maxY+20)
        guard let value = AXValueCreate(.cgPoint, &position) else { throw Failure.check("cannot encode position") }
        func showWindow() throws {
            try require(ProcessInfo.processInfo.systemUptime < inputDeadline && foreground(), "deadline/foreground changed before private window raise")
            try require(AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, value) == .success, "cannot place private sequence onscreen")
            _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
            _ = AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
            pump(0.2)
            let settleUntil = min(inputDeadline, ProcessInfo.processInfo.systemUptime+1.2)
            var settled = false
            while ProcessInfo.processInfo.systemUptime < settleUntil {
                try require(foreground(), "foreground changed while WindowServer frame settles")
                if try selectWindowNumber(window) { settled = true; break }
                pump(0.05)
            }
            try require(settled, "private AX window lacks unique matching native window after bounded settling")
        }
        // Each marker follows the previous gesture's read-only model snapshot.
        // All events below are real OS pointer events; no layer mutation hook is used.
        let cases: [(String, String, String?, String, [String])] = [
            ("ready", "Zebra Layer", "Zebra Layer", "same-row", names),
            ("same-row", "Default", "Zebra Layer", "default-source", names),
            ("default-source", "Alpha Layer", nil, "outside", names),
            ("outside", "Middle Layer", "Zebra Layer", "move-up", ["Middle Layer","Zebra Layer","Alpha Layer","Default"]),
            ("move-up", "Zebra Layer", "Default", "above-default", ["Middle Layer","Alpha Layer","Zebra Layer","Default"]),
        ]
        for (marker, from, to, stage, expected) in cases {
            if marker != "ready" { try waitFor(marker) }
            try showWindow()
            try verifyFocus(stage, app: app)
            let (rows, scroll) = try labels(in: window)
            let source = rows[from]!.bounds
            let start = CGPoint(x: source.midX, y: source.midY)
            let end: CGPoint
            if let to {
                let targetBounds = rows[to]!.bounds
                end = CGPoint(x: to == from ? min(source.maxX-3, start.x+30) : targetBounds.midX, y: targetBounds.midY)
            } else {
                end = CGPoint(x: start.x, y: rows.values.map { $0.bounds.maxY }.max()!+50)
                try require(scroll.contains(end), "outside-row destination is outside layer scroll area")
                var hit: AXUIElement?
                try require(AXUIElementCopyElementAtPosition(app, Float(end.x), Float(end.y), &hit) == .success, "cannot inspect blank drop target")
                let role = hit.map { text(attribute($0, kAXRoleAttribute)) } ?? ""
                try require([kAXScrollAreaRole,kAXGroupRole,kAXUnknownRole].contains(role), "outside-row destination is a control: \(role)")
            }
            try require(scroll.contains(start) && scroll.contains(end), "drag points outside layer panel")
            try mouse(.mouseMoved,start); pump(0.05)
            try mouse(.leftMouseDown,start); pump(0.2)
            for i in 1...20 {
                let t = CGFloat(i)/20
                try mouse(.leftMouseDragged,CGPoint(x:start.x+(end.x-start.x)*t,y:start.y+(end.y-start.y)*t)); pump(0.035)
            }
            pump(0.2)
            try mouse(.leftMouseUp,end); pump(0.45)
            let (after, _) = try labels(in: window)
            let observed = after.sorted { $0.value.bounds.midY < $1.value.bounds.midY }.map(\.key)
            dragRecords.append(["stage":stage,"from":from,"to":to ?? "blank","start":[start.x,start.y],"end":[end.x,end.y],
                "observedOrder":observed,"expectedOrder":expected])
            try require(observed == expected, "\(stage) AX row order mismatch: \(observed)")
        }
        try waitFor("above-default")
        report["inputCompleted"] = true
        while target?.isTerminated == false && Date() < deadline { pump() }
        try require(target?.isTerminated == true, "private app did not exit before cleanup deadline")
        report["passed"] = true
        }
    } catch { report["error"] = String(describing:error) }
}
