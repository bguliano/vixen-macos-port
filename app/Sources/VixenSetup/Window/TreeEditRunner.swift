import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// The element tree's menu, keys and drags for a window that isn't Display Setup (`ElementTreeEditor` in VixenKit): the same commands and prompts as
/// `SetupModel`'s elements panel, as sheets on the asking window. Upstream's tree there has property editing off, so Item properties is left out.
///
/// It keeps one working copy for as long as the display stays as it left it, so Copy is still there for Paste, and each edit goes from the copy straight
/// to the running display, unsaved. Rows come and go as node id paths, since this copy's own row numbers mean nothing to the asking window.
final class TreeEditRunner: ElementTreeEditing {
    private let host: ElementTreeHost
    private var session: DisplaySetupSession?
    /// The display's revision the copy matches. Anyone else's change moves it on, and the copy is opened again.
    private var revision: UInt64 = 0
    private var generation = 0
    private var running = false

    static func register() {
        ElementTreeEditor.make = { TreeEditRunner(host: $0) }
    }

    init(host: ElementTreeHost) {
        self.host = host
        reopen()
    }

    func invalidate() {
        session = nil
        reopen()
    }

    /// Opens the copy ahead of the first right click, off the main thread.
    private func reopen() {
        guard let core = host.app.core else { return }
        generation += 1
        let mine = generation
        Task {
            let at = core.displayRevision()
            let fresh = await offMain { core.openDisplaySetup() }
            if generation == mine, session == nil {
                session = fresh
                revision = at
            }
        }
    }

    /// The working copy, opened here and now if the display changed under it.
    private func current() -> DisplaySetupSession? {
        guard let core = host.app.core else { return nil }
        if let session, core.displayRevision() == revision {
            return session
        }
        generation += 1
        revision = core.displayRevision()
        let fresh = core.openDisplaySetup()
        session = fresh
        return fresh
    }

    private func run(_ work: @escaping (TreeEditRunner) async -> Void) -> (() -> Void) {
        { [weak self] in
            guard let self, !self.running else { return }
            self.running = true
            Task {
                await work(self)
                self.running = false
            }
        }
    }

    /// One edit: made in the copy, applied to the running display, then the asking tree is told what to show.
    private func edit(_ work: @escaping @Sendable (DisplaySetupSession) -> SetupEditResult?) async {
        guard let session = current(), let core = host.app.core else { return }
        let started = ContinuousClock.now
        let shown: ElementTreeChange? = await offMain {
            guard let edit = work(session) else { return nil }
            _ = session.applyLive()
            return ElementTreeChange(select: edit.changesSelection ? session.nodePathsOf(rows: edit.select) : nil, expand: session.nodePathsOf(rows: edit.expand))
        }
        guard let shown else { return }
        revision = core.displayRevision()
        let applied = ContinuousClock.now
        await host.changed(shown)
        if UITestHarness.isActive {
            func ms(_ time: Duration) -> String { String(format: "%.1f", Double(time.components.seconds) * 1000 + Double(time.components.attoseconds) / 1e15) }
            UITestHarness.note("tree edit: \(ms(applied - started)) ms to the display, \(ms(ContinuousClock.now - started)) ms until the tree showed it")
        }
    }

    private func ask(_ title: String, _ text: String, _ buttons: [String]) async -> Int? {
        guard let window = host.window() else { return nil }
        return await TemplateRunner.alert(on: window, title, text, buttons)
    }

    private func tell(_ title: String, _ text: String) async {
        _ = await ask(title, text, ["OK"])
    }

    private func promptText(title: String, prompt: String, initial: String = "", selectAll: Bool = false) async -> String? {
        guard let window = host.window() else { return nil }
        return await TemplateRunner.sheet(on: window) { TextPromptSheet(title: title, prompt: prompt, initial: initial, selectAll: selectAll, onFinish: $0) }
    }

    private func checkLosesPatches(_ parent: String?) async -> Bool {
        guard let session = current() else { return false }
        let loses = await offMain { session.wouldLosePatches(destination: parent.flatMap { session.rowsForNodePaths(paths: [$0]).first }) }
        guard loses else { return true }
        let answer = await ask(
            "Convert Element to Group?",
            "Adding items to this element will convert it into a Group, which will remove any patches it may have. Are you sure you want to continue?",
            ["Convert to Group", "Cancel"]
        )
        return answer == 0
    }

    // MARK: The menu

    /// Display Setup's F32, F33 and F54, in the same order.
    func menu(rows: [String], primary: String?) -> NSMenu? {
        guard let session = current() else { return nil }
        let selection = session.rowsForNodePaths(paths: rows)
        let state = session.contextMenuState(selection: selection, primary: primary.flatMap { session.rowsForNodePaths(paths: [$0]).first })
        let tags = session.tagMenu(selection: selection)
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(SetupMenuItem("Cut", enabled: false))
        menu.addItem(SetupMenuItem("Copy", enabled: state.copy, handler: run { $0.copy(rows) }))
        menu.addItem(SetupMenuItem("Paste", enabled: state.paste, handler: run { await $0.paste(to: primary, asNew: false) }))
        menu.addItem(SetupMenuItem("Paste as New", enabled: state.pasteAsNew, handler: run { await $0.paste(to: primary, asNew: true) }))
        menu.addItem(.separator())
        menu.addItem(SetupMenuItem("Add", handler: run { await $0.add(to: primary) }))
        menu.addItem(SetupMenuItem("Add Multiple", handler: run { await $0.addMultiple(to: primary) }))
        menu.addItem(SetupMenuItem("Delete", enabled: state.delete, handler: run { await $0.delete(rows) }))
        menu.addItem(SetupMenuItem("Create Group", enabled: state.createGroup, handler: run { await $0.createGroup(rows) }))
        menu.addItem(SetupMenuItem("Rename", enabled: state.rename, handler: run { await $0.rename(rows) }))
        menu.addItem(SetupMenuItem("Find/Replace", enabled: state.findReplace, handler: run { await $0.findReplace(rows) }))
        menu.addItem(SetupMenuItem("Collapse All", handler: run { $0.host.collapseAll() }))
        menu.addItem(SetupMenuItem("Reverse Elements", enabled: state.reverse, handler: run { await $0.edit { $0.reverseRows(selection: $0.rowsForNodePaths(paths: rows)) } }))
        menu.addItem(SetupMenuItem("Sort", enabled: state.sort, handler: run { runner in
            guard let first = rows.first else { return }
            await runner.edit { s in s.rowsForNodePaths(paths: [first]).first.map { s.sortGroup(path: $0) } }
        }))
        let wire = NSMenu()
        wire.autoenablesItems = false
        wire.addItem(SetupMenuItem("Front Wire Diagram", handler: run { await $0.exportWireDiagram(rows.first, flip: false) }))
        wire.addItem(SetupMenuItem("Back Wire Diagram", handler: run { await $0.exportWireDiagram(rows.first, flip: true) }))
        let wireItem = SetupMenuItem("Export Wire Diagram", enabled: state.exportWireDiagram)
        wireItem.submenu = wire
        menu.addItem(wireItem)
        menu.addItem(SetupMenuItem("Export Element Tree", enabled: state.exportElementTree, tooltip: "Export the element tree for importing into other profiles.", handler: run { await $0.exportElementTree() }))
        menu.addItem(.separator())

        let tagMenu = NSMenu()
        tagMenu.autoenablesItems = false
        tagMenu.addItem(SetupMenuItem("⌘-Click to Cascade Tag", enabled: false))
        tagMenu.addItem(.separator())
        for tag in tags {
            let id = tag.id
            let item = SetupMenuItem(tag.name, handler: run { runner in
                let cascade = NSEvent.modifierFlags.contains(.command)
                await runner.retag { s in _ = try? s.toggleTag(selection: s.rowsForNodePaths(paths: rows), tag: id, cascade: cascade) }
            })
            switch tag.check {
            case .off: item.state = .off
            case .on: item.state = .on
            case .mixed: item.state = .mixed
            }
            if let color = tag.color {
                item.image = Self.dot(color)
            }
            tagMenu.addItem(item)
        }
        tagMenu.addItem(.separator())
        tagMenu.addItem(SetupMenuItem("Manage Tags...", handler: run { await $0.manageTags() }))
        let tagsItem = SetupMenuItem("Tags", enabled: state.tags)
        tagsItem.submenu = tagMenu
        menu.addItem(tagsItem)
        return menu
    }

    private static func dot(_ argb: UInt32) -> NSImage {
        NSImage(size: NSSize(width: 10, height: 10), flipped: false) { rect in
            NSColor(argb: argb).setFill()
            NSBezierPath(ovalIn: rect.insetBy(dx: 1, dy: 1)).fill()
            return true
        }
    }

    // MARK: Keys and the Edit menu

    func canPerform(_ command: OutlineCommand, rows: [String]) -> Bool {
        switch command {
        case .copy, .delete: !rows.isEmpty
        case .paste, .pasteAsNew: true
        }
    }

    func perform(_ command: OutlineCommand, rows: [String], primary: String?) {
        switch command {
        case .copy: run { $0.copy(rows) }()
        case .paste: run { await $0.paste(to: primary, asNew: false) }()
        case .pasteAsNew: run { await $0.paste(to: primary, asNew: true) }()
        case .delete: run { await $0.delete(rows) }()
        }
    }

    // MARK: Commands

    private func copy(_ rows: [String]) {
        guard let session = current() else { return }
        session.copyRows(selection: session.rowsForNodePaths(paths: rows))
    }

    private func paste(to destination: String?, asNew: Bool) async {
        guard let session = current() else { return }
        let can = await offMain { session.canPaste(destination: destination.flatMap { session.rowsForNodePaths(paths: [$0]).first }) }
        guard can.count == 2, asNew ? can[1] : can[0] else {
            NSSound.beep()
            return
        }
        guard await checkLosesPatches(destination) else { return }
        await edit { s in
            let row = destination.flatMap { s.rowsForNodePaths(paths: [$0]).first }
            return asNew ? s.pasteAsNew(destination: row) : s.pasteLinks(destination: row)
        }
    }

    private func add(to parent: String?) async {
        guard let name = await promptText(title: "", prompt: "Element Name?"), await checkLosesPatches(parent) else { return }
        await edit { s in s.addElement(parent: parent.flatMap { s.rowsForNodePaths(paths: [$0]).first }, name: name) }
    }

    private func addMultiple(to parent: String?) async {
        guard let session = current(), let window = host.window() else { return }
        var seed = "NewName"
        if let parent {
            seed = (await offMain { session.namesOf(selection: session.rowsForNodePaths(paths: [parent])) }).first ?? seed
        }
        let chosen = seed
        let names: [String]? = await TemplateRunner.sheet(on: window) { NameGeneratorSheet(seed: chosen, oldNames: nil, onFinish: $0) }
        guard let names, await checkLosesPatches(parent) else { return }
        guard !names.isEmpty else {
            await tell("", "Could not create elements.  Ensure you use a valid name and try again.")
            return
        }
        await edit { s in s.addElements(parent: parent.flatMap { s.rowsForNodePaths(paths: [$0]).first }, names: names) }
    }

    private func delete(_ rows: [String]) async {
        guard !rows.isEmpty, await ask("Delete Elements?", "Are you sure you want to delete these element(s)?", ["Delete", "Cancel"]) == 0 else { return }
        await edit { $0.deleteRows(selection: $0.rowsForNodePaths(paths: rows)) }
    }

    private func createGroup(_ rows: [String]) async {
        guard !rows.isEmpty, let name = await promptText(title: "", prompt: "Element Name?") else { return }
        await edit { $0.createGroup(selection: $0.rowsForNodePaths(paths: rows), name: name) }
    }

    private func rename(_ rows: [String]) async {
        guard let session = current(), let window = host.window(), !rows.isEmpty else { return }
        let names = await offMain { session.namesOf(selection: session.rowsForNodePaths(paths: rows)) }
        var newNames: [String]?
        if rows.count == 1 {
            guard let text = await promptText(title: "Rename item", prompt: "Item name?", initial: names.first ?? "", selectAll: true), !text.isEmpty, text != names.first else { return }
            newNames = [text]
        } else {
            newNames = await TemplateRunner.sheet(on: window) { NameGeneratorSheet(seed: "NewName", oldNames: names, onFinish: $0) }
        }
        guard let newNames else { return }
        await edit { $0.renameRows(selection: $0.rowsForNodePaths(paths: rows), names: newNames) }
    }

    private func findReplace(_ rows: [String]) async {
        guard let session = current(), let window = host.window(), !rows.isEmpty else { return }
        let names = await offMain { session.namesOf(selection: session.rowsForNodePaths(paths: rows)) }
        let newNames: [String]? = await TemplateRunner.sheet(on: window) { SubstitutionRenamerSheet(oldNames: names, onFinish: $0) }
        guard let newNames else { return }
        await edit { $0.renameRows(selection: $0.rowsForNodePaths(paths: rows), names: newNames) }
    }

    private func exportWireDiagram(_ group: String?, flip: Bool) async {
        guard let session = current(), let window = host.window(), let group, let row = session.rowsForNodePaths(paths: [group]).first else { return }
        let message = "Every element in the group needs a Location property to export a wire diagram."
        guard await offMain({ session.wireDiagramHasLocations(groupPath: row) }) else {
            await tell("Export Wire Diagram", message)
            return
        }
        let options: SetupWireOptions? = await TemplateRunner.sheet(on: window) { WireDiagramSheet(onFinish: $0) }
        guard let options else { return }
        let result = await offMain { Result { try session.exportWireDiagram(groupPath: row, flip: flip, options: options) } }
        switch result {
        case .success(let path?):
            if UITestHarness.isActive {
                UITestHarness.note("wire diagram \(path)")
            } else {
                NSWorkspace.shared.open(URL(fileURLWithPath: path))
            }
        case .success(nil):
            await tell("Export Wire Diagram", message)
        case .failure(let error):
            await tell("Export Wire Diagram", error.userMessage)
        }
    }

    private func exportElementTree() async {
        guard let session = current(), let window = host.window() else { return }
        var destination: URL?
        if UITestHarness.isActive {
            let folder = ProcessInfo.processInfo.environment["VIXEN_UI_TEST_OUT"] ?? NSTemporaryDirectory()
            destination = URL(fileURLWithPath: folder).appending(path: "tree.v3e")
        } else {
            let panel = NSSavePanel()
            panel.allowedContentTypes = [UTType(filenameExtension: "v3e") ?? .json]
            panel.canCreateDirectories = true
            panel.title = "Export Element Tree"
            panel.message = "Export the element tree for importing into other profiles."
            if let root = host.app.dataRoot {
                panel.directoryURL = URL(fileURLWithPath: root).appending(path: "Sequence")
            }
            destination = await panel.beginSheetModal(for: window) == .OK ? panel.url : nil
        }
        guard let path = destination?.path else { return }
        if case .failure(let error) = await offMain({ Result { try session.exportElementTree(path: path) } }) {
            await tell("Export Element Tree", error.userMessage)
        } else if UITestHarness.isActive {
            UITestHarness.note("element tree \(path)")
        }
    }

    /// A tag edit: nothing about the rows themselves changes, so the selection stays.
    private func retag(_ work: @escaping @Sendable (DisplaySetupSession) -> Void) async {
        guard let session = current(), let core = host.app.core else { return }
        await offMain {
            work(session)
            _ = session.applyLive()
        }
        revision = core.displayRevision()
        await host.changed(ElementTreeChange())
    }

    /// Preview F86: upstream opens the manager with `saveOnClose: false` here, and so does this: the colours go to the display, and the asking window's save writes them.
    private func manageTags() async {
        guard let session = current(), let window = host.window() else { return }
        let rows = await offMain { session.tagColors() }
        let colors: [SetupTagColor]? = await TemplateRunner.sheet(on: window) { ManageTagColorsSheet(rows: rows, onFinish: $0) }
        guard let colors else { return }
        await retag { $0.setTagColors(colors: colors) }
    }

    // MARK: Drag within the tree

    func canDrop(items: [String], parent: String?, index: Int?, copy: Bool) -> Bool {
        guard let session = current() else { return false }
        let sources = session.rowsForNodePaths(paths: items)
        let target = parent.flatMap { session.rowsForNodePaths(paths: [$0]).first }
        guard sources.count == items.count, (parent == nil) == (target == nil) else { return false }
        return session.canDropRows(sources: sources, parent: target, index: index.map { UInt32($0) }, copy: copy)
    }

    func drop(items: [String], parent: String?, index: Int?, copy: Bool) {
        run { runner in
            guard await runner.checkLosesPatches(parent) else { return }
            await runner.edit { s in
                s.dropRows(sources: s.rowsForNodePaths(paths: items), parent: parent.flatMap { s.rowsForNodePaths(paths: [$0]).first }, index: index.map { UInt32($0) }, copy: copy)
            }
        }()
    }
}
