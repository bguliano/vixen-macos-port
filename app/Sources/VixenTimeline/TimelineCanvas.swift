import AppKit
import QuartzCore
import VixenBridge
import VixenKit

/// What the timeline asks of the window around it. Everything has a default, so a window with no opinions gets a working timeline.
@MainActor
public protocol TimelineDelegate: AnyObject {
    /// The selection changed. Copy, cut and delete enabling, and the effect editor, read `selectedElements()`.
    func timelineSelectionChanged()
    /// The edit cursor moved: the status bar's time.
    func timelineCursorMoved(ticks: Int64)
    /// An entry went on the session's undo stack. Return true if the window's `UndoManager` has been told; otherwise the timeline tells it.
    func timelineUndoRecorded() -> Bool
    /// F54: build and show the effect context menu at this point in `view`. Return false to have nothing shown.
    func timelineShowEffectMenu(elements: [UInt64], time: Int64, row: UInt32, at point: NSPoint, in view: NSView)
    /// F101, F103, F124. Return true if handled; otherwise the timeline asks the session to play.
    func timelinePlay(start: Int64, end: Int64?) -> Bool
    /// F38. The tag manager is another area's. Call `tagsChanged()` after it saves.
    func timelineManageTags()
    /// F137: the phonemes for these words, or nil if there's no dictionary.
    func timelinePhonemes(for words: [String]) -> [TimelineWordPhonemes]?
    /// The sequence or view state changed in a way the window may want to mark as modified.
    func timelineDocumentChanged()
    /// F153: zoom, scroll or row height changed for the first time, so closing should save silently. `isEditorStateModified()` says the same.
    func timelineEditorStateModified()
    /// F89: what a drag over the grid would do at this time, row and effect. The sequencer knows effect types, gradients, curves, colours and files.
    func timelineValidateDrop(_ info: NSDraggingInfo, time: Int64, row: UInt32?, element: UInt64?) -> NSDragOperation
    /// F89: the drop. Return true if it was taken.
    func timelinePerformDrop(_ info: NSDraggingInfo, time: Int64, row: UInt32?, element: UInt64?) -> Bool
}

extension TimelineDelegate {
    public func timelineSelectionChanged() {}
    public func timelineCursorMoved(ticks: Int64) {}
    public func timelineUndoRecorded() -> Bool { false }
    public func timelineShowEffectMenu(elements: [UInt64], time: Int64, row: UInt32, at point: NSPoint, in view: NSView) {}
    public func timelinePlay(start: Int64, end: Int64?) -> Bool { false }
    public func timelineManageTags() {}
    public func timelinePhonemes(for words: [String]) -> [TimelineWordPhonemes]? { nil }
    public func timelineDocumentChanged() {}
    public func timelineEditorStateModified() {}
    public func timelineValidateDrop(_ info: NSDraggingInfo, time: Int64, row: UInt32?, element: UInt64?) -> NSDragOperation { [] }
    public func timelinePerformDrop(_ info: NSDraggingInfo, time: Int64, row: UInt32?, element: UInt64?) -> Bool { false }
}

/// The timeline for one window: the views, the Rust state behind them, and what keeps them in step.
@MainActor
public final class TimelineCanvas {
    /// The object every command goes through. See `docs/parity/timeline.md` and the report for what to call.
    public let controller: TimelineController
    public weak var delegate: TimelineDelegate?

    let pictures: PictureStore
    let grid: GridCanvasView
    let gridViewport: GridViewportView
    let ruler: RulerView
    let marksBar: MarksBarView
    let waveform: WaveformView
    let rows: RowHeaderView
    let timePanel: TimePanelView
    let rowPane: RowPaneView
    public let view: TimelineContainerView

    private(set) var state: TimelineView
    private var autoscrollTimer: Timer?
    private var autoscrollOwner: AutoscrollOwner = .grid
    private var refreshTimer: Timer?
    private var playbackLink: CADisplayLink?
    /// Playing or paused: the line and arrow are up, and stopping is what takes them down and brings the range back.
    private var wasActive = false
    private var layingOut = false
    var scale: CGFloat = 2

    enum AutoscrollOwner {
        case grid
        case marks
    }

    /// For the test scene's dump and benchmark.
    static weak var current: TimelineCanvas?

    public init(session: SequencerSession, controller: TimelineController? = nil, delegate: TimelineDelegate? = nil, loadProfileSettings: Bool = false) {
        let controller = controller ?? TimelineController(session: session)
        self.controller = controller
        self.delegate = delegate
        state = controller.view()
        let box = WeakCanvasBox()
        pictures = PictureStore(controller: controller) { ids in
            MainActor.assumeIsolated { box.canvas?.picturesArrived(ids) }
        }
        grid = GridCanvasView(controller: controller, pictures: pictures)
        gridViewport = GridViewportView(canvas: grid)
        ruler = RulerView(controller: controller)
        marksBar = MarksBarView(controller: controller)
        waveform = WaveformView(controller: controller)
        rows = RowHeaderView(controller: controller)
        timePanel = TimePanelView(waveform: waveform, ruler: ruler, marks: marksBar, grid: gridViewport)
        rowPane = RowPaneView(rows: rows)
        view = TimelineContainerView(rowPane: rowPane, timePanel: timePanel)
        box.canvas = self
        for surface in [grid as TimelineSurface, ruler, marksBar, waveform, rows] {
            surface.canvas = self
        }
        gridViewport.canvas = self
        timePanel.canvas = self
        rowPane.canvas = self
        view.canvas = self
        TimelineCanvas.current = self
        if UITestHarness.isActive {
            let earlier = UITestHarness.hooks["timelineAllPlayback"]
            UITestHarness.hooks["timelineAllPlayback"] = { [weak self] args in
                var rows: [[String: Any]] = []
                if let earlier, let data = await earlier(args).data(using: .utf8),
                   let previous = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] { rows = previous }
                if let self, let window = self.view.window, window.isVisible {
                    let session = self.controller.session()
                    rows.append(["window": window.title, "transport": "\(session.transport())", "positionMs": session.positionMs(),
                                 "playbackLineX": self.grid.playbackLineX.map { Double($0) } ?? -1])
                }
                guard let data = try? JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys]) else { return "FAIL playback state encoding" }
                return String(decoding: data, as: UTF8.self)
            }
        }
        TimelineTestHooks.install()

        for plane in grid.planes { plane.onNeedPictures = { _ in } }
        if loadProfileSettings {
            apply(controller.loadProfileSettings())
            view.splitterDistance = CGFloat(controller.settings().splitterDistance)
        }
        rulerMetricsChanged()
        startRefreshing()
    }

    // MARK: Applying updates

    /// Everything Rust said changed: scroll what moved, repaint what's dirty, and act on what it asks for.
    public func apply(_ u: TimelineUpdate) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let old = state
        state = u.view
        let heightsChanged = old.waveformHeight != u.view.waveformHeight || old.rulerHeight != u.view.rulerHeight || old.marksBarHeight != u.view.marksBarHeight
        let zoomed = old.timePerPixel != u.view.timePerPixel
        if zoomed {
            pictures.holdRasterizing(for: 0.15)
        }
        if old.rulerHeight != u.view.rulerHeight {
            rulerMetricsChanged()
        }
        if heightsChanged {
            relayout()
        }
        syncView()
        // A zoom has its own new sets of tiles, in `TilePlane`, and everything repaints with those.
        if !zoomed {
            if u.gridAll {
                for plane in grid.planes { plane.invalidateAll() }
            } else {
                for r in u.gridRects {
                    for plane in grid.planes { plane.invalidate(CGRect(x: CGFloat(r.x), y: CGFloat(r.y), width: CGFloat(r.w), height: CGFloat(r.h))) }
                }
            }
            if u.ruler {
                ruler.plane.invalidateAll()
            }
            if u.marksBar {
                marksBar.plane.invalidateAll()
            }
            if u.waveform {
                waveform.plane.invalidateAll()
            }
        }
        if u.rowHeader {
            rows.plane.invalidateAll()
        }
        for plane in grid.planes { plane.pump() }
        if u.overlay || zoomed || grid.overlayCoverIsStale() {
            refreshOverlay()
        }
        CATransaction.commit()

        if u.selectionChanged {
            delegate?.timelineSelectionChanged()
            grid.setAccessibilityValue("\(controller.selectedElements().count) selected")
        }
        if u.cursorMoved {
            delegate?.timelineCursorMoved(ticks: u.view.cursor)
        }
        if u.editorStateModified {
            delegate?.timelineEditorStateModified()
        }
        setAutoscroll(u.autoscroll)
        for request in u.requests {
            handle(request)
        }
    }

    /// Moves every surface to the current scroll. A scroll frame is this and nothing else, unless a new tile comes into view.
    func syncView() {
        let v = state
        let x = (CGFloat(v.scrollX) * scale).rounded(.down) / scale
        let y = CGFloat(v.verticalOffset)
        grid.setView(scrollX: x, scrollY: y, state: v)
        ruler.setView(scrollX: x, state: v)
        marksBar.setView(scrollX: x, state: v)
        waveform.setView(scrollX: x, state: v)
        rows.setView(scrollY: y)
        gridViewport.updateScrollers(state: v, gridSize: grid.bounds.size)
    }

    private func relayout() {
        guard !layingOut else { return }
        layingOut = true
        timePanel.needsLayout = true
        rowPane.needsLayout = true
        timePanel.layoutSubtreeIfNeeded()
        rowPane.layoutSubtreeIfNeeded()
        layingOut = false
        viewportChanged()
    }

    /// The grid's size is the viewport Rust clamps against.
    func viewportChanged() {
        let size = grid.bounds.size
        guard size.width > 0, size.height > 0 else { return }
        let update = controller.setViewport(width: Int32(size.width), height: Int32(size.height), panelWidth: Int32(timePanel.bounds.width))
        apply(update)
        syncView()
        for plane in grid.planes { plane.pump() }
    }

    func refreshOverlay() {
        let o = grid.refreshOverlay()
        ruler.overlayChanged(o)
        waveform.overlayChanged(guides: o.guides, state: state)
    }

    func picturesArrived(_ ids: Set<UInt64>) {
        for plane in grid.planes {
            plane.picturesArrived(ids)
            plane.pump()
        }
        grid.picturesArrived(ids)
    }

    /// The label font's glyph widths, so a scroll needs no text measuring across the bindings.
    private func rulerMetricsChanged() {
        let font = rulerFont(height: state.rulerHeight)
        let width = { (text: String) in Double(measure(textLine(text, font: font, color: TimelineColors.white)).0) }
        apply(controller.setRulerMetrics(digit: width("0"), colon: width(":"), dot: width(".")))
    }

    // MARK: Autoscroll

    func noteAutoscrollOwner(_ owner: AutoscrollOwner) {
        autoscrollOwner = owner
    }

    private func setAutoscroll(_ on: Bool) {
        if on, autoscrollTimer == nil {
            let timer = Timer(timeInterval: 0.05, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.autoscrollTick() }
            }
            RunLoop.main.add(timer, forMode: .common)
            autoscrollTimer = timer
        } else if !on, let timer = autoscrollTimer {
            timer.invalidate()
            autoscrollTimer = nil
        }
    }

    private func autoscrollTick() {
        let mods = timelineMods(NSEvent.modifierFlags)
        switch autoscrollOwner {
        case .grid:
            apply(controller.gridAutoscrollTick(mods: mods))
        case .marks:
            apply(controller.marksAutoscrollTick(mods: mods))
        }
    }

    // MARK: Following the session

    /// Finished renders and other panels' edits, and playback starting and stopping. Rust's side of this is two atomic reads when nothing changed.
    private func startRefreshing() {
        guard refreshTimer == nil else { return }
        let timer = Timer(timeInterval: 0.1, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.refreshTick() }
        }
        RunLoop.main.add(timer, forMode: .common)
        refreshTimer = timer
    }

    private func refreshTick() {
        guard view.window != nil else { return }
        apply(controller.refresh())
        // F151, FIX Q45: one alert, and the last good content stays. The details are in the log.
        if let failure = controller.takeDrawFailure() {
            showAlert(message: "Error", informative: failure)
        }
        // F94: a pause holds the line and arrow where they are, and the remembered range comes back only when playback stops
        // (`context_ContextEnded`). The paused position doesn't change, so nothing needs following meanwhile.
        switch controller.session().transport() {
        case .playing:
            startPlaybackLink()
            wasActive = true
        case .paused:
            // Upstream's timer ticks through a pause, so one that lands before this tick ever saw "playing" still gets its line.
            if !wasActive {
                playbackFrame()
            }
            pausePlaybackLink()
            wasActive = true
        case .stopped:
            if wasActive {
                stopPlaybackLink()
            }
            wasActive = false
        }
    }

    private func startPlaybackLink() {
        guard playbackLink == nil else { return }
        let link = grid.displayLink(target: PlaybackTarget(canvas: self), selector: #selector(PlaybackTarget.tick))
        link.add(to: .main, forMode: .common)
        playbackLink = link
    }

    private func pausePlaybackLink() {
        playbackLink?.invalidate()
        playbackLink = nil
    }

    private func stopPlaybackLink() {
        pausePlaybackLink()
        grid.clearPlayback()
        ruler.clearPlayback()
        apply(controller.playbackEnded())
    }

    /// One display frame while playing: F94, F95. Reads the position once and moves layers.
    func playbackFrame() {
        let ticks = Int64(controller.session().positionMs() * 10_000)
        apply(controller.followPlayback(position: ticks))
        grid.setPlayback(x: CGFloat(ticks) / CGFloat(max(state.timePerPixel, 1)), state: state)
        ruler.setPlayback(x: CGFloat(ticks) / CGFloat(max(state.timePerPixel, 1)), state: state)
    }

    // MARK: Requests

    private func handle(_ request: TimelineRequest) {
        switch request {
        case let .chooseOverlapping(items, x, y):
            showOverlapMenu(items, at: NSPoint(x: CGFloat(x), y: CGFloat(y)))
        case let .contextMenu(elements, time, row):
            let point = grid.convert(grid.window?.mouseLocationOutsideOfEventStream ?? .zero, from: nil)
            delegate?.timelineShowEffectMenu(elements: elements, time: time, row: row, at: point, in: grid)
        case let .rowContextMenu(row):
            showRowMenu(row: row)
        case let .warning(text, title):
            showAlert(message: title, informative: text)
        case .undoRecorded:
            undoRecorded()
        case .elementDoubleClicked:
            break
        case let .playRange(start, end):
            if delegate?.timelinePlay(start: start, end: end) != true {
                let session = controller.session()
                // F124: `PlaySequenceBetween` stops a paused sequence first.
                if session.transport() == .paused {
                    session.stop()
                }
                session.play(startMs: Double(start) / 10_000, endMs: end.map { Double($0) / 10_000 } ?? Double(state.total) / 10_000, looping: false)
            }
        case .markMenu:
            showMarkDeleteMenu()
        case let .marksContextMenu(pressX, pressY):
            showMarksMenu(pressX: pressX, pressY: pressY)
        case let .needsNewCollection(time, fillGap, label):
            askNewCollection(time: time, fillGap: fillGap, label: label)
        case let .needsEnableDefault(time, fillGap, label):
            askEnableDefault(time: time, fillGap: fillGap, label: label)
        case let .promptMarkLabel(time, fillGap):
            askText(title: "New Mark", message: "Enter the Mark label text.", initial: "", confirm: "Add Mark", selectAll: false) { [weak self] text in
                // Cancel keeps an empty label and still adds.
                guard let self else { return }
                self.apply(self.controller.addMarkAt(time: time, fillGap: fillGap, label: text ?? ""))
            }
        }
    }

    /// Mirrors the session's stack in the window's `UndoManager`, so the Edit menu names the action.
    private func undoRecorded() {
        if delegate?.timelineUndoRecorded() == true {
            return
        }
        guard let manager = view.window?.undoManager else { return }
        let session = controller.session()
        let name = session.undoDescriptions().first ?? ""
        manager.setActionName(name)
        manager.registerUndo(withTarget: view) { [weak self] _ in
            MainActor.assumeIsolated { self?.undoOne(manager) }
        }
        manager.setActionName(name)
        delegate?.timelineDocumentChanged()
    }

    private func undoOne(_ manager: UndoManager) {
        let session = controller.session()
        let name = session.undoDescriptions().first ?? ""
        _ = session.undo(steps: 1)
        apply(controller.refresh())
        manager.registerUndo(withTarget: view) { [weak self] _ in
            MainActor.assumeIsolated { self?.redoOne(manager) }
        }
        manager.setActionName(name)
    }

    private func redoOne(_ manager: UndoManager) {
        let session = controller.session()
        let name = session.redoDescriptions().first ?? ""
        _ = session.redo(steps: 1)
        apply(controller.refresh())
        manager.registerUndo(withTarget: view) { [weak self] _ in
            MainActor.assumeIsolated { self?.undoOne(manager) }
        }
        manager.setActionName(name)
    }

    // MARK: Menus and alerts

    private func point(inGrid p: NSPoint) -> NSPoint {
        NSPoint(x: p.x, y: grid.bounds.height - p.y)
    }

    private func showOverlapMenu(_ items: [TimelineMenuItem], at clientPoint: NSPoint) {
        let menu = NSMenu()
        menu.autoenablesItems = false
        var chosen: UInt64?
        for item in items {
            menu.addItem(ClosureMenuItem(title: item.label) { chosen = item.id })
        }
        let location = point(inGrid: clientPoint)
        DispatchQueue.main.async { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                if !UITestHarness.isActive {
                    menu.popUp(positioning: nil, at: location, in: self.grid)
                }
                self.apply(self.controller.finishChooser(chosen: chosen))
            }
        }
    }

    private func showRowMenu(row: UInt32) {
        let model = controller.rowMenu()
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(ClosureMenuItem(title: "Collapse All Groups") { [weak self] in
            guard let self else { return }
            self.apply(self.controller.collapseAllRows())
        })
        menu.addItem(ClosureMenuItem(title: "Reset All Rows to Default Height") { [weak self] in
            guard let self else { return }
            self.apply(self.controller.resetRowHeights())
        })
        let resetSelected = ClosureMenuItem(title: "Reset Selected and Child rows to Default Height") { [weak self] in
            guard let self else { return }
            self.apply(self.controller.resetSelectedRowHeights())
        }
        resetSelected.isEnabled = model.resetSelectedEnabled
        menu.addItem(resetSelected)
        menu.addItem(.separator())

        let tags = NSMenu()
        tags.autoenablesItems = false
        let hint = NSMenuItem(title: "⌘-Click to cascade tag", action: nil, keyEquivalent: "")
        hint.isEnabled = false
        tags.addItem(hint)
        tags.addItem(.separator())
        for tag in model.tags {
            let item = ClosureMenuItem(title: tag.name) { [weak self] in
                guard let self else { return }
                let cascade = NSEvent.modifierFlags.contains(.command)
                self.apply(self.controller.toggleTagOnSelected(tag: tag.tag, cascade: cascade))
            }
            switch tag.state {
            case .unchecked: item.state = .off
            case .checked: item.state = .on
            case .mixed: item.state = .mixed
            }
            if let color = tag.color {
                item.image = colorDot(color)
            }
            tags.addItem(item)
        }
        tags.addItem(.separator())
        tags.addItem(ClosureMenuItem(title: "Manage Tags...") { [weak self] in
            self?.delegate?.timelineManageTags()
        })
        let tagsItem = NSMenuItem(title: "Tags", action: nil, keyEquivalent: "")
        tagsItem.submenu = tags
        tagsItem.isEnabled = model.tagsEnabled
        menu.addItem(tagsItem)
        popUp(menu, in: rows)
    }

    private func popUp(_ menu: NSMenu, in view: NSView) {
        guard !UITestHarness.isActive else { return }
        let location = view.convert(view.window?.mouseLocationOutsideOfEventStream ?? .zero, from: nil)
        DispatchQueue.main.async {
            menu.popUp(positioning: nil, at: location, in: view)
        }
    }

    private func colorDot(_ argb: UInt32) -> NSImage {
        NSImage(size: NSSize(width: 10, height: 10), flipped: false) { rect in
            NSColor(argb: argb).setFill()
            NSBezierPath(ovalIn: rect).fill()
            return true
        }
    }

    private func showMarkDeleteMenu() {
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(ClosureMenuItem(title: "Delete Selected Marks") { [weak self] in
            guard let self else { return }
            self.apply(self.controller.deleteMarksKey())
        })
        popUp(menu, in: ruler)
    }

    private func showMarksMenu(pressX: Int32, pressY: Int32) {
        let model = controller.marksContextMenu()
        let menu = NSMenu()
        menu.autoenablesItems = false
        func add(_ title: String, enabled: Bool, _ action: @escaping @MainActor () -> Void) {
            let item = ClosureMenuItem(title: title, action: action)
            item.isEnabled = enabled
            menu.addItem(item)
        }
        add("Edit Text", enabled: model.editText) { [weak self] in self?.editMarkText() }
        add("Cut", enabled: model.cut) { [weak self] in
            guard let self else { return }
            self.apply(self.controller.cutMarks())
        }
        add("Copy", enabled: model.copy) { [weak self] in self?.controller.copyMarks() }
        add("Paste", enabled: model.paste) { [weak self] in self?.pasteMarks(pressX: pressX, pressY: pressY) }
        menu.addItem(.separator())
        add("Delete", enabled: model.delete) { [weak self] in
            guard let self else { return }
            self.apply(self.controller.deleteSelectedMarks())
        }
        if model.phoneme || model.breakdownPhrase || model.breakdownWord {
            menu.addItem(.separator())
        }
        if model.phoneme {
            let sub = NSMenu()
            sub.autoenablesItems = false
            for name in controller.phonemeNames() {
                sub.addItem(ClosureMenuItem(title: name) { [weak self] in
                    guard let self else { return }
                    self.apply(self.controller.setSelectedMarksText(text: name))
                })
            }
            let item = NSMenuItem(title: "Phoneme", action: nil, keyEquivalent: "")
            item.submenu = sub
            menu.addItem(item)
        }
        if model.breakdownPhrase {
            add("Breakdown Phrase", enabled: true) { [weak self] in
                guard let self else { return }
                self.apply(self.controller.breakdownPhrase())
            }
        }
        if model.breakdownWord {
            add("Breakdown Word", enabled: true) { [weak self] in self?.breakdownWord() }
        }
        popUp(menu, in: marksBar)
    }

    private func editMarkText() {
        let prompt = controller.editTextPrompt()
        askText(title: prompt.title, message: "Enter the new text.", initial: prompt.text, confirm: "OK", selectAll: prompt.selectAll) { [weak self] text in
            guard let self, let text else { return }
            self.apply(self.controller.setSelectedMarksText(text: text))
        }
    }

    private func pasteMarks(pressX: Int32, pressY: Int32) {
        let questions = controller.pasteMarkQuestions(pressX: pressX, pressY: pressY)
        var allowed: UInt32 = 0
        func next(_ index: Int) {
            guard index < questions.count else {
                apply(controller.pasteMarks(pressX: pressX, pressY: pressY, allowedMismatches: allowed))
                return
            }
            confirm(message: "Warning", informative: questions[index], verb: "Paste Anyway") { yes in
                if yes {
                    allowed += 1
                    next(index + 1)
                } else {
                    self.apply(self.controller.pasteMarks(pressX: pressX, pressY: pressY, allowedMismatches: allowed))
                }
            }
        }
        next(0)
    }

    private func breakdownWord() {
        let words = controller.marksInfo().filter(\.selected).map { $0.text.trimmingCharacters(in: .whitespaces) }
        guard let phonemes = delegate?.timelinePhonemes(for: words) else {
            // F137: the new collection stays.
            apply(controller.breakdownWord(phonemes: []))
            showAlert(message: "Error", informative: "Unable to find Standard Phoneme Dictionary")
            return
        }
        apply(controller.breakdownWord(phonemes: phonemes))
    }

    /// F111: playback pauses for these two questions.
    private func pauseIfPlaying() {
        let session = controller.session()
        if session.transport() == .playing {
            session.pause()
        }
    }

    private func askNewCollection(time: Int64, fillGap: Bool, label: String?) {
        pauseIfPlaying()
        confirm(
            message: "Create a Mark Collection",
            informative: "Marks are stored in Mark Collections. There are no mark collections available to store this mark. Would you like to create a new one?",
            verb: "Create Collection"
        ) { [weak self] yes in
            guard let self, yes else { return }
            self.apply(self.controller.createDefaultMarkCollection())
            self.apply(self.controller.addMarkAt(time: time, fillGap: fillGap, label: label))
        }
    }

    private func askEnableDefault(time: Int64, fillGap: Bool, label: String?) {
        pauseIfPlaying()
        confirm(
            message: "New Mark",
            informative: "The active mark collection is not visible on the timeline. Would you like to enable it to add the mark?",
            verb: "Enable and Add"
        ) { [weak self] yes in
            guard let self, yes else { return }
            self.apply(self.controller.enableDefaultMarkCollection())
            self.apply(self.controller.addMarkAt(time: time, fillGap: fillGap, label: label))
        }
    }

    func showAlert(message: String, informative: String) {
        let alert = NSAlert()
        alert.messageText = message
        alert.informativeText = informative
        if let window = view.window {
            alert.beginSheetModal(for: window)
        }
    }

    private func confirm(message: String, informative: String, verb: String, _ answer: @escaping (Bool) -> Void) {
        let alert = NSAlert()
        alert.messageText = message
        alert.informativeText = informative
        alert.addButton(withTitle: verb)
        alert.addButton(withTitle: "Cancel")
        guard let window = view.window else {
            answer(false)
            return
        }
        alert.beginSheetModal(for: window) { response in
            MainActor.assumeIsolated { answer(response == .alertFirstButtonReturn) }
        }
    }

    /// `TextDialog`, as a sheet. Enter is OK and Esc cancels. Cancel answers nil.
    private func askText(title: String, message: String, initial: String, confirm verb: String, selectAll: Bool, _ answer: @escaping (String?) -> Void) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.addButton(withTitle: verb)
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 280, height: 24))
        field.stringValue = initial
        alert.accessoryView = field
        guard let window = view.window else {
            answer(nil)
            return
        }
        alert.window.initialFirstResponder = field
        alert.beginSheetModal(for: window) { response in
            MainActor.assumeIsolated { answer(response == .alertFirstButtonReturn ? field.stringValue : nil) }
        }
        if selectAll {
            field.selectText(nil)
        }
    }

    // MARK: Drops

    /// The pasteboard types the window's delegate takes. F89.
    public func registerDropTypes(_ types: [NSPasteboard.PasteboardType]) {
        grid.registerForDraggedTypes(types)
    }

    // MARK: Going away

    /// Starts following the session again after `stop`.
    func resume() {
        startRefreshing()
    }

    /// Stops the timers. A window calls this when it closes, after `store_view_state` and `save_profile_settings` as it wants them.
    public func stop() {
        refreshTimer?.invalidate()
        refreshTimer = nil
        autoscrollTimer?.invalidate()
        autoscrollTimer = nil
        playbackLink?.invalidate()
        playbackLink = nil
    }
}

/// Lets a picture's arrival find the canvas without keeping it alive.
private final class WeakCanvasBox: @unchecked Sendable {
    @MainActor weak var canvas: TimelineCanvas?
}

private final class PlaybackTarget: NSObject {
    weak var canvas: TimelineCanvas?

    init(canvas: TimelineCanvas) {
        self.canvas = canvas
    }

    @objc func tick() {
        MainActor.assumeIsolated { canvas?.playbackFrame() }
    }
}

/// A menu item that runs a closure.
nonisolated final class ClosureMenuItem: NSMenuItem {
    private let run: @MainActor () -> Void

    init(title: String, action run: @escaping @MainActor () -> Void) {
        self.run = run
        super.init(title: title, action: #selector(fire), keyEquivalent: "")
        target = self
    }

    @available(*, unavailable)
    required init(coder: NSCoder) {
        fatalError("not from a nib")
    }

    @objc private func fire() {
        let run = self.run
        MainActor.assumeIsolated { run() }
    }
}

/// Modifiers as Rust tests them. ⌃ is ignored: it means right-click.
func timelineMods(_ flags: NSEvent.ModifierFlags) -> TimelineMods {
    TimelineMods(shift: flags.contains(.shift), cmd: flags.contains(.command), opt: flags.contains(.option))
}

extension NSCursor {
    static func timeline(_ kind: TimelineCursorKind) -> NSCursor {
        switch kind {
        case .arrow: return .arrow
        case .resizeLeftRight: return .resizeLeftRight
        case .resizeUpDown: return .resizeUpDown
        case .crosshair: return .crosshair
        case .move: return .closedHand
        case .hand: return .pointingHand
        }
    }
}

/// What every surface shares: a place to find the canvas, and pointer events floored to whole points in y-down client coordinates.
@MainActor
class TimelineSurface: NSView {
    weak var canvas: TimelineCanvas?
    let controller: TimelineController
    private var tracking: NSTrackingArea?
    private var cursorKind: TimelineCursorKind = .arrow
    /// A ⌃-press is a right press, and its drags and release go with it.
    private var pressIsRight = false

    init(controller: TimelineController) {
        self.controller = controller
        super.init(frame: .zero)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("not from a nib")
    }

    override var acceptsFirstResponder: Bool { true }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking {
            removeTrackingArea(tracking)
        }
        let area = NSTrackingArea(rect: .zero, options: [.mouseMoved, .cursorUpdate, .activeAlways, .inVisibleRect, .mouseEnteredAndExited], owner: self)
        addTrackingArea(area)
        tracking = area
    }

    override func cursorUpdate(with event: NSEvent) {
        NSCursor.timeline(cursorKind).set()
    }

    func setCursorKind(_ kind: TimelineCursorKind) {
        if kind != cursorKind {
            cursorKind = kind
            NSCursor.timeline(kind).set()
        }
    }

    func pointer(_ event: NSEvent, button: TimelineButton?) -> TimelinePointer {
        let p = convert(event.locationInWindow, from: nil)
        return TimelinePointer(
            x: Int32(p.x.rounded(.down)), y: Int32((bounds.height - p.y).rounded(.down)), button: button, clicks: UInt8(clamping: event.clickCount),
            mods: timelineMods(event.modifierFlags))
    }

    func press(_ pointer: TimelinePointer) {}
    func move(_ pointer: TimelinePointer) {}
    func release(_ pointer: TimelinePointer) {}

    /// F154. `makeFirstResponder` doesn't ask `acceptsFirstResponder`, so a surface that refuses focus has to not ask for it.
    private func takeFocus() {
        if acceptsFirstResponder {
            window?.makeFirstResponder(self)
        }
    }

    override func mouseDown(with event: NSEvent) {
        takeFocus()
        pressIsRight = event.modifierFlags.contains(.control)
        press(pointer(event, button: pressIsRight ? .right : .left))
    }

    override func mouseDragged(with event: NSEvent) {
        move(pointer(event, button: pressIsRight ? .right : .left))
    }

    override func mouseUp(with event: NSEvent) {
        release(pointer(event, button: pressIsRight ? .right : .left))
        pressIsRight = false
    }

    override func rightMouseDown(with event: NSEvent) {
        takeFocus()
        press(pointer(event, button: .right))
    }

    override func rightMouseDragged(with event: NSEvent) {
        move(pointer(event, button: .right))
    }

    override func rightMouseUp(with event: NSEvent) {
        release(pointer(event, button: .right))
    }

    override func otherMouseDown(with event: NSEvent) {
        if event.buttonNumber == 2 {
            takeFocus()
            press(pointer(event, button: .middle))
        }
    }

    override func otherMouseDragged(with event: NSEvent) {
        if event.buttonNumber == 2 {
            move(pointer(event, button: .middle))
        }
    }

    override func otherMouseUp(with event: NSEvent) {
        if event.buttonNumber == 2 {
            release(pointer(event, button: .middle))
        }
    }

    override func mouseMoved(with event: NSEvent) {
        move(pointer(event, button: nil))
    }

    // F11 to F14.
    var wheelSurface: TimelineSurfaceKind { .grid }

    override func scrollWheel(with event: NSEvent) {
        guard let canvas else { return }
        let inPanel = canvas.timePanel.convert(event.locationInWindow, from: nil)
        let inGrid = canvas.grid.convert(event.locationInWindow, from: nil)
        let divider = canvas.view.dividerThickness
        canvas.apply(
            controller.scrollWheel(
                surface: wheelSurface.bridged, dx: Double(event.scrollingDeltaX), dy: Double(event.scrollingDeltaY), precise: event.hasPreciseScrollingDeltas,
                mods: timelineMods(event.modifierFlags), pointerOffsetX: Int32(inPanel.x + divider), gridY: Int32((canvas.grid.bounds.height - inGrid.y).rounded(.down))))
    }

    // F17.
    override func magnify(with event: NSEvent) {
        guard let canvas, wheelSurface != .rowList else { return }
        let inPanel = canvas.timePanel.convert(event.locationInWindow, from: nil)
        canvas.apply(controller.magnify(magnification: Double(event.magnification), pointerOffsetX: Int32(inPanel.x + canvas.view.dividerThickness), mods: timelineMods(event.modifierFlags)))
    }
}

enum TimelineSurfaceKind {
    case grid, ruler, waveform, rowList, marksBar

    var bridged: TimelineSurface_ {
        switch self {
        case .grid: return .grid
        case .ruler: return .ruler
        case .waveform: return .waveform
        case .rowList: return .rowList
        case .marksBar: return .marksBar
        }
    }
}

typealias TimelineSurface_ = VixenBridge.TimelineSurface
