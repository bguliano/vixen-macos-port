// Private full-app Mapper drag verification. Real OS pointer/key input only.
// Setup/readback is in the companion harness; no native drop callback is invoked here.
import AppKit
import ApplicationServices
import CryptoKit
import Foundation

enum Stop: Error { case failed(String) }
func need(_ condition: Bool, _ message: String) throws {
  if !condition { throw Stop.failed(message) }
}
func identity(_ app: NSRunningApplication?) -> [String: Any] {
  [
    "pid": Int(app?.processIdentifier ?? -1), "bundleIdentifier": app?.bundleIdentifier ?? "",
    "executable": app?.executableURL?.standardizedFileURL.path ?? "",
  ]
}
func write(_ value: [String: Any], _ path: URL) throws {
  try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys]).write(
    to: path, options: .atomic)
}
// These pure predicates are shared by the real guarded paths and bounded
// failure-control checks; they perform no desktop, AX, or input operations.
func releaseAllowed(
  time: Double, limit: Double, owner: Bool, pointer: Bool, ownPhysicalOnly: Bool, point: Bool
) -> Bool {
  time < limit && owner && pointer && ownPhysicalOnly && point
}
func restoreAllowed(
  time: Double, limit: Double, previous: Bool, foreground: Bool, ownershipLost: Bool, pointer: Bool,
  clear: Bool
) -> Bool {
  time < limit && previous && foreground && !ownershipLost && pointer && clear
}
func markerBudget(age: Double) -> (input: Double, release: Double)? {
  guard age.isFinite, age >= 0, age < 4 else { return nil }
  return (5 - age, 6.5 - age)
}
func clockSamplesConsistent(wallElapsed: Double, monotonicElapsed: Double) -> Bool {
  wallElapsed.isFinite && monotonicElapsed.isFinite && wallElapsed >= 0 && monotonicElapsed >= 0
    && abs(wallElapsed - monotonicElapsed) < 0.25
}
if CommandLine.arguments.count == 2 && CommandLine.arguments[1] == "--guard-checks" {
  let cases: [(String, Bool)] = [
    (
      "normal marker gets bounded input reserve",
      markerBudget(age: 1)?.input == 4 && markerBudget(age: 1)?.release == 5.5
    ),
    ("future marker refused", markerBudget(age: -1) == nil),
    ("expired marker refused", markerBudget(age: 4) == nil),
    (
      "nonfinite marker refused",
      markerBudget(age: Double.nan) == nil && markerBudget(age: Double.infinity) == nil
    ),
    (
      "stable wall/monotonic samples accepted",
      clockSamplesConsistent(wallElapsed: 0.01, monotonicElapsed: 0.011)
    ),
    (
      "backward wall-clock jump refused",
      !clockSamplesConsistent(wallElapsed: -1, monotonicElapsed: 0.01)
    ),
    (
      "forward wall-clock jump refused",
      !clockSamplesConsistent(wallElapsed: 2, monotonicElapsed: 0.01)
    ),
    (
      "release within separate reserve after input cutoff",
      releaseAllowed(
        time: 71, limit: 80, owner: true, pointer: true, ownPhysicalOnly: true, point: true)
    ),
    (
      "release refused at cleanup deadline",
      !releaseAllowed(
        time: 80, limit: 80, owner: true, pointer: true, ownPhysicalOnly: true, point: true)
    ),
    (
      "release refused to new window owner",
      !releaseAllowed(
        time: 71, limit: 80, owner: false, pointer: true, ownPhysicalOnly: true, point: true)
    ),
    (
      "release refused after pointer interference",
      !releaseAllowed(
        time: 71, limit: 80, owner: true, pointer: false, ownPhysicalOnly: true, point: true)
    ),
    (
      "release refused with foreign physical input",
      !releaseAllowed(
        time: 71, limit: 80, owner: true, pointer: true, ownPhysicalOnly: false, point: true)
    ),
    (
      "release refused outside frozen CG window",
      !releaseAllowed(
        time: 71, limit: 80, owner: true, pointer: true, ownPhysicalOnly: true, point: false)
    ),
    (
      "restore allowed only to exact idle previous owner",
      restoreAllowed(
        time: 79, limit: 80, previous: true, foreground: true, ownershipLost: false, pointer: true,
        clear: true)
    ),
    (
      "restore refuses held physical input",
      !restoreAllowed(
        time: 79, limit: 80, previous: true, foreground: true, ownershipLost: false, pointer: true,
        clear: false)
    ),
    (
      "restore refuses replaced previous identity",
      !restoreAllowed(
        time: 79, limit: 80, previous: false, foreground: true, ownershipLost: false, pointer: true,
        clear: true)
    ),
    (
      "restore refuses a third foreground app",
      !restoreAllowed(
        time: 79, limit: 80, previous: true, foreground: false, ownershipLost: false, pointer: true,
        clear: true)
    ),
    (
      "restore refuses moved pointer",
      !restoreAllowed(
        time: 79, limit: 80, previous: true, foreground: true, ownershipLost: false, pointer: false,
        clear: true)
    ),
  ]
  let passed = cases.allSatisfy { $0.1 }
  let data = try JSONSerialization.data(
    withJSONObject: [
      "passed": passed, "nativeOperations": 0,
      "checks": cases.map { ["name": $0.0, "passed": $0.1] as [String: Any] },
    ], options: [.prettyPrinted, .sortedKeys])
  print(String(data: data, encoding: .utf8)!)
  exit(passed ? 0 : 1)
}
if CommandLine.arguments.count == 2 && CommandLine.arguments[1] == "--desktop" {
  let point = CGEvent(source: nil)?.location
  let value: [String: Any] = [
    "application": identity(NSWorkspace.shared.frontmostApplication),
    "pointer": point.map { [$0.x, $0.y] } ?? [], "accessibility": AXIsProcessTrusted(),
    "eventPosting": CGPreflightPostEventAccess(), "noPhysicalInputHeld": physicallyClear(),
  ]
  let data = try JSONSerialization.data(withJSONObject: value, options: .sortedKeys)
  print(String(data: data, encoding: .utf8)!)
  exit(0)
}
guard CommandLine.arguments.count == 2 else { exit(2) }
let config =
  try JSONSerialization.jsonObject(
    with: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))) as! [String: Any]
let pid = pid_t(config["pid"] as! Int)
let out = URL(fileURLWithPath: config["out"] as! String)
let executable = config["executable"] as! String
let bundle = config["bundleIdentifier"] as! String
let previous = config["previousApplication"] as! [String: Any]
let oldPointer = config["pointer"] as! [Double]
let deadline = ProcessInfo.processInfo.systemUptime + 90
let cleanupDeadline = ProcessInfo.processInfo.systemUptime + 105
let root = AXUIElementCreateApplication(pid)
var target = NSRunningApplication(processIdentifier: pid)
var heldMouse = false
var heldKey: CGKeyCode?
var downs = 0
var ups = 0
var keyDowns = 0
var keyUps = 0
var lastPoint = oldPointer.count == 2 ? CGPoint(x: oldPointer[0], y: oldPointer[1]) : CGPoint.zero
var pointerPosted = false
var ownershipLost = false
var activated = false
var ownedMapper: AXUIElement?
var ownedParent: AXUIElement?
var ownedWindowNumber: Int?
var report: [String: Any] = [
  "passed": false, "pid": Int(pid), "expectedExecutable": executable,
  "scope":
    "Real CG drag/drop and Delete events; distinct Mapper table/outline routes. Existing hooks supply setup/tree/save and read state. No normal-Quit or file-panel navigation claim.",
]
var observations = [[String: Any]]()
var events = [[String: Any]]()
var stageDeadline = deadline
var stageCleanupDeadline = cleanupDeadline
var cleanupReading = false
func readDeadline() -> Double {
  cleanupReading
    ? min(cleanupDeadline, (heldMouse || heldKey != nil) ? stageCleanupDeadline : cleanupDeadline)
    : min(deadline, stageDeadline)
}
func now() -> Double { ProcessInfo.processInfo.systemUptime }
func aborted() -> Bool {
  FileManager.default.fileExists(atPath: out.appendingPathComponent("abort-request.json").path)
}
func pump(_ seconds: Double = 0.05) {
  RunLoop.current.run(until: Date().addingTimeInterval(seconds))
}
func owned(_ node: AXUIElement) -> Bool {
  var owner: pid_t = -1
  return AXUIElementGetPid(node, &owner) == .success && owner == pid
}
func attr(_ node: AXUIElement, _ key: String) -> CFTypeRef? {
  guard now() < readDeadline(), owned(node) else { return nil }
  AXUIElementSetMessagingTimeout(node, 0.2)
  var value: CFTypeRef?
  return AXUIElementCopyAttributeValue(node, key as CFString, &value) == .success ? value : nil
}
func ax(_ value: CFTypeRef?) -> AXUIElement? {
  guard let value, CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }
  return (value as! AXUIElement)
}
func str(_ node: AXUIElement, _ key: String) -> String { attr(node, key) as? String ?? "" }
func children(_ node: AXUIElement, _ key: String = kAXChildrenAttribute) -> [AXUIElement] {
  attr(node, key) as? [AXUIElement] ?? []
}
func bounds(_ node: AXUIElement) throws -> CGRect {
  guard let p = attr(node, kAXPositionAttribute), let s = attr(node, kAXSizeAttribute),
    CFGetTypeID(p) == AXValueGetTypeID(), CFGetTypeID(s) == AXValueGetTypeID()
  else { throw Stop.failed("AX bounds unavailable") }
  var point = CGPoint.zero
  var size = CGSize.zero
  try need(
    AXValueGetValue(p as! AXValue, .cgPoint, &point)
      && AXValueGetValue(s as! AXValue, .cgSize, &size), "Invalid AX bounds")
  try need(
    [point.x, point.y, size.width, size.height].allSatisfy(\.isFinite) && size.width > 0
      && size.height > 0, "Empty/nonfinite bounds")
  return CGRect(origin: point, size: size)
}
func frame(_ rect: CGRect) -> [Double] { [rect.minX, rect.minY, rect.width, rect.height] }
func tree(_ node: AXUIElement) -> [AXUIElement] {
  var found = [AXUIElement]()
  func visit(_ item: AXUIElement, _ depth: Int) {
    guard now() < readDeadline(), depth < 18, found.count < 600, owned(item) else { return }
    found.append(item)
    for child in children(item) { visit(child, depth + 1) }
  }
  visit(node, 0)
  return found
}
// Read a complete bounded AX subtree. Communication failure is not evidence
// that a sheet disappeared; only an affirmative parent traversal can prove it.
func completeTree(_ node: AXUIElement) throws -> [AXUIElement] {
  var found = [AXUIElement]()
  func visit(_ item: AXUIElement, _ depth: Int) throws {
    try need(
      now() < readDeadline() && depth < 18 && found.count < 600 && owned(item),
      "Dismissal observation exhausted its bounds")
    found.append(item)
    AXUIElementSetMessagingTimeout(item, 0.2)
    var raw: CFTypeRef?
    let status = AXUIElementCopyAttributeValue(item, kAXChildrenAttribute as CFString, &raw)
    if status == .attributeUnsupported || status == .noValue { return }
    try need(status == .success && raw is [AXUIElement], "Incomplete AX dismissal observation")
    for child in raw as! [AXUIElement] { try visit(child, depth + 1) }
  }
  try visit(node, 0)
  return found
}
func savedHash() throws -> String {
  let url = URL(fileURLWithPath: config["savedMap"] as! String)
  let size = try FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber
  try need(
    size != nil && size!.intValue > 0 && size!.intValue < 1_000_000,
    "Private saved map missing or unexpectedly large")
  return SHA256.hash(data: try Data(contentsOf: url)).map { String(format: "%02x", $0) }.joined()
}
func observeDismissal(_ mapper: AXUIElement) throws {
  let end = min(stageDeadline, now() + 2)
  var dismissed = false
  repeat {
    try need(
      foreground() && pointerMatches() && physicallyClear() && !aborted(),
      "Ownership changed while observing Cancel dismissal")
    if let parent = ownedParent, !same(parent, mapper),
      let windows = attr(root, kAXWindowsAttribute) as? [AXUIElement],
      windows.contains(where: { same($0, parent) }),
      let nodes = try? completeTree(parent), !nodes.contains(where: { same($0, mapper) })
    {
      dismissed = true
      break
    }
    pump()
  } while now() < end
  try need(
    dismissed && now() < stageDeadline && foreground() && pointerMatches()
      && physicallyClear() && !aborted(), "Cancel did not observably dismiss the exact Mapper sheet"
  )
  report["cancelDismissalObserved"] = true
}
func targetIdentity() -> Bool {
  let app = NSRunningApplication(processIdentifier: pid)
  return app?.isTerminated == false && app?.executableURL?.standardizedFileURL.path == executable
    && app?.bundleIdentifier == bundle
}
func previousIdentity() -> Bool {
  guard let id = previous["pid"] as? Int,
    let app = NSRunningApplication(processIdentifier: pid_t(id))
  else { return false }
  return !app.isTerminated && app.bundleIdentifier == previous["bundleIdentifier"] as? String
    && app.executableURL?.standardizedFileURL.path == previous["executable"] as? String
}
func foreground() -> Bool {
  targetIdentity() && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid
    && NSRunningApplication(processIdentifier: pid)?.isActive == true
}
func same(_ a: AXUIElement?, _ b: AXUIElement?) -> Bool {
  guard let a, let b else { return false }
  return CFEqual(a, b)
}
func exactFocus() -> Bool {
  guard foreground(), let mapper = ownedMapper, let parent = ownedParent else { return false }
  guard
    str(mapper, kAXTitleAttribute).hasPrefix("Element Mapper - ")
      || str(mapper, kAXRoleAttribute) == "AXSheet"
  else { return false }
  let focused = ax(attr(root, kAXFocusedWindowAttribute))
  let main = ax(attr(root, kAXMainWindowAttribute))
  // Native sheets may expose their parent as focused/main, or expose the sheet
  // itself. In either case the exact observed sheet must remain that parent's child.
  var attached = same(mapper, parent)
  var ancestor: AXUIElement? = mapper
  for _ in 0..<8 where !attached {
    ancestor = ancestor.flatMap { ax(attr($0, kAXParentAttribute)) }
    attached = same(ancestor, parent)
    if ancestor == nil { break }
  }
  return (same(focused, mapper) || same(focused, parent))
    && (same(main, mapper) || same(main, parent)) && attached
}
func pointOwned(_ point: CGPoint) -> Bool {
  guard
    let list = CGWindowListCopyWindowInfo(
      [.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]],
    let number = ownedWindowNumber
  else { return false }
  for entry in list {
    guard (entry[kCGWindowLayer as String] as? Int) == 0,
      (entry[kCGWindowAlpha as String] as? Double ?? 0) > 0,
      let raw = entry[kCGWindowBounds as String] as? [String: Any],
      let box = CGRect(dictionaryRepresentation: raw as CFDictionary), box.contains(point)
    else { continue }
    return entry[kCGWindowOwnerPID as String] as? Int == Int(pid)
      && entry[kCGWindowNumber as String] as? Int == number
  }
  return false
}
func pointerMatches() -> Bool {
  guard let p = CGEvent(source: nil)?.location else { return false }
  return hypot(p.x - lastPoint.x, p.y - lastPoint.y) <= 3
}
func physicalInput() -> [String: Any] {
  let buttons: [(String, CGMouseButton)] = [
    ("left", .left), ("right", .right), ("center", .center),
  ]
  let heldButtons = buttons.filter {
    CGEventSource.buttonState(.combinedSessionState, button: $0.1)
  }.map(\.0)
  let keys = (0..<128).filter { CGEventSource.keyState(.combinedSessionState, key: CGKeyCode($0)) }
  let modifiers = CGEventSource.flagsState(.combinedSessionState).intersection([
    .maskShift, .maskControl, .maskAlternate, .maskCommand, .maskSecondaryFn,
  ])
  return ["buttons": heldButtons, "keys": keys, "modifiers": modifiers.rawValue]
}
func physicallyClear() -> Bool {
  let state = physicalInput()
  return (state["buttons"] as! [String]).isEmpty && (state["keys"] as! [Int]).isEmpty
    && state["modifiers"] as! UInt64 == 0
}
func physicalOnlyOurHeld() -> Bool {
  let state = physicalInput()
  let buttons = state["buttons"] as! [String]
  let keys = state["keys"] as! [Int]
  return buttons.allSatisfy { heldMouse && $0 == "left" }
    && keys.allSatisfy { heldKey != nil && $0 == Int(heldKey!) }
    && state["modifiers"] as! UInt64 == 0
}
func guardInput(_ point: CGPoint? = nil) throws {
  let limit = min(deadline, stageDeadline)
  try need(now() < limit && !aborted(), "Input/stage deadline or cooperative abort")
  let first = exactFocus() && pointerMatches() && (point.map(pointOwned) ?? true)
  let finalOwned =
    foreground() && pointerMatches() && physicalOnlyOurHeld() && (point.map(pointOwned) ?? true)
  // Expired AX reads do not prove ownership loss. Cleanup gets its separate
  // reserve and must freshly establish the exact owner before releasing.
  if (!first && now() < limit) || !finalOwned { ownershipLost = true }
  try need(now() < limit && !aborted(), "Input/stage deadline after ownership checks")
  try need(first && finalOwned, "Exact Mapper foreground/window/pointer ownership lost")
}
func mouse(_ type: CGEventType, _ point: CGPoint) throws {
  try guardInput(point)
  guard
    let event = CGEvent(
      mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)
  else { throw Stop.failed("CG mouse event unavailable") }
  event.setIntegerValueField(.mouseEventClickState, value: 1)
  try guardInput(point)
  if type == .leftMouseDown {
    try need(!heldMouse, "Duplicate down")
    heldMouse = true
    downs += 1
  }
  if type == .leftMouseUp { try need(heldMouse, "Unpaired up") }
  event.post(tap: .cghidEventTap)
  lastPoint = point
  pointerPosted = true
  if type == .leftMouseUp {
    heldMouse = false
    ups += 1
  }
  events.append(["type": type.rawValue, "point": [point.x, point.y]])
}
func key(_ code: CGKeyCode) throws {
  for down in [true, false] {
    try guardInput()
    guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else {
      throw Stop.failed("CG key event unavailable")
    }
    try guardInput()
    if down {
      heldKey = code
      keyDowns += 1
    }
    event.post(tap: .cghidEventTap)
    if !down {
      heldKey = nil
      keyUps += 1
    }
  }
  pump(0.15)
}
func waitMarker(_ name: String) throws {
  let marker = out.appendingPathComponent("ui/\(name).png")
  while !FileManager.default.fileExists(atPath: marker.path) {
    try need(
      now() < deadline && !aborted() && targetIdentity(),
      "Marker timeout/abort/owned app exit: \(name)")
    pump()
  }
  let wallBefore = Date().timeIntervalSince1970
  let monoBefore = now()
  let attributes = try FileManager.default.attributesOfItem(atPath: marker.path)
  guard let modified = attributes[.modificationDate] as? Date else {
    throw Stop.failed("No marker timestamp")
  }
  // Harness waits seven seconds after completing the snapshot. The PNG's
  // modification time is a conservative earlier bound, leaving a two-second
  // owned-release reserve before the next harness action can start.
  let wallAfter = Date().timeIntervalSince1970
  let monoAfter = now()
  try need(
    clockSamplesConsistent(
      wallElapsed: wallAfter - wallBefore, monotonicElapsed: monoAfter - monoBefore),
    "Wall clock changed during marker sampling")
  let age = wallAfter - modified.timeIntervalSince1970
  guard let budget = markerBudget(age: age) else {
    throw Stop.failed("Future, invalid, or expired marker timestamp")
  }
  stageDeadline = min(deadline, monoAfter + budget.input)
  stageCleanupDeadline = min(cleanupDeadline, monoAfter + budget.release)
  try need(now() + 1 < stageDeadline, "Marker too old for bounded interaction: \(name)")
  observations.append([
    "stage": name, "markerAgeSeconds": age, "inputSecondsRemaining": stageDeadline - now(),
    "releaseSecondsRemaining": stageCleanupDeadline - now(),
  ])
  pump(0.15)
}
func discover() throws -> AXUIElement {
  let windows = children(root, kAXWindowsAttribute)
  var matches = [(AXUIElement, AXUIElement)]()
  for parent in windows {
    for node in tree(parent) {
      guard ["AXWindow", "AXSheet"].contains(str(node, kAXRoleAttribute)) else { continue }
      let nodes = tree(node)
      if nodes.contains(where: { str($0, kAXDescriptionAttribute) == "Destination elements" })
        && nodes.contains(where: {
          ["Element mappings", "Source elements"].contains(str($0, kAXDescriptionAttribute))
        })
      {
        // Prefer the innermost sheet over its outer administration window.
        matches.append((node, parent))
      }
    }
  }
  if matches.contains(where: { str($0.0, kAXRoleAttribute) == "AXSheet" }) {
    matches.removeAll { str($0.0, kAXRoleAttribute) != "AXSheet" }
  }
  var unique = [(AXUIElement, AXUIElement)]()
  for pair in matches where !unique.contains(where: { CFEqual($0.0, pair.0) }) {
    unique.append(pair)
  }
  try need(unique.count == 1, "No unique observed Mapper window/sheet")
  ownedMapper = unique[0].0
  ownedParent = unique[0].1
  let box = try bounds(unique[0].0)
  let center = CGPoint(x: box.midX, y: box.midY)
  guard
    let list = CGWindowListCopyWindowInfo(
      [.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
  else { throw Stop.failed("No CG windows") }
  ownedWindowNumber = nil
  for e in list {
    guard (e[kCGWindowLayer as String] as? Int) == 0,
      (e[kCGWindowAlpha as String] as? Double ?? 0) > 0,
      let raw = e[kCGWindowBounds as String] as? [String: Any],
      let r = CGRect(dictionaryRepresentation: raw as CFDictionary), r.contains(center)
    else { continue }
    try need(
      e[kCGWindowOwnerPID as String] as? Int == Int(pid), "Foreign window over Mapper center")
    ownedWindowNumber = e[kCGWindowNumber as String] as? Int
    observations.append([
      "stage": "window", "axFrame": frame(box), "cgFrame": frame(r),
      "cgWindow": ownedWindowNumber ?? -1, "role": str(unique[0].0, kAXRoleAttribute),
      "title": str(unique[0].0, kAXTitleAttribute),
    ])
    break
  }
  try need(
    ownedWindowNumber != nil && exactFocus(), "Exact observed Mapper focus/window unavailable")
  return unique[0].0
}
func container(_ name: String, _ mapper: AXUIElement) throws -> AXUIElement {
  let matches = tree(mapper).filter { node in
    ["AXTable", "AXOutline"].contains(str(node, kAXRoleAttribute))
      && [kAXDescriptionAttribute, kAXTitleAttribute, kAXHelpAttribute].contains {
        str(node, $0) == name
      }
  }
  try need(matches.count == 1, "Unique AX container unavailable: \(name)")
  return matches[0]
}
func label(_ name: String, in owner: AXUIElement) throws -> CGRect {
  let matches = tree(owner).compactMap { node -> CGRect? in
    guard
      [kAXValueAttribute, kAXTitleAttribute, kAXDescriptionAttribute].contains(where: {
        str(node, $0) == name || str(node, $0).hasPrefix(name + " → ")
      }), let r = try? bounds(node), r.height < 50, r.height >= 10, r.width > 30
    else { return nil }
    return r
  }
  guard let found = matches.min(by: { $0.width * $0.height < $1.width * $1.height }) else {
    throw Stop.failed("No observed row label: \(name)")
  }
  return found
}
func drag(_ from: CGPoint, _ to: CGPoint) throws {
  try mouse(.mouseMoved, from)
  pump(0.05)
  try mouse(.leftMouseDown, from)
  pump(0.18)
  for i in 1...18 {
    let f = CGFloat(i) / 18
    try mouse(
      .leftMouseDragged, CGPoint(x: from.x + (to.x - from.x) * f, y: from.y + (to.y - from.y) * f))
    pump(0.025)
  }
  pump(0.15)
  try mouse(.leftMouseUp, to)
  pump(0.25)
}
func snapshot(_ stage: String, _ mapper: AXUIElement) {
  let rows = tree(mapper).map { node -> [String: Any] in
    [
      "role": str(node, kAXRoleAttribute), "title": str(node, kAXTitleAttribute),
      "description": str(node, kAXDescriptionAttribute), "value": str(node, kAXValueAttribute),
      "frame": (try? bounds(node)).map(frame) ?? [],
    ]
  }
  try? write(["stage": stage, "rows": rows], out.appendingPathComponent(stage + ".ax.json"))
}
func cleanup() {
  cleanupReading = true
  var errors = [String]()
  func releaseSafe(_ point: CGPoint? = nil) -> Bool {
    guard now() < readDeadline(), !ownershipLost, exactFocus(), pointerMatches(),
      physicalOnlyOurHeld()
    else { return false }
    let allowed = releaseAllowed(
      time: now(), limit: readDeadline(), owner: foreground(), pointer: pointerMatches(),
      ownPhysicalOnly: physicalOnlyOurHeld(), point: point.map(pointOwned) ?? true)
    return allowed && now() < readDeadline()
  }
  if heldMouse, releaseSafe(lastPoint),
    let e = CGEvent(
      mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: lastPoint,
      mouseButton: .left), releaseSafe(lastPoint), now() < readDeadline()
  {
    e.post(tap: .cghidEventTap)
    heldMouse = false
    ups += 1
    pump(0.05)
  }
  if let code = heldKey, releaseSafe(),
    let e = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false), releaseSafe(),
    now() < readDeadline()
  {
    e.post(tap: .cghidEventTap)
    heldKey = nil
    keyUps += 1
    pump(0.05)
  }
  if heldMouse || heldKey != nil {
    errors.append("Input remained held; ownership guards forbid redirecting release")
  }
  let observedNoHeld = physicallyClear()
  report["physicalInputAfter"] = physicalInput()
  func canRestore() -> Bool {
    let front = NSWorkspace.shared.frontmostApplication?.processIdentifier
    let allowed = restoreAllowed(
      time: now(), limit: cleanupDeadline, previous: previousIdentity(),
      foreground: front == pid || front == pid_t(previous["pid"] as! Int),
      ownershipLost: ownershipLost, pointer: pointerMatches(), clear: physicallyClear())
    return allowed && now() < cleanupDeadline
  }
  if observedNoHeld && canRestore() && oldPointer.count == 2 && oldPointer.allSatisfy(\.isFinite) {
    if canRestore() && now() < cleanupDeadline {
      report["restoredPointer"] =
        CGWarpMouseCursorPosition(CGPoint(x: oldPointer[0], y: oldPointer[1])) == .success
      lastPoint = CGPoint(x: oldPointer[0], y: oldPointer[1])
      pump(0.05)
    }
    if canRestore() && NSWorkspace.shared.frontmostApplication?.processIdentifier == pid
      && now() < cleanupDeadline
    {
      _ = NSRunningApplication(processIdentifier: pid_t(previous["pid"] as! Int))?.activate(
        options: [.activateIgnoringOtherApps])
    }
    while NSWorkspace.shared.frontmostApplication?.processIdentifier
      != pid_t(previous["pid"] as! Int) && now() < cleanupDeadline
    { pump() }
    report["restoredPrevious"] =
      previousIdentity()
      && NSWorkspace.shared.frontmostApplication?.processIdentifier
        == pid_t(previous["pid"] as! Int)
  } else {
    report["restoredPrevious"] = false
    errors.append("Previous identity/foreground/pointer/held input changed; no restore input")
  }
  report["cleanupErrors"] = errors
  report["noInputHeld"] =
    !heldMouse && heldKey == nil && downs == ups && keyDowns == keyUps && observedNoHeld
  report["targetExited"] = NSRunningApplication(processIdentifier: pid)?.isTerminated ?? true
  report["mouseDowns"] = downs
  report["mouseUps"] = ups
  report["keyDowns"] = keyDowns
  report["keyUps"] = keyUps
  report["events"] = events
  report["observations"] = observations
  report["activated"] = activated
  if !errors.isEmpty || report["restoredPrevious"] as? Bool != true
    || report["restoredPointer"] as? Bool != true || report["noInputHeld"] as? Bool != true
  {
    report["passed"] = false
  }
  try? write(report, out.appendingPathComponent("foreground.json"))
}

do {
  defer { cleanup() }
  do {
    try need(
      AXIsProcessTrusted() && CGPreflightPostEventAccess(), "Native input permissions unavailable")
    try need(previousIdentity(), "Original foreground identity changed before startup")
    try need(
      oldPointer.count == 2 && oldPointer.allSatisfy(\.isFinite), "Original pointer unavailable")
    report["physicalInputBefore"] = physicalInput()
    try need(physicallyClear(), "Existing held key/button/modifier; no input allowed")
    // Match the accepted cross-window helper: the new process may exist before
    // Launch Services exposes NSRunningApplication. No input or activation here.
    let registrationStarted = now()
    let registrationEnd = min(deadline, now() + 8)
    while now() < registrationEnd {
      let front = NSWorkspace.shared.frontmostApplication
      let priorOrTarget =
        (front?.processIdentifier == pid && targetIdentity())
        || (previousIdentity() && front?.processIdentifier == pid_t(previous["pid"] as! Int))
      try need(
        priorOrTarget && !(front?.bundleIdentifier ?? "").lowercased().contains("securityagent")
          && pointerMatches() && physicallyClear() && !aborted(),
        "Foreground/input changed during process registration")
      target = NSRunningApplication(processIdentifier: pid)
      if targetIdentity() && target?.isFinishedLaunching == true { break }
      pump()
    }
    try need(
      targetIdentity() && target?.isFinishedLaunching == true
        && bundle.hasPrefix("com.vixenlights.verification."),
      "Bounded private process registration/launch failed")
    report["registrationWaitSeconds"] = now() - registrationStarted
    try waitMarker("table-a")
    target = NSRunningApplication(processIdentifier: pid)
    try need(targetIdentity(), "Target identity invalid")
    let front = NSWorkspace.shared.frontmostApplication?.processIdentifier
    try need(
      front == pid || front == pid_t(previous["pid"] as! Int),
      "Unrelated foreground owner before activation")
    if front != pid {
      try need(
        target?.activate(options: [.activateIgnoringOtherApps]) == true,
        "Private app activation failed")
      activated = true
    }
    for _ in 0..<20 where !foreground() { pump() }
    try need(foreground(), "Private app did not activate")
    let stages = config["stages"] as! [[String: String]]
    for stage in stages {
      let name = stage["marker"]!
      if name != "table-a" { try waitMarker(name) }
      let mapper = try discover()
      snapshot("before-" + name, mapper)
      let source = try container("Destination elements", mapper)
      let destination = try container(stage["container"]!, mapper)
      if stage["operation"] == "clear" {
        let row = try label(stage["target"]!, in: destination)
        let point = CGPoint(x: row.midX, y: row.midY)
        try mouse(.mouseMoved, point)
        pump(0.05)
        try mouse(.leftMouseDown, point)
        pump(0.05)
        try mouse(.leftMouseUp, point)
        pump(0.1)
        try key(51)
      } else {
        if stage["operation"] == "cancel" {
          let beforeLabel = stage["expectedBefore"]!
          try need(
            tree(destination).contains { node in
              [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].contains {
                str(node, $0) == beforeLabel
              }
            }, "Final Cancel drag must change an existing distinct mapping")
          report["savedHashBeforeUnsavedChange"] = try savedHash()
          report["unsavedMappingBefore"] = beforeLabel
        }
        let row = try label(stage["source"]!, in: source)
        let start = CGPoint(x: row.midX, y: row.midY)
        let end: CGPoint
        if stage["target"] == "blank" {
          let box = try bounds(destination)
          let all = tree(destination).compactMap { try? bounds($0) }.filter {
            $0.height < 50 && $0.height >= 10 && $0.width > 30
          }
          let bottom = all.map(\.maxY).max() ?? box.minY
          end = CGPoint(x: box.midX, y: max(bottom + 25, box.maxY - 24))
          try need(end.y < box.maxY - 8, "No unoccupied table/outline drop area")
        } else {
          let targetRow = try label(stage["target"]!, in: destination)
          end = CGPoint(x: targetRow.midX, y: targetRow.midY)
        }
        try drag(start, end)
        observations.append([
          "stage": name, "from": stage["source"]!, "to": stage["target"]!,
          "start": [start.x, start.y], "end": [end.x, end.y],
        ])
        if stage["operation"] == "cancel" {
          let changedLabel = stage["target"]! + " → " + stage["source"]!
          try need(
            tree(destination).contains { node in
              [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].contains {
                str(node, $0) == changedLabel
              }
            }, "Unsaved final mapping did not reach its observed row")
          report["unsavedMappingAfter"] = changedLabel
          snapshot("changed-before-cancel", mapper)
          let cancel = tree(mapper).filter { node in
            str(node, kAXRoleAttribute) == "AXButton"
              && [kAXTitleAttribute, kAXDescriptionAttribute].contains { str(node, $0) == "Cancel" }
          }
          try need(cancel.count == 1, "No unique Cancel button")
          try guardInput()
          // Closing a sheet can invalidate the AX request before its reply.
          // Record transport status, then require the actual native outcome.
          let status = AXUIElementPerformAction(cancel[0], kAXPressAction as CFString)
          report["cancelAXStatus"] = status.rawValue
          try observeDismissal(mapper)
          report["savedHashAfterCancel"] = try savedHash()
        }
      }
      if stage["operation"] != "cancel" { snapshot("after-" + name, mapper) }
      try write(
        ["completed": true, "stage": name], out.appendingPathComponent("driver-" + name + ".json"))
    }
    report["inputCompleted"] = true
    while NSRunningApplication(processIdentifier: pid)?.isTerminated == false && now() < deadline
      && !aborted()
    { pump() }
    try need(
      NSRunningApplication(processIdentifier: pid)?.isTerminated ?? true,
      "Private harness did not exit normally before deadline")
    report["savedHashAfterReopen"] = try savedHash()
    report["passed"] = true
  } catch { report["error"] = String(describing: error) }
}
exit(report["passed"] as? Bool == true ? 0 : 1)
