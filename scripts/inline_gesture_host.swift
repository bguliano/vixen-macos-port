// Normal private foreground fixture using optimized production inline controls.
// No Core/show, synthetic handler calls, method replacement, or chosen-value hooks.
import AppKit
import Foundation
import VixenKit
import VixenBridge

@MainActor final class InlineGestureHost: NSObject, NSApplicationDelegate {
    let output: URL
    let started = ProcessInfo.processInfo.systemUptime
    let level = CurveStripNSView(frame: NSRect(x: 210, y: 230, width: 200, height: 48))
    let point = CurveStripNSView(frame: NSRect(x: 210, y: 145, width: 200, height: 32))
    let gradient = GradientStripNSView(frame: NSRect(x: 210, y: 65, width: 200, height: 42))
    var window: NSWindow!
    var timer: Timer?, watchdog: Timer?
    var commits = ["level": 0, "point": 0, "gradient": 0]
    var history: [[String: Any]] = []
    var lastState: Data?
    var terminated = false
    init(output: URL) { self.output = output }

    func curve(_ value: CurveValue) -> [String: Any] {
        ["points": value.points.map { [$0.x, $0.y] }, "libraryName": value.libraryName,
         "isCurrentLibrary": value.isCurrentLibrary]
    }
    func gradientValue(_ value: GradientValue) -> [String: Any] {
        ["stops": value.stops.map { ["position": $0.position, "focus": $0.focus,
            "x": $0.x, "y": $0.y, "z": $0.z, "argb": $0.argb] as [String: Any] },
         "alphas": value.alphas.map { ["alpha": $0.alpha, "position": $0.position, "focus": $0.focus] },
         "gammacorrected": value.gammacorrected, "title": value.title as Any? ?? NSNull(),
         "libraryName": value.libraryName, "isCurrentLibrary": value.isCurrentLibrary]
    }
    func texts(_ view: NSView?) -> [String] {
        guard let view else { return [] }
        if let field = view as? NSTextField { return [field.stringValue] }
        return view.subviews.flatMap { texts($0) }
    }
    func rect(_ value: NSRect) -> [Double] { [value.minX, value.minY, value.width, value.height] }
    func cgPoint(_ view: NSView, _ point: NSPoint) -> [Double] {
        let screen = window.convertPoint(toScreen: view.convert(point, to: nil))
        return [screen.x, (NSScreen.screens.first?.frame.maxY ?? 0) - screen.y]
    }
    func geometry(_ view: NSView) -> [String: Any] {
        ["bounds": rect(view.bounds), "flipped": view.isFlipped,
         "topLeftCG": cgPoint(view, NSPoint(x: view.bounds.minX, y: view.bounds.minY)),
         "bottomRightCG": cgPoint(view, NSPoint(x: view.bounds.maxX, y: view.bounds.maxY))]
    }
    func state() -> [String: Any] {
        let tips = NSApp.windows.filter { $0 is NSPanel && $0.isVisible }.map { panel in
            ["windowNumber": panel.windowNumber, "texts": texts(panel.contentView),
             "alpha": panel.alphaValue, "frame": rect(panel.frame),
             "nonactivating": panel.styleMask.contains(.nonactivatingPanel),
             "key": panel.isKeyWindow, "visible": panel.isVisible] as [String: Any]
        }
        return ["level": curve(level.value), "point": curve(point.value),
                "gradient": gradientValue(gradient.value), "commits": commits,
                "levelIntrinsicHeight": level.intrinsicContentSize.height,
                "tooltips": tips, "windowKey": window.isKeyWindow,
                "windowVisible": window.isVisible, "active": NSApp.isActive]
    }
    func record(_ event: String, force: Bool = false) {
        let snapshot = state()
        guard let data = try? JSONSerialization.data(withJSONObject: snapshot, options: .sortedKeys) else { exit(3) }
        if force || data != lastState {
            history.append(["event": event, "elapsedSeconds": ProcessInfo.processInfo.systemUptime - started, "state": snapshot])
            lastState = data
        }
        let report: [String: Any] = ["pid": Int(getpid()), "bundleIdentifier": Bundle.main.bundleIdentifier ?? "",
            "executable": Bundle.main.executableURL?.standardizedFileURL.path ?? "",
            "windowNumber": window.windowNumber, "windowTitle": window.title,
            "windowFrame": rect(window.frame), "screenTop": NSScreen.screens.first?.frame.maxY ?? 0,
            "geometry": ["level": geometry(level), "point": geometry(point), "gradient": geometry(gradient)],
            "current": snapshot, "events": history, "terminated": terminated,
            "testMode": UITestHarness.isActive, "watchdogSeconds": 55,
            "scope": "Real production native views in a normal visible host; passive values/tooltips and release callbacks only. No Core, show, handler invocation, method replacement or test-set hook."]
        do {
            try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: output, options: .atomic)
        } catch { fputs("Cannot write passive fixture observations: \(error)\n", stderr); exit(3) }
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        precondition(!UITestHarness.isActive)
        window = NSWindow(contentRect: NSRect(x: 160, y: 200, width: 490, height: 330),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "Private Inline Gesture Verification"
        window.isReleasedWhenClosed = false
        window.acceptsMouseMovedEvents = true
        let content = NSView(frame: NSRect(x: 0, y: 0, width: 490, height: 330))
        for (text, y) in [("Shift: level handle", 242.0), ("Curve: middle point", 150.0), ("Gradient: paired stops", 76.0)] {
            let label = NSTextField(labelWithString: text)
            label.frame = NSRect(x: 20, y: y, width: 185, height: 22)
            content.addSubview(label)
        }
        level.setValue(CurveValue(points: [KitPoint(x: 0, y: 25), KitPoint(x: 100, y: 25)], libraryName: "", isCurrentLibrary: false))
        point.setValue(CurveValue(points: [KitPoint(x: 0, y: 0), KitPoint(x: 50, y: 50), KitPoint(x: 100, y: 100)], libraryName: "", isCurrentLibrary: false))
        var initial = solidGradientValue(argb: 0xffff0000)
        var red = initial.stops[0], green = solidGradientValue(argb: 0xff00ff00).stops[0], blue = solidGradientValue(argb: 0xff0000ff).stops[0]
        red.position = 0.25; green.position = 0.25; blue.position = 0.75
        initial.stops = [red, green, blue]
        gradient.setValue(initial, discrete: false)
        for (name, view) in [("level", level), ("point", point)] {
            view.onCommit = { [weak self] _ in
                guard let self else { return }; self.commits[name, default: 0] += 1
                self.record(name + "-commit", force: true)
            }
            content.addSubview(view)
        }
        gradient.onCommit = { [weak self] _ in
            guard let self else { return }; self.commits["gradient", default: 0] += 1
            self.record("gradient-commit", force: true)
        }
        content.addSubview(gradient)
        let finishButton = NSButton(title: "Finish Check", target: self, action: #selector(finish))
        finishButton.frame = NSRect(x: 350, y: 15, width: 120, height: 32)
        content.addSubview(finishButton)
        window.contentView = content
        let menu = NSMenu(), item = NSMenuItem(), submenu = NSMenu()
        submenu.addItem(withTitle: "Quit Gesture Check", action: #selector(finish), keyEquivalent: "q").target = self
        item.submenu = submenu; menu.addItem(item); NSApp.mainMenu = menu
        window.makeKeyAndOrderFront(nil)
        record("ready", force: true)
        timer = Timer.scheduledTimer(withTimeInterval: 0.025, repeats: true) { _ in
            MainActor.assumeIsolated { self.record("observation") }
        }
        watchdog = Timer.scheduledTimer(withTimeInterval: 55, repeats: false) { _ in
            MainActor.assumeIsolated { self.record("watchdog", force: true); NSApp.terminate(nil) }
        }
    }
    @objc func finish() { record("finished", force: true); NSApp.terminate(nil) }
    func applicationWillTerminate(_ notification: Notification) {
        timer?.invalidate(); watchdog?.invalidate(); terminated = true
        record("terminated", force: true)
    }
}

@main struct InlineGestureApplication {
    @MainActor static func main() {
        guard CommandLine.arguments.count == 2 else { exit(2) }
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let delegate = InlineGestureHost(output: URL(fileURLWithPath: CommandLine.arguments[1]))
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
