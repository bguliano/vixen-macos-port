// Actual SequencerModel palette callbacks with a private muted profile. No
// windows are ordered and no system input is posted. File panels use the
// existing harness path only; this is callback/file-content evidence.
import AppKit
@testable import VixenKit
@testable import VixenSequencer
import VixenBridge

@main struct PaletteSessionChecks {
    @MainActor static func main() async throws {
        precondition(UITestHarness.isActive)
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let core = try VixenCore.open(dataRoot: ProcessInfo.processInfo.environment["VIXEN_DATA"]!, muted: true)
        let appModel = AppModel(); appModel.adopt(core: core)
        let model = SequencerModel(app: appModel, ref: .new(), registry: SequencerRegistry())
        await model.open()
        guard let shell = model.shell else { print("FAIL no shell"); exit(2) }
        let green: UInt32 = 0xff00ff00, blue: UInt32 = 0xff0000ff, red: UInt32 = 0xffff0000
        let yellow: UInt32 = 0xffffff00, white: UInt32 = 0xffffffff, cyan: UInt32 = 0xff00ffff
        let base = [green, blue, red]
        let panel = ColorPanelSession.shared
        var checks: [[String: Any]] = []
        let exchange = ProcessInfo.processInfo.environment["VIXEN_TEST_FILE"]!

        func reset(_ colors: [UInt32]? = nil) {
            panel.finish()
            model.setPalette(colors ?? base)
        }
        func edit(_ index: Int = 1) {
            model.editColor(at: index)
            panel.testSetColor(yellow)
        }
        func record(_ name: String, _ expected: [UInt32], _ mutate: () throws -> Void) rethrows {
            try mutate()
            let during = model.palette
            panel.finish()
            let after = model.palette
            checks.append(["name": name, "passed": during == expected && after == expected && shell.palette() == expected,
                           "expected": expected, "afterOperation": during, "afterPanelFinish": after, "persisted": shell.palette()])
        }
        reset(); edit()
        record("pending_edit_single_reorder", [yellow, red, green]) { model.moveEntry(from: 0, to: 2, kind: .color) }
        reset(); edit()
        record("pending_edit_selected_reorder", [yellow, green, red]) {
            model.moveSelection(["1"], to: model.entries(.color)[0], kind: .color)
        }
        reset(); edit()
        record("pending_edit_selected_to_end", [green, red, yellow]) {
            model.moveSelection(["1"], to: nil, kind: .color)
        }
        reset(); edit()
        record("pending_edit_remove_earlier", [yellow, red]) { model.remove([model.entries(.color)[0]], kind: .color) }
        reset(); edit()
        record("pending_edit_remove_edited", [green, red]) { model.remove([model.entries(.color)[1]], kind: .color) }
        reset([green, blue, blue, red]); edit(2)
        record("pending_edit_remove_edited_duplicate", [green, blue, red]) { model.remove([model.entries(.color)[2]], kind: .color) }
        reset([green, blue, blue, red]); edit(2)
        record("pending_edit_remove_other_duplicate", [green, yellow, red]) { model.remove([model.entries(.color)[1]], kind: .color) }
        reset(); edit()
        record("pending_edit_receive_before_target", [white, green, yellow, red]) { model.receive(.color(white), at: 0) }
        reset(); edit()
        record("pending_edit_receive_append", [green, yellow, red, white]) { model.receive(.color(white), at: nil) }
        reset(); edit()
        try shell.exportPalette(path: exchange, colors: [white])
        record("pending_edit_import_commits_before_snapshot", [green, yellow, red, white]) { model.importLibrary(.color) }
        reset(); model.newColor(); panel.testSetColor(cyan)
        record("pending_new_import_preserves_operation_order", [green, blue, red, cyan, white]) { model.importLibrary(.color) }
        reset(); edit()
        model.exportLibrary(.color)
        let exported = try shell.importPalette(path: exchange)
        panel.finish()
        checks.append(["name": "pending_edit_export_contains_pending_color", "passed": exported == [green, yellow, red],
                       "exported": exported, "expected": [green, yellow, red]])
        reset(); edit(); model.editColor(at: 1)
        let restartedColor = NSColorPanel.shared.color.argb
        panel.finish()
        checks.append(["name": "repeated_edit_starts_at_committed_color", "passed": restartedColor == yellow && model.palette == [green, yellow, red],
                       "pickerStart": restartedColor, "palette": model.palette])
        reset(); model.editColor(at: 1); panel.finish()
        checks.append(["name": "unchanged_edit_preserves_palette", "passed": model.palette == base, "palette": model.palette])
        reset(); model.newColor(); panel.finish()
        checks.append(["name": "unchanged_new_white_still_added", "passed": model.palette == base + [white], "palette": model.palette])
        reset()
        let captured = model.entries(.color)[1]
        edit()
        let payload = model.dragValue(for: captured, link: false)
        let chosen: UInt32?
        if case .color(let value) = payload { chosen = value } else { chosen = nil }
        let beforeFinish = model.palette
        panel.finish()
        checks.append(["name": "pending_edit_drag_payload_uses_committed_color",
                       "passed": chosen == yellow && beforeFinish == [green, yellow, red],
                       "payload": chosen as Any? ?? NSNull(), "afterPayload": beforeFinish, "afterPanelFinish": model.palette])
        panel.finish()
        let front = NSWorkspace.shared.frontmostApplication
        let report: [String: Any] = ["checks": checks, "passed": checks.allSatisfy { $0["passed"] as? Bool == true },
            "appActive": NSApp.isActive, "frontmostPID": front?.processIdentifier ?? 0,
            "ownPID": getpid(), "scope": "Actual production SequencerModel callbacks, ColorPanelSession and Rust palette persistence. Generated private VFC input/output, no native file-panel or system input claim."]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            .write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
        print("Palette callback checks: \(checks.count)")
        // A new untitled document can ask to save at application shutdown.
        // This fixture verifies palette callbacks only; normal quit has its own
        // native suite. End this private muted process without opening a prompt.
        exit(0)
    }
}
