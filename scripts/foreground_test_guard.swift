// Bounded activation for one explicitly supplied private test process.
// Does not send keys, inspect other windows, grant permissions, or use AppleEvents.
import AppKit
import Foundation
import Darwin

let arguments = CommandLine.arguments
if arguments.count == 2 && arguments[1] == "--status" {
    let app = NSWorkspace.shared.frontmostApplication
    let value: [String: Any] = ["pid": app?.processIdentifier ?? 0,
        "bundle": app?.bundleIdentifier ?? "", "name": app?.localizedName ?? ""]
    FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]))
    exit(0)
}
guard arguments.count == 4, let pid = pid_t(arguments[1]), let duration = Double(arguments[3]), duration > 0, duration <= 180,
      let target = NSRunningApplication(processIdentifier: pid),
      target.executableURL?.lastPathComponent == "Vixen" else { exit(2) }
let output = URL(fileURLWithPath: arguments[2])
let previous = NSWorkspace.shared.frontmostApplication
func isAuthorization(_ app: NSRunningApplication?) -> Bool {
    let name = app?.bundleIdentifier ?? app?.localizedName ?? ""
    return name.localizedCaseInsensitiveContains("securityagent") || name.localizedCaseInsensitiveContains("authorization")
}
var report: [String: Any] = ["pid": pid, "previousPID": previous?.processIdentifier ?? 0,
    "previousBundle": previous?.bundleIdentifier ?? "", "activated": false, "blocked": false]
func save() { try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: output, options: .atomic) }
if isAuthorization(previous) {
    report["blocked"] = true; report["reason"] = "Authorization app was frontmost; no activation attempted"
    save(); exit(3)
}
report["activated"] = target.activate(options: [.activateIgnoringOtherApps])
let activationDeadline = Date().addingTimeInterval(2)
while (NSWorkspace.shared.frontmostApplication?.processIdentifier != pid || !target.isActive), Date() < activationDeadline {
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
}
report["frontmostAfterActivationPID"] = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
report["activeAfterActivation"] = target.isActive
save()
let deadline = Date().addingTimeInterval(duration)
var observations: [[String: Any]] = []
var previousObservation = ""
while !target.isTerminated && Date() < deadline {
    let front = NSWorkspace.shared.frontmostApplication
    let observation = "\(front?.processIdentifier ?? 0):\(target.isActive)"
    if observation != previousObservation {
        observations.append(["utc": ISO8601DateFormatter().string(from: Date()), "frontmostPID": front?.processIdentifier ?? 0, "active": target.isActive])
        previousObservation = observation
        report["observations"] = observations
        save()
    }
    if isAuthorization(front) {
        report["blocked"] = true; report["reason"] = "Authorization app appeared; test stopped without interacting"
        _ = kill(pid, SIGTERM)
        break
    }
    if front?.processIdentifier != pid || !target.isActive {
        report["blocked"] = true; report["reason"] = "Owned test app lost foreground ownership; test stopped without further activation"
        _ = kill(pid, SIGTERM)
        break
    }
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
}
if !target.isTerminated {
    report["timedOut"] = !(report["blocked"] as? Bool ?? false)
    _ = kill(pid, SIGTERM)
    let cleanup = Date().addingTimeInterval(3)
    while !target.isTerminated && Date() < cleanup { RunLoop.current.run(until: Date().addingTimeInterval(0.1)) }
    if !target.isTerminated { _ = kill(pid, SIGKILL) }
}
report["targetExited"] = target.isTerminated
let front = NSWorkspace.shared.frontmostApplication
if !isAuthorization(front), let previous, !previous.isTerminated {
    report["restored"] = previous.activate(options: [.activateIgnoringOtherApps])
    RunLoop.current.run(until: Date().addingTimeInterval(0.3))
    report["restoredPID"] = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
} else { report["restored"] = false }
save()
