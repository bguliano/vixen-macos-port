// Private-process presentation guard; production models, representable, native
// color-well activation and callbacks remain compiled and unmodified.
import AppKit
import SwiftUI
import ObjectiveC
@testable import VixenKit
@testable import VixenTools
@testable import VixenScheduler
import VixenBridge

@main struct PropColorTakeoverChecks {
    @MainActor static func main() async throws {
        precondition(UITestHarness.isActive)
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let frontBefore = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        let env = ProcessInfo.processInfo.environment
        let output = URL(fileURLWithPath: CommandLine.arguments[1]).deletingLastPathComponent()
        var suppressed: [String] = []
        func install(_ selector: Selector, _ block: Any) {
            guard let method = class_getInstanceMethod(NSColorPanel.self, selector), let types = method_getTypeEncoding(method) else {
                fatalError("Missing public panel presentation method")
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
        let core = try VixenCore.open(dataRoot: env["VIXEN_DATA"]!, muted: true)
        let vix = env["VIXEN_V2_INPUT"]!
        let prop = PropEditorModel()
        prop.replace(try CustomPropEditor.open(path: env["VIXEN_PROP_INPUT"]!))
        prop.editor.setTab(tab: .state); prop.refresh()
        let originalPropID = prop.states.items[0].id
        let originalPropColor = prop.states.items[0].color
        let host = NSHostingView(rootView: PropStatePanel(model: prop).frame(width: 460, height: 520))
        let window = NSWindow(contentRect: .init(x: -12000, y: -12000, width: 460, height: 520),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host; host.layoutSubtreeIfNeeded()
        try await Task.sleep(for: .milliseconds(200))
        func table(in view: NSView) -> NSTableView? {
            if let table = view as? NSTableView, table.accessibilityLabel() == "State Items" { return table }
            for child in view.subviews { if let found = table(in: child) { return found } }
            return nil
        }
        func well(in view: NSView) -> NSColorWell? {
            if let well = view as? NSColorWell, well.identifier?.rawValue == originalPropID { return well }
            for child in view.subviews { if let found = well(in: child) { return found } }
            return nil
        }
        guard let stateTable = table(in: host), let cell = stateTable.view(atColumn: 1, row: 0, makeIfNecessary: true),
              let nativeWell = well(in: cell) else { fatalError("No actual StateColorWell produced") }
        let green: UInt32 = 0xff00ff00, blue: UInt32 = 0xff0000ff, yellow: UInt32 = 0xffffff00, cyan: UInt32 = 0xff00ffff
        func importer() throws -> Vixen2Import {
            let result = try core.vixen2Import(path: vix); try result.finishInput(audio: nil, profile: nil); return result
        }
        func map(_ name: String) throws -> (Vixen2MapModel, Vixen2Import) {
            let owner = try importer(), dialog = owner.editMap()
            guard let leaf = dialog.treeRows(parent: nil).first(where: { $0.childCount == 0 }) else { fatalError("No generated destination leaf") }
            try dialog.applyDrop(start: 0, nodes: [leaf.id], repeat: 6, mixing: false, order: "RGB")
            dialog.setColor(indices: [0], color: green)
            let model = Vixen2MapModel(dialog: dialog, name: name); model.selected = [0]
            return (model, owner)
        }
        func mapColor(_ model: Vixen2MapModel) -> UInt32 { model.dialog.rows()[0].importColor }
        func propColor() -> UInt32 { prop.editor.states().items.first(where: { $0.id == originalPropID })!.color }
        func chooseForSession(_ color: UInt32) {
            panel.color = NSColor(argb: color)
            _ = session.perform(NSSelectorFromString("colorChanged:"), with: panel)
        }
        var events: [[String: Any]] = []
        func record(_ label: String, _ model: Vixen2MapModel) {
            events.append(["event": label, "mapColor": mapColor(model), "propColor": propColor(), "wellColor": nativeWell.color.argb,
                           "wellActive": nativeWell.isActive, "panelVisible": panel.isVisible, "propRevision": prop.state.revision])
        }
        var checks: [[String: Any]] = []
        func check(_ name: String, _ passed: Bool) { checks.append(["name": name, "passed": passed]) }
        check("real_native_well_and_callbacks", String(describing: type(of: nativeWell)).contains("StateColorWell") && nativeWell.target != nil && nativeWell.action != nil)
        let (prior, priorOwner) = try map("Prior Color Owner")
        prior.color(); chooseForSession(yellow); record("prior-yellow-pending", prior)
        nativeWell.activate(true)
        try await Task.sleep(for: .milliseconds(100))
        let activeAfterTakeover = nativeWell.isActive
        record("prop-well-activated", prior)
        check("actual_well_activation_succeeded", activeAfterTakeover)
        check("prior_session_committed_before_well_takeover", mapColor(prior) == yellow)
        panel.color = NSColor(argb: blue)
        try await Task.sleep(for: .milliseconds(100))
        record("prop-native-blue-change", prior)
        check("native_panel_reached_real_prop_callback", propColor() == blue)
        session.finish(); record("old-session-finished", prior)
        check("prop_color_did_not_retarget_prior_map", mapColor(prior) == yellow)
        try priorOwner.saveMap(name: prior.name, map: prior.dialog)
        let reread = try importer(); try reread.selectMap(name: prior.name)
        check("prior_expected_color_persisted", reread.editMap().rows()[0].importColor == yellow)
        _ = try prop.editor.saveAs(path: output.appendingPathComponent("prop-after-native-well.prp").path)

        // Require an actually active native well, and observe the production
        // callback's model mutation rather than manually invoking wellChanged.
        let activeBeforeReverse = nativeWell.isActive
        let beforeReverse = propColor()
        let (next, nextOwner) = try map("Next Color Owner")
        next.color(); record("shared-session-takes-over-native-well", next)
        let activeAfterReverse = nativeWell.isActive
        chooseForSession(cyan)
        try await Task.sleep(for: .milliseconds(100))
        record("shared-session-cyan-choice", next)
        session.finish(); record("shared-session-finished", next)
        check("reverse_route_had_active_native_well", activeBeforeReverse)
        check("shared_session_deactivates_previous_native_well", !activeAfterReverse)
        check("shared_session_does_not_recolor_prop", propColor() == beforeReverse)
        check("next_session_commits_its_color", mapColor(next) == cyan)
        try nextOwner.saveMap(name: next.name, map: next.dialog)
        _ = try prop.editor.saveAs(path: output.appendingPathComponent("prop-after-shared-session.prp").path)
        let fanOutObserved = activeBeforeReverse && propColor() != beforeReverse
        nativeWell.deactivate(); session.finish()
        // Supply the click coordinates to the actual table target without
        // synthesizing mouse input or copying its doubleClick implementation.
        let (beforeDouble, doubleOwner) = try map("Owner Before Prop Double Click")
        beforeDouble.color(); chooseForSession(yellow)
        let rowSelector = #selector(getter: NSTableView.clickedRow)
        let columnSelector = #selector(getter: NSTableView.clickedColumn)
        let rowMethod = class_getInstanceMethod(NSTableView.self, rowSelector)!
        let columnMethod = class_getInstanceMethod(NSTableView.self, columnSelector)!
        let oldRow = method_getImplementation(rowMethod), oldColumn = method_getImplementation(columnMethod)
        let row: @convention(block) (NSTableView) -> Int = { _ in 0 }
        let column: @convention(block) (NSTableView) -> Int = { _ in 1 }
        method_setImplementation(rowMethod, imp_implementationWithBlock(row))
        method_setImplementation(columnMethod, imp_implementationWithBlock(column))
        let performedDouble = app.sendAction(stateTable.doubleAction!, to: stateTable.target, from: stateTable)
        method_setImplementation(rowMethod, oldRow); method_setImplementation(columnMethod, oldColumn)
        record("actual-prop-doubleClick-target", beforeDouble)
        check("actual_prop_doubleClick_action_performed", performedDouble)
        check("doubleClick_finishes_prior_owner", mapColor(beforeDouble) == yellow)
        check("doubleClick_preserves_noncontinuous_setting", !panel.isContinuous)
        let beforeDoubleChoice = prop.state.revision
        panel.color = NSColor(argb: 0xffff00ff)
        try await Task.sleep(for: .milliseconds(100))
        let beforeCloseRevision = prop.state.revision
        NotificationCenter.default.post(name: NSWindow.willCloseNotification, object: panel)
        session.finish(); record("prop-doubleClick-magenta-finished", beforeDouble)
        check("doubleClick_color_reaches_real_model", propColor() == 0xffff00ff && beforeCloseRevision == beforeDoubleChoice + 1)
        check("doubleClick_finish_does_not_add_duplicate_undo", prop.state.revision == beforeCloseRevision)
        check("doubleClick_does_not_recolor_prior_map", mapColor(beforeDouble) == yellow)
        try doubleOwner.saveMap(name: beforeDouble.name, map: beforeDouble.dialog)
        _ = try prop.editor.saveAs(path: output.appendingPathComponent("prop-after-double-click.prp").path)
        // The same Prop table can transfer from its bespoke double-click path
        // to a native well. A final no-op must not destroy the new live well.
        method_setImplementation(rowMethod, imp_implementationWithBlock(row))
        method_setImplementation(columnMethod, imp_implementationWithBlock(column))
        _ = app.sendAction(stateTable.doubleAction!, to: stateTable.target, from: stateTable)
        method_setImplementation(rowMethod, oldRow); method_setImplementation(columnMethod, oldColumn)
        panel.color = NSColor(argb: green)
        try await Task.sleep(for: .milliseconds(100))
        func anyWell(in view: NSView) -> NSColorWell? {
            if let value = view as? NSColorWell { return value }
            for child in view.subviews { if let value = anyWell(in: child) { return value } }
            return nil
        }
        guard let secondCell = stateTable.view(atColumn: 1, row: 1, makeIfNecessary: true), let secondWell = anyWell(in: secondCell) else {
            fatalError("No actual second Prop well")
        }
        let secondID = secondWell.identifier!.rawValue
        secondWell.activate(true)
        try await Task.sleep(for: .milliseconds(150))
        let secondActive = secondWell.isActive
        panel.color = NSColor(argb: yellow)
        try await Task.sleep(for: .milliseconds(100))
        let secondColor = prop.editor.states().items.first { $0.id == secondID }!.color
        check("same_prop_doubleClick_to_well_remains_active", secondActive)
        check("same_prop_doubleClick_to_well_changes_only_new_item", secondColor == yellow && propColor() == green)
        secondWell.deactivate(); session.finish()

        func ownedWell() -> NSColorWell? {
            for child in Mirror(reflecting: session).children where child.label?.contains("ownershipWell") == true {
                if let well = child.value as? NSColorWell { return well }
                if let wrapped = Mirror(reflecting: child.value).children.first?.value as? NSColorWell { return wrapped }
            }
            return nil
        }
        var commits: [UInt32] = [], ends = 0
        session.begin(title: "Actual close", argb: green, onEnd: { ends += 1 }) { commits.append($0) }
        panel.color = NSColor(argb: blue)
        panel.close(); session.finish(); PendingCommits.flushAll()
        check("actual_panel_close_commits_once", commits == [blue] && ends == 1)
        commits = []; ends = 0
        session.begin(title: "Actual deactivation", argb: green, onEnd: { ends += 1 }) { commits.append($0) }
        panel.color = NSColor(argb: blue)
        let privateWell = ownedWell(), activePrivate = privateWell?.isActive == true
        privateWell?.deactivate(); session.finish(); PendingCommits.flushAll()
        check("actual_owner_deactivation_commits_once", activePrivate && commits == [blue] && ends == 1)
        commits = []; ends = 0
        session.begin(title: "Pending flush", argb: green, onEnd: { ends += 1 }) { commits.append($0) }
        panel.color = NSColor(argb: blue); PendingCommits.flushAll(); session.finish()
        check("pending_flush_commits_once", commits == [blue] && ends == 1 && ownedWell()?.isActive != true)
        for inEnd in [true, false] {
            var old: [UInt32] = [], new: [UInt32] = []
            let replace = { session.begin(title: "Reentrant new owner", argb: green) { new.append($0) } }
            session.begin(title: "Reentrant old owner", argb: green, onEnd: { if inEnd { replace() } }) { color in
                old.append(color); if !inEnd { replace() }
            }
            panel.color = NSColor(argb: blue); session.finish()
            // Use native target dispatch only: a late setTarget(nil) would
            // leave the new owner's current value unchanged and fail.
            panel.color = NSColor(argb: yellow); session.finish()
            check(inEnd ? "onEnd_reentrant_owner_keeps_target" : "onFinish_reentrant_owner_keeps_target", old == [blue] && new == [yellow])
        }
        let frontAfter = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0
        check("no_window_ordered_or_activated", !NSApp.isActive && NSApp.windows.allSatisfy { !$0.isVisible } && frontBefore == frontAfter)
        let report: [String: Any] = ["checks": checks, "events": events, "suppressedPresentationRequests": suppressed,
            "frontmostBefore": frontBefore, "frontmostAfter": frontAfter, "ownPID": getpid(), "appActive": NSApp.isActive,
            "originalPropItemID": originalPropID, "originalPropColor": originalPropColor,
            "activeWellObservedBeforeReverse": activeBeforeReverse, "activeWellObservedAfterReverse": activeAfterReverse,
            "fanOutObserved": fanOutObserved,
            "scope": "Actual compiled PropStatePanel/StateColorWell and Vixen2MapModel/ColorPanelSession in a private process. NSColorPanel presentation is suppressed via public-method runtime overrides. Controlled NSTableView clickedRow/Column getters supply input to the actual doubleAction target. Native panel color changes dispatch to the real well callback; no copied callback, production hook, physical picker or global input. Negative fan-out results are inconclusive if real well activation fails."]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
        exit(0)
    }
}
