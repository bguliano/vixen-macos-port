import AppKit
import VixenBridge
import VixenKit

#if canImport(VixenTimeline)
import VixenTimeline

/// Holds a menu item's closure: the item keeps it alive as its represented object.
private final class MenuAction: NSObject {
    private let run: () -> Void

    init(run: @escaping () -> Void) {
        self.run = run
    }

    @objc func fire() {
        run()
    }
}

/// A menu item that runs a closure.
@MainActor
private func ActionItem(
    _ title: String,
    image: String? = nil,
    symbol: String? = nil,
    key: String = "",
    modifiers: NSEvent.ModifierFlags = [],
    tooltip: String? = nil,
    enabled: Bool = true,
    run: @escaping () -> Void
) -> NSMenuItem {
    let action = MenuAction(run: run)
    let item = NSMenuItem(title: title, action: #selector(MenuAction.fire), keyEquivalent: key)
    item.target = action
    item.representedObject = action
    item.keyEquivalentModifierMask = modifiers
    item.toolTip = tooltip
    item.isEnabled = enabled
    if let image {
        item.image = menuImage(image)
    } else if let symbol {
        item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
    }
    return item
}

@MainActor
private func menuImage(_ path: String) -> NSImage? {
    guard let source = VixenResources.image(path), let copy = source.copy() as? NSImage else { return nil }
    copy.size = NSSize(width: 16, height: 16)
    return copy
}

private func sequencerImage(_ name: String) -> String {
    "images/sequencer/\(name).png"
}

/// `timelineControl_ContextSelected`: F226 to F254. Everything it shows comes from one read of Rust's `ShellContextMenu`, taken as it opens.
@MainActor
struct EffectContextMenu {
    let model: SequencerModel
    let shell: SequencerShell
    let canvas: TimelineCanvas
    let elements: [UInt64]
    let timeTicks: Int64
    let row: UInt32

    private var timeMs: Double {
        Double(timeTicks) / 10_000
    }

    func build() -> NSMenu {
        let info = shell.contextMenu(row: row, underPointer: elements)
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(addEffectItem(info))
        if let layers = info.layers {
            menu.addItem(layerItem(layers))
        }
        menu.addItem(alignmentItem(info))
        if info.showManipulation {
            menu.addItem(manipulationItem(info))
        }
        if info.showEditTime {
            menu.addItem(ActionItem("Edit Time", image: sequencerImage("clock_edit"), tooltip: info.editTimeTooltip.isEmpty ? nil : info.editTimeTooltip, enabled: info.editTimeEnabled) {
                guard let element = info.element, let editor = shell.editTime(clicked: element) else { return }
                model.sheet = .editTime(clicked: element, editor: editor)
            })
        }
        menu.addItem(.separator())
        menu.addItem(ActionItem("Cut", symbol: "scissors", key: "x", modifiers: .command, enabled: info.cutCopyEnabled) { model.cut() })
        menu.addItem(ActionItem("Copy", symbol: "doc.on.doc", key: "c", modifiers: .command, enabled: info.cutCopyEnabled) { model.copy() })
        menu.addItem(ActionItem("Paste", symbol: "doc.on.clipboard", key: "v", modifiers: .command, enabled: info.pasteEnabled) { model.paste(.default) })
        addPasteSpecials(info, to: menu)
        if info.showDelete {
            menu.addItem(ActionItem("Delete Effect(s)", symbol: "trash", key: "\u{8}", modifiers: []) { model.run(.deleteSelected) })
        }
        if !info.collections.isEmpty {
            menu.addItem(.separator())
            menu.addItem(collectionsItem(info))
        }
        menu.addItem(.separator())
        menu.addItem(ActionItem("Add Marks to Effects", image: sequencerImage("marks")) {
            model.afterCommand(shell.addMarksToEffects())
        })
        return menu
    }

    /// F226.
    private func addEffectItem(_ info: ShellContextMenu) -> NSMenuItem {
        let parent = NSMenuItem(title: "Add Effect(s)", action: nil, keyEquivalent: "")
        parent.image = menuImage(sequencerImage("effects"))
        parent.isEnabled = info.addEffectEnabled
        parent.toolTip = info.addEffectTooltip.isEmpty ? nil : info.addEffectTooltip
        let submenu = NSMenu()
        submenu.autoenablesItems = false
        for (index, group) in info.addEffectGroups.enumerated() {
            if index > 0 {
                submenu.addItem(.separator())
            }
            for effect in group {
                submenu.addItem(ActionItem(effect.name, image: "effects/\(effect.icon).png", tooltip: "Use Shift key to add multiple effects of the same type.") {
                    addEffect(effect)
                })
            }
        }
        parent.submenu = submenu
        return parent
    }

    /// Exactly ⇧ or ⇧⌘ opens Add Multiple. Otherwise one at the grid time.
    private func addEffect(_ effect: ShellEffect) {
        let flags = NSEvent.modifierFlags.intersection([.shift, .command, .option, .control])
        if flags == .shift || flags == [.shift, .command] || model.testShift {
            let dialog = shell.addMultiple(effectName: effect.name, timeMs: timeMs)
            model.sheet = .addMultiple(dialog: dialog, typeId: effect.typeId, row: row)
            return
        }
        _ = shell.addEffectAt(typeId: effect.typeId, row: row, timeMs: timeMs)
        model.refresh()
    }

    /// F227.
    private func layerItem(_ layers: ShellLayerMenu) -> NSMenuItem {
        let parent = NSMenuItem(title: "Layer", action: nil, keyEquivalent: "")
        parent.image = menuImage(sequencerImage("layers"))
        parent.toolTip = "Assign effects to a layer"
        let submenu = NSMenu()
        submenu.autoenablesItems = false
        for layer in layers.items {
            let item = ActionItem(layer.name, tooltip: layer.filterName) { [elements] in
                shell.assignLayer(underPointer: elements, layer: layer.id)
                model.refresh()
            }
            if layer.inUse {
                item.state = layers.singleInUse ? .on : .mixed
            }
            submenu.addItem(item)
        }
        parent.submenu = submenu
        return parent
    }

    /// `S` and so on, exact Shift being the alternate behaviour, read as the item fires.
    private func align(_ kind: TimelineAlign) {
        let hold = NSEvent.modifierFlags.intersection([.shift, .command, .option, .control]) == .shift
        canvas.apply(canvas.controller.align(kind: kind, reference: elements.first, hold: hold))
        model.refresh()
    }

    /// F228 to F240.
    private func alignmentItem(_ info: ShellContextMenu) -> NSMenuItem {
        let parent = NSMenuItem(title: "Alignment", action: nil, keyEquivalent: "")
        parent.image = menuImage(sequencerImage("alignment"))
        parent.isEnabled = info.alignmentEnabled
        parent.toolTip = info.alignmentTooltip.isEmpty ? nil : info.alignmentTooltip
        let submenu = NSMenu()
        submenu.autoenablesItems = false
        let reference = info.referenceEnabled
        func add(_ title: String, _ image: String, _ key: String, _ tooltip: String?, _ enabled: Bool, _ modifiers: NSEvent.ModifierFlags = [], run: @escaping () -> Void) {
            submenu.addItem(ActionItem(title, image: sequencerImage(image), key: key, modifiers: modifiers, tooltip: tooltip, enabled: enabled, run: run))
        }
        add("Align Start Times", "alignStart", "s", "S - Move start time only\nShift+S - Move start time and hold duration", reference) { align(.start) }
        add("Align End Times", "alignEnd", "e", "E - Move end time only\nShift+E - Move end time and hold duration", reference) { align(.end) }
        add("Align Both Times", "alignBoth", "b", "Move both start and end times to match duration", reference) { align(.both) }
        add("Align Centerpoints", "alignCenter", "c", "Move both start and end times, keeping durations constant", reference) { align(.centers) }
        add("Match Duration", "matchDuration", "u", "U - Make durations the same by adjusting the end time\nShift+U - Make durations the same by ajusting the start time", reference) { align(.durations) }
        add("Align Start to End", "alignStartEnd", "r", "R - Move start time, holding the duration constant\nShift+R - Move start time only", reference) { align(.startToEnd) }
        add("Align End to Start", "alignStartEnd", "n", "N - Move end time, holding the duration constant\nShift+N - Move end time only", reference) { align(.endToStart) }
        add("Distribute Equally", "distribute", "d", "Distribute selected effects equally over the total time span of effects", info.distributeEnabled) { model.run(.distributeEqually) }
        add("Distribute Effects", "distribute", "i", "Customize start and end times", info.distributeEnabled) { model.run(.distributeDialog) }
        add("Align Start to nearest mark", "alignStartMark", "s", nil, info.marksEnabled, [.shift, .command]) { model.run(.alignStartToMarks) }
        add("Align End to nearest mark", "alignEndMark", "e", nil, info.marksEnabled, [.shift, .command]) { model.run(.alignEndToMarks) }
        add("Align Both to nearest mark", "alignBothMark", "b", nil, info.marksEnabled, [.shift, .command]) { model.run(.alignBothToMarks) }
        parent.submenu = submenu
        return parent
    }

    /// F241 to F244.
    private func manipulationItem(_ info: ShellContextMenu) -> NSMenuItem {
        let parent = NSMenuItem(title: "Manipulation", action: nil, keyEquivalent: "")
        let submenu = NSMenu()
        submenu.autoenablesItems = false
        submenu.addItem(ActionItem("Divide at cursor", image: sequencerImage("divide"), key: "d", modifiers: .command) {
            if model.ui?.hasSelection == true {
                canvas.apply(canvas.controller.splitSelectedAt(time: timeTicks))
            } else if let element = info.element {
                canvas.apply(canvas.controller.splitAt(ids: [element], time: timeTicks))
            }
            model.refresh()
        })
        submenu.addItem(ActionItem("Clone", symbol: "doc.on.doc") {
            guard let element = info.element else { return }
            _ = shell.cloneElements(clicked: element)
            model.refresh()
        })
        submenu.addItem(ActionItem("Clone to selected effects", image: sequencerImage("copySelect"), enabled: info.cloneToSelectedEnabled) {
            guard let element = info.element else { return }
            if let prompt = shell.cloneToSelectedPrompt(source: element) {
                model.showAlert(prompt)
            } else {
                shell.cloneToSelected(source: element)
                model.refresh()
            }
        })
        parent.submenu = submenu
        return parent
    }

    /// F249 to F251.
    private func addPasteSpecials(_ info: ShellContextMenu, to menu: NSMenu) {
        let marks = ActionItem("Paste - Visible Mark/s", image: sequencerImage("paste_marks"), enabled: info.pasteChildrenEnabled) { model.paste(.visibleMarks) }
        let invert = ActionItem("Paste - Invert", image: sequencerImage("paste_invert"), enabled: info.pasteChildrenEnabled) { model.paste(.invert) }
        if info.pasteSpecialSubmenu {
            let parent = NSMenuItem(title: "Paste Special", action: nil, keyEquivalent: "")
            parent.image = menuImage(sequencerImage("paste_special"))
            let submenu = NSMenu()
            submenu.autoenablesItems = false
            submenu.addItem(invert)
            submenu.addItem(marks)
            parent.submenu = submenu
            menu.addItem(parent)
        } else if info.pasteMarksAlone {
            menu.addItem(marks)
        } else if info.pasteInvertAlone {
            menu.addItem(invert)
        }
    }

    /// F253.
    private func collectionsItem(_ info: ShellContextMenu) -> NSMenuItem {
        func order(_ title: String, _ image: String, random: Bool) -> NSMenuItem {
            let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
            item.image = menuImage(sequencerImage(image))
            let submenu = NSMenu()
            submenu.autoenablesItems = false
            for collection in info.collections {
                submenu.addItem(ActionItem(collection.name, tooltip: collection.description.isEmpty ? nil : collection.description) {
                    model.afterCommand(shell.applyColorCollection(name: collection.name, random: random))
                })
            }
            item.submenu = submenu
            return item
        }
        let colors = NSMenuItem(title: "Colors", action: nil, keyEquivalent: "")
        colors.image = menuImage(sequencerImage("colors"))
        let colorsMenu = NSMenu()
        colorsMenu.addItem(order("Random", "randomColors", random: true))
        colorsMenu.addItem(order("Sequential", "sequentialColors", random: false))
        colors.submenu = colorsMenu
        let parent = NSMenuItem(title: "Collections", action: nil, keyEquivalent: "")
        parent.image = menuImage(sequencerImage("collection"))
        let parentMenu = NSMenu()
        parentMenu.addItem(colors)
        parent.submenu = parentMenu
        return parent
    }
}

extension SequencerModel {
    func showEffectMenu(elements: [UInt64], time: Int64, row: UInt32, at point: NSPoint, in view: NSView) {
        guard let shell, let canvas else { return }
        let menu = EffectContextMenu(model: self, shell: shell, canvas: canvas, elements: elements, timeTicks: time, row: row).build()
        testContextMenu = menu
        if UITestHarness.isActive {
            testContextPaths = Self.paths(of: menu)
            // A menu's tracking would hold the test's run loop. Its text is written out instead.
            writeContextMenuForTest(menu)
            return
        }
        menu.popUp(positioning: nil, at: point, in: view)
        refresh()
    }

    /// `menus-context.txt`: what the last context menu held, for scripts.
    private func writeContextMenuForTest(_ menu: NSMenu) {
        guard let out = ProcessInfo.processInfo.environment["VIXEN_UI_TEST_OUT"] else { return }
        try? SequencerModel.text(of: menu).write(to: URL(fileURLWithPath: out).appending(path: "menu-context.txt"), atomically: true, encoding: .utf8)
    }

    private static func paths(of menu: NSMenu, prefix: String = "") -> [String] {
        menu.items.flatMap { item -> [String] in
            guard !item.isSeparatorItem else { return [] }
            let path = prefix + item.title
            if let submenu = item.submenu {
                return paths(of: submenu, prefix: path + "/")
            }
            return [path]
        }
    }

    /// Runs an item of the last context menu by its path, for scripts.
    func performContextMenuItem(path: String) {
        var menu = testContextMenu
        let parts = path.components(separatedBy: "/")
        for (index, title) in parts.enumerated() {
            guard let item = menu?.items.first(where: { $0.title == title }) else { return }
            if index == parts.count - 1, let parent = item.menu {
                parent.performActionForItem(at: parent.index(of: item))
            }
            menu = item.submenu
        }
    }
}
#endif
