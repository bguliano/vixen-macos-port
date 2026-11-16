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
let args = CommandLine.arguments
if args.count == 3, args[1] == "--restore", let previousPID = Int32(args[2]) {
    let restored = NSRunningApplication(processIdentifier: previousPID)?.activate(options: [.activateIgnoringOtherApps]) ?? false
    print("{\"restored\":\(restored)}")
    exit(restored ? 0 : 1)
}
guard (args.count == 4 || (args.count == 5 && args[4] == "--inspect")), let rawPID = Int32(args[1]) else { fatalError("PID, output folder, expected executable, optional --inspect required") }
let inspectOnly = args.count == 5
let pid = pid_t(rawPID), out = URL(fileURLWithPath: args[2]), expectedExecutable = URL(fileURLWithPath: args[3]).standardizedFileURL
let previous = NSWorkspace.shared.frontmostApplication
let oldPointer = CGEvent(source: nil)?.location
let deadline = Date().addingTimeInterval(inspectOnly ? 25 : 85)
var report: [String: Any] = ["passed": false, "targetPID": Int(pid), "previousPID": previous.map { Int($0.processIdentifier) } ?? -1,
                           "previousBundle": previous?.bundleIdentifier ?? "", "focus": [], "inspectOnly": inspectOnly]
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
        if mouseHeld, let event = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: lastPoint, mouseButton: .left) { event.post(tap: .cghidEventTap) }
        if !inspectOnly {
            if let oldPointer { report["restoredPointer"] = CGWarpMouseCursorPosition(oldPointer) == .success }
            _ = previous?.activate(options: [.activateIgnoringOtherApps])
            for _ in 0..<10 where NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier { pump() }
            report["restoredPrevious"] = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        } else {
            report["foregroundUnchanged"] = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        }
        report["focus"] = focusRecords
        report["drags"] = dragRecords
        report["targetExited"] = target?.isTerminated ?? true
        if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: out.appending(path: "foreground.json"))
        }
    }
    do {
        try require(target?.executableURL?.standardizedFileURL == expectedExecutable, "PID executable mismatch")
        try require(AXIsProcessTrusted(), "Accessibility permission unavailable; no input sent")
        try waitFor("ready")
        let app = AXUIElementCreateApplication(pid)
        AXUIElementSetMessagingTimeout(app, 2)
        guard let window = elements(app, kAXWindowsAttribute).first(where: { text(attribute($0,kAXTitleAttribute)).contains("Layer Drag Verification") }) else {
            throw Failure.check("no private sequence AX window")
        }
        if inspectOnly {
            let (rows, scroll) = try labels(in: window)
            report["rows"] = rows.mapValues { ["role":$0.role,"labels":$0.labels,"bounds":[$0.bounds.minX,$0.bounds.minY,$0.bounds.width,$0.bounds.height]] as [String:Any] }
            report["scroll"] = [scroll.minX,scroll.minY,scroll.width,scroll.height]
            report["passed"] = true
        } else {
        try require(CGPreflightPostEventAccess(), "CG event posting permission unavailable; no input sent")
        try require(target?.activate(options: [.activateIgnoringOtherApps]) == true, "activation failed")
        for _ in 0..<30 where !foreground() { pump() }
        try require(foreground(), "activation did not make owned PID foreground")
        guard let screen = NSScreen.screens.first else { throw Failure.check("no display") }
        var position = CGPoint(x: screen.visibleFrame.minX+20, y: screen.frame.maxY-screen.visibleFrame.maxY+20)
        guard let value = AXValueCreate(.cgPoint, &position) else { throw Failure.check("cannot encode position") }
        func showWindow() throws {
            try require(foreground(), "foreground changed before private window raise")
            try require(AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, value) == .success, "cannot place private sequence onscreen")
            _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
            _ = AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
            pump(0.2)
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
