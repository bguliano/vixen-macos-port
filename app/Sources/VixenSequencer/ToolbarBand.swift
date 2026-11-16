import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// `toolStripContainer`'s top panel: rows of independently draggable strips, which `NSToolbar` can't do. A strip dropped into a row goes in at that point, and on drop
/// every strip packs left, because rows hold only order (F98).
struct ToolbarBand: View {
    let model: SequencerModel
    @State private var stripFrames: [ShellStrip: CGRect] = [:]
    @State private var rowFrames: [Int: CGRect] = [:]
    @State private var dragging: ShellStrip?
    @State private var dragOffset: CGSize = .zero

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            let rows = visibleRows
            ForEach(rows.indices, id: \.self) { index in
                StripRowLayout(spacing: 3) {
                    ForEach(rows[index], id: \.strip) { info in
                        StripView(model: model, info: info) { translation in
                            dragging = info.strip
                            dragOffset = translation
                        } dragEnded: { translation in
                            finishDrag(info.strip, translation: translation)
                        }
                        .offset(dragging == info.strip ? dragOffset : .zero)
                        .zIndex(dragging == info.strip ? 1 : 0)
                        .onGeometryChange(for: CGRect.self) { $0.frame(in: .named("toolbarBand")) } action: { stripFrames[info.strip] = $0 }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onGeometryChange(for: CGRect.self) { $0.frame(in: .named("toolbarBand")) } action: { rowFrames[index] = $0 }
            }
        }
        .coordinateSpace(name: "toolbarBand")
        .padding(.horizontal, 3)
        .padding(.vertical, 2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.bar)
        // Right-click the empty panel: only Toolbars (F100).
        .contextMenu {
            ToolbarsList(model: model)
        }
        .disabled(model.ui == nil || model.app.systemConfigSessionActive)
    }

    /// Rows holding at least one showing strip. Each keeps its index in the full list out of the picture: drops are in terms of what's visible.
    private var visibleRows: [[ShellStripInfo]] {
        (model.toolbars?.rows ?? []).map { $0.filter(\.visible) }.filter { !$0.isEmpty }
    }

    /// `toolStrip_EndDrag`.
    private func finishDrag(_ strip: ShellStrip, translation: CGSize) {
        dragging = nil
        dragOffset = .zero
        guard let toolbars = model.toolbars, let frame = stripFrames[strip] else { return }
        let point = CGPoint(x: frame.midX + translation.width, y: frame.midY + translation.height)
        let shown = visibleRows
        // The row under the drop, or a new one past the last.
        guard let target = rowFrames.filter({ $0.key < shown.count }).first(where: { point.y <= $0.value.maxY && point.y >= $0.value.minY })?.key else {
            let fullIndex = point.y < 0 ? 0 : toolbars.rows.count
            model.moveStrip(strip, row: fullIndex, index: 0)
            return
        }
        // Translate to the full row list: hidden strips stay where they are.
        let row = shown[target]
        guard let fullRow = toolbars.rows.firstIndex(where: { full in full.contains(where: { $0.strip == row[0].strip }) }) else { return }
        let before = row.first { stripFrames[$0.strip].map { point.x < $0.midX } ?? false }
        let index = before.flatMap { b in toolbars.rows[fullRow].firstIndex { $0.strip == b.strip } } ?? toolbars.rows[fullRow].count
        model.moveStrip(strip, row: fullRow, index: index)
    }
}

/// One strip with its grip. The items are per strip, below.
struct StripView: View {
    let model: SequencerModel
    let info: ShellStripInfo
    let dragChanged: (CGSize) -> Void
    let dragEnded: (CGSize) -> Void
    @State private var showingItems = false

    var body: some View {
        HStack(spacing: 1) {
            StripGrip(dragChanged: dragChanged, dragEnded: dragEnded)
                .accessibilityLabel("Move \(info.title) toolbar")
            items
        }
        .padding(.trailing, 3)
        .padding(.vertical, 1)
        .background(Theme.background.opacity(0.35), in: RoundedRectangle(cornerRadius: 4))
        .contextMenu {
            StripContextMenu(model: model, info: info) { showingItems = true }
        }
        .popover(isPresented: $showingItems, arrowEdge: .bottom) {
            StripItemsPopover(model: model, info: info)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(info.name)
        .accessibilityLabel(info.title)
    }

    @ViewBuilder
    private var items: some View {
        let context = StripContext(model: model, info: info) { showingItems = true }
        switch info.strip {
        case .playBack: PlayBackStrip(context: context)
        case .edit: EditStrip(context: context)
        case .mode: ModeStrip(context: context)
        case .view: ViewStrip(context: context)
        case .audio: AudioStrip(context: context)
        case .file: FileStrip(context: context)
        case .alignment: AlignmentStrip(context: context)
        case .tools: ToolsStrip(context: context)
        case .effects: EffectsStrip(context: context)
        case .libraryColor: LibraryStrip(context: context, kind: .color)
        case .libraryCurve: LibraryStrip(context: context, kind: .curve)
        case .libraryGradient: LibraryStrip(context: context, kind: .gradient)
        }
    }
}

/// The dotted grip a strip is dragged by.
struct StripGrip: View {
    let dragChanged: (CGSize) -> Void
    let dragEnded: (CGSize) -> Void

    var body: some View {
        Canvas { context, size in
            let dot = Path(ellipseIn: CGRect(x: 0, y: 0, width: 2, height: 2))
            var y: CGFloat = 3
            while y < size.height - 3 {
                context.fill(dot.offsetBy(dx: 1, dy: y), with: .color(.secondary))
                y += 4
            }
        }
        .frame(width: 5)
        .frame(maxHeight: .infinity)
        .padding(.horizontal, 2)
        .contentShape(Rectangle())
        .onHover { inside in
            if inside {
                NSCursor.openHand.push()
            } else {
                NSCursor.pop()
            }
        }
        .gesture(
            DragGesture(minimumDistance: 3, coordinateSpace: .named("toolbarBand"))
                .onChanged { dragChanged($0.translation) }
                .onEnded { dragEnded($0.translation) }
        )
    }
}

/// `contextMenuStripAll`: Show/Hide Items, Toolbars, Toggle Image Size, Reset Toolbar. F100 to F104. The Effects strip has its own (F151).
struct StripContextMenu: View {
    let model: SequencerModel
    let info: ShellStripInfo
    let showItems: () -> Void

    var body: some View {
        if info.strip == .effects {
            EffectsStripMenu(model: model, showEffects: showItems)
        } else {
            Button("Show/Hide Items...", action: showItems)
            Menu("Toolbars") {
                ToolbarsList(model: model)
            }
            Button("Toggle Image Size") { model.toggleStripSize(info.strip) }
            Button("Reset Toolbar") { model.resetStrip(info.strip) }
        }
    }
}

/// F101, F152, F177: upstream keeps these menus open while ticking, which a Mac menu can't. A popover does, and has the room for a long list.
struct StripItemsPopover: View {
    let model: SequencerModel
    let info: ShellStripInfo

    private struct Row: Identifiable {
        let id: String
        let title: String
    }

    private var libraryKind: LibraryKind? {
        switch info.strip {
        case .libraryColor: .color
        case .libraryCurve: .curve
        case .libraryGradient: .gradient
        default: nil
        }
    }

    private var rows: [Row] {
        if let libraryKind {
            return model.entries(libraryKind).map { Row(id: $0.name, title: $0.name) }
        }
        return (model.shell?.stripItems(strip: info.strip) ?? []).map { Row(id: $0.name, title: $0.tooltip) }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 4) {
                if info.strip == .effects {
                    effectGroups
                } else {
                    ForEach(rows) { row in
                        Toggle(row.title, isOn: Binding(
                            get: { model.isItemVisible(info.name, row.id) },
                            set: { model.setItem(info.strip, item: row.id, visible: $0) }
                        ))
                    }
                }
            }
            .padding(12)
        }
        .frame(minWidth: 220, maxHeight: 480)
    }

    @ViewBuilder
    private var effectGroups: some View {
        let prefs = model.ui?.prefs
        group("Basic", .basic, .basicEffects, prefs?.basicEffects ?? true)
        group("Pixel", .pixel, .pixelEffects, prefs?.pixelEffects ?? true)
        group("Device", .device, .deviceEffects, prefs?.deviceEffects ?? true)
        group("Fixture", .intelligentFixture, .fixtureEffects, prefs?.fixtureEffects ?? true)
    }

    private func group(_ title: String, _ group: ShellEffectGroup, _ option: ShellOption, _ isOn: Bool) -> some View {
        DisclosureGroup {
            ForEach(model.effects.filter { $0.group == group }, id: \.typeId) { effect in
                Toggle(effect.name, isOn: Binding(
                    get: { model.isItemVisible("toolStripEffects", effect.name) },
                    set: { on in
                        // Clicking an effect check turns its group on too.
                        model.set(option, true)
                        model.setItem(.effects, item: effect.name, visible: on)
                    }
                ))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        } label: {
            Toggle(title, isOn: Binding(get: { isOn }, set: { model.set(option, $0) }))
        }
    }
}

/// The Toolbars list, alphabetical, in every place it appears. F99.
struct ToolbarsList: View {
    let model: SequencerModel

    var body: some View {
        ForEach(model.toolbars?.list ?? [], id: \.strip) { info in
            Toggle(info.title, isOn: Binding(
                get: { model.toolbars?.list.first { $0.strip == info.strip }?.visible ?? false },
                set: { model.setStrip(info.strip, visible: $0) }
            ))
        }
    }
}

/// What every item needs: the model, the strip's size, and whether it's shown.
struct StripContext {
    let model: SequencerModel
    let info: ShellStripInfo
    let showItems: () -> Void

    var ui: ShellUiState? { model.ui }

    func shows(_ item: String) -> Bool {
        model.isItemVisible(info.name, item)
    }

    /// 28 points large, 16 small: `ChangeToolStripImageSize`.
    var imageSize: CGFloat { info.large ? 28 : 16 }
}

/// An icon: an SF Symbol where one clearly means the same thing, otherwise Vixen's own.
enum ToolImage {
    case symbol(String)
    case resource(String)
    /// An effect's own picture.
    case effect(String)
    /// A library swatch or thumbnail, drawn only when it's shown.
    case drawn(() -> NSImage)
}

struct ToolIcon: View {
    let image: ToolImage
    let size: CGFloat

    var body: some View {
        switch image {
        case .symbol(let name):
            Image(systemName: name)
                .resizable()
                .scaledToFit()
                .padding(size * 0.14)
                .frame(width: size, height: size)
        case .resource(let name):
            CachedResourceImage(path: "images/sequencer/\(name).png", size: size)
        case .effect(let name):
            CachedResourceImage(path: "effects/\(name).png", size: size)
        case .drawn(let draw):
            Image(nsImage: draw())
                .resizable()
                .frame(width: size, height: size)
        }
    }
}

/// Icons load once and stay: toolbars redraw often.
struct CachedResourceImage: View {
    let path: String
    let size: CGFloat

    @MainActor private static var cache: [String: NSImage] = [:]

    var body: some View {
        if let image = Self.load(path) {
            Image(nsImage: image)
                .resizable()
                .interpolation(.high)
                .scaledToFit()
                .frame(width: size, height: size)
        } else {
            Color.clear.frame(width: size, height: size)
        }
    }

    private static func load(_ path: String) -> NSImage? {
        if let hit = cache[path] {
            return hit
        }
        let image = VixenResources.image(path)
        if let image {
            cache[path] = image
        }
        return image
    }
}

struct ToolButton: View {
    let id: String
    let tooltip: String
    let image: ToolImage
    let context: StripContext
    var enabled = true
    let action: () -> Void

    var body: some View {
        if context.shows(id) {
            Button(action: action) {
                ToolIcon(image: image, size: context.imageSize)
            }
            .buttonStyle(StripButtonStyle())
            .disabled(!enabled)
            .help(tooltip)
            .accessibilityLabel(tooltip)
            .accessibilityIdentifier(id)
            .stripEntry(StripEntry(title: tooltip, image: image, enabled: enabled, action: action))
        }
    }
}

struct ToolToggle: View {
    let id: String
    let tooltip: String
    let image: ToolImage
    let context: StripContext
    let isOn: Bool
    let set: (Bool) -> Void

    var body: some View {
        if context.shows(id) {
            Toggle(isOn: Binding(get: { isOn }, set: set)) {
                ToolIcon(image: image, size: context.imageSize)
            }
            .toggleStyle(StripToggleStyle())
            .help(tooltip)
            .accessibilityLabel(tooltip)
            .accessibilityIdentifier(id)
            .stripEntry(StripEntry(title: tooltip, image: image, checked: isOn, action: { set(!isOn) }))
        }
    }
}

// MARK: PlayBack, F105 to F109

struct PlayBackStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        let transport = context.ui?.transport
        StripItems(context) {
            ToolButton(id: "playBackToolStripButton_Start", tooltip: "Start", image: .symbol("backward.end.fill"), context: context) {
                model.run(.gotoStart)
            }
            ToolButton(
                id: "playBackToolStripButton_Play",
                tooltip: transport?.playTooltip ?? "Play F5",
                image: playImage(transport?.icon),
                context: context,
                enabled: transport?.playEnabled ?? false
            ) {
                model.run(.playPause)
            }
            ToolButton(
                id: "playBackToolStripButton_Stop",
                tooltip: "Stop F8",
                image: .symbol("stop.fill"),
                context: context,
                enabled: transport?.stopEnabled ?? false
            ) {
                model.run(.stop)
            }
            ToolButton(id: "playBackToolStripButton_End", tooltip: "End", image: .symbol("forward.end.fill"), context: context) {
                model.run(.gotoEnd)
            }
            ToolToggle(id: "playBackToolStripButton_Loop", tooltip: "Loop F9", image: .symbol("repeat"), context: context, isOn: transport?.looping ?? false) { _ in
                model.run(.toggleLoop)
            }
        }
    }

    private func playImage(_ icon: ShellPlayIcon?) -> ToolImage {
        switch icon {
        case .pause: .symbol("pause.fill")
        case .hourglass: .symbol("hourglass")
        default: .symbol("play.fill")
        }
    }
}

// MARK: Edit, F110 to F117

struct EditStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        let ui = context.ui
        StripItems(context) {
            if context.shows("editToolStripButton_Undo") {
                UndoSplitButton(context: context, redo: false)
            }
            if context.shows("editToolStripButton_Redo") {
                UndoSplitButton(context: context, redo: true)
            }
            ToolButton(id: "editToolStripButton_Cut", tooltip: "Cut", image: .symbol("scissors"), context: context, enabled: ui?.hasSelection ?? false) {
                model.cut()
            }
            ToolButton(id: "editToolStripButton_Copy", tooltip: "Copy", image: .symbol("doc.on.doc"), context: context, enabled: ui?.hasSelection ?? false) {
                model.copy()
            }
            ToolButton(id: "editToolStripButton_Paste", tooltip: "Paste", image: .symbol("doc.on.clipboard"), context: context, enabled: ui?.pasteEnabled ?? false) {
                model.paste(.default)
            }
            if context.shows("editToolStripButton_PasteDropDown") {
                Menu {
                    // Hidden rather than disabled when they don't apply (F116, F117).
                    if ui?.pasteInvertEnabled ?? false {
                        Button("Paste - Invert") { model.paste(.invert) }
                    }
                    if ui?.pasteMarksEnabled ?? false {
                        Button("Paste - Visible Mark/s") { model.paste(.visibleMarks) }
                    }
                } label: {
                    ToolIcon(image: .resource("paste_special"), size: context.imageSize)
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.visible)
                .fixedSize()
                .disabled(!(ui?.pasteSpecialEnabled ?? false))
                .help("Paste Special")
                .accessibilityLabel("Paste Special")
                .accessibilityIdentifier("editToolStripButton_PasteDropDown")
                .stripEntry(StripEntry(title: "Paste Special", image: .resource("paste_special"), enabled: ui?.pasteSpecialEnabled ?? false, submenu: {
                    var items: [StripEntry] = []
                    if model.ui?.pasteInvertEnabled ?? false {
                        items.append(StripEntry(title: "Paste - Invert", image: .resource("paste_invert"), action: { model.paste(.invert) }))
                    }
                    if model.ui?.pasteMarksEnabled ?? false {
                        items.append(StripEntry(title: "Paste - Visible Mark/s", image: .resource("paste_marks"), action: { model.paste(.visibleMarks) }))
                    }
                    return items
                }))
            }
        }
        .onAppear { model.updateClipboardCount() }
    }
}

/// `UndoButton`: the main click undoes one step; the arrow lists descriptions, newest first, and hovering highlights down to that row. F110, F111, F420.
/// The list is fetched when it opens, never per action (hazard 15).
struct UndoSplitButton: View {
    let context: StripContext
    let redo: Bool
    @State private var listing = false

    private var title: String { redo ? "Redo" : "Undo" }
    private var enabled: Bool { (redo ? context.ui?.redoName : context.ui?.undoName) != nil }

    var body: some View {
        let image = ToolImage.symbol(redo ? "arrow.uturn.forward" : "arrow.uturn.backward")
        let model = context.model
        HStack(spacing: 0) {
            Button {
                redo ? model.redo() : model.undo()
            } label: {
                ToolIcon(image: image, size: context.imageSize)
            }
            .buttonStyle(StripButtonStyle())
            .help(title)
            .accessibilityLabel(title)
            .accessibilityIdentifier(redo ? "editToolStripButton_Redo" : "editToolStripButton_Undo")
            Button {
                listing = true
            } label: {
                Image(systemName: "chevron.down")
                    .font(.system(size: 8, weight: .semibold))
                    .frame(width: 8, height: context.imageSize)
            }
            .buttonStyle(StripButtonStyle())
            .help("\(title) list")
            .accessibilityLabel("\(title) list")
            .popover(isPresented: $listing, arrowEdge: .bottom) {
                UndoListView(context: context, redo: redo) { listing = false }
            }
        }
        .disabled(!enabled)
        .stripEntry(StripEntry(title: title, image: image, enabled: enabled, action: { redo ? model.redo() : model.undo() }))
    }
}

/// `UndoDropDownControl`: footer `Undo <n> Action[s]`, and choosing a row runs that many.
private struct UndoListView: View {
    let context: StripContext
    let redo: Bool
    let dismiss: () -> Void
    @State private var items: [String] = []
    @State private var hovered: Int?

    private var footer: String {
        let n = (hovered ?? -1) + 1
        return "\(redo ? "Redo" : "Undo") \(n) Action\(n == 1 ? "" : "s")"
    }

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(items.indices, id: \.self) { index in
                        Button {
                            let steps = UInt32(index + 1)
                            redo ? context.model.redo(steps) : context.model.undo(steps)
                            dismiss()
                        } label: {
                            Text(items[index])
                                .lineLimit(1)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 2)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(index <= (hovered ?? -1) ? Color.accentColor.opacity(0.5) : Color.clear)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .onHover { inside in
                            if inside {
                                hovered = index
                            } else if hovered == index {
                                hovered = nil
                            }
                        }
                    }
                }
            }
            .frame(width: 200, height: 200)
            Divider()
            Text(footer)
                .font(.callout)
                .monospacedDigit()
                .padding(4)
        }
        .onAppear {
            if let session = context.model.session {
                items = redo ? session.redoDescriptions() : session.undoDescriptions()
            }
        }
    }
}

// MARK: Mode, F118 to F123

struct ModeStrip: View {
    let context: StripContext
    @State private var filterOpen = false

    var body: some View {
        let model = context.model
        let ui = context.ui
        StripItems(context) {
            ToolToggle(id: "modeToolStripButton_DrawMode", tooltip: "Draw Mode", image: .symbol("pencil"), context: context, isOn: ui?.drawMode ?? false) { _ in
                // Clicking while on stays on.
                model.run(.drawMode)
            }
            ToolToggle(id: "modeToolStripButton_SelectionMode", tooltip: "Selection Mode", image: .symbol("cursorarrow"), context: context, isOn: ui?.selectionMode ?? true) { _ in
                model.run(.selectionMode)
            }
            ToolToggle(id: "modeToolStripButton_SnapTo", tooltip: "Snap To Marks / Effect", image: .resource("magnet"), context: context, isOn: ui?.prefs.snapTo ?? true) {
                model.set(.snapTo, $0)
            }
            if context.shows("modeToolStripDropDownButton_SnapToStrength") {
                Menu {
                    ForEach(1...4, id: \.self) { strength in
                        Toggle("\(strength)", isOn: Binding(
                            get: { (ui?.prefs.snapStrength ?? 2) == strength },
                            set: { _ in setSnapStrength(strength) }
                        ))
                    }
                } label: {
                    Text("\(ui?.prefs.snapStrength ?? 2)")
                        .font(.caption)
                        .monospacedDigit()
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.visible)
                .fixedSize()
                .help("Snap Strength")
                .accessibilityLabel("Snap Strength")
                .accessibilityIdentifier("modeToolStripDropDownButton_SnapToStrength")
                .stripEntry(StripEntry(title: "Snap Strength", submenu: {
                    (1...4).map { strength in
                        StripEntry(title: "\(strength)", checked: (model.ui?.prefs.snapStrength ?? 2) == strength, action: { setSnapStrength(strength) })
                    }
                }))
            }
            ToolToggle(id: "modeToolStripButton_DragBoxFilter", tooltip: "Drag Box Filter", image: .resource("table_select_big"), context: context, isOn: ui?.dragBoxFilter ?? false) {
                model.set(.dragBoxFilter, $0)
            }
            if context.shows("modeToolStripDropDownButton_DragBoxFilter") {
                Button {
                    filterOpen = true
                } label: {
                    Image(systemName: "chevron.down")
                        .font(.system(size: 8, weight: .semibold))
                        .frame(width: 8, height: context.imageSize)
                }
                .buttonStyle(StripButtonStyle())
                .help("Drag Box Filter")
                .accessibilityLabel("Drag Box Filter list")
                .accessibilityIdentifier("modeToolStripDropDownButton_DragBoxFilter")
                .popover(isPresented: $filterOpen, arrowEdge: .bottom) {
                    DragBoxFilterList(model: model)
                }
                .stripEntry(StripEntry(title: "Drag Box Filter", submenu: { DragBoxFilterList.entries(model) }))
            }
        }
    }

    private func setSnapStrength(_ strength: Int) {
        context.model.shell?.setSnapStrength(strength: Int32(strength))
        context.model.refresh()
    }
}

/// A popover of checkboxes, so several can be ticked in one visit: `NSMenu` always closes. F123.
struct DragBoxFilterList: View {
    let model: SequencerModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 3) {
                Toggle("Invert Selection", isOn: Binding(
                    get: { model.ui?.dragBoxInvertChecked ?? false },
                    set: { _ in model.run(.invertDragBoxFilter) }
                ))
                Divider()
                ForEach(model.effects, id: \.typeId) { effect in
                    Toggle(isOn: Binding(
                        get: { model.ui?.dragBoxFilterTypes.contains(effect.typeId) ?? false },
                        set: { _ in
                            model.shell?.toggleDragBoxType(typeId: effect.typeId)
                            model.refresh()
                        }
                    )) {
                        HStack(spacing: 4) {
                            ToolIcon(image: .effect(effect.icon), size: 16)
                            Text(effect.name)
                        }
                    }
                }
            }
            .padding(10)
        }
        .frame(width: 220, height: 420)
    }

    /// The same list in the overflow menu, where each choice closes it.
    static func entries(_ model: SequencerModel) -> [StripEntry] {
        var items = [StripEntry(title: "Invert Selection", checked: model.ui?.dragBoxInvertChecked ?? false, action: { model.run(.invertDragBoxFilter) })]
        for effect in model.effects {
            items.append(StripEntry(title: effect.name, image: .effect(effect.icon), checked: model.ui?.dragBoxFilterTypes.contains(effect.typeId) ?? false, action: {
                model.shell?.toggleDragBoxType(typeId: effect.typeId)
                model.refresh()
            }))
        }
        return items
    }
}

// MARK: View, F124 to F127

struct ViewStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        StripItems(context) {
            ToolButton(id: "viewToolStripButton_ZoomTimeIn", tooltip: "Zoom Time In", image: .resource("zoom_time_in"), context: context) {
                model.run(.zoomTimeIn)
            }
            ToolButton(id: "viewToolStripButton_ZoomTimeOut", tooltip: "Zoom Time Out", image: .resource("zoom_time_out"), context: context) {
                model.run(.zoomTimeOut)
            }
            ToolButton(id: "viewToolStripButton_ZoomRowIn", tooltip: "Zoom Row In", image: .resource("zoom_row_in"), context: context) {
                model.run(.zoomRowsIn)
            }
            ToolButton(id: "viewToolStripButton_ZoomRowOut", tooltip: "Zoom Row Out", image: .resource("zoom_row_out"), context: context) {
                model.run(.zoomRowsOut)
            }
        }
    }
}

// MARK: Audio, F128 to F133

struct AudioStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        let ui = context.ui
        StripItems(context) {
            ToolButton(id: "toolStripButton_AssociateAudio", tooltip: ui?.audioTooltip ?? "Associate Audio", image: .symbol("music.note"), context: context) {
                model.associateAudio()
            }
            ToolButton(
                id: "toolStripButton_DecreaseTimingSpeed",
                tooltip: "Decrease Timing speed",
                image: .symbol("minus.circle.fill"),
                context: context,
                enabled: ui?.speed.decreaseEnabled ?? false
            ) {
                model.run(.speedDecrease)
            }
            if context.shows("audioToolStripLabel_TimingSpeed") {
                SpeedGaugeView(context: context)
                    .stripEntry(StripEntry(
                        title: "Timing Speed: \(ui?.speed.gaugeText ?? "100")%",
                        enabled: ui?.speed.gaugeEnabled ?? false,
                        action: { model.run(.speedReset) }
                    ))
            }
            ToolButton(
                id: "toolStripButton_IncreaseTimingSpeed",
                tooltip: "Increasing Timing speed",
                image: .symbol("plus.circle.fill"),
                context: context,
                enabled: ui?.speed.increaseEnabled ?? false
            ) {
                model.run(.speedIncrease)
            }
            if context.shows("audioToolStripButton_Audio_Devices") {
                AudioDevicesMenu(context: context)
                    .stripEntry(StripEntry(title: "Audio Devices", image: .resource("Audio_Devices"), submenu: {
                        (model.shell?.audioDevices() ?? []).map { device in
                            StripEntry(title: device.name, checked: device.checked, action: { model.shell?.setAudioDevice(id: device.id) })
                        }
                    }))
            }
        }
    }
}

/// F132: rebuilt each opening, and the check moves with the choice.
private struct AudioDevicesMenu: View {
    let context: StripContext
    @State private var devices: [ShellAudioDevice] = []

    var body: some View {
        Menu {
            ForEach(devices, id: \.id) { device in
                Toggle(device.name, isOn: Binding(get: { device.checked }, set: { _ in
                    context.model.shell?.setAudioDevice(id: device.id)
                    reload()
                }))
            }
        } label: {
            ToolIcon(image: .resource("Audio_Devices"), size: context.imageSize)
        }
        .menuIndicator(.hidden)
        .menuStyle(.borderlessButton)
        .fixedSize()
        .onHover { inside in
            if inside {
                reload()
            }
        }
        .help("Audio Devices")
        .accessibilityLabel("Audio Devices")
        .accessibilityIdentifier("audioToolStripButton_Audio_Devices")
    }

    private func reload() {
        devices = context.model.shell?.audioDevices() ?? []
    }
}

/// `SpeedVisualisation`: drawn from `speed_gauge`, 32 by 32. Click for 100%, double-click for 400%. The wheel works even disabled (KEEP F130). F133.
struct SpeedGaugeView: View {
    let context: StripContext

    var body: some View {
        let speed = context.ui?.speed
        Canvas { canvas, size in
            guard let speed else { return }
            let scale = size.width / 32
            canvas.scaleBy(x: scale, y: scale)
            let color = Color(.sRGB, red: Double(speed.gaugeRed) / 255, green: Double(speed.gaugeGreen) / 255, blue: Double(speed.gaugeBlue) / 255, opacity: 1)
            canvas.draw(
                Text(speed.gaugeText).font(.system(size: 10, weight: .bold)).foregroundStyle(color),
                at: CGPoint(x: CGFloat(speed.gaugeTextX), y: 0),
                anchor: .topLeading
            )
            // `DrawArc(arch, Rectangle(3, 16, 26, 29), 180, 180)`.
            var arc = Path()
            arc.addArc(
                center: CGPoint(x: 16, y: 16 + 14.5),
                radius: 13,
                startAngle: .degrees(180),
                endAngle: .degrees(360),
                clockwise: false,
                transform: CGAffineTransform(translationX: 16, y: 30.5).scaledBy(x: 1, y: 14.5 / 13).translatedBy(x: -16, y: -30.5)
            )
            canvas.stroke(arc, with: .color(color), lineWidth: 5)
            var needle = Path()
            needle.move(to: CGPoint(x: 16, y: 32))
            needle.addLine(to: CGPoint(x: CGFloat(speed.needleEndX), y: CGFloat(speed.needleEndY)))
            canvas.stroke(needle, with: .color(Theme.foreground), lineWidth: 3)
        }
        .frame(width: max(context.imageSize, 24), height: max(context.imageSize, 24))
        .contentShape(Rectangle())
        .opacity((speed?.gaugeEnabled ?? false) ? 1 : 0.5)
        // The double click runs first, then the single, as a tap on each.
        .onTapGesture(count: 2) {
            if speed?.gaugeEnabled ?? false {
                context.model.run(.speedMax)
            }
        }
        .onTapGesture {
            if speed?.gaugeEnabled ?? false {
                context.model.run(.speedReset)
            }
        }
        .overlay {
            ScrollCatcher { up in
                context.model.run(up ? .speedWheelUp : .speedWheelDown)
            }
        }
        .help("Timing speed")
        .accessibilityLabel("Timing speed")
        .accessibilityValue(speed?.gaugeText ?? "")
        .accessibilityIdentifier("audioToolStripLabel_TimingSpeed")
    }
}

/// Hears the wheel and two-finger scrolling over a view, and leaves clicks to the view underneath.
struct ScrollCatcher: NSViewRepresentable {
    let scrolled: (Bool) -> Void

    final class CatcherView: NSView {
        var scrolled: ((Bool) -> Void)?
        private var accumulated: CGFloat = 0

        override func scrollWheel(with event: NSEvent) {
            // One step per notch, or per some travel on a trackpad.
            accumulated += event.scrollingDeltaY
            let threshold: CGFloat = event.hasPreciseScrollingDeltas ? 12 : 0.5
            if abs(accumulated) >= threshold {
                scrolled?(accumulated > 0)
                accumulated = 0
            }
        }

        override func hitTest(_ point: NSPoint) -> NSView? {
            // Only scroll events come here. A click passes through.
            NSApp.currentEvent?.type == .scrollWheel ? super.hitTest(point) : nil
        }
    }

    func makeNSView(context: Context) -> CatcherView {
        let view = CatcherView()
        view.scrolled = scrolled
        return view
    }

    func updateNSView(_ nsView: CatcherView, context: Context) {
        nsView.scrolled = scrolled
    }
}

// MARK: File, F134 to F138

struct FileStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        StripItems(context) {
            ToolButton(id: "fileToolStripButton_Save", tooltip: "Save", image: .resource("Save"), context: context) {
                Task { await model.save(forcePanel: false) }
            }
            ToolButton(id: "fileToolStripButton_SaveAs", tooltip: "Save As", image: .resource("SaveAs"), context: context) {
                Task { await model.save(forcePanel: true) }
            }
            ToolToggle(id: "fileToolStripButton_AutoSave", tooltip: "Auto Save", image: .resource("Auto_Save"), context: context, isOn: context.ui?.prefs.autoSave ?? true) {
                model.set(.autoSave, $0)
            }
            // Upstream's button has no handler. It's wired to the same place as the menu item here (F137's FIX).
            ToolButton(id: "fileToolStripButton_Export", tooltip: "Export", image: .symbol("square.and.arrow.up"), context: context, enabled: model.canExport) {
                model.export()
            }
            ToolButton(id: "fileToolStripButton_Close", tooltip: "Close", image: .symbol("xmark"), context: context) {
                model.window?.performClose(nil)
            }
        }
    }
}

// MARK: Alignment, F139 to F145

struct AlignmentStrip: View {
    let context: StripContext

    var body: some View {
        let model = context.model
        let ui = context.ui
        StripItems(context) {
            ToolButton(id: "alignmentToolStripButton_CloseGaps", tooltip: "Close Gaps", image: .resource("fill_gaps"), context: context) { model.run(.closeGaps) }
            if context.shows("alignmentToolStripDropDownButton_CloseGaps") {
                ThresholdMenu(
                    id: "alignmentToolStripDropDownButton_CloseGaps",
                    tooltip: "Close Gaps",
                    choices: [".100", ".200", ".300", ".400"],
                    current: ui?.prefs.closeGapThreshold ?? ".100"
                ) {
                    model.shell?.setCloseGapThreshold(text: $0)
                    model.refresh()
                }
            }
            if context.shows("alignmentToolStripDropDownButton_AlignTo") {
                ThresholdMenu(
                    id: "alignmentToolStripDropDownButton_AlignTo",
                    tooltip: "Align To",
                    choices: [".200", ".400", ".600", ".800", "1.000", "2.000"],
                    current: ui?.prefs.alignToThreshold ?? ".800"
                ) {
                    model.shell?.setAlignToThreshold(text: $0)
                    model.refresh()
                }
            }
            ToolButton(id: "alignmentToolStripButton_Start", tooltip: "Align Start to nearest mark", image: .resource("alignStartMark"), context: context) {
                model.run(.alignStartToMarks)
            }
            ToolButton(id: "alignmentToolStripButton_End", tooltip: "Align End to nearest mark", image: .resource("alignEndMark"), context: context) {
                model.run(.alignEndToMarks)
            }
            ToolButton(id: "alignmentToolStripButton_Both", tooltip: "Align Both to nearest mark", image: .resource("alignBothMark"), context: context) {
                model.run(.alignBothToMarks)
            }
            ToolButton(id: "alignmentToolStripButton_Distribute", tooltip: "Distribute Mark", image: .resource("distribute"), context: context) {
                model.run(.distributeEqually)
            }
        }
    }
}

/// An exclusive choice of a time text. F140, F141. A hand-edited value matching none leaves nothing checked but stays in effect.
struct ThresholdMenu: View {
    let id: String
    let tooltip: String
    let choices: [String]
    let current: String
    let choose: (String) -> Void

    var body: some View {
        Menu {
            ForEach(choices, id: \.self) { choice in
                Toggle(choice, isOn: Binding(get: { current == choice }, set: { _ in choose(choice) }))
            }
        } label: {
            Text(current)
                .font(.caption)
                .monospacedDigit()
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.visible)
        .fixedSize()
        .help(tooltip)
        .accessibilityLabel(tooltip)
        .accessibilityIdentifier(id)
        .stripEntry(StripEntry(title: tooltip, submenu: {
            choices.map { choice in StripEntry(title: choice, checked: current == choice, action: { choose(choice) }) }
        }))
    }
}

// MARK: Tools, F146 to F149

struct ToolsStrip: View {
    let context: StripContext

    var body: some View {
        StripItems(context) {
            ToolButton(id: "toolsToolStripButton_LipSync", tooltip: "Edit LipSync Map", image: .resource("Lipsync"), context: context, enabled: SequencerModel.hasImageMaps) { context.model.openImageMaps() }
            ToolButton(id: "toolsToolStripButton_CurveLibrary", tooltip: "Curve Library", image: .resource("Curve"), context: context) { context.model.openCurveSelector() }
            ToolButton(id: "toolsToolStripButton_ColorLibrary", tooltip: "Color Library", image: .resource("Color"), context: context) { context.model.openColorCollections() }
            ToolButton(id: "toolsToolStripButton_ColorGradient", tooltip: "Color Gradient Library", image: .resource("ColorGradient"), context: context) { context.model.openGradientSelector() }
        }
    }
}

// MARK: Effects, F150 to F163

struct EffectsStrip: View {
    let context: StripContext

    private func groupOn(_ group: ShellEffectGroup) -> Bool {
        guard let prefs = context.ui?.prefs else { return true }
        switch group {
        case .basic: return prefs.basicEffects
        case .pixel: return prefs.pixelEffects
        case .device: return prefs.deviceEffects
        case .intelligentFixture: return prefs.fixtureEffects
        }
    }

    /// Basic, Pixel, Device, Fixture, in descriptor order within a group.
    private var ordered: [ShellEffect] {
        let order: [ShellEffectGroup] = [.basic, .pixel, .device, .intelligentFixture]
        return order.flatMap { group in context.model.effects.filter { $0.group == group && groupOn(group) } }
    }

    var body: some View {
        let model = context.model
        let label = context.ui?.prefs.effectLabelIndex ?? "2"
        let drawMode = context.ui?.drawMode ?? false
        StripItems(context) {
            ForEach(ordered, id: \.typeId) { effect in
                if context.shows(effect.name) {
                    let chosen = drawMode && context.ui?.selectedEffect == effect.typeId
                    // In Draw Mode a press also chooses that effect to draw. Outside it a button is never left checked.
                    let choose = {
                        if model.ui?.drawMode ?? false {
                            _ = model.shell?.toggleSelectedEffect(typeId: effect.typeId)
                            model.refresh()
                        }
                    }
                    EffectStripLabel(effect: effect, labelIndex: label)
                        .padding(.horizontal, 3)
                        .padding(.vertical, 1)
                        .background(chosen ? Color.accentColor.opacity(0.5) : Color.clear, in: RoundedRectangle(cornerRadius: 4))
                        .overlay {
                            EffectDragSource(typeId: effect.typeId, icon: effect.icon, clicked: choose)
                        }
                        .help(effect.name)
                        .accessibilityElement(children: .ignore)
                        .accessibilityAddTraits(.isButton)
                        .accessibilityLabel(effect.name)
                        // A menu item can't start a drag, so in the drop-down only choosing for Draw Mode is left.
                        .stripEntry(StripEntry(title: effect.name, image: .effect(effect.icon), enabled: drawMode, checked: chosen, action: choose))
                }
            }
        }
    }
}

/// F157: No Label is tag 0, Above Image 1, Below Image 2.
private struct EffectStripLabel: View {
    let effect: ShellEffect
    let labelIndex: String

    var body: some View {
        VStack(spacing: 0) {
            if labelIndex == "1" {
                text
            }
            ToolIcon(image: .effect(effect.icon), size: 16)
            if labelIndex == "2" {
                text
            }
        }
    }

    private var text: some View {
        Text(effect.name)
            .font(.system(size: 9))
            .lineLimit(1)
    }
}

/// `contextMenuStripEffect`. F151 to F163. Items that stay open on click upstream close here, since `NSMenu` always does; F163's Fixture key and close are both fixed by using one path.
struct EffectsStripMenu: View {
    let model: SequencerModel
    let showEffects: () -> Void

    var body: some View {
        let prefs = model.ui?.prefs
        Button("Show/Hide Effects...", action: showEffects)
        // Upstream's spelling (KEEP).
        Menu("Label Postion") {
            ForEach([("No Label", "0"), ("Above Image", "1"), ("Below Image", "2")], id: \.1) { title, index in
                Toggle(title, isOn: Binding(
                    get: { (prefs?.effectLabelIndex ?? "2") == index },
                    set: { _ in
                        model.shell?.setEffectLabelIndex(index: index)
                        model.refresh()
                    }
                ))
            }
        }
        Menu("Toolbars") {
            ToolbarsList(model: model)
        }
        Button("Reset Toolbar") {
            model.set(.basicEffects, true)
            model.set(.pixelEffects, true)
            model.set(.deviceEffects, true)
            model.set(.fixtureEffects, true)
            for effect in model.effects where !model.isItemVisible("toolStripEffects", effect.name) {
                model.setItem(.effects, item: effect.name, visible: true)
            }
        }
    }
}

/// The drag of an effect type from the strip or the Effects panel, for the grid to accept: F150, F305, F320.
enum EffectDrag {
    static let typeIdentifier = "org.vixenlights.effect-type"
    static let pasteboardType = NSPasteboard.PasteboardType(typeIdentifier)

    static func pasteboardItem(typeId: String) -> NSPasteboardItem {
        let item = NSPasteboardItem()
        item.setData(Data(typeId.utf8), forType: pasteboardType)
        return item
    }
}

/// F150, F320: presses over an effect. A drag carries its type, so the grid reads a known pasteboard type; a plain click is the row's or button's own.
struct EffectDragSource: NSViewRepresentable {
    let typeId: String
    let icon: String
    let clicked: () -> Void

    final class SourceView: NSView, NSDraggingSource {
        var typeId = ""
        var icon = ""
        var clicked: (() -> Void)?
        private var downPoint: NSPoint?
        private var dragging = false

        override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
            true
        }

        override func mouseDown(with event: NSEvent) {
            downPoint = event.locationInWindow
            dragging = false
        }

        override func mouseDragged(with event: NSEvent) {
            guard let downPoint, !dragging else { return }
            guard hypot(event.locationInWindow.x - downPoint.x, event.locationInWindow.y - downPoint.y) > 3 else { return }
            dragging = true
            let item = NSDraggingItem(pasteboardWriter: EffectDrag.pasteboardItem(typeId: typeId))
            let image = VixenResources.image("effects/\(icon).png")
            let at = convert(event.locationInWindow, from: nil)
            item.setDraggingFrame(NSRect(x: at.x - 12, y: at.y - 12, width: 24, height: 24), contents: image)
            beginDraggingSession(with: [item], event: event, source: self)
        }

        override func mouseUp(with event: NSEvent) {
            if !dragging, downPoint != nil {
                clicked?()
            }
            downPoint = nil
        }

        func draggingSession(_ session: NSDraggingSession, sourceOperationMaskFor context: NSDraggingContext) -> NSDragOperation {
            .copy
        }
    }

    func makeNSView(context: Context) -> SourceView {
        SourceView(frame: .zero)
    }

    func updateNSView(_ view: SourceView, context: Context) {
        view.typeId = typeId
        view.icon = icon
        view.clicked = clicked
    }
}
