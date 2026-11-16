import AppKit
import SwiftUI
import VixenBridge
import VixenKit

// MARK: Elements, F80 to F86

/// Rows go by path, because one element can have two parents: a row's last part is its node.
private final class ElementsProvider: OutlineDataProvider {
    private weak var model: PreviewDesignerModel?

    init(model: PreviewDesignerModel) {
        self.model = model
    }

    func outlineChildren(of parent: String?) -> [OutlineRow] {
        guard let designer = model?.designer else { return [] }
        let node = parent.flatMap { $0.split(separator: "/").last }.map(String.init)
        return designer.elementRows(parent: node).map { row in
            OutlineRow(id: parent.map { "\($0)/\(row.id)" } ?? row.id, title: row.name, hasChildren: row.hasChildren, trailingDots: row.tagColors)
        }
    }

    func outlineCanDrop(items: [String], parent: String?, index: Int?, copy: Bool) -> Bool {
        model?.treeEditor?.canDrop(items: items, parent: parent, index: index, copy: copy) ?? false
    }

    func outlineDrop(items: [String], parent: String?, index: Int?, copy: Bool) {
        model?.treeEditor?.drop(items: items, parent: parent, index: index, copy: copy)
    }
}

struct ElementsPanel: View {
    let model: PreviewDesignerModel
    @State private var provider: ElementsProvider?
    @State private var template = elementTemplateNames().first ?? ""

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                PanelTitle(text: "Elements")
                FloatPanelButton(model: model, kind: .elements)
            }
            HStack(spacing: 4) {
                Text("Add:")
                    .font(.caption)
                Picker("Add:", selection: $template) {
                    ForEach(elementTemplateNames(), id: \.self) { Text($0) }
                }
                .labelsHidden()
                .controlSize(.small)
                Button {
                    Task { await model.addElementTemplate(name: template) }
                } label: {
                    Image(systemName: "plus")
                }
                .controlSize(.small)
                .disabled(template.isEmpty || !model.canRunElementTemplates)
                .help("Add Element Template")
                .accessibilityLabel("Add Element Template")
            }
            .padding(.horizontal, 6)
            .padding(.bottom, 4)
            if let provider {
                OutlineView(
                    provider: provider,
                    handle: model.treeHandle,
                    allowsDragging: ElementTreeEditor.make != nil,
                    accessibilityLabel: "Elements",
                    onSelectionChange: { model.elementsSelected(model.treeNodes($0)) },
                    onCommand: { model.performOnElements($0) },
                    canPerform: { model.canPerformOnElements($0) },
                    menu: { model.elementMenu() }
                )
            }
        }
        .onAppear {
            if provider == nil {
                provider = ElementsProvider(model: model)
            }
        }
    }
}

/// Takes a panel out into a floating one, as upstream's dock panels could be dragged out. Hidden while it floats: its close button docks it.
struct FloatPanelButton: View {
    let model: PreviewDesignerModel
    let kind: DesignerPanelKind

    var body: some View {
        let floating = kind == .elements ? model.elementsFloating : model.propertiesFloating
        if !floating {
            Button {
                model.setFloating(kind, true)
            } label: {
                Image(systemName: "arrow.up.forward.square")
            }
            .buttonStyle(.borderless)
            .controlSize(.small)
            .padding(.trailing, 6)
            .help("Float the \(kind.title) panel")
            .accessibilityLabel("Float the \(kind.title) panel")
        }
    }
}

struct PanelTitle: View {
    let text: String

    var body: some View {
        HStack {
            Text(text)
                .font(.caption)
                .bold()
            Spacer()
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 3)
    }
}

// MARK: Properties, F90 to F92

/// What the panel shows. Reads from Rust only when the selection changes or a drag ends, never in a view's body, and only sets rows that differ.
@Observable
final class DesignerPropertiesModel {
    private(set) var shape: UInt32?
    private(set) var title = "Properties"
    private(set) var helpURL: String?
    private(set) var isGroup = false
    private(set) var rows: [DesignerProperty] = []
    private(set) var stringNames: [String] = []
    /// The group's string shown in the panel.
    var selectedString = 0 {
        didSet {
            if oldValue != selectedString {
                reload()
            }
        }
    }
    /// The grid's toolbar. Per panel, and each newly selected shape goes back to categorized.
    var alphabetical = false {
        didSet {
            if oldValue != alphabetical {
                reload()
            }
        }
    }
    var focusedRow: String?

    @ObservationIgnored private weak var designer: PreviewDesigner?
    @ObservationIgnored private(set) weak var model: PreviewDesignerModel?

    func show(shape: UInt32, designer: PreviewDesigner, model: PreviewDesignerModel) {
        self.designer = designer
        self.model = model
        guard let info = designer.shapeInfo(shape: shape) else {
            clear()
            return
        }
        self.shape = shape
        title = info.title
        helpURL = info.helpUrl
        isGroup = info.isGroup
        selectedString = 0
        alphabetical = false
        focusedRow = nil
        reload()
    }

    func clear() {
        guard shape != nil else { return }
        shape = nil
        title = "Properties"
        helpURL = nil
        isGroup = false
        rows = []
        stringNames = []
        focusedRow = nil
    }

    func reload() {
        guard let shape, let designer else { return }
        let new: [DesignerProperty]
        if isGroup {
            let names = designer.groupStringNames(shape: shape)
            if names != stringNames {
                stringNames = names
            }
            new = designer.groupStringProperties(shape: shape, string: UInt32(selectedString), alphabetical: alphabetical)
        } else {
            new = designer.properties(shape: shape, alphabetical: alphabetical)
        }
        if new != rows {
            rows = new
        }
    }

    func commit(id: String, value: DesignerPropertyValue) {
        guard let shape, let designer else { return }
        do {
            let change = isGroup
                ? try designer.setGroupStringProperty(shape: shape, string: UInt32(selectedString), id: id, value: value)
                : try designer.setProperty(shape: shape, id: id, value: value)
            model?.apply(change)
        } catch {
            model?.alert("Properties", error.userMessage)
        }
        reload()
    }

    func syncLightSizes() {
        guard let shape, let designer else { return }
        model?.apply(designer.syncStringLightSizes(shape: shape, string: UInt32(selectedString)))
        reload()
    }

    func editLinks() {
        guard let shape else { return }
        model?.openLinker(shape: shape)
    }

    func editFixtureLink(current: String?) {
        guard let shape else { return }
        model?.sheet = .fixtureLink(shape: shape, current: current)
    }
}

struct PropertiesPanel: View {
    let model: PreviewDesignerModel
    @Bindable var panel: DesignerPropertiesModel

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 6) {
                Text(panel.title)
                    .font(.caption)
                    .bold()
                    .lineLimit(1)
                Spacer()
                if panel.shape != nil {
                    Picker("View", selection: $panel.alphabetical) {
                        Image(systemName: "list.bullet.indent").tag(false)
                        Image(systemName: "textformat.abc").tag(true)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .controlSize(.mini)
                    .frame(width: 58)
                    .help("Categorized or alphabetical")
                }
                if let help = panel.helpURL {
                    Button {
                        model.help(help)
                    } label: {
                        Image(systemName: "questionmark.circle")
                    }
                    .buttonStyle(.borderless)
                    .help("Help")
                    .accessibilityLabel("Help")
                }
                FloatPanelButton(model: model, kind: .properties)
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            if panel.shape != nil {
                if panel.isGroup {
                    GroupStringHeader(panel: panel)
                }
                PropertyGridRows(panel: panel)
                Divider()
                DescriptionPane(panel: panel)
            } else {
                Spacer()
            }
        }
    }
}

/// F92.
private struct GroupStringHeader: View {
    @Bindable var panel: DesignerPropertiesModel

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Picker("String", selection: $panel.selectedString) {
                ForEach(Array(panel.stringNames.enumerated()), id: \.offset) { index, name in
                    Text(name).tag(index)
                }
            }
            .controlSize(.small)
            HStack(spacing: 6) {
                Button("Sync") { panel.syncLightSizes() }
                    .controlSize(.small)
                Text("Syncronize all light sizes with string.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 6)
        .padding(.bottom, 4)
    }
}

/// The grid: name on the left, editor on the right, and small enough to read like one. These are rows of a form with at most about twenty-five of them.
private struct PropertyGridRows: View {
    @Bindable var panel: DesignerPropertiesModel

    private var categories: [String] {
        var seen = Set<String>()
        return panel.rows.map(\.category).filter { seen.insert($0).inserted }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 1) {
                if panel.alphabetical {
                    ForEach(panel.rows, id: \.id) { row in
                        PropertyRow(row: row, panel: panel)
                    }
                } else {
                    ForEach(categories, id: \.self) { category in
                        DisclosureGroup(isExpanded: .constant(true)) {
                            ForEach(panel.rows.filter { $0.category == category }, id: \.id) { row in
                                PropertyRow(row: row, panel: panel)
                            }
                        } label: {
                            Text(category)
                                .font(.caption)
                                .bold()
                        }
                    }
                }
            }
            .padding(.horizontal, 6)
            .padding(.bottom, 6)
        }
    }
}

private struct DescriptionPane: View {
    let panel: DesignerPropertiesModel

    var body: some View {
        let row = panel.rows.first { $0.id == panel.focusedRow }
        VStack(alignment: .leading, spacing: 1) {
            Text(row?.name ?? " ")
                .font(.caption)
                .bold()
            Text(row?.description ?? " ")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(3)
        }
        .frame(maxWidth: .infinity, minHeight: 46, alignment: .topLeading)
        .padding(.horizontal, 6)
        .padding(.vertical, 4)
    }
}

/// One row, keeping what's being typed until Return or the focus leaving, as a PropertyGrid commits. Per keystroke would resize shapes mid-typing.
private struct PropertyRow: View {
    let row: DesignerProperty
    let panel: DesignerPropertiesModel
    @State private var text = ""
    /// A point row opens to X and Y, closed to begin with, as the grid's "+" does.
    @State private var expanded = false
    @FocusState private var focused: Bool

    private var isPoint: Bool {
        if case .point = row.value { return true }
        return false
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            HStack(spacing: 4) {
                HStack(spacing: 2) {
                    if isPoint {
                        Button {
                            expanded.toggle()
                        } label: {
                            Image(systemName: expanded ? "chevron.down" : "chevron.right")
                                .font(.system(size: 8, weight: .semibold))
                                .frame(width: 10)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(expanded ? "Hide X and Y" : "Show X and Y")
                    }
                    Text(row.name)
                        .font(.caption)
                        .lineLimit(1)
                }
                .frame(width: 92, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture(count: 2) { cycle() }
                editor
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            if expanded, case .point(let x, let y) = row.value {
                HStack(spacing: 4) {
                    PointPartField(title: "X", value: x) { commitPoint(x: $0, y: y) }
                    PointPartField(title: "Y", value: y) { commitPoint(x: x, y: $0) }
                }
                .padding(.leading, 96)
                .disabled(row.readOnly)
            }
        }
        .padding(.vertical, 0.5)
        .contentShape(Rectangle())
        .simultaneousGesture(TapGesture().onEnded { panel.focusedRow = row.id })
        .onChange(of: focused) {
            if focused {
                panel.focusedRow = row.id
            } else {
                commitText()
            }
        }
        .onAppear { text = displayText }
        .onChange(of: row.value) { text = displayText }
    }

    /// A PropertyGrid's double-click: a True/False or a list steps to its next value.
    private func cycle() {
        guard !row.readOnly else { return }
        switch (row.kind, row.value) {
        case (.bool, .bool(let value)):
            panel.commit(id: row.id, value: .bool(value: !value))
        case (.enumeration(let members), .enumeration(let value)):
            if let at = members.firstIndex(of: value), !members.isEmpty {
                panel.commit(id: row.id, value: .enumeration(value: members[(at + 1) % members.count]))
            }
        default:
            break
        }
    }

    private var displayText: String {
        switch row.value {
        case .int(let value): "\(value)"
        case .double(let value): "\(value)"
        case .text(let value): value
        case .point(let x, let y): "\(x), \(y)"
        default: ""
        }
    }

    @ViewBuilder
    private var editor: some View {
        switch row.kind {
        case .int, .double, .text, .point:
            TextField(row.name, text: $text)
                .labelsHidden()
                .textFieldStyle(.roundedBorder)
                .controlSize(.small)
                .font(.caption.monospacedDigit())
                .focused($focused)
                .disabled(row.readOnly)
                .onSubmit { commitText() }
        case .bool:
            Picker(row.name, selection: Binding(
                get: {
                    if case .bool(let value) = row.value { return value }
                    return false
                },
                set: { panel.commit(id: row.id, value: .bool(value: $0)) }
            )) {
                Text("True").tag(true)
                Text("False").tag(false)
            }
            .labelsHidden()
            .controlSize(.small)
            .disabled(row.readOnly)
        case .enumeration(let members):
            Picker(row.name, selection: Binding(
                get: {
                    if case .enumeration(let value) = row.value { return value }
                    return ""
                },
                set: { panel.commit(id: row.id, value: .enumeration(value: $0)) }
            )) {
                ForEach(members, id: \.self) { Text($0).tag($0) }
            }
            .labelsHidden()
            .controlSize(.small)
            .disabled(row.readOnly)
        case .linkedElements:
            HStack(spacing: 4) {
                Text("(Collection)")
                    .font(.caption)
                Spacer()
                Button("...") { panel.editLinks() }
                    .controlSize(.small)
                    .accessibilityLabel("Linked Elements")
            }
        case .linkedElement:
            HStack(spacing: 4) {
                Text(currentNode == nil ? "(none)" : "(Element)")
                    .font(.caption)
                Spacer()
                Button("...") { panel.editFixtureLink(current: currentNode) }
                    .controlSize(.small)
                    .accessibilityLabel("Linked Element")
            }
        }
    }

    private var currentNode: String? {
        if case .linkedElement(let node) = row.value { return node }
        return nil
    }

    private func commitPoint(x: Int32, y: Int32) {
        panel.commit(id: row.id, value: .point(x: x, y: y))
    }

    private func commitText() {
        guard text != displayText else { return }
        switch row.kind {
        case .int:
            if let value = Int32(text.trimmingCharacters(in: .whitespaces)) {
                panel.commit(id: row.id, value: .int(value: value))
            } else {
                text = displayText
            }
        case .double:
            if let value = Double(text.trimmingCharacters(in: .whitespaces)) {
                panel.commit(id: row.id, value: .double(value: value))
            } else {
                text = displayText
            }
        case .text:
            panel.commit(id: row.id, value: .text(value: text))
        case .point:
            let parts = text.split(separator: ",").compactMap { Int32($0.trimmingCharacters(in: .whitespaces)) }
            if parts.count == 2 {
                commitPoint(x: parts[0], y: parts[1])
            } else {
                text = displayText
            }
        default:
            break
        }
    }
}

private struct PointPartField: View {
    let title: String
    let value: Int32
    let commit: (Int32) -> Void
    @State private var text = ""

    var body: some View {
        HStack(spacing: 2) {
            Text(title)
                .font(.caption2)
            TextField(title, text: $text)
                .labelsHidden()
                .textFieldStyle(.roundedBorder)
                .controlSize(.mini)
                .font(.caption2.monospacedDigit())
                .onSubmit {
                    if let parsed = Int32(text) {
                        commit(parsed)
                    } else {
                        text = "\(value)"
                    }
                }
        }
        .onAppear { text = "\(value)" }
        .onChange(of: value) { text = "\(value)" }
    }
}
