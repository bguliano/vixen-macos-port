// External, deadline-limited WindowServer gestures for one frozen private host.
import AppKit
import ApplicationServices
import Foundation
import CryptoKit

enum GestureFailure: Error { case failed(String) }
func require(_ value: Bool, _ message: String) throws { if !value { throw GestureFailure.failed(message) } }
func sha(_ path: String) throws -> String { SHA256.hash(data: try Data(contentsOf: URL(fileURLWithPath: path))).map { String(format: "%02x", $0) }.joined() }
func pump(_ duration: Double) { RunLoop.current.run(until: Date().addingTimeInterval(duration)) }
func identity(_ app: NSRunningApplication?) -> [String: Any] {
    ["pid": Int(app?.processIdentifier ?? -1), "bundleIdentifier": app?.bundleIdentifier ?? "", "executable": app?.executableURL?.standardizedFileURL.path ?? ""]
}

@main struct InlineGestureDriver {
    @MainActor static func main() throws {
        let args = CommandLine.arguments
        if args.count == 3 && args[1] == "--desktop" {
            let result: [String: Any] = ["application": identity(NSWorkspace.shared.frontmostApplication),
                "accessibilityTrusted": AXIsProcessTrusted(), "eventPostingPreflight": CGPreflightPostEventAccess(),
                "inputPosted": false, "permissionRequested": false]
            try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted,.sortedKeys]).write(to: URL(fileURLWithPath: args[2]))
            return
        }
        guard args.count == 2 else { exit(2) }
        let config = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: args[1]))) as! [String: Any]
        let pid = pid_t(config["pid"] as! Int), executable = config["executable"] as! String
        let bundle = config["bundleIdentifier"] as! String
        let output = URL(fileURLWithPath: config["output"] as! String)
        let events = URL(fileURLWithPath: config["events"] as! String)
        let previous = config["previousApplication"] as! [String: Any]
        let began = Date(), deadline = began.addingTimeInterval(40), cleanupDeadline = began.addingTimeInterval(48)
        let oldPointer = CGEvent(source: nil)?.location
        var heldMouse = false, heldShift = false, posted = false, activated = false, pointerInterference = false
        var lastPoint: CGPoint?, frozenWindowNumber: Int?, frozenFrame: [Double]?
        var report: [String: Any] = ["passed":false, "inputDeadlineSeconds":40, "cleanupDeadlineSeconds":48,
            "pid":Int(pid), "executable":executable, "bundleIdentifier":bundle,
            "scope":"Actual CG pointer/Shift input into three optimized production native controls; actual passive tooltip/value observations. No handler calls, swizzles, Core or show."]
        var observations = [[String: Any]](), pointerEvents = [[String: Any]]()
        func read() throws -> [String: Any] {
            let value = try JSONSerialization.jsonObject(with: Data(contentsOf: events)) as! [String: Any]
            try require(value["pid"] as? Int == Int(pid) && value["executable"] as? String == executable && value["bundleIdentifier"] as? String == bundle && value["testMode"] as? Bool == false, "Passive observation identity/test-mode mismatch")
            return value
        }
        func target() -> NSRunningApplication? { NSRunningApplication(processIdentifier: pid) }
        func same(_ app: NSRunningApplication?, _ expected: [String: Any]) -> Bool {
            app?.processIdentifier == pid_t(expected["pid"] as! Int) && app?.bundleIdentifier == expected["bundleIdentifier"] as? String && app?.executableURL?.standardizedFileURL.path == expected["executable"] as? String
        }
        func owned() -> Bool {
            let app = target()
            return app?.isTerminated == false && app?.executableURL?.standardizedFileURL.path == executable && app?.bundleIdentifier == bundle
        }
        func active() -> Bool { owned() && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid && target()?.isActive == true }
        func nodeOwned(_ node: AXUIElement) -> Bool { var owner: pid_t = -1; return AXUIElementGetPid(node, &owner) == .success && owner == pid }
        func attr(_ node: AXUIElement, _ name: String) -> CFTypeRef? {
            guard Date() < deadline, nodeOwned(node) else { return nil }
            AXUIElementSetMessagingTimeout(node, 0.15)
            var value: CFTypeRef?
            return AXUIElementCopyAttributeValue(node, name as CFString, &value) == .success ? value : nil
        }
        let root = AXUIElementCreateApplication(pid)
        func windows() -> [AXUIElement] {
            (attr(root, kAXWindowsAttribute) as? [AXUIElement] ?? []).filter { attr($0,kAXTitleAttribute) as? String == "Private Inline Gesture Verification" }
        }
        func guardInput() throws {
            try require(Date() < deadline && active(), "Input cutoff or private foreground ownership lost")
            let value = try read()
            try require(value["windowNumber"] as? Int == frozenWindowNumber && value["windowFrame"] as? [Double] == frozenFrame,
                        "Owned window moved or was replaced")
            try require(windows().count == 1 && Date() < deadline && active(), "Observed private AX window is not unique or ownership changed")
            if let lastPoint, let actual = CGEvent(source: nil)?.location {
                if hypot(actual.x-lastPoint.x, actual.y-lastPoint.y) >= 1 {
                    pointerInterference = true
                    throw GestureFailure.failed("Pointer moved independently; further input withheld")
                }
            }
        }
        func pointerOwned(_ point: CGPoint) -> Bool {
            guard let rows = CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements], kCGNullWindowID) as? [[String:Any]] else { return false }
            for row in rows {
                guard row[kCGWindowLayer as String] as? Int == 0, (row[kCGWindowAlpha as String] as? Double ?? 0) > 0,
                      let raw = row[kCGWindowBounds as String] as? [String:Any], let rect = CGRect(dictionaryRepresentation: raw as CFDictionary), rect.contains(point) else { continue }
                return row[kCGWindowOwnerPID as String] as? Int == Int(pid) && row[kCGWindowNumber as String] as? Int == frozenWindowNumber
            }
            return false
        }
        func mouse(_ kind: CGEventType, _ point: CGPoint, shift: Bool = false) throws {
            try guardInput()
            try require(pointerOwned(point), "Private WindowServer window does not own target point")
            guard let event = CGEvent(mouseEventSource:nil, mouseType:kind, mouseCursorPosition:point, mouseButton:.left) else { throw GestureFailure.failed("Cannot create pointer event") }
            event.flags = shift ? .maskShift : []
            event.setIntegerValueField(.mouseEventClickState, value:1)
            try require(Date() < deadline && active(), "Ownership changed immediately before pointer post")
            event.post(tap:.cghidEventTap); posted = true; lastPoint = point
            if kind == .leftMouseDown { heldMouse = true }
            if kind == .leftMouseUp { heldMouse = false }
            pointerEvents.append(["type":kind.rawValue,"point":[point.x,point.y],"shift":shift,"elapsedSeconds":Date().timeIntervalSince(began)])
            pump(0.18)
        }
        func shift(_ down: Bool) throws {
            try guardInput()
            guard let event = CGEvent(keyboardEventSource:nil,virtualKey:56,keyDown:down) else { throw GestureFailure.failed("Cannot create Shift event") }
            event.flags = down ? .maskShift : []
            try require(Date() < deadline && active(), "Ownership changed before Shift post")
            event.post(tap:.cghidEventTap); heldShift = down; posted = true; pump(0.12)
        }
        func point(_ kind: String, _ x: Double, _ y: Double) throws -> CGPoint {
            let value = try read(), all = value["geometry"] as! [String:[String:Any]], geometry = all[kind]!
            let start = geometry["topLeftCG"] as! [Double], end = geometry["bottomRightCG"] as! [Double], bounds = geometry["bounds"] as! [Double]
            try require(geometry["flipped"] as? Bool == true && bounds[0] == 0 && bounds[1] == 0 && bounds[2] == 200 && end[0]-start[0] == 200 && end[1]-start[1] == bounds[3] && x >= 0 && x < 200 && y >= 0 && y < bounds[3], "Exported control geometry is not the expected unscaled 200pt native view")
            return CGPoint(x:start[0]+x,y:start[1]+y)
        }
        func observe(_ name: String, _ predicate: ([String:Any]) -> Bool) throws -> [String:Any] {
            let until = min(deadline, Date().addingTimeInterval(1.5))
            var value: [String:Any] = [:]
            repeat {
                try guardInput(); value = try read()
                if predicate(value["current"] as! [String:Any]) { observations.append(["name":name,"observation":value]); return value }
                pump(0.05)
            } while Date() < until
            observations.append(["name":name,"failedObservation":value]); throw GestureFailure.failed("Passive state assertion failed: "+name)
        }
        func tips(_ state: [String:Any]) -> [String] { (state["tooltips"] as? [[String:Any]] ?? []).flatMap { $0["texts"] as? [String] ?? [] } }
        func count(_ state: [String:Any], _ name: String) -> Int { (state["commits"] as! [String:Int])[name]! }
        func points(_ state: [String:Any], _ name: String) -> [[Double]] { (state[name] as! [String:Any])["points"] as! [[Double]] }
        func equal(_ a: Any, _ b: Any) -> Bool { NSDictionary(dictionary:["value":a]).isEqual(to:["value":b]) }
        func visibleTip(_ state: [String:Any], _ text: String) -> Bool {
            let panels = state["tooltips"] as? [[String:Any]] ?? []
            guard tips(state) == [text] && panels.count == 1,
                  let frame = panels[0]["frame"] as? [Double], frame.count == 4 else { return false }
            let rect = NSRect(x:frame[0],y:frame[1],width:frame[2],height:frame[3])
            return panels[0]["alpha"] as? Double == 1 && panels[0]["nonactivating"] as? Bool == true && panels[0]["key"] as? Bool == false && panels[0]["visible"] as? Bool == true && NSScreen.screens.contains { $0.visibleFrame.contains(rect) }
        }
        defer {
            // Release only into the still-owned foreground. Interference never permits
            // a key or button release to be sent to another application.
            if heldMouse, Date() < cleanupDeadline, active(), let p = CGEvent(source:nil)?.location, pointerOwned(p),
               let event = CGEvent(mouseEventSource:nil,mouseType:.leftMouseUp,mouseCursorPosition:p,mouseButton:.left), Date() < cleanupDeadline, active() {
                event.flags = heldShift ? .maskShift : []; event.post(tap:.cghidEventTap); heldMouse = false
            }
            if heldShift, Date() < cleanupDeadline, active(), let event = CGEvent(keyboardEventSource:nil,virtualKey:56,keyDown:false), Date() < cleanupDeadline, active() {
                event.flags=[];event.post(tap:.cghidEventTap);heldShift=false
            }
            if owned() && Date() < cleanupDeadline { _ = target()?.terminate() }
            let softExitDeadline = min(Date().addingTimeInterval(1.5),cleanupDeadline.addingTimeInterval(-2))
            while owned() && Date() < softExitDeadline { pump(0.05) }
            if owned() && Date() < cleanupDeadline { _ = target()?.forceTerminate() }
            let hardExitDeadline = min(Date().addingTimeInterval(0.5),cleanupDeadline.addingTimeInterval(-1))
            while owned() && Date() < hardExitDeadline { pump(0.05) }
            func restorable() -> Bool {
                let current = NSWorkspace.shared.frontmostApplication
                return Date() < cleanupDeadline && (current?.processIdentifier == pid || same(current,previous)) && same(NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int)),previous)
            }
            if activated && restorable() {
                if posted && !pointerInterference, let oldPointer, restorable() { CGWarpMouseCursorPosition(oldPointer) }
                if restorable() { _ = NSRunningApplication(processIdentifier:pid_t(previous["pid"] as! Int))?.activate(options:[]) }
                while !same(NSWorkspace.shared.frontmostApplication,previous) && Date() < cleanupDeadline { pump(0.05) }
            }
            report["restoredPrevious"] = same(NSWorkspace.shared.frontmostApplication,previous)
            report["targetExited"] = !owned()
            report["noInputHeld"] = !heldMouse && !heldShift
            report["heldMouse"] = heldMouse;report["heldShift"] = heldShift;report["inputPosted"] = posted
            report["pointerInterferenceDetected"] = pointerInterference
            report["elapsedSeconds"] = Date().timeIntervalSince(began)
            report["observations"] = observations;report["pointerEvents"] = pointerEvents
            if report["targetExited"] as? Bool != true || report["noInputHeld"] as? Bool != true || report["restoredPrevious"] as? Bool != true { report["passed"] = false }
            if let data = try? JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]) { try? data.write(to:output,options:.atomic) }
        }
        do {
            try require(try sha(executable) == config["executableSha256"] as? String && bundle.hasPrefix("com.vixenlights.verification."), "Private executable pin mismatch")
            try require(AXIsProcessTrusted() && CGPreflightPostEventAccess(), "Existing permissions unavailable; no prompt requested")
            try require(CGEvent(source:nil)?.flags.intersection([.maskShift,.maskControl,.maskAlternate,.maskCommand]).isEmpty == true, "A physical modifier is held; input withheld")
            while (!owned() || !FileManager.default.fileExists(atPath:events.path)) && Date() < min(deadline,began.addingTimeInterval(8)) { pump(0.05) }
            let initial = try read(); frozenWindowNumber = initial["windowNumber"] as? Int; frozenFrame = initial["windowFrame"] as? [Double]
            try require(owned() && frozenWindowNumber != nil && frozenFrame?.count == 4, "Private host not ready")
            try require(same(NSWorkspace.shared.frontmostApplication,previous) || NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Desktop changed before activation")
            try require(!(previous["bundleIdentifier"] as! String).lowercased().contains("securityagent"), "System authorization owns desktop")
            try require(Date() < deadline && target()?.activate(options:[]) == true, "Private activation refused")
            activated = true
            while !active() && Date() < deadline { pump(0.05) }
            let first = try observe("initial-native-values") { state in
                count(state,"level") == 0 && count(state,"point") == 0 && count(state,"gradient") == 0 && points(state,"level") == [[0,25],[100,25]] && points(state,"point") == [[0,0],[50,50],[100,100]] && tips(state).isEmpty
            }
            let originalGradient = (first["current"] as! [String:Any])["gradient"] as! [String:Any]
            try mouse(.mouseMoved, try point("level",50,16)); try shift(true)
            try mouse(.mouseMoved, try point("level",50,16), shift:true)
            _ = try observe("shift-exposes-level") { $0["levelIntrinsicHeight"] as? Double == 48 }
            try mouse(.mouseMoved,try point("level",50,40),shift:true)
            try mouse(.leftMouseDown,try point("level",50,40),shift:true)
            _ = try observe("level-press-tooltip25") { visibleTip($0,"25") && count($0,"level") == 0 && points($0,"level") == [[0,25],[100,25]] }
            for (x,tip) in [(40.0,"20"),(80.0,"40")] {
                try mouse(.leftMouseDragged,try point("level",x,40),shift:true)
                _ = try observe("level-preview-"+tip) { visibleTip($0,tip) && count($0,"level") == 0 && points($0,"level") == [[0,25],[100,25]] }
            }
            try mouse(.leftMouseUp,try point("level",80,40),shift:true); try shift(false)
            _ = try observe("level-one-release-commit") { count($0,"level") == 1 && points($0,"level") == [[0,40],[100,40]] && tips($0).isEmpty }
            try mouse(.mouseMoved,try point("point",100,16));try mouse(.leftMouseDown,try point("point",100,16))
            _ = try observe("point-press-no-tooltip") { tips($0).isEmpty && count($0,"point") == 0 }
            try mouse(.leftMouseDragged,try point("point",103,16))
            _ = try observe("point-first-crossing-arms") { tips($0).isEmpty && count($0,"point") == 0 }
            for (x,y,tip) in [(125.0,12.0,"63, 63"),(140.0,8.0,"70, 75")] {
                try mouse(.leftMouseDragged,try point("point",x,y))
                _ = try observe("point-preview-"+tip) { visibleTip($0,tip) && count($0,"point") == 0 && points($0,"point") == [[0,0],[50,50],[100,100]] }
            }
            try mouse(.leftMouseUp,try point("point",140,8))
            _ = try observe("point-one-release-commit") { count($0,"point") == 1 && points($0,"point") == [[0,0],[70,75],[100,100]] && tips($0).isEmpty }
            try mouse(.mouseMoved,try point("gradient",50,37));try mouse(.leftMouseDown,try point("gradient",50,37))
            _ = try observe("gradient-press-no-tooltip") { tips($0).isEmpty && count($0,"gradient") == 0 }
            try mouse(.leftMouseDragged,try point("gradient",53,37))
            _ = try observe("gradient-first-crossing-arms") { tips($0).isEmpty && count($0,"gradient") == 0 }
            for (x,tip) in [(67.0,"34"),(80.0,"40")] {
                try mouse(.leftMouseDragged,try point("gradient",x,37))
                _ = try observe("gradient-preview-"+tip) { visibleTip($0,tip) && count($0,"gradient") == 0 && equal($0["gradient"]!,originalGradient) }
            }
            try mouse(.leftMouseUp,try point("gradient",80,37))
            var expectedGradient = originalGradient
            var stops = originalGradient["stops"] as! [[String:Any]]
            try require(stops.count == 3 && stops.map { $0["position"] as! Double } == [0.25,0.25,0.75], "Initial paired gradient fixture invalid")
            stops[0]["position"] = 0.4;stops[1]["position"] = 0.4;expectedGradient["stops"] = stops
            _ = try observe("gradient-shared-stops-one-release-full-value") { count($0,"gradient") == 1 && equal($0["gradient"]!,expectedGradient) && tips($0).isEmpty }
            _ = try observe("all-final-values-one-commit") { count($0,"level") == 1 && count($0,"point") == 1 && count($0,"gradient") == 1 && points($0,"level") == [[0,40],[100,40]] && points($0,"point") == [[0,0],[70,75],[100,100]] && equal($0["gradient"]!,expectedGradient) && tips($0).isEmpty }
            try guardInput()
            var matches = [AXUIElement](), visits = 0
            func findFinish(_ node: AXUIElement, depth: Int) {
                guard depth < 8 && visits < 100 && Date() < deadline else { return };visits += 1
                if attr(node,kAXRoleAttribute) as? String == "AXButton" && attr(node,kAXTitleAttribute) as? String == "Finish Check" { matches.append(node) }
                for child in attr(node,kAXChildrenAttribute) as? [AXUIElement] ?? [] { findFinish(child,depth:depth+1) }
            }
            for window in windows() { findFinish(window,depth:0) }
            try require(matches.count == 1 && nodeOwned(matches[0]), "Unique owned Finish Check unavailable")
            try guardInput();let status = AXUIElementPerformAction(matches[0],kAXPressAction as CFString)
            let finishDeadline = min(deadline,Date().addingTimeInterval(1.5))
            while owned() && Date() < finishDeadline { pump(0.05) }
            try require(status == .success || !owned(), "Native finish rejected")
            try require(!owned(), "Host did not exit after finish")
            let final = try read(), names = (final["events"] as! [[String:Any]]).map { $0["event"] as! String }
            try require(final["terminated"] as? Bool == true && !names.contains("watchdog") && names.suffix(2) == ["finished","terminated"], "Normal host finish unproven")
            report["passed"] = true;report["normalFinish"] = true
        } catch { report["error"] = String(describing:error) }
    }
}
