// Standalone private fixture: mount the compiled production SwiftUI picker.
// Only NSColorPanel presentation is suppressed; activation and bindings are real.
import AppKit
import SwiftUI
import ObjectiveC
@testable import VixenDialogs
@testable import VixenKit

@MainActor final class PickerProbeValue: ObservableObject {
    @Published var argb: UInt32 = 0xff00ff00
    var stage = "mount"
    var updates: [[String: Any]] = []
    func set(_ color: UInt32) {
        updates.append(["stage": stage, "old": argb, "new": color])
        argb = color
    }
}

@main struct SwiftUIColorTakeoverChecks {
    @MainActor static func main() async throws {
        precondition(UITestHarness.isActive)
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let frontBefore = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        var suppressed: [String] = []
        func install(_ selector: Selector, _ block: Any) {
            guard let method = class_getInstanceMethod(NSColorPanel.self, selector), let types = method_getTypeEncoding(method) else {
                fatalError("Missing public presentation method")
            }
            class_replaceMethod(NSColorPanel.self, selector, imp_implementationWithBlock(block), types)
        }
        let front: @convention(block) (NSColorPanel, AnyObject?) -> Void = { _, _ in
            MainActor.assumeIsolated { suppressed.append("orderFront") }
        }
        let key: @convention(block) (NSColorPanel, AnyObject?) -> Void = { _, _ in
            MainActor.assumeIsolated { suppressed.append("makeKeyAndOrderFront") }
        }
        let order: @convention(block) (NSColorPanel, Int, Int) -> Void = { _, mode, _ in
            MainActor.assumeIsolated { suppressed.append("orderWindow:\(mode)") }
        }
        install(#selector(NSWindow.orderFront(_:)), front)
        install(#selector(NSWindow.makeKeyAndOrderFront(_:)), key)
        install(#selector(NSWindow.order(_:relativeTo:)), order)
        let panel = NSColorPanel.shared, session = ColorPanelSession.shared
        precondition(!panel.isVisible)
        let value = PickerProbeValue()
        let picker = ArgbColorPicker(argb: Binding(get: { value.argb }, set: { value.set($0) }))
        let host = NSHostingView(rootView: picker.frame(width: 220, height: 90))
        let window = NSWindow(contentRect: .init(x: -12000, y: -12000, width: 240, height: 120),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        try await Task.sleep(for: .milliseconds(250))
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        let allViews = descendants(host)
        let nativeWells = allViews.compactMap { $0 as? NSColorWell }
        guard nativeWells.count == 1, let well = nativeWells.first else {
            let report: [String: Any] = ["inconclusive": true, "error": "Expected one actual SwiftUI NSColorWell",
                "viewClasses": allViews.map { String(describing: type(of: $0)) }, "wellCount": nativeWells.count]
            try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
                .write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
            exit(2)
        }
        var events: [[String: Any]] = [], checks: [[String: Any]] = []
        var prior: [UInt32] = [], next: [UInt32] = [], reopened: [UInt32] = []
        func record(_ stage: String) {
            value.stage = stage
            events.append(["stage": stage, "pickerColor": value.argb, "wellColor": well.color.argb,
                           "panelColor": panel.color.argb, "wellActive": well.isActive,
                           "panelVisible": panel.isVisible, "priorCommits": prior,
                           "nextCommits": next, "reopenedCommits": reopened])
        }
        func check(_ name: String, _ passed: Bool) { checks.append(["name": name, "passed": passed]) }
        func chooseForSession(_ argb: UInt32) {
            panel.color = NSColor(argb: argb)
            _ = session.perform(NSSelectorFromString("colorChanged:"), with: panel)
        }
        check("compiled_picker_produced_native_well", well.target != nil && well.action != nil)
        record("mounted")
        session.begin(title: "Prior shared owner", argb: 0xff0000ff) { prior.append($0) }
        chooseForSession(0xffffff00)
        record("prior-yellow-pending")
        well.activate(true)
        try await Task.sleep(for: .milliseconds(100))
        record("swiftui-picker-takes-over")
        check("actual_swiftui_well_activation_succeeded", well.isActive)
        check("prior_session_commits_once_at_picker_takeover", prior == [0xffffff00])
        value.stage = "native-blue-choice"
        panel.color = NSColor(argb: 0xff0000ff)
        try await Task.sleep(for: .milliseconds(100))
        record("native-blue-choice")
        check("actual_panel_change_reaches_picker_binding", value.argb == 0xff0000ff)
        session.finish()
        value.stage = "native-magenta-after-old-finish"
        panel.color = NSColor(argb: 0xffff00ff)
        try await Task.sleep(for: .milliseconds(100))
        record("native-magenta-after-old-finish")
        check("old_session_finish_keeps_picker_binding_owner", value.argb == 0xffff00ff && well.isActive)
        check("old_session_committed_original_value_once", prior == [0xffffff00])

        let beforeReverse = value.argb, activeBeforeReverse = well.isActive
        session.begin(title: "Next shared owner", argb: 0xff00ff00) { next.append($0) }
        record("shared-session-takes-over-picker")
        let activeAfterReverse = well.isActive
        check("reverse_route_started_with_real_active_picker", activeBeforeReverse)
        check("shared_session_deactivates_picker_before_initial_color", !activeAfterReverse && value.argb == beforeReverse)
        value.stage = "shared-cyan-choice"
        chooseForSession(0xff00ffff)
        try await Task.sleep(for: .milliseconds(100))
        record("shared-cyan-choice")
        session.finish()
        record("shared-session-finished")
        check("shared_choice_does_not_recolor_picker", value.argb == beforeReverse)
        check("next_session_commits_its_own_color_once", next == [0xff00ffff])

        session.begin(title: "Reopened shared owner", argb: 0xff0000ff) { reopened.append($0) }
        chooseForSession(0xffffff00)
        well.activate(true)
        try await Task.sleep(for: .milliseconds(100))
        record("picker-reacquires")
        check("picker_can_reacquire_and_commit_pending_session", well.isActive && reopened == [0xffffff00])
        value.stage = "native-red-reacquired"
        panel.color = NSColor(argb: 0xffff0000)
        try await Task.sleep(for: .milliseconds(100))
        record("native-red-reacquired")
        check("reacquired_picker_binding_receives_native_change", value.argb == 0xffff0000)
        session.finish()
        well.deactivate()
        let frontAfter = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        check("no_window_ordered_or_activation", !app.isActive && NSApp.windows.allSatisfy { !$0.isVisible } && frontBefore == frontAfter)
        let report: [String: Any] = ["checks": checks, "events": events, "bindingUpdates": value.updates,
            "wellClass": String(describing: type(of: well)), "wellTargetClass": well.target.map { String(describing: type(of: $0)) } ?? "nil",
            "wellAction": well.action.map(NSStringFromSelector) ?? "nil", "viewClasses": allViews.map { String(describing: type(of: $0)) },
            "priorCommits": prior, "nextCommits": next, "reopenedCommits": reopened,
            "activeWellObservedBeforeReverse": activeBeforeReverse, "activeWellObservedAfterReverse": activeAfterReverse,
            "suppressedPresentationRequests": suppressed, "frontmostBefore": frontBefore, "frontmostAfter": frontAfter,
            "ownPID": getpid(), "appActive": app.isActive, "allWindowsHidden": NSApp.windows.allSatisfy { !$0.isVisible },
            "scope": "Actual compiled VixenDialogs.ArgbColorPicker mounted in NSHostingView; actual nested NSColorWell activation and SwiftUI binding updates. Panel presentation alone suppressed via public-method overrides. Shared-session color uses existing production colorChanged: callback after panel.color; picker changes use native panel.color alone. No copied picker, global input, physical chooser, Core profile, audio or controller." ]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            .write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
        exit(0)
    }
}
