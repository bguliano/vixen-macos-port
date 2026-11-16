// Linked against the actual debug VixenKit object and its matching Rust library.
// No WindowServer input is posted. The system colour panel remains hidden.
import AppKit
import SwiftUI
@testable import VixenKit
import VixenBridge

@main
struct ColorSessionChecks {
    @MainActor static var checks: [[String: Any]] = []
    @MainActor static let panel = ColorPanelSession.shared
    @MainActor static func record(_ name: String, _ passed: Bool, _ details: [String: Any] = [:]) {
        checks.append(["name": name, "passed": passed, "details": details])
    }

    @MainActor static func begin(_ start: UInt32, creating: Bool = false,
        live: ((UInt32) -> Void)? = nil, end: (() -> Void)? = nil,
        done: @escaping (UInt32) -> Void) {
        #if COLOR_FIXED
        panel.begin(title: "Private colour regression", argb: start, onLive: live,
                    notifyUnchanged: creating, onEnd: end, onFinish: done)
        #else
        // These are the original clients' callback semantics. In particular,
        // temporary preview cleanup formerly lived inside the commit callback.
        panel.begin(title: "Private colour regression", argb: start, onLive: live,
                    onFinish: { value in end?(); done(value) })
        #endif
    }

    @MainActor static func callbacks() {
        let white: UInt32 = 0xffffffff, red: UInt32 = 0xffff0000
        for revert in [false, true] {
            var commits: [UInt32] = []
            begin(white, creating: true) { commits.append($0) }
            if revert { panel.testSetColor(red); panel.testSetColor(white) }
            panel.finish(); panel.finish()
            record(revert ? "new_reverted_white_added_once" : "new_unchanged_white_added_once",
                   commits == [white], ["commits": commits])
        }
        for revert in [false, true] {
            var commits: [UInt32] = [], live: UInt32?, ends = 0
            begin(white, live: { live = $0 }, end: { live = nil; ends += 1 }) { commits.append($0) }
            if revert { panel.testSetColor(red); panel.testSetColor(white) }
            panel.finish(); panel.finish()
            record(revert ? "edit_reverted_no_commit" : "edit_unchanged_no_commit", commits.isEmpty,
                   ["commits": commits])
            record(revert ? "edit_reverted_preview_cleared" : "edit_unchanged_ended_once",
                   live == nil && ends == 1, ["live": live as Any? ?? NSNull(), "ends": ends])
        }
        var old: [UInt32] = [], new: [UInt32] = []
        begin(white) { old.append($0) }; panel.testSetColor(red)
        begin(white, creating: true) { new.append($0) }; panel.finish()
        record("replacement_finishes_previous_once", old == [red] && new == [white],
               ["oldCommits": old, "newCommits": new])
    }

    @MainActor static func model() -> GradientEditorViewModel {
        var value = defaultGradientValue()
        value = gradientSetStopColor(value: value, stop: 0, argb: 0xff00ff00)
        value = gradientAddStop(value: value, position: 1, discrete: false, validColors: [])
        value = gradientSetStopColor(value: value, stop: 1, argb: 0xff0000ff)
        let model = GradientEditorViewModel(session: GradientEditorSession(
            gradient: value, libraryName: nil, discrete: false, validColors: []))
        model.state = model.session.state(width: 500)
        return model
    }

    @MainActor static func view(_ model: GradientEditorViewModel) -> (NSWindow, GradientBarNSView) {
        let window = NSWindow(contentRect: NSRect(x: -12000, y: -12000, width: 500, height: 90),
                              styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let view = GradientBarNSView(frame: NSRect(x: 0, y: 0, width: 500, height: 90))
        view.model = model; window.contentView = view
        return (window, view)
    }

    @MainActor static func click(_ index: Int, _ view: GradientBarNSView) {
        let point = view.convert(NSPoint(x: view.model!.state.faders[index].x + 10, y: 82), to: nil)
        for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            let event = NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: view.window!.windowNumber,
                context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!
            if type == .leftMouseDown { view.mouseDown(with: event) } else { view.mouseUp(with: event) }
        }
    }

    @MainActor static func gradient() {
        for reverted in [false, true] {
            let model = model(), (window, bar) = view(model)
            click(0, bar)
            var atCommit: [[UInt32]] = []
            begin(model.state.swatchArgb, live: { model.liveSwatch = $0 }, end: { model.liveSwatch = nil }) {
                atCommit.append(model.session.result().stops.map(\.argb))
                model.state = model.session.setSelectedColor(argb: $0)
            }
            panel.testSetColor(0xffff0000)
            if reverted { panel.testSetColor(0xff00ff00) }
            click(1, bar); panel.finish()
            let colors = model.session.result().stops.map(\.argb)
            let expected: [UInt32] = [reverted ? 0xff00ff00 : 0xffff0000, 0xff0000ff]
            record(reverted ? "gradient_reverted_then_select_keeps_original_stops" : "gradient_pending_then_select_commits_original_stop",
                   colors == expected, ["colors": colors, "expected": expected, "atCommit": atCommit])
            record(reverted ? "gradient_reverted_then_select_clears_preview" : "gradient_pending_then_select_clears_preview",
                   model.liveSwatch == nil && model.state.swatchArgb == 0xff0000ff,
                   ["live": model.liveSwatch as Any? ?? NSNull(), "selectedSwatch": model.state.swatchArgb])
            window.close()
        }
        for key: UInt16 in [51, 117] {
            let model = model(), (window, bar) = view(model)
            click(0, bar)
            var atCommit: [[UInt32]] = []
            begin(model.state.swatchArgb, end: { model.liveSwatch = nil }) {
                atCommit.append(model.session.result().stops.map(\.argb))
                model.state = model.session.setSelectedColor(argb: $0)
            }
            panel.testSetColor(0xffff0000)
            bar.keyDown(with: NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                context: nil, characters: "\u{7f}", charactersIgnoringModifiers: "\u{7f}",
                isARepeat: false, keyCode: key)!)
            panel.finish()
            let colors = model.session.result().stops.map(\.argb)
            record("gradient_pending_then_delete_\(key)_commits_before_removal",
                   atCommit == [[0xff00ff00, 0xff0000ff]] && colors == [0xff0000ff],
                   ["atCommit": atCommit, "remainingColors": colors])
            window.close()
        }
    }

    @MainActor static func main() throws {
        precondition(ProcessInfo.processInfo.environment["VIXEN_UI_TEST"] != nil)
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.accessory)
        callbacks(); gradient(); panel.finish()
        let result: [String: Any] = ["checks": checks, "passed": checks.allSatisfy { $0["passed"] as? Bool == true },
            "scope": "Real ColorPanelSession callbacks, actual GradientBarNSView mouse/key handlers and Rust gradient state. No system input, system colour-panel selection, preset or collection controls exercised."]
        let data = try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
        print(String(data: data, encoding: .utf8)!)
    }
}
