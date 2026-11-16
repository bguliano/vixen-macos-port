// A private, muted host for the actual production sheet views. Hooks only set
// the colour picker's chosen value, inspect state, or select a real gradient
// fader through its NSView event handler; sheet buttons/pickers use the harness.
import AppKit
import SwiftUI
@testable import VixenKit
@testable import VixenSequencer
@testable import VixenEffectEditor
@testable import VixenMarks
import VixenBridge

@MainActor final class ColorSheetDelegate: NSObject, NSApplicationDelegate {
    var core: VixenCore!
    var window: NSWindow!
    var collection: ShellCollectionEditor?
    var model: SequencerModel?
    var result: GradientValue?

    func json(_ value: Any) -> String {
        String(data: try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), encoding: .utf8)!
    }

    func resetWindow() {
        ColorPanelSession.shared.finish()
        if let window {
            while let sheet = window.attachedSheet { window.endSheet(sheet); sheet.orderOut(nil) }
            window.close()
        }
        window = NSWindow(contentRect: NSRect(x: -12000, y: -12000, width: 620, height: 510),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.title = "Private Colour Controls"
        window.contentView = NSView(frame: NSRect(x: 0, y: 0, width: 620, height: 510))
        window.orderFront(nil)
        window.makeKey()
    }

    func findBar(_ view: NSView?) -> GradientBarNSView? {
        guard let view else { return nil }
        if let bar = view as? GradientBarNSView { return bar }
        return view.subviews.lazy.compactMap { self.findBar($0) }.first
    }

    func bar() -> GradientBarNSView? { findBar(window.attachedSheet?.contentView) }

    // SwiftUI's menu picker cannot open a WindowServer menu offscreen. Obtain
    // the production closure from the compiled view body instead of copying its
    // implementation. This is a binding callback test, not menu interaction.
    func collectionBinding() -> Binding<String>? {
        guard let model, let collection else { return nil }
        let root = SequencerSheetView(model: model, sheet: .colorCollections(collection))
        func sheet(_ value: Any, _ depth: Int = 0) -> (any View)? {
            guard depth < 20 else { return nil }
            if String(reflecting: type(of: value)).hasSuffix(".ColorCollectionsSheet") { return value as? any View }
            return Mirror(reflecting: value).children.lazy.compactMap { sheet($0.value, depth + 1) }.first
        }
        func binding(_ value: Any, _ depth: Int = 0) -> Binding<String>? {
            guard depth < 30 else { return nil }
            if let value = value as? Binding<String>, value.wrappedValue == collection.values().selected?.name { return value }
            return Mirror(reflecting: value).children.lazy.compactMap { binding($0.value, depth + 1) }.first
        }
        guard let view = sheet(root.body) else { return nil }
        return binding(view.body)
    }

    func selectFader(_ index: Int, double: Bool) -> String {
        guard let bar = bar(), let state = bar.model?.state, state.faders.indices.contains(index), let host = bar.window else {
            return "FAIL no requested gradient fader"
        }
        let location = bar.convert(NSPoint(x: state.faders[index].x + 10, y: bar.bounds.height - 8), to: nil)
        for clicks in (double ? [1, 2] : [1]) {
            let event = NSEvent.mouseEvent(with: .leftMouseDown, location: location, modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: host.windowNumber,
                context: nil, eventNumber: 0, clickCount: clicks, pressure: 1)!
            bar.mouseDown(with: event)
            bar.mouseUp(with: event)
        }
        return "selected actual fader \(index), double=\(double)"
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            core = try VixenCore.open(dataRoot: ProcessInfo.processInfo.environment["VIXEN_DATA"]!, muted: true)
        } catch { print("FAIL core: \(error)"); exit(2) }
        _ = core.libraryAddGradient(name: "Regression Preset", gradient: solidGradientValue(argb: 0xffffff00))
        UITestHarness.hooks["color.choose"] = { args in
            guard let text = args.first, let color = UInt32(text, radix: 16) else { return "FAIL requires ARGB hex" }
            ColorPanelSession.shared.testSetColor(color)
            return "pending \(text)"
        }
        UITestHarness.hooks["color.finish"] = { _ in ColorPanelSession.shared.finish(); return "finished" }
        UITestHarness.hooks["controls.clientRevert"] = { [self] _ in
            let effect = EffectEditorModel(session: core.newSequence())
            var live: UInt32?, commits: [UInt32] = []
            openColorEditor(title: "Private effect colour", argb: 0xff00ff00, valid: [], model: effect,
                            live: { live = $0 }) { commits.append($0) }
            ColorPanelSession.shared.testSetColor(0xffff0000)
            ColorPanelSession.shared.testSetColor(0xff00ff00)
            ColorPanelSession.shared.finish()
            let marks = MarksPanelModel(session: core.newSequence())
            marks.addCollection()
            guard let row = marks.rows.first else { return "FAIL no mark row" }
            let revision = marks.panel.revision()
            marks.chooseColor(row)
            ColorPanelSession.shared.testSetColor(row.argb == 0xffff0000 ? 0xff0000ff : 0xffff0000)
            ColorPanelSession.shared.testSetColor(row.argb)
            ColorPanelSession.shared.finish()
            return json(["effectLiveCleared": live == nil, "effectCommits": commits,
                         "marksLiveCleared": marks.liveColors.isEmpty, "marksRevisionUnchanged": marks.panel.revision() == revision])
        }
        UITestHarness.hooks["controls.collections"] = { [self] _ in
            resetWindow()
            let session = core.newSequence()
            let editor = session.makeShell().colorCollectionEditor()
            _ = editor.newCollection(name: "Collection A", overwrite: true)
            editor.addColor(argb: 0xff00ff00)
            _ = editor.newCollection(name: "Collection B", overwrite: true)
            editor.addColor(argb: 0xff0000ff)
            editor.select(name: "Collection A")
            collection = editor
            let app = AppModel(); app.adopt(core: core)
            model = SequencerModel(app: app, ref: .new(), registry: SequencerRegistry())
            model!.window = window
            window.contentView = NSHostingView(rootView: SequencerSheetView(model: model!, sheet: .colorCollections(editor)))
            return "mounted production ColorCollectionsSheet"
        }
        UITestHarness.hooks["controls.collectionState"] = { [self] _ in
            guard let state = collection?.values() else { return "FAIL no collection" }
            return json(["selected": state.selected?.name ?? "", "colors": state.selected?.colors ?? [],
                         "lastColor": state.lastColor, "dirty": state.dirty])
        }
        UITestHarness.hooks["controls.chooseCollection"] = { [self] args in
            guard let name = args.first, let binding = collectionBinding() else { return "FAIL no production collection binding" }
            binding.wrappedValue = name
            return "called actual compiled Picker Binding setter for \(name)"
        }
        UITestHarness.hooks["controls.gradient"] = { [self] _ in
            resetWindow(); result = nil
            var gradient = defaultGradientValue()
            gradient = gradientSetStopColor(value: gradient, stop: 0, argb: 0xff00ff00)
            gradient = gradientAddStop(value: gradient, position: 1, discrete: false, validColors: [])
            gradient = gradientSetStopColor(value: gradient, stop: 1, argb: 0xff0000ff)
            GradientEditorDialog.present(on: window, core: core, gradient: gradient) { [self] in result = $0 }
            return "mounted production GradientEditorDialog"
        }
        UITestHarness.hooks["controls.linkedGradient"] = { [self] _ in
            resetWindow(); result = nil
            let fixture = GradientEditorSession(gradient: solidGradientValue(argb: 0xffffff00), libraryName: nil,
                                               discrete: false, validColors: [])
            _ = fixture.loadFromLibrary(name: "Regression Preset", entry: solidGradientValue(argb: 0xffffff00))
            GradientEditorDialog.present(on: window, core: core, gradient: fixture.result()) { [self] in result = $0 }
            return "mounted linked fixture in production GradientEditorDialog"
        }
        UITestHarness.hooks["controls.fader"] = { [self] args in
            guard let index = args.first.flatMap(Int.init) else { return "FAIL needs index" }
            return selectFader(index, double: args.last == "edit")
        }
        UITestHarness.hooks["controls.gradientState"] = { [self] _ in
            guard let model = bar()?.model else { return "FAIL no gradient model" }
            let value = model.session.result()
            return json(["colors": value.stops.map(\.argb), "positions": value.stops.map(\.position),
                         "live": model.liveSwatch as Any? ?? NSNull(), "link": value.libraryName,
                         "swatch": model.state.swatchArgb])
        }
        UITestHarness.hooks["controls.savedPreset"] = { [self] _ in
            return json(core.libraryGradients().filter { $0.name == "Pending Save" }.map {
                ["name": $0.name, "colors": $0.gradient.stops.map(\.argb)]
            })
        }
        resetWindow()
        UITestHarness.startIfRequested()
    }
}

@main struct ColorSheetChecks {
    @MainActor static func main() {
        precondition(UITestHarness.isActive)
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let delegate = ColorSheetDelegate()
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
}
