import AppKit
import SwiftUI
import VixenBridge
import VixenKit

#if canImport(VixenDialogs)
import VixenDialogs
#endif

// MARK: Layers, F329 to F340

struct LayersPanel: View {
    let model: SequencerModel
    @State private var layers: [ShellLayer] = []
    @State private var selection: String?
    @State private var expanded: Set<String> = []
    private let filters = layerFilterNames()

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Button(action: importLayers) {
                    Image(systemName: "square.and.arrow.down")
                }
                .help("Import Layers")
                .accessibilityLabel("Import Layers")
                Button(action: exportLayers) {
                    Image(systemName: "square.and.arrow.up")
                }
                .help("Export Layers")
                .accessibilityLabel("Export Layers")
                .disabled(!layers.contains { !$0.isDefault })
                Spacer()
            }
            .buttonStyle(.accessoryBar)
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            // Upstream's row under the toolbar: two buttons of equal width. A new layer is neither selected nor opened there.
            HStack(spacing: 6) {
                Button {
                    _ = model.shell?.addLayer()
                    changed()
                } label: {
                    Text("Add Layer").frame(maxWidth: .infinity)
                }
                Button {
                    if let selection {
                        _ = model.shell?.removeLayer(id: selection)
                    }
                    selection = nil
                    changed()
                } label: {
                    Text("Remove Layer").frame(maxWidth: .infinity)
                }
                .disabled(selectedLayer == nil || selectedLayer?.isDefault == true)
            }
            .controlSize(.small)
            .padding(.horizontal, 6)
            .padding(.bottom, 5)
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(layers.enumerated()), id: \.element.id) { index, layer in
                        LayerRow(
                            model: model,
                            layer: layer,
                            filters: filters,
                            isSelected: selection == layer.id,
                            expanded: Binding(get: { expanded.contains(layer.id) }, set: {
                                if $0 { expanded.insert(layer.id) } else { expanded.remove(layer.id) }
                            }),
                            select: { selection = layer.id },
                            changed: changed,
                            drop: { items in
                                guard let id = items.first, !id.isEmpty else { return false }
                                // The last row is the default's, which stays last.
                                let target = index == layers.count - 1 ? index - 1 : index
                                let done = model.shell?.moveLayer(id: id, index: Int32(target)) ?? false
                                changed()
                                return done
                            }
                        )
                    }
                }
                .padding(6)
            }
        }
        .background(PanelFocusCapture(testHook: "layersFocus"))
        .onAppear {
            reload()
            if UITestHarness.isActive {
                UITestHarness.hooks["sequencer.layers"] = { args in
                    reload()
                    if let action = args.first, action != "state" {
                        guard args.count > 1, let index = Int(args[1]), layers.indices.contains(index) else { return "FAIL no layer" }
                        if action == "expand" { expanded.insert(layers[index].id) }
                        else if action == "select" { selection = layers[index].id }
                        else { return "FAIL unknown layer action" }
                    }
                    return layers.map { "\($0.name):\($0.filterName):default=\($0.isDefault)" }.joined(separator: " | ")
                }
            }
        }
        .onChange(of: model.editRevision) { reload() }
    }

    private var selectedLayer: ShellLayer? {
        layers.first { $0.id == selection }
    }

    private func reload() {
        layers = model.shell?.layers() ?? []
    }

    private func changed() {
        reload()
        model.refresh()
    }

    /// F337. The system panel already asks about replacing a file.
    private func exportLayers() {
        guard let shell = model.shell, let url = LibraryFilePanels.save(title: "Export Layers", name: "Layers", ext: "v3l", typeName: "Vixen 3 Layers") else { return }
        do {
            let stamp = ISO8601DateFormatter().string(from: Date())
            try shell.exportLayers(path: url.path, exportedUtc: stamp)
        } catch {
            Task { await SequencerAlerts.tell(title: "Export Layers", message: "The layers could not be exported: \(error.userMessage)", style: .error, window: model.window) }
        }
    }

    /// F338.
    private func importLayers() {
        guard let shell = model.shell, let url = LibraryFilePanels.open(title: "Import Layers", ext: "v3l", typeName: "Vixen 3 Layers") else { return }
        let plan = shell.layerImportPlan(path: url.path)
        Task {
            if let failure = plan.failure {
                await SequencerAlerts.tell(title: "Import Layers", message: failure, style: .error, window: model.window)
                return
            }
            if plan.importable == 0 {
                await SequencerAlerts.tell(title: "Import Layers", message: "The file does not contain any layers that can be imported.", style: .error, window: model.window)
                return
            }
            if let skipped = plan.skippedMessage {
                let go = await SequencerAlerts.confirm(title: "Import Layers", message: skipped, style: .warning, confirm: "Continue", cancel: "Cancel", window: model.window)
                if !go {
                    return
                }
            }
            _ = shell.importLayers(path: url.path)
            changed()
        }
    }
}

private struct LayerRow: View {
    let model: SequencerModel
    let layer: ShellLayer
    let filters: [String]
    let isSelected: Bool
    @Binding var expanded: Bool
    let select: () -> Void
    let changed: () -> Void
    let drop: ([String]) -> Bool
    @Environment(\.panelIsShowing) private var showing
    @State private var draft: String?
    @FocusState private var nameFocused: Bool

    @ViewBuilder
    var body: some View {
        // Opacity and hit testing do not remove SwiftUI's native drop registration.
        // Hidden Layers rows can otherwise intercept native color drags in another tab.
        if showing {
            row
                .draggable(layer.isDefault ? "" : layer.id)
                .dropDestination(for: String.self) { items, _ in return drop(items) }
        } else {
            row
        }
    }

    private var row: some View {
        DisclosureGroup(isExpanded: $expanded) {
            Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 6) {
                GridRow {
                    Text("Name")
                    HStack(spacing: 4) {
                        TextField("", text: Binding(get: { draft ?? layer.name }, set: { draft = $0 }))
                            .labelsHidden()
                            .accessibilityLabel("Layer Name")
                            .focused($nameFocused)
                            .onSubmit(commitName)
                            .onExitCommand { draft = nil }
                            .onChange(of: nameFocused) { _, focused in
                                if !focused {
                                    commitName()
                                }
                            }
                        if !layer.isDefault {
                            Button {
                                _ = model.shell?.quickRenameLayer(id: layer.id)
                                draft = nil
                                changed()
                            } label: {
                                Image(systemName: "wand.and.stars")
                            }
                            .buttonStyle(.borderless)
                            .help("Rename to filter type")
                            .accessibilityLabel("Rename to filter type")
                        }
                    }
                }
                if !layer.isDefault {
                    GridRow {
                        Text("Type")
                        Picker("", selection: Binding(get: { layer.filterName }, set: { name in
                            _ = model.shell?.setLayerFilter(id: layer.id, filterName: name)
                            changed()
                        })) {
                            ForEach(filters, id: \.self) { Text($0).tag($0) }
                        }
                        .labelsHidden()
                    }
                    if layer.hasSetup {
                        GridRow {
                            Color.clear.gridCellUnsizedAxes([.horizontal, .vertical])
                            Button("Configuration", action: configure)
                        }
                    }
                }
            }
            .padding(.vertical, 4)
        } label: {
            Text(layer.name)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture(perform: select)
        }
        .padding(.horizontal, 4)
        .padding(.vertical, 2)
        .background(isSelected ? Color.accentColor.opacity(0.25) : Color.clear, in: RoundedRectangle(cornerRadius: 4))
    }

    private func commitName() {
        guard let name = draft else { return }
        draft = nil
        if name != layer.name {
            _ = model.shell?.renameLayer(id: layer.id, name: name)
            changed()
        }
    }

    /// F334: `Setup()`.
    private func configure() {
        #if canImport(VixenDialogs)
        guard let window = model.window, let session = model.session, let context = model.app.core?.dialogContext() else { return }
        KitSheet.present(on: window) { dismiss in
            LayerFilterSetupSheet(layer: layer.id, session: session, context: context) { done in
                dismiss()
                if done {
                    model.shell?.noteLayerConfigured()
                    changed()
                }
            }
        }
        #endif
    }
}

// MARK: Find Effects/Layers, F321 to F327

struct FindPanel: View {
    let model: SequencerModel
    @State private var layersMode = false
    @State private var available: [String] = []
    @State private var search: String?
    @State private var rows: [ShellFindRow] = []
    @State private var selection: Set<UInt64> = []
    @State private var collapse = true

    var body: some View {
        VStack(spacing: 6) {
            // Upstream's rows: the Find combo with Collapse all Groups beside it, then the Available combo across the panel.
            HStack(spacing: 12) {
                Picker("Find", selection: Binding(get: { layersMode }, set: { on in
                    layersMode = on
                    search = nil
                    rows = []
                    selection = []
                    refill()
                })) {
                    Text("Effects").tag(false)
                    Text("Layers").tag(true)
                }
                .labelsHidden()
                .fixedSize()
                .accessibilityLabel("Find")
                Toggle("Collapse all Groups", isOn: $collapse)
                    .help("When enabled will collapse all groups prior to finding selected Effects.")
                Spacer(minLength: 0)
            }
            Picker("", selection: Binding(get: { search ?? "" }, set: { name in
                search = name
                update()
            })) {
                if search == nil {
                    Text("").tag("")
                }
                ForEach(available, id: \.self) { Text($0).tag($0) }
            }
            .labelsHidden()
            .frame(minWidth: 200)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityLabel("Available")
            // Filled on opening, so a new effect type appears then and not before (F323).
            .simultaneousGesture(TapGesture().onEnded { refill() })
            Table(rows, selection: $selection) {
                TableColumn("Element", value: \.rowName)
                    .width(min: 60, ideal: 90)
                TableColumn("Effect Start Time", value: \.startText)
                    .width(min: 118, ideal: 122)
                TableColumn(layersMode ? "Effects" : "Layers", value: \.other)
                    .width(min: 40, ideal: 60)
            }
            .contextMenu(forSelectionType: UInt64.self) { items in
                Button("Find all selected effects") { display(items) }
                    .disabled(items.isEmpty)
            } primaryAction: { items in
                display(items)
            }
        }
        .padding(6)
        .onAppear(perform: refill)
        .onChange(of: model.editRevision) { update() }
    }

    private func refill() {
        available = model.shell?.findAvailable(layersMode: layersMode) ?? []
    }

    private func update() {
        guard let search, let shell = model.shell else { return }
        rows = shell.findRows(layersMode: layersMode, search: search)
        selection = selection.filter { id in rows.contains { $0.id == id } }
    }

    private func display(_ items: Set<UInt64>) {
        model.shell?.findDisplay(elements: rows.map(\.element).filter(items.contains), collapseGroups: collapse)
        model.refresh()
    }
}

extension ShellFindRow: Identifiable {
    public var id: UInt64 { element }
}
