// Read-only native view-tree evidence from the actual retained DockPane.
// The generated controller-free profile and sequence are private test data.
import AppKit
import SwiftUI
import VixenBridge
import VixenKit
@testable import VixenSequencer

@MainActor
final class DockVisibilityDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var core: VixenCore!
    var model: SequencerModel!

    func applicationDidFinishLaunching(_ notification: Notification) {
        Task {
            do { try await inspect(); exit(0) }
            catch { print("FAIL \(error)"); exit(2) }
        }
    }

    func inspect() async throws {
        let output = URL(fileURLWithPath: CommandLine.arguments[1])
        let previous = NSWorkspace.shared.frontmostApplication?.processIdentifier
        core = try VixenCore.open(dataRoot: ProcessInfo.processInfo.environment["VIXEN_DATA"]!, muted: true)
        defer { core.shutdown() }
        let setup = core.openDisplaySetup()
        precondition(setup.controllerRows().isEmpty)
        let added = setup.addElement(parent: nil, name: "Private Library Drop Target")
        if let color = setupPropertyTypes().first(where: { $0.name == "Color" }) {
            _ = try setup.addProperty(selection: added.select, typeId: color.id)
        }
        _ = try setup.commit()
        let app = AppModel(); app.adopt(core: core)
        model = SequencerModel(app: app, ref: .new(), registry: SequencerRegistry())
        window = NSWindow(contentRect: NSRect(x: 10000, y: 10000, width: 400, height: 700),
                          styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        model.window = window
        await model.open()
        guard let shell = model.shell, let controller = model.canvas?.controller else { preconditionFailure("No private shell") }
        if model.panels?.rightTabs.contains(where: { $0.panel == .effectEditor }) != true { model.togglePanel(.effectEditor) }
        if model.panels?.rightTabs.contains(where: { $0.panel == .curveLibrary }) != true { model.togglePanel(.curveLibrary) }
        window.contentView = NSHostingView(rootView: DockPane(model: model, side: .right))
        window.orderFront(nil) // Offscreen, prohibited activation, never key.
        var observations: [[String: Any]] = []
        for effectName in ["Pulse", "Set Level"] {
            guard let effect = model.effects.first(where: { $0.name == effectName }),
                  let id = shell.addEffectAt(typeId: effect.typeId, row: 0, timeMs: 0) else {
                preconditionFailure("Cannot create private \(effectName)")
            }
            _ = controller.selectElement(id: id)
            model.timelineSelectionChanged()
            try await Task.sleep(for: .milliseconds(400))
            precondition(model.editorSelection == [id])
            for panel in [ShellPanel.effectEditor, .curveLibrary, .effectEditor] {
                model.selectPanel(panel, side: .right)
                try await Task.sleep(for: .milliseconds(500))
                window.contentView?.layoutSubtreeIfNeeded()
                var rows: [[String: Any]] = []
                @MainActor func visit(_ view: NSView) {
                    if let value = view as? ValueDragView {
                        var row: [String: Any] = ["class": String(describing: type(of: value)),
                            "hidden": value.isHiddenOrHasHiddenAncestor,
                            "registered": value.registeredDraggedTypes.count,
                            "frameInWindow": NSStringFromRect(value.convert(value.bounds, to: nil)),
                            "visibleRect": NSStringFromRect(value.visibleRect)]
                        if let cell = value as? LibraryCellView, let boundary = cell.tracker?.boundaryView {
                            row["viewportBoundsInWindow"] = NSStringFromRect(boundary.convert(boundary.bounds, to: nil))
                            row["viewportVisibleRectInWindow"] = NSStringFromRect(boundary.convert(boundary.visibleRect, to: nil))
                        }
                        rows.append(row)
                    }
                    view.subviews.forEach(visit)
                }
                visit(window.contentView!)
                observations.append(["effect": effectName, "selectedPanel": String(describing: panel),
                                     "selectedEffectIDs": model.editorSelection, "views": rows])
            }
        }
        window.orderOut(nil)
        let report: [String: Any] = ["observations": observations,
            "foregroundUnchanged": NSWorkspace.shared.frontmostApplication?.processIdentifier == previous,
            "scope": "Actual production DockPane and selected Effect Editor controls; read-only view tree in an offscreen window; no drag callbacks or WindowServer input."]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: output)
    }
}

@main struct CheckLibraryDockVisibility {
    @MainActor static func main() {
        precondition(CommandLine.arguments.count == 2)
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let delegate = DockVisibilityDelegate()
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
