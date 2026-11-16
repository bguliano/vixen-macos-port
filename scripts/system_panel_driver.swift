// Scoped external Accessibility driver for one owned private panel host.
// Target one observed window; prune large font lists and never traverse all windows per action.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit

enum Check: Error { case failed(String) }
func require(_ value: Bool, _ note: String) throws { if !value { throw Check.failed(note) } }
func pump(_ seconds: Double) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func sha(_ url: URL) throws -> String { SHA256.hash(data: try Data(contentsOf: url)).map { String(format: "%02x", $0) }.joined() }

do {
    let cfgURL = URL(fileURLWithPath: CommandLine.arguments[1])
    let cfg = try JSONSerialization.jsonObject(with: Data(contentsOf: cfgURL)) as! [String: Any]
    let action = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : "dump"
    let started = Date()
    let deadline = min(started.addingTimeInterval(12), Date(timeIntervalSince1970: cfg["actionDeadlineEpoch"] as? Double ?? started.addingTimeInterval(12).timeIntervalSince1970))
    try require(Date() < deadline, "action deadline expired")
    var result: [String: Any] = ["action": action]
    func publish() throws {
        result["elapsedSeconds"] = Date().timeIntervalSince(started)
        try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
            .write(to: URL(fileURLWithPath: cfg["output"] as! String), options: .atomic)
    }
    func describe(_ app: NSRunningApplication?) -> [String: Any] {
        ["pid": Int(app?.processIdentifier ?? -1), "bundleIdentifier": app?.bundleIdentifier ?? "",
         "executable": app?.executableURL?.standardizedFileURL.path ?? ""]
    }
    if action == "desktop" {
        result["application"] = describe(NSWorkspace.shared.frontmostApplication)
        try publish(); exit(0)
    }
    let executable = URL(fileURLWithPath: cfg["executable"] as! String)
    try require(try sha(executable) == cfg["executableSha256"] as? String, "private executable hash changed")
    let pid = pid_t(cfg["pid"] as! Int)
    if action == "restore" {
        let old = cfg["previousApplication"] as! [String: Any]
        let previous = NSRunningApplication(processIdentifier: pid_t(old["pid"] as! Int))
        let current = NSWorkspace.shared.frontmostApplication
        try require(current?.processIdentifier == pid || current?.processIdentifier == previous?.processIdentifier,
                    "another application took foreground; restoration withheld")
        try require(previous?.bundleIdentifier == old["bundleIdentifier"] as? String &&
                    previous?.executableURL?.standardizedFileURL.path == old["executable"] as? String,
                    "previous application identity changed")
        if current?.processIdentifier != previous?.processIdentifier {
            try require(Date() < deadline, "restoration deadline expired")
            try require(previous?.activate(options: []) == true, "previous application activation refused")
        }
        for _ in 0..<20 where Date() < deadline && NSWorkspace.shared.frontmostApplication?.processIdentifier != previous?.processIdentifier { pump(0.05) }
        result["restoredPrevious"] = NSWorkspace.shared.frontmostApplication?.processIdentifier == previous?.processIdentifier
        result["application"] = describe(NSWorkspace.shared.frontmostApplication)
        try publish(); exit(0)
    }
    var app: NSRunningApplication?
    repeat { app = NSRunningApplication(processIdentifier: pid); if app?.executableURL != nil { break }; pump(0.1) } while Date() < deadline
    try require(app?.executableURL?.standardizedFileURL.path == cfg["executable"] as? String &&
                app?.bundleIdentifier == cfg["bundleIdentifier"] as? String &&
                (app?.bundleIdentifier ?? "").hasPrefix("com.vixenlights.verification."), "owned identity mismatch")
    try require(AXIsProcessTrusted(), "AX not granted; no prompt requested")
    let root = AXUIElementCreateApplication(pid)
    func owned(_ node: AXUIElement) -> Bool { var owner: pid_t = -1; return AXUIElementGetPid(node, &owner) == .success && owner == pid }
    func attr(_ node: AXUIElement, _ name: String) -> CFTypeRef? {
        guard owned(node), Date() < deadline else { return nil }
        AXUIElementSetMessagingTimeout(node, 0.2)
        var value: CFTypeRef?
        return AXUIElementCopyAttributeValue(node, name as CFString, &value) == .success ? value : nil
    }
    func str(_ node: AXUIElement, _ name: String) -> String {
        if let s = attr(node, name) as? String { return s }
        if let n = attr(node, name) as? NSNumber { return n.stringValue }
        return ""
    }
    func kids(_ node: AXUIElement, _ name: String = kAXChildrenAttribute) -> [AXUIElement] { attr(node, name) as? [AXUIElement] ?? [] }
    func active() -> Bool { NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && app?.isActive == true }
    func guardInput(_ node: AXUIElement? = nil) throws {
        let current = NSRunningApplication(processIdentifier: pid)
        try require(Date() < deadline && current?.isTerminated == false &&
                    current?.executableURL?.standardizedFileURL.path == cfg["executable"] as? String &&
                    current?.bundleIdentifier == cfg["bundleIdentifier"] as? String && active(),
                    "deadline, owned identity, or foreground changed before input")
        if let node { try require(owned(node) && attr(node, kAXEnabledAttribute) as? Bool == true, "observed control disabled or unavailable") }
        try require(Date() < deadline && active(), "deadline or foreground changed while validating control")
    }
    if action == "activate" {
        try require(!(NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "").lowercased().contains("securityagent"), "system authorization owns foreground")
        let previous = cfg["previousApplication"] as! [String: Any]
        let current = NSWorkspace.shared.frontmostApplication
        try require(current?.processIdentifier == pid ||
                    (current?.processIdentifier == pid_t(previous["pid"] as! Int) &&
                     current?.executableURL?.standardizedFileURL.path == previous["executable"] as? String),
                    "another application took foreground before activation; activation withheld")
        try require(Date() < deadline && app?.activate(options: []) == true, "activation refused or expired")
        while !active() && Date() < deadline { pump(0.1) }
        try require(active(), "owned app did not activate")
        result["foregroundOwned"] = true
        result["windows"] = kids(root, kAXWindowsAttribute).map { str($0, kAXTitleAttribute) }
        try publish(); exit(0)
    }
    try require(active(), "foreground changed; no input sent")
    let selector = try JSONSerialization.jsonObject(with: Data(CommandLine.arguments[3].utf8)) as! [String: String]
    guard let title = selector["window"] else { throw Check.failed("explicit observed window required") }
    struct Row { let node: AXUIElement; let path: [Int] }
    var rows: [Row] = []
    let pruned: Set<String> = ["AXTable", "AXOutline", "AXBrowser", "AXList", "AXScrollArea"]
    func collect(_ node: AXUIElement, path: [Int]) {
        guard owned(node), rows.count < 160, path.count < 9, Date() < deadline else { return }
        rows.append(Row(node: node, path: path))
        if pruned.contains(str(node, kAXRoleAttribute)) { return }
        for (index, child) in kids(node).enumerated() { collect(child, path: path + [index]) }
    }
    func refreshRows() throws {
        try guardInput()
        rows.removeAll()
        let windows = kids(root, kAXWindowsAttribute).filter { str($0, kAXTitleAttribute) == title }
        try require(windows.count <= 1, "target window is not unique: \(windows.count)")
        if let window = windows.first { collect(window, path: []) }
        try require(Date() < deadline && rows.count < 160, "bounded target-window traversal exceeded")
    }
    let names = ["role": kAXRoleAttribute, "title": kAXTitleAttribute, "description": kAXDescriptionAttribute,
                 "subrole": kAXSubroleAttribute, "value": kAXValueAttribute]
    func dump(_ row: Row) -> [String: Any] {
        var actions: CFArray?
        if owned(row.node), Date() < deadline {
            AXUIElementSetMessagingTimeout(row.node, 0.2)
            AXUIElementCopyActionNames(row.node, &actions)
        }
        var value: [String: Any] = ["window": title, "path": row.path, "actions": actions as? [String] ?? []]
        for (key, name) in names { value[key] = str(row.node, name) }
        return value
    }
    func findControl(_ wanted: [String: String]) throws -> Row {
        repeat {
            try refreshRows()
            let matches = rows.filter { row in wanted.allSatisfy { key, value in key == "window" || names[key].map { str(row.node, $0) == value } == true } }
            try require(matches.count <= 1, "selector is not unique: \(matches.count)")
            if let row = matches.first { return row }
            if Date() < deadline { pump(min(0.05, max(0, deadline.timeIntervalSinceNow))) }
        } while Date() < deadline
        throw Check.failed("observed control unavailable before deadline")
    }
    func requireRGBContext() throws {
        let popup = rows.filter { str($0.node, kAXRoleAttribute) == "AXPopUpButton" }
        try require(popup.count == 1 && str(popup[0].node, kAXValueAttribute) == "RGB Sliders" &&
                    rows.allSatisfy { let role = str($0.node, kAXRoleAttribute); return !role.isEmpty && role != "AXMenu" && role != "AXMenuItem" },
                    "RGB picker is not settled with its menu closed")
    }
    if action == "press" || action == "cancel" || action == "set" || action == "set-rgb" {
        var row = try findControl(selector)
        if action == "set-rgb" {
            try require(selector["role"] == "AXTextField" && selector["value"]?.range(of: "^[0-9A-F]{6}$", options: .regularExpression) != nil &&
                        CommandLine.arguments[4].range(of: "^[0-9A-F]{6}$", options: .regularExpression) != nil, "exact RGB hexadecimal selector and value required")
            try requireRGBContext()
        }
        result["observedControl"] = dump(row)
        try guardInput(row.node)
        if action == "press" || action == "cancel" {
            let nativeAction = action == "cancel" ? kAXCancelAction : kAXPressAction
            var supported: CFArray?
            AXUIElementCopyActionNames(row.node, &supported)
            try require((supported as? [String] ?? []).contains(nativeAction), "observed control does not advertise action")
            if action == "cancel" { try require(selector["role"] == "AXMenuItem" && selector["title"] == "RGB Sliders", "only observed RGB menu cancellation allowed") }
            try guardInput(row.node)
            let status = AXUIElementPerformAction(row.node, nativeAction as CFString)
            if status != .success, selector["title"] == "Finish Check" {
                let until = Date().addingTimeInterval(1)
                while app?.isTerminated == false && Date() < min(until, deadline) { pump(0.05) }
            }
            try require(status == .success || (selector["title"] == "Finish Check" && app?.isTerminated == true), "AXPress failed: \(status.rawValue)")
            if app?.isTerminated == false { pump(min(0.1, max(0, deadline.timeIntervalSinceNow))) }
        } else {
            let text = CommandLine.arguments[4]
            try guardInput(row.node)
            try require(AXUIElementSetAttributeValue(row.node, kAXFocusedAttribute as CFString, kCFBooleanTrue) == .success, "cannot focus observed field")
            row = try findControl(selector) // Focus can replace the field; never write through a stale node.
            if action == "set-rgb" { try requireRGBContext() }
            try guardInput(row.node)
            try require(AXUIElementSetAttributeValue(row.node, kAXValueAttribute as CFString, text as CFString) == .success && str(row.node, kAXValueAttribute) == text, "field value not accepted")
            pump(min(0.05, max(0, deadline.timeIntervalSinceNow)))
            var changedSelector = selector; changedSelector["value"] = text
            row = try findControl(changedSelector)
            if action == "set-rgb" { try requireRGBContext() }
            var supported: CFArray?
            AXUIElementCopyActionNames(row.node, &supported)
            try require((supported as? [String] ?? []).contains(kAXConfirmAction), "field has no native confirm action")
            try guardInput(row.node)
            try require(AXUIElementPerformAction(row.node, kAXConfirmAction as CFString) == .success, "native field confirmation failed")
            pump(min(0.1, max(0, deadline.timeIntervalSinceNow)))
            result["confirmedValue"] = str(row.node, kAXValueAttribute)
        }
        result["rowsVisited"] = rows.count
    } else {
        try require(action == "dump", "unknown action")
        try refreshRows()
        result["rows"] = rows.map(dump)
        result["prunedRoles"] = Array(pruned).sorted()
    }
    result["pid"] = Int(pid)
    result["foregroundOwned"] = active()
    result["ownedHostTerminated"] = action == "press" && selector["window"] == "Private System Panel Verification" &&
        selector["role"] == "AXButton" && selector["title"] == "Finish Check" && app?.isTerminated == true
    try publish()
} catch {
    fputs("Scoped panel check stopped: \(error)\n", stderr)
    exit(1)
}
