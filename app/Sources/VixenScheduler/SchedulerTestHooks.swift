import AppKit
import VixenBridge
import VixenKit

/// What UI test scripts can ask of this area: `{ "action": "hook", "name": "...", "path": [arguments] }`.
enum SchedulerTestHooks {
    static func install(model: AppModel) {
        guard UITestHarness.isActive else { return }

        UITestHarness.hooks["songInventory"] = { args in
            switch args.first ?? "state" {
            case "open":
                let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first { $0.isVisible && $0.contentView != nil }
                SongInventoryPresenter.present(over: parent) { _, _ in }; return "opening"
            case "result": return SongInventoryModel.lastResult
            case "provider": if let index = args.dropFirst().first.flatMap(UInt32.init) { SongInventoryModel.current?.provider(index) }
            case "song": if args.count > 1 { SongInventoryModel.current?.select(args[1]) }
            case "link":
                if let model = SongInventoryModel.current { model.open(args.last == "vendor" ? model.view.website : model.view.selected?.link) }
            case "state": break
            default: return "FAIL unknown song inventory action"
            }
            guard let browser = SongInventoryModel.current else { return "closed" }
            return "loading=\(browser.loading) providers=\(browser.view.vendors.count) provider=\(browser.view.vendorName) selected=\(browser.view.selected?.title ?? "nil")"
        }

        UITestHarness.hooks["vixen2"] = { args in
            do {
                switch args.first ?? "state" {
                case "open":
                    guard args.count > 1, let core = model.core else { return "FAIL open path requires profile" }
                    let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first { $0.isVisible && $0.contentView != nil }
                    Vixen2Presenter.present(core: core, path: args[1], over: parent) { _ in }
                    return "opening"
                case "select": Vixen2ConverterModel.current?.select(args.count > 1 ? args[1] : nil)
                case "edit": Vixen2ConverterModel.current?.edit()
                case "mapName": Vixen2MapModel.current?.name = args.count > 1 ? args[1] : ""
                case "drop":
                    guard args.count > 5, let map = Vixen2MapModel.current else { return "FAIL drop start repeat mixing order node-ids" }
                    map.repeatCount = Int(args[2]) ?? 1; map.mixing = args[3] == "on"; map.order = args[4]
                    map.drop(start: Int(args[1]) ?? 0, nodes: Array(args.dropFirst(5)))
                case "color":
                    guard args.count > 2, let map = Vixen2MapModel.current, let color = UInt32(args[1], radix: 16) else { return "FAIL color argb row-indices" }
                    map.dialog.setColor(indices: args.dropFirst(2).compactMap(UInt32.init), color: color); map.refresh()
                case "clear":
                    guard let map = Vixen2MapModel.current else { return "FAIL no mapper" }
                    map.dialog.clear(indices: args.dropFirst().compactMap(UInt32.init)); map.refresh()
                case "mapState":
                    guard let map = Vixen2MapModel.current else { return "FAIL no mapper" }
                    return "rows=\(map.rows.count) mapped=\(map.rows.filter { $0.target != nil }.count) name=\(map.name) mixing=\(map.rows.filter(\.mixing).count)"
                case "result":
                    guard let session = Vixen2ConverterModel.lastSession else { return "none" }
                    if args.count > 1 { try session.save(path: args[1]) }
                    return "effects=\(session.effectCount()) length=\(session.lengthMs()) path=\(session.filePath() ?? "nil") modified=\(session.isModified())"
                case "state": break
                default: return "FAIL unknown Vixen 2 action"
                }
                guard let converter = Vixen2ConverterModel.current else { return "closed" }
                let info = converter.dialog.info()
                return "maps=\(info.maps.joined(separator: "|")) selected=\(info.selected ?? "nil") channels=\(info.channels) converting=\(converter.converting)"
            } catch { return "FAIL \(error.userMessage)" }
        }

        // Opens a sequence from its file and presents the Export Sequence dialog over the key window, as the sequencer's Sequence > Export does
        // with its own session.
        UITestHarness.hooks["exportSequence"] = { args in
            guard let path = args.first else { return "FAIL needs the sequence's path" }
            for _ in 0..<400 where model.core == nil {
                try? await Task.sleep(for: .milliseconds(50))
            }
            guard let core = model.core else { return "FAIL no profile: \(model.phase)" }
            let session: SequencerSession
            do {
                session = try await Task.detached { try core.openSequence(path: path) }.value
            } catch {
                return "FAIL \(error.userMessage)"
            }
            let previous = ExportSheetModel.current
            let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first { $0.isVisible && $0.contentView != nil }
            guard let present = SequenceExport.present else { return "FAIL nothing presents the dialog" }
            present(session, parent)
            for _ in 0..<200 where ExportSheetModel.current === previous || ExportSheetModel.current == nil {
                try? await Task.sleep(for: .milliseconds(25))
            }
            try? await Task.sleep(for: .milliseconds(300))
            return "presented over \(parent?.title ?? "no window")"
        }

        UITestHarness.hooks["wizard"] = { args in await wizard(args) }
        UITestHarness.hooks["package"] = { args in
            let importing = args.first == "import"
            guard let wizard = importing ? PackageWizardModel.currentImport : PackageWizardModel.currentExport else { return "FAIL no package wizard" }
            let action = args.count > 1 ? args[1] : "state"
            let text = args.count > 2 ? args[2] : ""
            do {
                switch action {
                case "add": _ = wizard.session.addSequences(paths: Array(args.dropFirst(2)))
                case "addAll": wizard.session.addAll()
                case "remove": wizard.session.removeRows(rows: args.dropFirst(2).compactMap(UInt32.init))
                case "input": try wizard.session.setInput(path: text)
                case "map": wizard.session.setMap(path: text)
                case "output": wizard.session.setOutput(path: text)
                case "audio": wizard.session.setAudio(include: text == "on")
                case "select": wizard.session.setSelected(rows: [UInt32(text) ?? 0], selected: args.last == "on")
                case "next": wizard.next()
                case "previous": wizard.previous()
                case "createMap": wizard.editMap(false)
                case "editMap": wizard.editMap(true)
                case "cancel": wizard.cancel()
                case "finish": await wizard.close(accept: true)
                case "wait":
                    let start = Date(), limit = Double(text) ?? 120
                    while wizard.session.view().running && Date().timeIntervalSince(start) < limit { try? await Task.sleep(for: .milliseconds(100)) }
                case "state": break
                default: return "FAIL unknown package action"
                }
                wizard.refresh()
                let v = wizard.view
                return "stage=\(v.stage) next=\(v.canNext) running=\(v.running) rows=\(v.rows.count) selected=\(v.rows.filter(\.selected).count) output=\(v.output) map=\(v.map) final=\(v.finalText) completed=\(v.completed) errors=\(v.errors.joined(separator: ";"))"
            } catch { return "FAIL \(error.userMessage)" }
        }
        UITestHarness.hooks["elementMapper"] = { args in
            if args.first == "open", args.count > 1, let core = model.core {
                do {
                    let path = args[1]
                    let sources = try await Task.detached {
                        let data = try Data(contentsOf: URL(fileURLWithPath: path))
                        let rows = try JSONSerialization.jsonObject(with: data) as? [[String: String]] ?? []
                        return rows.compactMap { row -> ElementMappingSource? in
                            guard let id = row["id"], let name = row["name"] else { return nil }
                            return ElementMappingSource(id: id, name: name)
                        }
                    }.value
                    let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first { $0.isVisible && $0.contentView != nil }
                    let before = ElementMapperModel.current, started = Date()
                    ElementMapperPresenter.present(core: core, name: "Mapper Verification", sources: sources, over: parent) { _ in }
                    for _ in 0..<200 where ElementMapperModel.current === before || ElementMapperModel.current == nil { try? await Task.sleep(for: .milliseconds(25)) }
                    guard ElementMapperModel.current !== before, ElementMapperModel.current != nil else { return "FAIL mapper did not open" }
                    return "presented \(sources.count) sources in \(Int(Date().timeIntervalSince(started) * 1000))ms"
                } catch { return "FAIL \(error.userMessage)" }
            }
            if args.first == "result" { return ElementMapperModel.lastResult }
            guard let mapper = ElementMapperModel.current else { return "FAIL no element mapper" }
            do {
                switch args.first ?? "state" {
                case "show":
                    guard let window = mapper.window else { return "FAIL mapper window missing" }
                    let parent = window.sheetParent
                    let prior = UITestHarness.staysOnScreen
                    UITestHarness.staysOnScreen = { [weak window, weak parent] candidate in candidate === window || candidate === parent || prior?(candidate) == true }
                    parent?.setFrameOrigin(NSPoint(x: 180, y: 180)); parent?.makeKeyAndOrderFront(nil)
                    if parent == nil { window.center() }
                    window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
                case "assign":
                    guard args.count > 2 else { return "FAIL assign source-id target-id" }
                    mapper.assign(sources: [args[1]], target: args[2])
                case "clear":
                    guard args.count > 1 else { return "FAIL clear source-id" }
                    mapper.assign(sources: [args[1]], target: nil)
                case "save":
                    guard args.count > 1 else { return "FAIL save path" }
                    try mapper.dialog.saveMap(path: args[1]); mapper.refresh()
                case "load":
                    guard args.count > 1 else { return "FAIL load path" }
                    try mapper.dialog.openMap(path: args[1]); mapper.refresh()
                case "tree":
                    guard args.count > 1 else { return "FAIL tree path" }
                    try mapper.dialog.loadTree(path: args[1]); mapper.refresh()
                case "accept": mapper.accept()
                case "cancel": mapper.complete(nil)
                case "state": break
                default: return "FAIL unknown mapper action"
                }
                let entries = mapper.view.rows.prefix(100).map { "\($0.sourceName)[\($0.sourceId)]=\($0.targetId ?? "")" }.joined(separator: "|")
                return "name=\(mapper.view.name) modified=\(mapper.view.modified) tree=\(mapper.view.hasSourceTree) rows=\(mapper.view.rows.count) \(entries)"
            } catch { return "FAIL \(error.userMessage)" }
        }
        UITestHarness.hooks["shows"] = { args in await shows(args) }
        UITestHarness.hooks["showEditor"] = { args in await showEditor(args) }
        UITestHarness.hooks["scheduler"] = { args in await scheduler(args) }

        // `idle|Export Complete|0` and the like.
        UITestHarness.hooks["exportStatus"] = { _ in
            guard let dialog = ExportSheetModel.current else { return "FAIL no export dialog" }
            return status(dialog)
        }

        // Waits for a run to finish, up to the given seconds (default 120).
        UITestHarness.hooks["exportWait"] = { args in
            guard let dialog = ExportSheetModel.current else { return "FAIL no export dialog" }
            let limit = Double(args.first ?? "") ?? 120
            let start = Date()
            // A run starts on the next pass of the main actor after Start.
            try? await Task.sleep(for: .milliseconds(200))
            while dialog.working, Date().timeIntervalSince(start) < limit {
                try? await Task.sleep(for: .milliseconds(100))
            }
            return dialog.working ? "FAIL still working: \(status(dialog))" : status(dialog)
        }
    }

    /// Drives the Export Wizard as its buttons do, for the icon buttons and panels a script can't reach: `["addAll"]`, `["next"]`,
    /// `["previous"]`, `["falcon", path]`, `["format", index]`, `["interval", text]`, `["includeAudio", "falcon"|"audio", "on"|"off"]`,
    /// `["create", "on"|"off"]`, `["direct", "on"|"off"]`, `["host", text]`, `["save", name]`, `["wait", seconds]`, `["state"]`.
    static func wizard(_ args: [String]) async -> String {
        // The wizard's window appears before the profile has loaded.
        for _ in 0..<400 where ExportWizardModel.current == nil {
            try? await Task.sleep(for: .milliseconds(50))
        }
        guard let model = ExportWizardModel.current else { return "FAIL no wizard" }
        let on = args.last == "on"
        switch args.first ?? "" {
        case "addAll":
            model.session.addAllSequences()
        case "next":
            await model.next()
        case "previous":
            await model.previous()
        case "direct":
            await model.setDirectUpload(on)
        case "host":
            model.setHost(args.count > 1 ? args[1] : "")
            await model.checkHost(alert: false)
        case "hostCommit":
            // Return while the same production commit action presents its sheet.
            model.setHost(args.count > 1 ? args[1] : "")
            Task { await model.checkHost(alert: true) }
        case "falcon":
            model.falconFolderText = args.count > 1 ? args[1] : ""
            await model.commitFalconFolder()
        case "format":
            model.session.setFormat(index: UInt32(args.count > 1 ? Int(args[1]) ?? 0 : 0))
        case "interval":
            model.setInterval(args.count > 1 ? args[1] : "")
        case "includeAudio":
            model.session.setIncludeAudio(falconBox: args.count > 1 && args[1] == "falcon", on: on)
        case "create":
            model.session.setCreateUniverseFile(on: on)
        case "save":
            model.session.setSaveSettings(on: true)
            model.saveName = args.count > 1 ? args[1] : ""
            model.session.setSaveName(name: model.saveName)
        case "wait":
            let limit = Double(args.count > 1 ? args[1] : "") ?? 120
            let start = Date()
            try? await Task.sleep(for: .milliseconds(300))
            while model.view.running || model.session.isRunning(), Date().timeIntervalSince(start) < limit {
                try? await Task.sleep(for: .milliseconds(100))
                model.refresh()
            }
        case "state":
            break
        case "controllers":
            let rows: [[String: Any]] = model.view.controllers.map { row in
                ["id": row.id, "name": row.name, "active": row.active,
                 "selected": model.selectedControllers.contains(row.id),
                 "channels": row.channels, "start": row.start.map(Int.init) ?? 0,
                 "end": row.end.map(Int.init) ?? 0]
            }
            guard let data = try? JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys]),
                  let json = String(data: data, encoding: .utf8) else { return "FAIL controller serialization" }
            return json
        case "focusControllers":
            guard model.view.stage == 2, let window = model.window else { return "FAIL not on controllers" }
            func table(in view: NSView) -> NSTableView? {
                if let table = view as? NSTableView { return table }
                for child in view.subviews { if let found = table(in: child) { return found } }
                return nil
            }
            guard let content = window.contentView, let table = table(in: content) else { return "FAIL no controller table" }
            return "focused=\(window.makeFirstResponder(table))"
        case "controllerKey":
            guard model.view.stage == 2, let window = model.window, args.count > 1 else { return "FAIL not on controllers" }
            let keys: [String: (String, UInt16)] = ["a": ("a", 0), "space": (" ", 49), "escape": ("\u{1b}", 53)]
            guard let (text, code) = keys[args[1]] else { return "FAIL unsupported controller key" }
            let modifiers: NSEvent.ModifierFlags = args.contains("command") ? [.command] : []
            for type in [NSEvent.EventType.keyDown, .keyUp] {
                guard let event = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: modifiers,
                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                    context: nil, characters: text, charactersIgnoringModifiers: text, isARepeat: false, keyCode: code) else { return "FAIL key event" }
                NSApp.postEvent(event, atStart: false)
            }
            try? await Task.sleep(for: .milliseconds(150))
        default:
            return "FAIL unknown wizard action \(args)"
        }
        model.refresh()
        let v = model.view
        let device = v.summary.map { " to \($0.destination) device \($0.deviceHeading ?? "-") rows \($0.deviceRows.count)" } ?? ""
        return "stage \(v.stage) next \(v.canNext) files \(v.sequenceFiles.count) format \(v.formatNames[Int(v.formatIndex)]) interval \(v.intervalText) direct \(v.directUpload) host \(v.hostText)\(device) preparing \(v.preparingNetwork) networkError \(v.networkError ?? "-") final \(v.finalText) task \(model.taskText) overall \(model.overallText)"
    }

    private static func status(_ dialog: ExportSheetModel) -> String {
        "\(dialog.working ? "working" : "idle")|\(dialog.statusText)|\(dialog.percent)|\(dialog.view.formatNames[Int(dialog.view.formatIndex)])|\(dialog.timingText)"
    }

    private static func number(_ args: [String], _ at: Int) -> Int {
        args.count > at ? Int(args[at]) ?? 0 : 0
    }

    /// The Shows window's buttons: `["state"]`, `["add"]`, `["select", row]`, `["edit"]`, `["rename", row, name]`, `["delete"]`.
    static func shows(_ args: [String]) async -> String {
        for _ in 0..<400 where ShowsModel.current == nil {
            try? await Task.sleep(for: .milliseconds(50))
        }
        guard let model = ShowsModel.current else { return "FAIL no Shows window" }
        switch args.first ?? "" {
        case "add": model.add()
        case "select": model.selection = number(args, 1)
        case "edit": model.edit()
        case "rename":
            model.beginRename(number(args, 1))
            model.renameText = args.count > 2 ? args[2] : ""
            model.commitRename()
        case "delete": Task { await model.delete() }
        case "state": break
        default: return "FAIL unknown shows action \(args)"
        }
        try? await Task.sleep(for: .milliseconds(200))
        return "shows \(model.names) selected \(model.selection.map(String.init) ?? "none") editor \(model.editor != nil)"
    }

    /// The Setup a Show sheet: `["state"]`, `["name", text]`, `["tab", index]`, `["add"]`, `["select", row]`, `["delete"]`, `["move", from, to]`,
    /// `["action", index]`, `["sequence", path]`, `["program", text]`, `["commandLine", text]`, `["window"|"wait", "on"|"off"]`, `["pause", seconds]`,
    /// `["test"]`, `["ok"]`, `["cancel"]`.
    static func showEditor(_ args: [String]) async -> String {
        guard let model = ShowEditorModel.current else { return "FAIL no show editor" }
        let text = args.count > 1 ? args[1] : ""
        let on = args.last == "on"
        switch args.first ?? "" {
        case "name": model.change { $0.setName(name: text) }
        case "tab": model.change { $0.setTab(tab: UInt32(number(args, 1))) }
        case "add": model.change { $0.addItem() }
        case "select": model.change { $0.select(row: UInt32(number(args, 1))) }
        case "delete": model.change { $0.deleteItem() }
        case "move": model.change { $0.moveItem(from: UInt32(number(args, 1)), to: UInt32(number(args, 2))) }
        case "action": model.change { $0.setAction(action: UInt32(number(args, 1))) }
        case "sequence": model.setSequence(text)
        case "program": model.change { $0.setLaunchProgram(text: text) }
        case "commandLine": model.change { $0.setLaunchCommandLine(text: text) }
        case "window": model.change { $0.setLaunchShowCommandWindow(on: on) }
        case "wait": model.change { $0.setLaunchWaitForExit(on: on) }
        case "pause": model.change { $0.setPause(seconds: Int32(number(args, 1))) }
        case "test": model.testLaunch()
        case "ok": Task { await model.ok() }
        case "cancel": Task { await model.cancel() }
        case "state": break
        default: return "FAIL unknown show editor action \(args)"
        }
        try? await Task.sleep(for: .milliseconds(200))
        let v = model.view
        return "name \(v.name) tab \(v.tab) rows \(v.rows) selected \(v.selected.map(String.init) ?? "none") action \(v.action.map(String.init) ?? "none") sequence \(v.sequenceName) program \(v.launchProgram) pause \(v.pauseSeconds)"
    }

    /// The scheduler, its Schedules window and its Schedule a Show sheet: `["state"]`, `["enable", "on"|"off"]`, `["check"]`, `["drive", seconds]`, `["start"]`,
    /// `["stop", "now"|"gracefully"]`, `["log"]`, `["logLines"]`, `["select", row]`, `["add"]`, `["edit"]`, `["delete"]`, `["editShow"]`,
    /// `["sheetShow", index]`, `["sheetTimes", startSeconds, endSeconds]`, `["sheetDay", day, "on"|"off"]`, `["sheetOk"]`, `["sheetCancel"]`.
    static func scheduler(_ args: [String]) async -> String {
        for _ in 0..<400 where SchedulerHub.current == nil {
            try? await Task.sleep(for: .milliseconds(50))
        }
        guard let hub = SchedulerHub.current else { return "FAIL no scheduler" }
        let schedules = SchedulesModel.current
        let sheet = ScheduleSheetModel.current
        switch args.first ?? "" {
        case "enable": hub.setEnabled(args.last == "on")
        case "check":
            hub.session.checkNow()
            hub.refresh()
        case "drive":
            // The timer doesn't run under test, so this stands in for it for a few seconds.
            for _ in 0..<(max(number(args, 1), 1) * 10) {
                hub.session.checkNow()
                try? await Task.sleep(for: .milliseconds(100))
            }
        case "start": hub.start()
        case "stop": hub.stop(gracefully: args.last == "gracefully")
        case "log": hub.toggleLog()
        case "logLines":
            try? await Task.sleep(for: .milliseconds(300))
            hub.refresh()
            return hub.status.log.joined(separator: " | ")
        case "select": schedules?.selection = number(args, 1)
        case "add": schedules?.add()
        case "edit": schedules?.edit()
        case "delete": Task { await schedules?.delete() }
        case "editShow": Task { await schedules?.editShow() }
        case "sheetShow": sheet?.show = number(args, 1)
        case "sheetTimes":
            guard let sheet else { return "FAIL no sheet" }
            let midnight = Calendar.current.startOfDay(for: sheet.startTime)
            sheet.startTime = midnight.addingTimeInterval(Double(number(args, 1)))
            sheet.endTime = midnight.addingTimeInterval(Double(number(args, 2)))
        case "sheetDay": sheet?.days[number(args, 1)] = args.last == "on"
        case "sheetOk": await sheet?.ok()
        case "sheetCancel": sheet?.cancel()
        case "state": break
        default: return "FAIL unknown scheduler action \(args)"
        }
        try? await Task.sleep(for: .milliseconds(300))
        hub.refresh()
        let s = hub.status
        let rows = hub.rows.map { "\($0.show); \($0.schedule); \($0.status)" }.joined(separator: " / ")
        return "enabled \(s.enabled) start \(s.canStart) stop \(s.canStop) status [\(s.text)] log \(s.log.count) rows [\(rows)]" + (sheet.map { " sheet [\($0.durationText)]" } ?? "")
    }
}
