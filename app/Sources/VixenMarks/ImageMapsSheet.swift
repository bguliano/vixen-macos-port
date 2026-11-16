import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// Tools > LipSync > Edit Image Maps: the library of LipSync picture maps. Present it as a sheet; `onClose` runs after the library has been
/// saved, and says whether anything changed.
public struct ImageMapsSheet: View {
    @State private var model: ImageMapsModel
    private let onClose: (Bool) -> Void
    private let testCore: VixenCore?

    public init(core: VixenCore, onClose: @escaping (Bool) -> Void) {
        _model = State(initialValue: ImageMapsModel(maps: core.imageMaps()))
        self.onClose = onClose
        testCore = UITestHarness.isActive ? core : nil
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetTitle("Image Maps")
            HStack(alignment: .top, spacing: 10) {
                list
                VStack(spacing: 8) {
                    Button("New") { model.new() }
                        .disabled(!model.canNew)
                    Button("Edit") { model.editSelected() }
                        .disabled(!model.canEdit)
                    Button("Remove") { model.askRemove() }
                        .disabled(!model.canRemove)
                    Button("Clone") { model.cloneSelected() }
                        .disabled(!model.canClone)
                }
                .buttonStyle(EqualWidthButtonStyle())
                .frame(width: 80)
            }
            HStack {
                Spacer()
                Button("OK") { close() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(model.renaming != nil)
            }
        }
        .padding(16)
        .frame(minWidth: 375, idealWidth: 462, maxWidth: 640, minHeight: 300, idealHeight: 356, maxHeight: 480)
        .onExitCommand {
            if model.renaming != nil {
                model.cancelRename()
            } else {
                close()
            }
        }
        .sheet(item: $model.editing) { editing in
            ImageMapEditorSheet(model: editing) { model.editorFinished(editing, saved: $0) }
        }
        .alert(model.alert?.title ?? "", isPresented: Binding(get: { model.alert != nil }, set: { if !$0 { model.alert = nil } })) {
            Button("OK") { model.alert = nil }
        } message: {
            Text(model.alert?.message ?? "")
        }
        .confirmationDialog("Delete the mapping?", isPresented: $model.confirmingRemove) {
            Button("Delete", role: .destructive) { model.removeSelected() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("If you delete this mapping, ALL places it is used will be unlinked and will revert to the default Mapping. Are you sure you want to continue?")
        }
        .onDisappear { model.cancelPendingRename() }
        .onAppear {
            if UITestHarness.isActive, let testCore {
                ImageMapsTestHooks.current = model
                ImageMapsTestHooks.install { testCore }
            }
        }
    }

    private var list: some View {
        Table(model.rows, selection: $model.selection) {
            TableColumn("Name") { row in
                if model.renaming == row.name {
                    RenameField(model: model)
                } else {
                    Text(row.name)
                        .fontWeight(row.isDefault ? .bold : .regular)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                        .simultaneousGesture(TapGesture().onEnded { model.nameClicked(row.name) })
                }
            }
            TableColumn("Notes") { row in
                Text(row.notes).lineLimit(1)
            }
        }
        .contextMenu(forSelectionType: String.self) { names in
            if names.count == 1, let name = names.first {
                Button("Edit") { model.edit(name) }
                Button("Rename") { model.beginRename(name) }
            }
            if !names.isEmpty {
                Button("Clone") { model.clone(Array(names)) }
                Button("Remove", role: .destructive) {
                    model.selection = names
                    model.askRemove()
                }
                .disabled(!model.maps.canRemove(names: Array(names)))
            }
        } primaryAction: { names in
            if names.count == 1, let name = names.first {
                model.edit(name)
            }
        }
        .onDeleteCommand { model.askRemove() }
        .onChange(of: model.selection) { model.selectionChanged() }
    }

    private func close() {
        let changed = model.close()
        ImageMapsTestHooks.lastClose = changed
        onClose(changed)
    }
}

private struct EqualWidthButtonStyle: PrimitiveButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) {
            configuration.label.frame(maxWidth: .infinity)
        }
    }
}

/// The name being edited in place. Return accepts, Escape cancels.
private struct RenameField: View {
    @Bindable var model: ImageMapsModel
    @FocusState private var focused: Bool

    var body: some View {
        TextField("Name", text: $model.renameText)
            .textFieldStyle(.roundedBorder)
            .focused($focused)
            .onSubmit { model.commitRename() }
            .onExitCommand { model.cancelRename() }
            .onAppear { focused = true }
    }
}

extension ImageMapRow: Identifiable {
    public var id: String { name }
}

struct ImageMapsAlert {
    let title: String
    let message: String
}

@Observable
final class ImageMapsModel {
    let maps: ImageMaps
    var rows: [ImageMapRow]
    var selection: Set<String> = []
    var renaming: String?
    var renameText = ""
    var editing: ImageMapEditorModel?
    var alert: ImageMapsAlert?
    var confirmingRemove = false
    /// Upstream leaves only New enabled after a label edit, until the selection changes.
    private var afterRename = false
    private var selectedAt = Date.distantPast
    @ObservationIgnored private var pendingRename: Task<Void, Never>?

    init(maps: ImageMaps) {
        self.maps = maps
        rows = maps.rows()
    }

    var canNew: Bool { renaming == nil }
    var canEdit: Bool { renaming == nil && !afterRename && selection.count == 1 }
    var canClone: Bool { renaming == nil && !afterRename && !selection.isEmpty }
    var canRemove: Bool { renaming == nil && !afterRename && !selection.isEmpty && maps.canRemove(names: Array(selection)) }

    func reload() {
        cancelPendingRename()
        rows = maps.rows()
        let names = Set(rows.map(\.name))
        selection = selection.intersection(names)
    }

    func selectionChanged() {
        cancelPendingRename()
        afterRename = false
        selectedAt = Date()
        if let renaming, selection != [renaming] {
            cancelRename()
        }
    }

    /// Decide rename eligibility at the click, but let a double-click open the editor first.
    func nameClicked(_ name: String) {
        cancelPendingRename()
        guard renaming == nil, editing == nil, selection == [name], Date().timeIntervalSince(selectedAt) > 0.6 else { return }
        let interval = NSEvent.doubleClickInterval
        pendingRename = Task { @MainActor [weak self] in
            do { try await Task.sleep(for: .seconds(interval)) } catch { return }
            guard !Task.isCancelled, let self, self.renaming == nil, self.editing == nil, self.selection == [name] else { return }
            self.beginRename(name)
        }
    }

    func cancelPendingRename() {
        pendingRename?.cancel()
        pendingRename = nil
    }

    func close() -> Bool {
        cancelPendingRename()
        return maps.close()
    }

    func new() {
        cancelPendingRename()
        do {
            editing = ImageMapEditorModel(editor: try maps.newMap())
        } catch {
            alert = ImageMapsAlert(title: "Error!", message: error.userMessage)
        }
        reload()
    }

    func editSelected() {
        if selection.count == 1, let name = selection.first {
            edit(name)
        }
    }

    func edit(_ name: String) {
        cancelPendingRename()
        guard renaming == nil else { return }
        do {
            editing = ImageMapEditorModel(editor: try maps.edit(name: name))
        } catch {
            alert = ImageMapsAlert(title: "Error!", message: error.userMessage)
        }
        reload()
    }

    func editorFinished(_ editor: ImageMapEditorModel, saved: String?) {
        editing = nil
        reload()
        if let saved {
            selection = [saved]
        }
    }

    func askRemove() {
        cancelPendingRename()
        if canRemove {
            confirmingRemove = true
        }
    }

    func removeSelected() {
        if let message = maps.remove(names: sorted(selection)) {
            alert = ImageMapsAlert(title: "Error!", message: message)
        }
        reload()
    }

    func cloneSelected() {
        clone(Array(selection))
    }

    func clone(_ names: [String]) {
        let outcome = maps.cloneMaps(names: sorted(Set(names)))
        if let message = outcome.error {
            alert = ImageMapsAlert(title: "Error!", message: message)
        }
        reload()
    }

    func beginRename(_ name: String) {
        cancelPendingRename()
        selection = [name]
        renameText = name
        renaming = name
    }

    func cancelRename() {
        cancelPendingRename()
        renaming = nil
        selection.removeAll()
        afterRename = true
    }

    func commitRename() {
        guard let old = renaming else { return }
        renaming = nil
        let outcome = maps.rename(old: old, new: renameText)
        if let message = outcome.error {
            alert = ImageMapsAlert(title: "Error!", message: message)
        }
        reload()
        // Upstream repopulates the list after label editing, clearing its selection.
        // Keeping the renamed row selected also lets the deferred selection callback
        // re-enable Edit/Remove/Clone before the user makes a new selection.
        selection.removeAll()
        afterRename = true
    }

    /// The list's order, so several maps are handled top to bottom as upstream's selected items are.
    private func sorted(_ names: Set<String>) -> [String] {
        rows.map(\.name).filter(names.contains)
    }
}

// MARK: - The editor

@Observable
final class ImageMapEditorModel: Identifiable {
    let editor: ImageMapEditor
    let phonemes: [String]
    var name: String { didSet { editor.setName(name: name) } }
    var notes: String { didSet { editor.setNotes(notes: notes) } }
    var phoneme: String
    var image: NSImage?
    var mapper: ImageMapperModel?
    var askOverwrite: String?
    var alert: ImageMapsAlert?
    /// The app the current picture is open in, while F103's edit is under way.
    var editingIn: String?
    /// Set when OK has saved: the name the map ended up with.
    var savedAs: String?
    var finished = false

    init(editor: ImageMapEditor) {
        self.editor = editor
        phonemes = editor.phonemes()
        name = editor.name()
        notes = editor.notes()
        phoneme = editor.phoneme()
        refresh()
    }

    func refresh() {
        phoneme = editor.phoneme()
        image = editor.image().flatMap { NSImage(data: $0) }
    }

    func next() {
        editor.next()
        refresh()
    }

    func previous() {
        editor.previous()
        refresh()
    }

    func clear() {
        editor.clear()
        refresh()
    }

    func chooseFiles() async {
        let files = await ImageMapFiles.choose()
        if !files.isEmpty {
            load(files)
        }
    }

    func drop(_ urls: [URL]) -> Bool {
        let files = urls.filter(\.isFileURL).map(\.path)
        guard !files.isEmpty, editingIn == nil, editor.acceptsDrop(files: files) else { return false }
        load(files)
        return true
    }

    func load(_ files: [String]) {
        if editor.loadFiles(files: files) {
            mapper = ImageMapperModel(phonemes: phonemes, files: files, choices: editor.mapperChoices(fileCount: UInt32(files.count)))
        }
        refresh()
    }

    func mapperFinished(_ model: ImageMapperModel, accepted: Bool) {
        mapper = nil
        if accepted {
            editor.applyMapper(choices: model.choices, files: model.files)
        }
        refresh()
    }

    func imageDoubleClicked() async {
        if image == nil {
            await chooseFiles()
        } else {
            beginExternalEdit()
        }
    }

    func beginExternalEdit() {
        guard editingIn == nil else { return }
        do {
            let path = try editor.beginExternalEdit()
            let url = URL(fileURLWithPath: path)
            let app = NSWorkspace.shared.urlForApplication(toOpen: url)
            editingIn = app.map { FileManager.default.displayName(atPath: $0.path).replacingOccurrences(of: ".app", with: "") } ?? "another app"
            if UITestHarness.isActive {
                UITestHarness.note("open \(path)")
            } else {
                NSWorkspace.shared.open(url)
            }
        } catch {
            alert = ImageMapsAlert(title: "Error!", message: error.userMessage)
        }
    }

    func finishExternalEdit() {
        editingIn = nil
        if let message = editor.finishExternalEdit() {
            alert = ImageMapsAlert(title: "Error!", message: message)
        }
        refresh()
    }

    func cancelExternalEdit() {
        editingIn = nil
        editor.cancelExternalEdit()
        refresh()
    }

    func ok(overwrite: Bool? = nil) {
        if editingIn != nil {
            finishExternalEdit()
        }
        switch editor.commit(overwrite: overwrite) {
        case let .saved(name, alert):
            savedAs = name
            if let alert {
                self.alert = ImageMapsAlert(title: "Error!", message: alert)
            } else {
                finished = true
            }
        case let .askOverwrite(name):
            askOverwrite = name
        }
    }

    func cancel() {
        if editingIn != nil {
            cancelExternalEdit()
        }
        editor.cancel()
        finished = true
    }

    func alertDismissed() {
        alert = nil
        if savedAs != nil {
            finished = true
        }
    }
}

struct ImageMapEditorSheet: View {
    @Bindable var model: ImageMapEditorModel
    let onFinish: (String?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetTitle("Image Map")
            HStack {
                Text("Name")
                TextField("Name", text: $model.name)
                    .labelsHidden()
                    .textFieldStyle(.roundedBorder)
            }
            HStack(spacing: 12) {
                Spacer()
                Button { model.previous() } label: { Image(systemName: "chevron.left") }
                    .help("Previous phoneme")
                    .accessibilityLabel("Previous phoneme")
                VStack(spacing: 2) {
                    Text(model.phoneme).font(.callout.bold())
                    MapPhonemeIcon(name: model.phoneme)
                }
                .frame(width: 60)
                Button { model.next() } label: { Image(systemName: "chevron.right") }
                    .help("Next phoneme")
                    .accessibilityLabel("Next phoneme")
                Spacer()
            }
            .disabled(model.editingIn != nil)
            HStack(alignment: .top, spacing: 10) {
                MappedImageView(image: model.image)
                    .onTapGesture(count: 2) { Task { await model.imageDoubleClicked() } }
                    .accessibilityLabel(model.image == nil ? "No image" : "Mapped image")
                if let app = model.editingIn {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Editing in \(app)")
                            .font(.callout)
                            .fixedSize(horizontal: false, vertical: true)
                        Button("Done") { model.finishExternalEdit() }
                        Button("Discard") { model.cancelExternalEdit() }
                    }
                    .buttonStyle(EqualWidthButtonStyle())
                    .frame(width: 80)
                } else {
                    VStack(spacing: 8) {
                        Button("File") { Task { await model.chooseFiles() } }
                        Button("Edit") { model.beginExternalEdit() }
                        Button("Clear") { model.clear() }
                    }
                    .buttonStyle(EqualWidthButtonStyle())
                    .frame(width: 70)
                }
            }
            Text("Notes:")
            TextEditor(text: $model.notes)
                .font(.body)
                .frame(height: 64)
                .overlay(RoundedRectangle(cornerRadius: 4).stroke(.separator))
                .accessibilityLabel("Notes")
            HStack {
                Spacer()
                Button("Cancel") { model.cancel() }
                    .keyboardShortcut(.cancelAction)
                Button("OK") { model.ok() }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 258)
        .dropDestination(for: URL.self) { urls, _ in model.drop(urls) }
        .sheet(item: $model.mapper) { mapper in
            ImageMapperSheet(model: mapper) { model.mapperFinished(mapper, accepted: $0) }
        }
        .alert(model.alert?.title ?? "", isPresented: Binding(get: { model.alert != nil }, set: { if !$0 { model.alertDismissed() } })) {
            Button("OK") { model.alertDismissed() }
        } message: {
            Text(model.alert?.message ?? "")
        }
        .confirmationDialog("Map exists", isPresented: Binding(get: { model.askOverwrite != nil }, set: { if !$0 { model.askOverwrite = nil } })) {
            Button("Overwrite") { model.ok(overwrite: true) }
            Button("Keep Both") { model.ok(overwrite: false) }
        } message: {
            Text("Overwrite existing \(model.askOverwrite ?? "") mapping?")
        }
        .onChange(of: model.finished) {
            if model.finished {
                onFinish(model.savedAs)
            }
        }
        .onAppear { ImageMapsTestHooks.editor = model }
    }
}

/// The phoneme's reference mouth, at upstream's 48 points.
private struct MapPhonemeIcon: View {
    let name: String

    var body: some View {
        if let image = VixenResources.image("marks/phonemes/\(name).jpg") {
            Image(nsImage: image)
                .resizable()
                .interpolation(.high)
                .scaledToFit()
                .frame(width: 48, height: 48)
        } else {
            Color.clear.frame(width: 48, height: 48)
        }
    }
}

/// The picture a phoneme maps to, aspect kept in a fixed 128 point square, or upstream's tiled "no image" pattern.
private struct MappedImageView: View {
    let image: NSImage?

    var body: some View {
        ZStack {
            if let image {
                Color.black.opacity(0.25)
                Image(nsImage: image)
                    .resizable()
                    .interpolation(.high)
                    .scaledToFit()
            } else if let tile = VixenResources.image("marks/NoImage.png") {
                Image(nsImage: tile).resizable(resizingMode: .tile)
            } else {
                Color.gray.opacity(0.2)
            }
        }
        .frame(width: 128, height: 128)
        .clipShape(Rectangle())
        .border(.separator)
        .contentShape(Rectangle())
    }
}

// MARK: - Image Mapper

@Observable
final class ImageMapperModel: Identifiable {
    let phonemes: [String]
    let files: [String]
    var choices: [ImageMapperChoice]

    init(phonemes: [String], files: [String], choices: [ImageMapperChoice]) {
        self.phonemes = phonemes
        self.files = files
        self.choices = choices
    }

    var fileNames: [String] { files.map { URL(fileURLWithPath: $0).lastPathComponent } }
}

struct ImageMapperSheet: View {
    @Bindable var model: ImageMapperModel
    let onFinish: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetTitle("Image Mapper")
            Text("Multiple images detected, \nPlease select the desired mapping")
                .fixedSize(horizontal: false, vertical: true)
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                ForEach(Array(model.phonemes.enumerated()), id: \.offset) { index, phoneme in
                    GridRow {
                        Text(phoneme)
                        Picker(phoneme, selection: $model.choices[index]) {
                            Text("<Unmapped>").tag(ImageMapperChoice.unmapped)
                            Text("<Mapped>").tag(ImageMapperChoice.mapped)
                            ForEach(Array(model.fileNames.enumerated()), id: \.offset) { fileIndex, file in
                                Text(file).tag(ImageMapperChoice.file(index: UInt32(fileIndex)))
                            }
                        }
                        .labelsHidden()
                        .frame(maxWidth: .infinity)
                    }
                }
            }
            HStack {
                Spacer()
                Button("Cancel") { onFinish(false) }
                    .keyboardShortcut(.cancelAction)
                Button("Accept") { onFinish(true) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 260)
        .onAppear { ImageMapsTestHooks.mapper = model }
    }
}

// MARK: - Files

enum ImageMapFiles {
    /// F101's multi-select open panel. Under test the files queued with `marksNextFile` stand in for it, all at once.
    static func choose() async -> [String] {
        if UITestHarness.isActive {
            defer { MarkFiles.queued.removeAll() }
            return MarkFiles.queued
        }
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        let accessory = FilterAccessory(panel: panel, filter: MarkFileFilter(description: "Image Files (*.bmp, *.jpg, *.png)", extensions: ["bmp", "jpg", "png"]))
        panel.accessoryView = accessory.view
        panel.isAccessoryViewDisclosed = true
        let response: NSApplication.ModalResponse
        if let window = NSApp.keyWindow {
            response = await panel.beginSheetModal(for: window)
        } else {
            response = panel.runModal()
        }
        _ = accessory
        return response == .OK ? panel.urls.map(\.path) : []
    }
}

// MARK: - Default Image Map

/// Tools > LipSync > Default Image Map: every map in library order, the default checked. Put it in a `Menu`.
public struct DefaultImageMapMenuItems: View {
    private let core: VixenCore
    private let onChange: () -> Void

    public init(core: VixenCore, onChange: @escaping () -> Void = {}) {
        self.core = core
        self.onChange = onChange
    }

    public var body: some View {
        let current = core.defaultImageMap()
        ForEach(core.imageMapNames(), id: \.self) { name in
            Toggle(name, isOn: Binding(get: { name == current }, set: { on in
                if on, core.setDefaultImageMap(name: name) {
                    onChange()
                }
            }))
        }
    }
}

// MARK: - Test hooks

/// What test scripts can ask of the image map sheets: `{ "action": "hook", "name": "imageMaps", "path": ["state"] }` and so on.
enum ImageMapsTestHooks {
    static weak var current: ImageMapsModel?
    static weak var editor: ImageMapEditorModel?
    static weak var mapper: ImageMapperModel?
    static var lastClose: Bool?

    static func install(core: @escaping () -> VixenCore?) {
        UITestHarness.hooks["imageMaps"] = { args in
            guard let verb = args.first else { return "FAIL needs a verb" }
            let rest = Array(args.dropFirst())
            switch verb {
            case "state":
                guard let core = core() else { return "FAIL no profile" }
                let rows = current?.rows.map { "\($0.name)\($0.isDefault ? "*" : "")" }.joined(separator: ", ") ?? "(closed)"
                let buttons = current.map { "new=\($0.canNew) edit=\($0.canEdit) remove=\($0.canRemove) clone=\($0.canClone)" } ?? ""
                return "rows [\(rows)] library [\(core.imageMapNames().joined(separator: ", "))] default \(core.defaultImageMap() ?? "-") changed \(current?.maps.changed() ?? false) lastClose \(lastClose.map(String.init) ?? "-") \(buttons)"
            case "default":
                guard let core = core(), let name = rest.first else { return "FAIL needs a name" }
                return "set \(core.setDefaultImageMap(name: name))"
            case "select":
                guard let model = current else { return "FAIL no Image Maps sheet" }
                model.selection = Set(rest)
                model.selectionChanged()
                return "selected \(rest.count)"
            case "rename":
                guard let model = current, rest.count == 2 else { return "FAIL needs the sheet, an old and a new name" }
                model.beginRename(rest[0])
                model.renameText = rest[1]
                model.commitRename()
                return model.alert.map { "alert \($0.message)" } ?? "renamed"
            case "beginRename":
                guard let model = current, let name = rest.first else { return "FAIL needs the sheet and a name" }
                model.beginRename(name)
                return "renaming"
            case "editor":
                guard let model = editor else { return "FAIL no editor" }
                return "name \(model.name) phoneme \(model.phoneme) image \(model.image.map { "\(Int($0.size.width))x\(Int($0.size.height))" } ?? "none") mapped [\(model.editor.mapped().joined(separator: ","))] editingIn \(model.editingIn ?? "-") temp \(model.editor.externalEditFile() ?? "-")"
            case "next":
                guard let model = editor else { return "FAIL no editor" }
                model.next()
                return model.phoneme
            case "previous":
                guard let model = editor else { return "FAIL no editor" }
                model.previous()
                return model.phoneme
            case "name":
                guard let model = editor, let name = rest.first else { return "FAIL needs the editor and a name" }
                model.name = name
                return "named"
            case "notes":
                guard let model = editor, let notes = rest.first else { return "FAIL needs the editor and notes" }
                model.notes = notes
                return "noted"
            case "drop":
                guard let model = editor else { return "FAIL no editor" }
                return "drop \(model.drop(rest.map { URL(fileURLWithPath: $0) }))"
            case "mapper":
                guard let model = mapper else { return "FAIL no mapper" }
                return model.choices.enumerated().map { index, choice in
                    switch choice {
                    case .unmapped: "\(model.phonemes[index])=-"
                    case .mapped: "\(model.phonemes[index])=mapped"
                    case let .file(file): "\(model.phonemes[index])=\(model.fileNames[Int(file)])"
                    }
                }.joined(separator: " ")
            case "map":
                guard let model = mapper, rest.count == 2, let row = model.phonemes.firstIndex(of: rest[0]) else { return "FAIL needs the mapper, a phoneme and a file name" }
                if let file = model.fileNames.firstIndex(of: rest[1]) {
                    model.choices[row] = .file(index: UInt32(file))
                } else {
                    model.choices[row] = rest[1] == "mapped" ? .mapped : .unmapped
                }
                return "mapped"
            default:
                return "FAIL unknown verb \(verb)"
            }
        }
    }
}
