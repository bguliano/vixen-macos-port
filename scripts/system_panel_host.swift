// Private host for the actual production system colour/font panel sessions.
// No Core/show is opened, and no chosen value is supplied by a test hook.
import AppKit
import Foundation
import VixenKit

@MainActor final class PanelHost: NSObject, NSApplicationDelegate {
    let output: URL
    var window: NSWindow!
    var events: [[String: Any]] = []
    var watchdog: Timer?
    var closeObservers: [NSObjectProtocol] = []
    let nativeWell = NSColorWell(frame: .zero)
    var colorSession = 0
    let started = Date()
    init(output: URL) { self.output = output }
    func record(_ event: String, _ fields: [String: Any] = [:]) {
        events.append(fields.merging(["event": event, "index": events.count, "elapsedSeconds": Date().timeIntervalSince(started)]) { _, new in new })
        let report: [String: Any] = ["events": events, "pid": Int(getpid()),
                                   "scope": "Actual production ColorPanelSession and FontPanelSession in an isolated native host; no show or output devices"]
        if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: output, options: .atomic)
        }
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        precondition(!UITestHarness.isActive)
        window = NSWindow(contentRect: NSRect(x: 160, y: 220, width: 440, height: 200), styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.title = "Private System Panel Verification"
        let content = NSView(frame: NSRect(x: 0, y: 0, width: 440, height: 200))
        for (index, entry) in [("Open Color", #selector(openColor)), ("Open Font", #selector(openFont)), ("Finish Check", #selector(finish))].enumerated() {
            let button = NSButton(title: entry.0, target: self, action: entry.1)
            button.frame = NSRect(x: 20 + index * 140, y: 115, width: 120, height: 32)
            content.addSubview(button)
        }
        nativeWell.frame = NSRect(x: 30, y: 45, width: 50, height: 30)
        nativeWell.color = NSColor(argb: 0xff00ff00)
        nativeWell.target = self; nativeWell.action = #selector(nativeColorChanged(_:))
        nativeWell.setAccessibilityLabel("Native Color Well")
        content.addSubview(nativeWell)
        let takeover = NSButton(title: "Use Native Well", target: self, action: #selector(useNativeWell))
        takeover.frame = NSRect(x: 100, y: 45, width: 160, height: 32)
        content.addSubview(takeover)
        window.contentView = content
        let menu = NSMenu(), item = NSMenuItem(), appMenu = NSMenu()
        appMenu.addItem(withTitle: "Quit Panel Check", action: #selector(finish), keyEquivalent: "q").target = self
        item.submenu = appMenu; menu.addItem(item); NSApp.mainMenu = menu
        window.makeKeyAndOrderFront(nil)
        record("ready")
        watchdog = Timer.scheduledTimer(withTimeInterval: 45, repeats: false) { _ in
            MainActor.assumeIsolated { self.record("watchdog"); NSApp.terminate(nil) }
        }
    }
    @objc func openColor() {
        colorSession += 1
        let session = colorSession
        record("color-opened", ["session": session])
        ColorPanelSession.shared.begin(title: "Private Color Check", argb: 0xff0000ff,
            onLive: { [weak self] color in self?.record("color-live", ["argb": color, "session": session]) },
            onEnd: { [weak self] in self?.record("color-ended", ["session": session]) },
            onFinish: { [weak self] color in self?.record("color-committed", ["argb": color, "session": session]) })
        record("shared-owner-ready", ["session": session, "nativeWellActive": nativeWell.isActive,
            "nativeWellColor": nativeWell.color.argb, "panelVisible": NSColorPanel.shared.isVisible,
            "panelAlpha": NSColorPanel.shared.alphaValue, "testMode": UITestHarness.isActive])
        if session == 1 { observeClose(NSColorPanel.shared, kind: "color") }
    }
    @objc func useNativeWell() {
        nativeWell.activate(true)
        record("native-owner-ready", ["active": nativeWell.isActive, "argb": nativeWell.color.argb,
            "panelColor": NSColorPanel.shared.color.argb, "panelTitle": NSColorPanel.shared.title])
    }
    @objc func nativeColorChanged(_ sender: NSColorWell) {
        record("native-color-changed", ["argb": sender.color.argb, "active": sender.isActive])
    }
    @objc func openFont() {
        record("font-opened")
        FontPanelSession.shared.begin(font: KitFont(name: "Arial", size: 12, bold: false, italic: false)) { [weak self] font in
            self?.record("font-committed", ["name": font.name, "size": font.size, "bold": font.bold, "italic": font.italic])
        }
        if let selected = NSFontManager.shared.selectedFont {
            record("font-initial-selection", ["name": selected.fontName, "family": selected.familyName ?? "", "size": selected.pointSize])
        }
        observeClose(NSFontPanel.shared, kind: "font")
    }
    func observeClose(_ panel: NSWindow, kind: String) {
        closeObservers.append(NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: panel, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.record("\(kind)-window-closed") }
        })
    }
    @objc func finish() {
        record("native-well-final", ["active": nativeWell.isActive, "argb": nativeWell.color.argb])
        ColorPanelSession.shared.finish()
        FontPanelSession.shared.finish()
        nativeWell.deactivate()
        record("finished")
        NSApp.terminate(nil)
    }
    func applicationWillTerminate(_ notification: Notification) {
        watchdog?.invalidate()
        closeObservers.forEach(NotificationCenter.default.removeObserver)
        record("terminated")
    }
}

@main struct SystemPanelHost {
    @MainActor static func main() {
        guard CommandLine.arguments.count == 2 else { exit(2) }
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let delegate = PanelHost(output: URL(fileURLWithPath: CommandLine.arguments[1]))
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
