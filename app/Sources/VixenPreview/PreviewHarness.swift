import AppKit
import VixenBridge
import VixenKit

/// Steps for test scripts, on top of the UI harness's own. Nothing here runs outside a script.
enum PreviewHarness {
    /// `previewTarget`: which preview a script's `window` step opens the designer on, since that step can't pass a value.
    private(set) static var target: String?
    private(set) static var designer: PreviewDesignerModel?
    private static var sequence: SequencerSession?

    static func targetPreviewID(app: AppModel) -> String? {
        guard UITestHarness.isActive, let core = app.core else { return nil }
        let previews = core.previews()
        return (target.flatMap { name in previews.first { $0.name == name } } ?? previews.first)?.id
    }

    static func designerOpened(_ model: PreviewDesignerModel) {
        designer = model
    }

    static func install(model: AppModel) {
        guard UITestHarness.isActive, UITestHarness.customActions["previewTarget"] == nil else { return }
        UITestHarness.staysOnScreen = { window in
            LivePreviewCoordinator.shared?.controllers.values.contains { $0.window === window && $0.keepOnScreen } == true || (designerOnScreen && designer?.window === window)
        }
        let actions = UITestHarness.customActions
        var added = actions
        added["previewTarget"] = { step in
            target = step.text
            return (true, nil)
        }
        added["liveWindowInfo"] = { _ in
            let lines = LivePreviewCoordinator.shared?.controllers.values.map { c -> String in
                let content = c.window.contentRect(forFrameRect: c.window.frame).size
                let stats = c.live?.stats()
                return "\(c.window.title) camera=\(c.isCamera) content=\(Int(content.width))x\(Int(content.height)) lights=\(stats?.lightCount ?? 0) sceneLights=\(stats?.sceneLights ?? 0) frames=\(stats?.framesRendered ?? 0) error=\(stats?.lastError ?? "none")"
            } ?? []
            return (!lines.isEmpty, lines.sorted().joined(separator: " | "))
        }
        added["designerInfo"] = { _ in
            guard let m = designer, let canvas = m.canvas, let s = m.designer?.settings() else { return (false, "no designer") }
            let frame = canvas.convert(canvas.bounds, to: nil)
            let height = m.window?.contentView?.bounds.height ?? 0
            let toolbar = m.toolbar
            let flagsText = "lock=\(toolbar.lock) unlock=\(toolbar.unlock) unlockAll=\(toolbar.unlockAll) align=\(toolbar.align) bulb=\(toolbar.bulb) shown=\(String(describing: m.properties.shape))"
            let info = flagsText + " canvas x=\(Int(frame.minX)) y=\(Int(height - frame.maxY)) w=\(Int(frame.width)) h=\(Int(frame.height)) shapes=\(s.shapeCount) lights=\(s.lightCount) linked=\(m.designer?.linkedLightCountForTest() ?? 0) bg=\(s.backgroundWidth)x\(s.backgroundHeight) blank=\(s.backgroundIsBlank) zoom=\(m.zoom) openMs=\(String(format: "%.1f", m.openMilliseconds)) tool=\(m.toolbar.tool) undo=\(m.undoLabels) panel=\(m.properties.title) rows=\(m.properties.rows.count) offset=\(s.locationOffsetX),\(s.locationOffsetY)"
            return (true, info)
        }
        added["designerMenuInfo"] = { _ in
            guard let m = designer else { return (false, "no designer") }
            func lines(_ menu: NSMenu?) -> [String] {
                (menu?.items ?? []).flatMap { item in
                    let entry = ["Undo", "Redo", "Save"].contains(where: item.title.hasPrefix)
                        ? ["\(item.title) key=\(item.keyEquivalent) enabled=\(item.isEnabled) action=\(String(describing: item.action)) target=\(String(describing: item.target))"] : []
                    return entry + lines(item.submenu)
                }
            }
            let info = "key=\(NSApp.keyWindow?.title ?? "nil") active=\(DesignerRegistry.shared.active === m) responder=\(String(describing: m.window?.firstResponder)) canUndo=\(m.toolbar.canUndo) windowUndo=\(String(describing: m.window?.undoManager)) canvasUndo=\(String(describing: m.canvas?.undoManager)) target=\(String(describing: NSApp.target(forAction: Selector(("undo:")))))"
            return (true, ([info] + lines(NSApp.mainMenu)).joined(separator: "\n"))
        }
        added["designerTool"] = { step in
            guard let m = designer, let name = step.text else { return (false, "no designer") }
            guard let tool = toolNames[name] else { return (false, "no tool \(name)") }
            m.choose(tool)
            return (true, nil)
        }
        added["designerCommand"] = { step in
            guard let m = designer, let name = step.text else { return (false, "no designer") }
            switch name {
            case "undo": m.undo()
            case "redo": m.redo()
            case "cut": m.cut()
            case "copy": m.copy()
            case "paste": m.paste()
            case "delete": m.deleteSelection()
            case "selectAll": m.selectAll()
            case "lock": m.apply(m.designer?.lock())
            case "unlock": m.apply(m.designer?.unlock())
            case "unlockAll": m.apply(m.designer?.unlockAll())
            case "hideLocked": m.toggleHideLocked()
            case "increaseBulb": m.apply(m.designer?.increaseBulb())
            case "decreaseBulb": m.apply(m.designer?.decreaseBulb())
            case "group": m.perform(.group)
            case "ungroup": m.perform(.ungroup)
            case "propInfo": m.showInfo.toggle()
            case "save": m.save()
            case "alignLeft": m.align(.left)
            case "alignRight": m.align(.right)
            case "alignTop": m.align(.top)
            case "alignBottom": m.align(.bottom)
            case "alignHorizontally": m.align(.horizontally)
            case "alignVertically": m.align(.vertically)
            case "distributeHorizontally": m.align(.distributeHorizontally)
            case "distributeVertically": m.align(.distributeVertically)
            case "matchProperties": m.align(.matchProperties)
            case "linkFirst":
                guard let shape = m.properties.shape else { return (false, "no shape shown") }
                m.openLinker(shape: shape)
            case "fixtureLink":
                guard let row = m.properties.rows.first(where: { if case .linkedElement = $0.value { return true }; return false }),
                      case .linkedElement(let node) = row.value else { return (false, "no fixture link property") }
                m.properties.editFixtureLink(current: node)
            case "resizeSheet": m.sheet = .resizeBackground
            case "offsetSheet": m.sheet = .locationOffset
            case "singleSheet": m.sheet = .singleBulb
            case "templatesSheet": m.sheet = .templates
            case "floatElements": m.setFloating(.elements, true)
            case "floatProperties": m.setFloating(.properties, true)
            case "dockElements": m.setFloating(.elements, false)
            case "dockProperties": m.setFloating(.properties, false)
            case "hideProperties": m.showProperties = false
            case "showProperties": m.showProperties = true
            default:
                // "importProp /path/to/file.prp": that prop file at (20, 20), every naming token answered "1".
                if name.hasPrefix("importProp ") {
                    await m.importPropFiles([String(name.dropFirst("importProp ".count))], x: 20, y: 20, atDrop: false, answer: { _ in "1" })
                    return (true, nil)
                }
                // "importPropAsking /path/to/file.prp": the same with every prompt shown, for the script to answer.
                if name.hasPrefix("importPropAsking ") {
                    let path = String(name.dropFirst("importPropAsking ".count))
                    Task { await m.importPropFiles([path], x: 20, y: 20, atDrop: false) }
                    return (true, nil)
                }
                // "treeMenu Tags/Manage Tags...": that item of the Elements tree's context menu, for the rows selected now.
                if name.hasPrefix("treeMenu ") {
                    func find(_ path: String, in menu: NSMenu?) -> NSMenuItem? {
                        if let exact = menu?.items.first(where: { $0.title == path }) { return exact }
                        for item in menu?.items ?? [] where path.hasPrefix(item.title + "/") {
                            if let found = find(String(path.dropFirst(item.title.count + 1)), in: item.submenu) { return found }
                        }
                        return nil
                    }
                    guard let item = find(String(name.dropFirst("treeMenu ".count)), in: m.elementMenu()) else { return (false, "no menu item") }
                    guard item.isEnabled, let action = item.action else { return (false, "\(item.title) is off") }
                    NSApp.sendAction(action, to: item.target, from: item)
                    return (true, nil)
                }
                // "treeSelect Relays/Star": that row by its names from a root down, with its parents opened.
                if name.hasPrefix("treeSelect ") {
                    guard let d = m.designer else { return (false, "no designer") }
                    var path: [String] = []
                    var opened: [String] = []
                    for title in name.dropFirst("treeSelect ".count).split(separator: "/") {
                        guard let row = d.elementRows(parent: path.last).first(where: { $0.name == title }) else { return (false, "no row \(title)") }
                        if !path.isEmpty {
                            opened.append(path.joined(separator: "/"))
                        }
                        path.append(row.id)
                    }
                    await m.treeHandle.select([path.joined(separator: "/")], expanding: opened)
                    return (true, nil)
                }
                // "treeDrop Relays/Star > Relays/Plug 11": the first row dragged onto the second ("root" for the top level), as a drop on the row itself.
                if name.hasPrefix("treeDrop ") {
                    guard let d = m.designer, let editor = m.treeEditor else { return (false, "no tree editor") }
                    func rowPath(_ names: Substring) -> String? {
                        var path: [String] = []
                        for title in names.split(separator: "/") {
                            guard let row = d.elementRows(parent: path.last).first(where: { $0.name == title }) else { return nil }
                            path.append(row.id)
                        }
                        return path.joined(separator: "/")
                    }
                    let parts = name.dropFirst("treeDrop ".count).split(separator: ">").map { $0.trimmingCharacters(in: .whitespaces) }
                    guard parts.count == 2, let source = rowPath(parts[0][...]) else { return (false, "no source row") }
                    let target = parts[1] == "root" ? nil : rowPath(parts[1][...])
                    guard parts[1] == "root" || target != nil else { return (false, "no target row") }
                    let copy = step.label == "copy"
                    guard editor.canDrop(items: [source], parent: target, index: nil, copy: copy) else { return (false, "refused") }
                    editor.drop(items: [source], parent: target, index: nil, copy: copy)
                    return (true, nil)
                }
                // "treeMenuDump": the menu as text, with what's off and what's ticked.
                if name == "treeMenuDump" {
                    func lines(_ menu: NSMenu?, _ indent: String) -> [String] {
                        (menu?.items ?? []).flatMap { item -> [String] in
                            let mark = item.state == .on ? " [on]" : item.state == .mixed ? " [mixed]" : ""
                            let line = item.isSeparatorItem ? "\(indent)---" : "\(indent)\(item.title)\(item.isEnabled ? "" : " (off)")\(mark)"
                            return [line] + lines(item.submenu, indent + "  ")
                        }
                    }
                    let text = lines(m.elementMenu(), "").joined(separator: "\n")
                    UITestHarness.note("tree menu:\n\(text)")
                    return (true, text.isEmpty ? "no menu" : nil)
                }
                if name.hasPrefix("treeKey ") {
                    let commands: [String: OutlineCommand] = ["copy": .copy, "paste": .paste, "pasteAsNew": .pasteAsNew, "delete": .delete]
                    guard let command = commands[String(name.dropFirst("treeKey ".count))] else { return (false, "no key") }
                    guard m.canPerformOnElements(command) else { return (false, "off") }
                    m.performOnElements(command)
                    return (true, nil)
                }
                // "addTemplate Arch": the Elements panel's Add button with that template chosen.
                guard name.hasPrefix("addTemplate ") else { return (false, "no command \(name)") }
                let template = String(name.dropFirst("addTemplate ".count))
                Task { await m.addElementTemplate(name: template) }
            }
            return (true, nil)
        }
        // A left drag on the canvas from `x`,`y` to `toX`,`toY` in view points, through the same calls the canvas makes.
        added["designerDrag"] = { step in
            guard let m = designer, let d = m.designer else { return (false, "no designer") }
            let none = DesignerMods(command: false, shift: false, option: false)
            let (x, y) = (Int32(step.x ?? 0), Int32(step.y ?? 0))
            let (dx, dy) = (Int32(step.toX ?? 0) - x, Int32(step.toY ?? 0) - y)
            m.apply(d.mouseDown(button: .left, x: x, y: y, mods: none))
            for i in 1...4 {
                m.apply(d.mouseMove(x: x + dx * Int32(i) / 4, y: y + dy * Int32(i) / 4, mods: none))
            }
            m.apply(d.mouseUp(button: .left, x: x + dx, y: y + dy, mods: none))
            return (true, nil)
        }
        added["designerClickLight"] = { step in
            guard let m = designer, let d = m.designer, let p = d.lightViewPointForTest(light: UInt32(step.index ?? 0)) else { return (false, "no such light") }
            let none = DesignerMods(command: false, shift: false, option: false)
            m.apply(d.mouseDown(button: .left, x: p.x, y: p.y, mods: none))
            m.apply(d.mouseUp(button: .left, x: p.x, y: p.y, mods: none))
            return (true, "at \(p.x),\(p.y)")
        }
        added["waitVisible"] = { step in
            let window = step.text == "designer" ? designer?.window : liveController(step.text)?.window
            guard let window else { return (false, "no window") }
            let deadline = Date().addingTimeInterval(step.seconds ?? 10)
            while !window.occlusionState.contains(.visible), Date() < deadline {
                try? await Task.sleep(for: .milliseconds(100))
            }
            let visible = window.occlusionState.contains(.visible)
            return (visible, "visible \(visible)")
        }
        added["designerResponder"] = { step in
            // Down the real responder chain, from whatever has the keyboard, since a test window is never key and the menu bar can't reach it.
            guard let window = designer?.window, let action = step.text else { return (false, "no designer") }
            let done = window.firstResponder?.tryToPerform(Selector(action), with: nil) ?? false
            return (done, done ? nil : "nothing took \(action)")
        }
        added["designerKeyEvent"] = { step in
            guard let m = designer, let window = m.window, let canvas = m.canvas,
                  let characters = step.text else { return (false, "needs designer and characters") }
            window.makeMain()
            window.makeKeyAndOrderFront(nil)
            window.makeFirstResponder(canvas)
            // Focus changes rebuild SwiftUI Commands asynchronously; a human key press follows that update.
            try? await Task.sleep(for: .milliseconds(200))
            let keyBefore = NSApp.keyWindow?.title ?? "nil"
            let undoTarget = String(describing: NSApp.target(forAction: Selector(("undo:"))))
            let fileSave = NSApp.mainMenu?.item(withTitle: "File")?.submenu?.item(withTitle: "Save") != nil
            let sequenceSave = NSApp.mainMenu?.item(withTitle: "Sequence")?.submenu?.item(withTitle: "Save")?.keyEquivalent ?? ""
            var flags: NSEvent.ModifierFlags = []
            for value in (step.label ?? "").split(separator: " ") {
                switch value {
                case "cmd": flags.insert(.command)
                case "shift": flags.insert(.shift)
                case "option": flags.insert(.option)
                default: break
                }
            }
            for down in [true, false] {
                // Keep the CG backing event: SwiftUI key equivalents inspect more than the character strings.
                guard let cg = CGEvent(keyboardEventSource: nil, virtualKey: UInt16(clamping: step.index ?? 0), keyDown: down) else {
                    return (false, "no keyboard event")
                }
                cg.flags = CGEventFlags(rawValue: UInt64(flags.rawValue))
                guard let event = NSEvent(cgEvent: cg) else { return (false, "no AppKit key event") }
                NSApp.postEvent(event, atStart: false)
            }
            try? await Task.sleep(for: .milliseconds(200))
            return (true, "posted native CG-backed key \(characters) keyBefore=\(keyBefore) keyAfter=\(NSApp.keyWindow?.title ?? "nil") main=\(NSApp.mainWindow?.title ?? "nil") undoTarget=\(undoTarget) previewActive=\(DesignerRegistry.shared.active === m) fileSave=\(fileSave) sequenceSaveKey=\(sequenceSave)")
        }
        added["designerBackground"] = { step in
            guard let m = designer, let path = step.text else { return (false, "no designer") }
            m.setBackground(path: path)
            return (true, nil)
        }
        added["designerZoom"] = { step in
            guard let m = designer, let z = step.x else { return (false, "no designer") }
            m.apply(m.designer?.setZoomLevel(zoom: z))
            return (true, nil)
        }
        added["designerOnScreen"] = { step in
            guard let m = designer, let window = m.window else { return (false, "no designer") }
            designerOnScreen = true
            putOnScreen(window, x: step.x, y: step.y)
            try? await Task.sleep(for: .milliseconds(300))
            return (window.occlusionState.contains(.visible), "visible \(window.occlusionState.contains(.visible))")
        }
        added["liveOnScreen"] = { step in
            guard let c = liveController(step.text) else { return (false, "no window") }
            c.keepOnScreen = true
            putOnScreen(c.window, x: step.x, y: step.y)
            try? await Task.sleep(for: .milliseconds(300))
            return (c.window.occlusionState.contains(.visible), "visible \(c.window.occlusionState.contains(.visible))")
        }
        added["liveMenu"] = { step in
            guard let c = liveController(step.text), let label = step.label else { return (false, "no window") }
            switch label {
            case "Show Status": c.toggleStatus()
            case "Show Borders": c.toggleBorders()
            case "Lock Position": c.toggleLockPosition()
            case "Always On Top": c.toggleAlwaysOnTop()
            case "Auto On Top": c.toggleAutoOnTop()
            case "Transparent Background": c.toggleTransparent()
            case "Reset Size": c.resetSize()
            case "Reset Zoom": c.resetZoom()
            case "Center Preview": c.centerPreview()
            case "Enable Light Scaling": c.toggleLightScaling()
            default: return (false, "no item \(label)")
            }
            return (true, nil)
        }
        added["playSequence"] = { step in
            guard let core = model.core, let path = step.text else { return (false, "needs a path") }
            do {
                let opened = try await Task.detached { try core.openSequence(path: path) }.value
                sequence = opened
                while opened.renderStatus().waiting > 0 {
                    try await Task.sleep(for: .milliseconds(50))
                }
                opened.play(startMs: (step.x ?? 0) * 1000, endMs: opened.lengthMs(), looping: false)
                return (true, "length \(Int(opened.lengthMs())) ms")
            } catch {
                return (false, error.userMessage)
            }
        }
        added["stopSequence"] = { _ in
            sequence?.stop()
            sequence = nil
            return (true, nil)
        }
        added["measureLive"] = { step in
            guard let c = liveController(step.text), let live = c.live else { return (false, "no window") }
            let seconds = step.seconds ?? 10
            let start = live.stats()
            let cpuStart = processCPUSeconds()
            let wallStart = Date()
            var stalls: [Double] = []
            let deadline = wallStart.addingTimeInterval(seconds)
            // The main thread's own gaps, as a stall would show in the UI: a 5 ms tick, late by however long it was held up.
            while Date() < deadline {
                let before = Date()
                try? await Task.sleep(for: .milliseconds(5))
                stalls.append(Date().timeIntervalSince(before) * 1000)
            }
            let end = live.stats()
            let wall = Date().timeIntervalSince(wallStart)
            let cpu = processCPUSeconds() - cpuStart
            let gaps = live.frameGapsMs().suffix(Int(end.framesRendered - start.framesRendered)).dropFirst()
            let line = "\(c.window.title): \(summary(frames: end.framesRendered - start.framesRendered, wall: wall, gaps: Array(gaps))) engineFrames \(end.engineFrames - start.engineFrames) changedFrames \(end.changedFrames - start.changedFrames) colourFramesTaken \(end.colorsTaken - start.colorsTaken) occluded \(end.framesOccluded - start.framesOccluded) dropped \(end.framesDropped - start.framesDropped) timedOut \(end.framesTimedOut - start.framesTimedOut) lastFrameCpu \(String(format: "%.3f", end.lastFrameCpuMs)) ms processCpu \(String(format: "%.1f", cpu / wall * 100))% mainTickP99 \(String(format: "%.1f", percentile(stalls, 0.99))) ms mainTickMax \(String(format: "%.1f", stalls.max() ?? 0)) ms"
            UITestHarness.note(line)
            return (true, line)
        }
        added["appPolicy"] = { step in
            switch step.text {
            case "regular": NSApp.setActivationPolicy(.regular)
            case "noNap": napActivity = ProcessInfo.processInfo.beginActivity(options: [.userInitiated, .latencyCritical], reason: "Measuring a preview")
            default: return (false, "no such policy")
            }
            return (true, nil)
        }
        added["liveRun"] = { step in
            guard let core = model.core, let name = step.text else { return (false, "no core") }
            guard let id = core.previews().first(where: { $0.name == name })?.id else { return (false, "no preview \(name)") }
            do {
                return (true, "running \(try core.livePreviewSetRunning(id: id, running: step.label != "stop"))")
            } catch {
                return (false, error.userMessage)
            }
        }
        added["liveSize"] = { step in
            guard let c = liveController(step.text), let w = step.x, let h = step.y else { return (false, "no window") }
            c.window.setContentSize(NSSize(width: w, height: h))
            return (true, nil)
        }
        added["designerProperty"] = { step in
            // Commits `text` to the shown shape's row `label` (its id or display name), as the grid does on Return.
            guard let m = designer, let key = step.label, let text = step.text else { return (false, "no designer") }
            let panel = m.properties
            guard let row = panel.rows.first(where: { $0.id == key || $0.name == key }) else {
                return (false, "no row \(key) in \(panel.rows.map(\.id))")
            }
            let value: DesignerPropertyValue
            switch row.kind {
            case .int:
                guard let v = Int32(text) else { return (false, "not a whole number") }
                value = .int(value: v)
            case .double:
                guard let v = Double(text) else { return (false, "not a number") }
                value = .double(value: v)
            case .bool:
                value = .bool(value: text.lowercased() == "true")
            case .enumeration:
                value = .enumeration(value: text)
            case .text:
                value = .text(value: text)
            case .point:
                let parts = text.split(separator: ",").compactMap { Int32($0.trimmingCharacters(in: .whitespaces)) }
                guard parts.count == 2 else { return (false, "needs x, y") }
                value = .point(x: parts[0], y: parts[1])
            default:
                return (false, "row \(key) isn't edited in place")
            }
            panel.commit(id: row.id, value: value)
            return (true, "\(row.id) = \(text)")
        }
        added["designerProperties"] = { _ in
            guard let panel = designer?.properties else { return (false, "no designer") }
            let values = panel.rows.map { "\($0.id)=\($0.value)" }.joined(separator: " | ")
            return (true, "group=\(panel.isGroup) string=\(panel.selectedString) names=\(panel.stringNames) | \(values)")
        }
        added["designerLinkerInfo"] = { step in
            guard case .link(let linker, _) = designer?.sheet else { return (false, "no linker") }
            let rows: [[String: Any]] = linker.rows(string: UInt32(step.index ?? 0)).map {
                ["number": $0.number, "name": $0.elementName, "node": $0.node ?? ""]
            }
            guard let data = try? JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys]),
                  let text = String(data: data, encoding: .utf8) else { return (false, "linker serialization failed") }
            return (true, text)
        }
        added["designerPixels"] = { step in
            // What the renderer last presented, which a snapshot can't see. The window has to be on screen, or nothing was drawn.
            guard let m = designer, let d = m.designer, let canvas = m.canvas else { return (false, "no designer") }
            try? await Task.sleep(for: .milliseconds(150))
            let width = Int(canvas.metal.pixelSize.width)
            let pixels = await Task.detached { d.readPixelsForTest() }.value
            return writePNG(pixels, width: width, name: step.name ?? "designer-pixels")
        }
        added["livePixels"] = { step in
            guard let c = liveController(step.text), let live = c.live else { return (false, "no window") }
            try? await Task.sleep(for: .milliseconds(150))
            let width = Int(c.canvas.metal.pixelSize.width)
            let pixels = await Task.detached { live.readPixelsForTest() }.value
            return writePNG(pixels, width: width, name: step.name ?? "live-pixels")
        }
        added["designerContextMenu"] = { step in
            // The right press at canvas point (x, y), then the menu it builds, written out, and the item `label` chosen if given ("Set Z Location to/3" for a sub-item).
            // `popUpContextMenu` runs its own loop, which a script can't get past.
            guard let m = designer, let canvas = m.canvas else { return (false, "no designer") }
            let mods = DesignerMods(command: step.text?.contains("cmd") == true, shift: step.text?.contains("shift") == true, option: false)
            guard let menu = canvas.contextMenu(atX: Int32(step.x ?? 0), y: Int32(step.y ?? 0), mods: mods) else {
                UITestHarness.note("context menu: none")
                return (step.label == nil, "no menu")
            }
            let listing = describe(menu)
            UITestHarness.note("context menu: \(listing)")
            guard let label = step.label else { return (true, listing) }
            return choose(label, in: menu, listing: listing)
        }
        added["liveContextMenu"] = { step in
            guard let c = liveController(step.text) else { return (false, "no window") }
            let menu = c.contextMenu()
            let listing = describe(menu)
            UITestHarness.note("live menu \(c.window.title): \(listing)")
            guard let label = step.label else { return (true, listing) }
            return choose(label, in: menu, listing: listing)
        }
        added["measureDrag"] = { step in
            await PreviewBenchmark.drag(step)
        }
        added["measureOpen"] = { step in
            await PreviewBenchmark.open(step, app: model)
        }
        UITestHarness.customActions = added
    }

    private(set) static var designerOnScreen = false
    private static var napActivity: NSObjectProtocol?

    /// Titles in order, `-` for a separator, `[x]` for a checked item, `(off)` for a disabled one, and a submenu's items in braces.
    private static func describe(_ menu: NSMenu) -> String {
        menu.items.map { item -> String in
            if item.isSeparatorItem { return "-" }
            var text = item.title
            if item.state == .on { text = "[x] " + text }
            if !item.isEnabled { text += " (off)" }
            if let sub = item.submenu { text += " {" + describe(sub) + "}" }
            return text
        }.joined(separator: " | ")
    }

    private static func choose(_ label: String, in menu: NSMenu, listing: String) -> (Bool, String?) {
        let path = label.split(separator: "/").map(String.init)
        var current: NSMenu? = menu
        var found: NSMenuItem?
        for part in path {
            found = current?.items.first { $0.title == part }
            current = found?.submenu
        }
        guard let item = found, let action = item.action, item.isEnabled else { return (false, "no item \(label) in \(listing)") }
        NSApp.sendAction(action, to: item.target, from: item)
        return (true, listing)
    }

    private static func writePNG(_ pixels: Data, width: Int, name: String) -> (Bool, String?) {
        guard width > 0, !pixels.isEmpty, pixels.count % (width * 4) == 0 else { return (false, "nothing drawn yet (\(pixels.count) bytes)") }
        let height = pixels.count / (width * 4)
        guard let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: width * 4, bitsPerPixel: 32
        ), let data = rep.bitmapData else { return (false, "no bitmap") }
        pixels.copyBytes(to: data, count: pixels.count)
        let out = URL(fileURLWithPath: ProcessInfo.processInfo.environment["VIXEN_UI_TEST_OUT"] ?? NSTemporaryDirectory())
        guard let png = rep.representation(using: .png, properties: [:]) else { return (false, "no png") }
        do {
            try png.write(to: out.appending(path: "\(name).png"))
            return (true, "\(width)x\(height)")
        } catch {
            return (false, "\(error)")
        }
    }

    /// In front of whatever the user is doing, on whichever Space that is, for as long as a measurement runs. A hidden window draws nothing.
    private static func putOnScreen(_ window: NSWindow, x: Double?, y: Double?) {
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        window.level = .floating
        window.setFrameOrigin(NSPoint(x: x ?? 40, y: y ?? 40))
        window.orderFrontRegardless()
    }

    private static func liveController(_ name: String?) -> LivePreviewController? {
        let all = LivePreviewCoordinator.shared?.controllers.values
        guard let name else { return all?.first }
        return all?.first { $0.window.title == name }
    }

    private static let toolNames: [String: DesignerTool] = [
        "select": .select, "single": .single, "line": .light, "rectangle": .rectangle, "ellipse": .ellipse, "triangle": .triangle, "polyLine": .polyLine,
        "arch": .arch, "net": .net, "megaTree": .megaTree, "starBurst": .starBurst, "multiString": .multiString, "cane": .cane, "movingHead": .movingHead,
        "star": .star, "pixelGrid": .pixelGrid, "icicle": .icicle,
    ]

    static func percentile(_ values: [Double], _ p: Double) -> Double {
        guard !values.isEmpty else { return 0 }
        let sorted = values.sorted()
        return sorted[min(Int(Double(sorted.count) * p), sorted.count - 1)]
    }

    static func summary(frames: UInt64, wall: Double, gaps: [Double]) -> String {
        "\(frames) frames in \(String(format: "%.1f", wall)) s (\(String(format: "%.1f", Double(frames) / wall)) fps), gaps p50 \(String(format: "%.1f", percentile(gaps, 0.5))) p99 \(String(format: "%.1f", percentile(gaps, 0.99))) max \(String(format: "%.1f", gaps.max() ?? 0)) ms"
    }

    static func processCPUSeconds() -> Double {
        var usage = rusage()
        getrusage(RUSAGE_SELF, &usage)
        return Double(usage.ru_utime.tv_sec) + Double(usage.ru_utime.tv_usec) / 1e6 + Double(usage.ru_stime.tv_sec) + Double(usage.ru_stime.tv_usec) / 1e6
    }
}

/// Timed runs for docs/benchmarks/preview-ui.md. Each drives the same FFI calls the canvas makes, because a test window's events can't keep to the display's rate on their own.
enum PreviewBenchmark {
    /// A drag of what's selected, or a rubber band: `seconds` long, one move per display frame, moving between (x, y) and (toX, toY) in canvas points.
    static func drag(_ step: UITestStep) async -> (Bool, String?) {
        guard let m = PreviewHarness.designer, let designer = m.designer else { return (false, "no designer") }
        var (x0, y0, x1, y1) = (Int32(step.x ?? 100), Int32(step.y ?? 100), Int32(step.toX ?? 400), Int32(step.toY ?? 300))
        // `label: "light"`: start on light `index`, and read toX and toY as how far to go from there.
        if step.label == "light" {
            guard let p = designer.lightViewPointForTest(light: UInt32(step.index ?? 0)) else { return (false, "no such light") }
            (x0, y0, x1, y1) = (p.x, p.y, p.x + Int32(step.toX ?? 40), p.y + Int32(step.toY ?? 30))
        }
        let seconds = step.seconds ?? 5
        let none = DesignerMods(command: false, shift: false, option: false)
        let startStats = designer.stats()
        let cpuStart = PreviewHarness.processCPUSeconds()
        let wallStart = Date()
        var down: [Double] = []
        var moves: [Double] = []
        func timed(_ call: () -> DesignerChange) -> Double {
            let t = DispatchTime.now().uptimeNanoseconds
            m.apply(call())
            return Double(DispatchTime.now().uptimeNanoseconds - t) / 1e6
        }
        m.mouseIsDown = true
        down.append(timed { designer.mouseDown(button: .left, x: x0, y: y0, mods: none) })
        let frame = 1.0 / 120.0
        var i = 0
        while Date().timeIntervalSince(wallStart) < seconds {
            let t = (sin(Double(i) * frame * 2) + 1) / 2
            let x = x0 + Int32(Double(x1 - x0) * t)
            let y = y0 + Int32(Double(y1 - y0) * t)
            moves.append(timed { designer.mouseMove(x: x, y: y, mods: none) })
            i += 1
            try? await Task.sleep(for: .seconds(frame))
        }
        let up = timed { designer.mouseUp(button: .left, x: x0, y: y0, mods: none) }
        m.mouseWentUp()
        let wall = Date().timeIntervalSince(wallStart)
        let cpu = PreviewHarness.processCPUSeconds() - cpuStart
        let end = designer.stats()
        // The first gap runs from the last frame before the drag, which is idle time, not pacing.
        let gaps = designer.frameGapsMs().suffix(Int(end.framesRendered - startStats.framesRendered)).dropFirst()
        let line = "\(step.name ?? "drag"): \(moves.count) moves, main thread per move p50 \(String(format: "%.3f", PreviewHarness.percentile(moves, 0.5))) p99 \(String(format: "%.3f", PreviewHarness.percentile(moves, 0.99))) max \(String(format: "%.3f", moves.max() ?? 0)) ms, down \(String(format: "%.3f", down.first ?? 0)) ms, up \(String(format: "%.3f", up)) ms, renderer \(PreviewHarness.summary(frames: end.framesRendered - startStats.framesRendered, wall: wall, gaps: Array(gaps))) occluded \(end.framesOccluded - startStats.framesOccluded) lastFrameCpu \(String(format: "%.3f", end.lastFrameCpuMs)) ms processCpu \(String(format: "%.1f", cpu / wall * 100))%"
        UITestHarness.note(line)
        return (true, line)
    }

    /// Opening a designer without a window, as far as the FFI goes: the XML, the tree, the scene and the repair, on a throwaway layer.
    static func open(_ step: UITestStep, app: AppModel) async -> (Bool, String?) {
        guard let core = app.core else { return (false, "no core") }
        let previews = core.previews()
        guard let id = (step.text.flatMap { name in previews.first { $0.name == name } } ?? previews.first)?.id else { return (false, "no preview") }
        let view = MetalLayerNSView(frame: NSRect(x: 0, y: 0, width: 800, height: 500))
        let start = DispatchTime.now().uptimeNanoseconds
        do {
            let opened = try core.openPreviewDesigner(id: id, metalLayer: view.layerAddress, widthPx: 800, heightPx: 500, scale: 1, widthPt: 800, heightPt: 500)
            let openMs = Double(DispatchTime.now().uptimeNanoseconds - start) / 1e6
            let decodeStart = DispatchTime.now().uptimeNanoseconds
            _ = await Task.detached { opened.loadBackground() }.value
            let decodeMs = Double(DispatchTime.now().uptimeNanoseconds - decodeStart) / 1e6
            opened.close()
            let line = "\(step.text ?? "first preview"): open on the main thread \(String(format: "%.1f", openMs)) ms, background decode off it \(String(format: "%.1f", decodeMs)) ms"
            UITestHarness.note(line)
            return (true, line)
        } catch {
            return (false, error.userMessage)
        }
    }
}
