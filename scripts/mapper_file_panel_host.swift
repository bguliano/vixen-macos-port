// Private fixture entry linked to exact copied, unmodified Element Mapper sources.
// Normal menus/panels supply all tested filenames. The initial map is fixture setup only.
import AppKit
import Foundation
import VixenBridge
import VixenKit

@MainActor final class MapperPanelHost: NSObject, NSApplicationDelegate {
    let config: [String: Any]
    let output: URL
    var core: VixenCore?
    var timer: Timer?, watchdog: Timer?
    var events: [[String: Any]] = []
    var previousState = ""
    init(config: [String: Any]) {
        self.config = config
        output = URL(fileURLWithPath: config["eventsFile"] as! String)
    }
    func record(_ name: String, _ fields: [String: Any] = [:]) {
        events.append(fields.merging(["event": name, "index": events.count]) { _, new in new })
        if let data = try? JSONSerialization.data(withJSONObject: ["pid": Int(getpid()), "events": events], options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: output, options: .atomic)
        }
    }
    func state() {
        guard let model = ElementMapperModel.current else { return }
        let view = model.dialog.view()
        let rows = view.rows.map { ["sourceId": $0.sourceId, "sourceName": $0.sourceName,
                                   "targetId": $0.targetId ?? "", "targetName": $0.targetName] }
        let value: [String: Any] = ["name": view.name, "path": view.path ?? "", "modified": view.modified,
                                   "rows": rows, "busy": model.busy, "windowTitle": model.window?.title ?? ""]
        if let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]),
           let text = String(data: data, encoding: .utf8), text != previousState {
            previousState = text; record("mapper-state", value)
        }
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        precondition(!UITestHarness.isActive)
        Task {
            do {
                let opened = try VixenCore.open(dataRoot: config["profile"] as! String, muted: true)
                core = opened
                let web = opened.webServer(wwwroot: config["wwwroot"] as! String)
                precondition(!web.settings().running)
                let inputs = (config["sources"] as! [[String: String]]).map { MapperSource(id: $0["id"]!, name: $0["name"]!) }
                let dialog = try opened.elementMapper(name: "Private Native Map", sources: inputs)
                try dialog.openMap(path: config["sourceMap"] as! String)
                ElementMapperPresenter.show(dialog: dialog, over: nil) { [weak self] result, path in
                    guard let self else { return }
                    record("mapper-completed", ["cancelled": result == nil, "path": path as Any? ?? NSNull()])
                    timer?.invalidate(); watchdog?.invalidate()
                    Task { await Task.detached { opened.shutdown() }.value; self.record("shutdown-complete"); NSApp.terminate(nil) }
                }
                state(); record("ready", ["webRunning": web.settings().running, "testMode": UITestHarness.isActive])
                timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.state() } }
                watchdog = Timer.scheduledTimer(withTimeInterval: 100, repeats: false) { [weak self] _ in
                    MainActor.assumeIsolated { self?.record("watchdog"); NSApp.terminate(nil) }
                }
            } catch { record("startup-error", ["error": String(describing: error)]); NSApp.terminate(nil) }
        }
    }
    func applicationWillTerminate(_ notification: Notification) {
        timer?.invalidate(); watchdog?.invalidate(); record("terminated")
    }
}

@main struct MapperPanelEntry {
    @MainActor static func main() throws {
        precondition(CommandLine.arguments.count == 2)
        let config = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))) as! [String: Any]
        let app = NSApplication.shared; app.setActivationPolicy(.regular)
        let delegate = MapperPanelHost(config: config); app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
