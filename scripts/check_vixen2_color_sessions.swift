// Compiled production callbacks on a private muted profile. No posted input.
// Current test presentation is transparent during real native ordering and hidden
// on return. Baseline directly invokes the original target, omitting presentation.
import AppKit
import ObjectiveC
@testable import VixenScheduler
@testable import VixenKit
import VixenBridge

@main struct Vixen2ColorChecks {
    @MainActor static func main() async throws {
        precondition(UITestHarness.isActive)
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let frontBefore = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        var orderingObservations: [[String: Any]] = []
#if !VIXEN2_BASELINE
        // Observation only: forward the real AppKit methods, recording both
        // the request and returned visibility. No presentation is suppressed.
        func state(_ panel: NSColorPanel, _ method: String, _ mode: Int, _ phase: String) {
            orderingObservations.append(["method": method, "mode": mode, "phase": phase,
                "alpha": panel.alphaValue, "visible": panel.isVisible, "frame": NSStringFromRect(panel.frame),
                "screenFrames": NSScreen.screens.map { NSStringFromRect($0.frame) }])
        }
        let orderSelector = #selector(NSWindow.order(_:relativeTo:))
        let originalOrder = class_getMethodImplementation(NSColorPanel.self, orderSelector)!
        typealias OrderFunction = @convention(c) (AnyObject, Selector, Int, Int) -> Void
        let forwardOrder = unsafeBitCast(originalOrder, to: OrderFunction.self)
        let observeOrder: @convention(block) (NSColorPanel, Int, Int) -> Void = { panel, mode, relative in
            MainActor.assumeIsolated {
                state(panel, "orderWindow", mode, "before")
                forwardOrder(panel, orderSelector, mode, relative)
                state(panel, "orderWindow", mode, "after")
            }
        }
        class_replaceMethod(NSColorPanel.self, orderSelector, imp_implementationWithBlock(observeOrder),
                            method_getTypeEncoding(class_getInstanceMethod(NSColorPanel.self, orderSelector)!))
        let frontSelector = #selector(NSWindow.orderFront(_:))
        let originalFront = class_getMethodImplementation(NSColorPanel.self, frontSelector)!
        typealias FrontFunction = @convention(c) (AnyObject, Selector, AnyObject?) -> Void
        let forwardFront = unsafeBitCast(originalFront, to: FrontFunction.self)
        let observeFront: @convention(block) (NSColorPanel, AnyObject?) -> Void = { panel, sender in
            MainActor.assumeIsolated {
                state(panel, "orderFront", 1, "before")
                forwardFront(panel, frontSelector, sender)
                state(panel, "orderFront", 1, "after")
            }
        }
        class_replaceMethod(NSColorPanel.self, frontSelector, imp_implementationWithBlock(observeFront),
                            method_getTypeEncoding(class_getInstanceMethod(NSColorPanel.self, frontSelector)!))
#endif
        let env = ProcessInfo.processInfo.environment
        let core = try VixenCore.open(dataRoot: env["VIXEN_DATA"]!, muted: true)
        let path = env["VIXEN_V2_INPUT"]!
        let session = ColorPanelSession.shared, panel = NSColorPanel.shared
        let green: UInt32 = 0xff00ff00, blue: UInt32 = 0xff0000ff, red: UInt32 = 0xffff0000
        let yellow: UInt32 = 0xffffff00, cyan: UInt32 = 0xff00ffff
        let base: [UInt32] = [0xff00ff00, 0xffffff00, 0xff00ffff, 0xff00ff00, 0xff00ff00, 0xff00ff00]
        var checks: [[String: Any]] = []
        var presentationStates: [[String: Any]] = []
        func check(_ name: String, _ condition: Bool, _ details: [String: Any] = [:]) {
            var row = details; row["name"] = name; row["passed"] = condition; checks.append(row)
        }
        func colors(_ model: Vixen2MapModel) -> [UInt32] { model.dialog.rows().map(\.importColor) }
        func importer() throws -> Vixen2Import {
            let result = try core.vixen2Import(path: path)
            try result.finishInput(audio: nil, profile: nil)
            return result
        }
        let initial = try importer().editMap()
        func firstLeaf(_ parent: String?) -> String? {
            for row in initial.treeRows(parent: parent) {
                if row.childCount == 0 { return row.id }
                if let leaf = firstLeaf(row.id) { return leaf }
            }
            return nil
        }
        guard let leaf = firstLeaf(nil) else { fatalError("Private fixture needs a destination leaf") }
        func model(_ name: String = "Callback Map") throws -> Vixen2MapModel {
            session.finish()
            let map = try importer().editMap()
            try map.applyDrop(start: 0, nodes: [leaf], repeat: 6, mixing: false, order: "RGB")
            for (i, value) in base.enumerated() { map.setColor(indices: [UInt32(i)], color: value) }
            return Vixen2MapModel(dialog: map, name: name)
        }
#if VIXEN2_BASELINE
        var legacyTarget: Vixen2ColorTarget?
#endif
        func begin(_ model: Vixen2MapModel) {
#if VIXEN2_BASELINE
            // This is the original color() target construction and initial
            // state, omitting only its prohibited makeKeyAndOrderFront call.
            panel.color = .red; panel.showsAlpha = false
            let target = Vixen2ColorTarget(model: model)
            model.colorTarget = target; legacyTarget = target
            panel.setTarget(target); panel.setAction(#selector(Vixen2ColorTarget.changed(_:)))
#else
            model.color()
            let owner = Mirror(reflecting: session).children.first { $0.label?.contains("ownershipWell") == true }
            let well = owner.flatMap { $0.value as? NSColorWell ?? Mirror(reflecting: $0.value).children.first?.value as? NSColorWell }
            presentationStates.append(["panelVisibleAfterBegin": panel.isVisible,
                "outsideAllScreens": !NSScreen.screens.isEmpty && NSScreen.screens.allSatisfy { !$0.frame.intersects(panel.frame) },
                "nativeOwnerActiveAfterOrderOut": well?.isActive == true, "alphaRestored": panel.alphaValue,
                "frame": NSStringFromRect(panel.frame), "screenFrames": NSScreen.screens.map { NSStringFromRect($0.frame) }])
#endif
        }
        func change(_ value: UInt32) {
            panel.color = NSColor(argb: value)
#if VIXEN2_BASELINE
            legacyTarget?.changed(panel)
#else
            _ = session.perform(NSSelectorFromString("colorChanged:"), with: panel)
#endif
        }
        func closePicker() { NotificationCenter.default.post(name: NSWindow.willCloseNotification, object: panel) }
        func cancel(_ model: Vixen2MapModel) {
#if VIXEN2_BASELINE
            model.complete(false)
#else
            model.cancel()
#endif
        }
        var m = try model(); m.selected = [0]; begin(m); m.selected = [1]; change(blue); closePicker()
        check("selection_change_keeps_original_row", colors(m) == [blue] + Array(base.dropFirst()), ["colors": colors(m)])
        m = try model(); m.selected = [0, 2]; begin(m); m.selected = [1]; change(blue); closePicker()
        var expected = base; expected[0] = blue; expected[2] = blue
        check("multiselection_keeps_original_rows", colors(m) == expected, ["colors": colors(m)])
        m = try model(); m.selected = []; begin(m); m.selected = [1]; change(blue); closePicker()
        check("empty_selection_does_not_retarget", colors(m) == base, ["colors": colors(m)])
        m = try model(); m.selected = [0]; begin(m); closePicker()
        check("unchanged_initial_red_commits", colors(m)[0] == red, ["color": colors(m)[0]])
        m = try model(); m.selected = [0]; begin(m); change(blue)
        check("color_choice_waits_for_commit", colors(m) == base, ["colors": colors(m)])
        closePicker()
        check("color_close_commits_once", colors(m)[0] == blue && m.revision == 1, ["revision": m.revision])

        let saved = try model("Accepted Color Map"), owner = try importer()
        var acceptedColors: [UInt32] = [], accepted = false, saveError: String?
        saved.complete = { value in
            accepted = value; acceptedColors = colors(saved)
            if value { do { try owner.saveMap(name: saved.name, map: saved.dialog) } catch { saveError = String(describing: error) } }
        }
        saved.selected = [0]; begin(saved); change(blue); saved.accept()
        check("accept_flushes_before_completion", accepted && acceptedColors[0] == blue && saveError == nil)
        let reopened = try importer(); try reopened.selectMap(name: saved.name)
        check("accepted_color_survives_reopen", reopened.editMap().rows()[0].importColor == blue)
        let modulePath = URL(fileURLWithPath: env["VIXEN_DATA"]!).appendingPathComponent("SystemData/ModuleStore.xml")
        let beforeCancel = try Data(contentsOf: modulePath)
        let draft = Vixen2MapModel(dialog: reopened.editMap(), name: saved.name)
        var canceled = false
        draft.complete = { value in
            canceled = !value
            if value { try? reopened.saveMap(name: draft.name, map: draft.dialog) }
        }
        draft.selected = [0]; begin(draft); change(cyan); cancel(draft)
        let endedColors = colors(draft); change(yellow); closePicker()
        check("cancel_ends_its_color_callback", canceled && colors(draft) == endedColors)
        let reopenedAfterCancel = try importer(); try reopenedAfterCancel.selectMap(name: saved.name)
        check("cancel_preserves_saved_file_and_colors", try Data(contentsOf: modulePath) == beforeCancel && reopenedAfterCancel.editMap().rows()[0].importColor == blue)

        m = try model(); m.selected = [0]; begin(m); change(blue); m.clear(); closePicker()
        check("clear_after_color_leaves_row_cleared", m.rows[0].target == nil && colors(m)[0] == 0)
        m = try model(); m.selected = [0]; begin(m); change(blue)
        // Clear the target through the fixture API to avoid an overwrite alert;
        // the production drop operation is still invoked and must flush first.
        m.dialog.clear(indices: [0]); m.refresh(); m.drop(start: 0, nodes: [leaf])
        try await Task.sleep(for: .milliseconds(100)); closePicker()
        check("drop_after_color_preserves_new_assignment", m.rows[0].target != nil && colors(m)[0] == m.rows[0].color)

        m = try model(); var prior: [UInt32] = []
        session.begin(title: "Prior owner", argb: green) { prior.append($0) }
        session.testSetColor(yellow); m.selected = [0]; begin(m)
        check("map_begin_finishes_previous_owner", prior == [yellow])
        change(blue); closePicker(); session.finish()
        m = try model(); m.selected = [0]; begin(m); change(blue)
        var next: [UInt32] = []
        session.begin(title: "Next owner", argb: green) { next.append($0) }
        session.testSetColor(cyan)
        check("other_owner_takeover_commits_map", colors(m)[0] == blue)
        m.accept(); cancel(m)
        check("old_map_completion_does_not_finish_new_owner", next.isEmpty)
        session.finish(); check("next_owner_remains_functional", next == [cyan])
        m = try model(); m.selected = [0]; begin(m); change(blue)
        m.selected = [1]; begin(m); change(cyan); closePicker()
        expected = base; expected[0] = blue; expected[1] = cyan
        check("repeated_color_edits_commit_separate_rows", colors(m) == expected)
        session.finish()
        let frontAfter = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        check("no_visible_windows_or_focus_changed", !NSApp.isActive && NSApp.windows.allSatisfy { !$0.isVisible } && frontAfter == frontBefore)
        let report: [String: Any] = ["checks": checks, "passed": checks.allSatisfy { $0["passed"] as? Bool == true },
            "frontmostBefore": frontBefore, "frontmostAfter": frontAfter, "ownPID": getpid(), "appActive": NSApp.isActive,
            "presentationStates": presentationStates,
            "orderingObservations": orderingObservations,
            "scope": "Actual compiled Vixen2 model/target or ColorPanelSession callbacks with private Rust map save/reopen. Baseline target construction omits ordering. Current fixed run uses production test presentation and observation-only wrappers which FORWARD real orderFront/orderWindow. Native activation transiently orders a transparent panel before actual orderOut; alpha, frames, active owner and returned visibility are recorded. No physical panel or menu gesture claim."]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
        exit(0)
    }
}
