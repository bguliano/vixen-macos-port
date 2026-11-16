import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// F52: a list with a selected row, then buttons. All full width, with the name as a header. The panel scrolls as one, so a list doesn't scroll by itself.
struct CollectionEditor: View {
    let row: EditorRow
    let model: EffectEditorModel

    private var param: String { row.key.param }

    /// F58. The panel holds it, so a press elsewhere clears it.
    private var selected: Int? {
        get { model.selectedListRow?.param == param ? model.selectedListRow?.index : nil }
        nonmutating set { model.selectedListRow = newValue.map { ListRow(param: param, index: $0) } }
    }

    var body: some View {
        content
            .background(ListRegionMarker(param: param, model: model))
    }

    @ViewBuilder
    private var content: some View {
        switch row.control {
        case let .gradientList(items, discrete, valid, _):
            VStack(spacing: 0) {
                ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                    GradientRowStrip(
                        rowKey: itemKey(index),
                        title: row.label,
                        value: item,
                        discrete: discrete,
                        valid: valid,
                        readOnly: row.readOnly,
                        model: model,
                        onPressed: { selected = index }
                    )
                    .padding(3)
                    .background(selectionBackground(index))
                }
                listButtons(count: items.count, canReorder: true)
            }
        case let .colorList(items, valid, _):
            VStack(spacing: 2) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 28, maximum: 28), spacing: 3)], alignment: .leading, spacing: 3) {
                    ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                        ColorListSwatch(rowKey: itemKey(index), title: row.label, argb: item, valid: valid, readOnly: row.readOnly, model: model) { selected = index }
                            .padding(1)
                            .background(selectionBackground(index))
                    }
                }
                listButtons(count: items.count, canReorder: true)
            }
        case let .gradientLevels(items, discrete, valid, _):
            VStack(spacing: 0) {
                ForEach(Array(items.enumerated()), id: \.offset) { index, pair in
                    VStack(spacing: 2) {
                        CurveRowStrip(
                            rowKey: pairKey(index, "Curve"),
                            title: row.label,
                            value: pair.curve,
                            readOnly: row.readOnly,
                            model: model,
                            onPressed: { selected = index }
                        )
                        .padding(.vertical, 5)
                        GradientRowStrip(
                            rowKey: pairKey(index, "ColorGradient"),
                            title: row.label,
                            value: pair.gradient,
                            discrete: discrete,
                            valid: valid,
                            readOnly: row.readOnly,
                            model: model,
                            onPressed: { selected = index },
                            opensOnReturn: false
                        )
                    }
                    .padding(3)
                    .background(selectionBackground(index))
                }
                listButtons(count: items.count, canReorder: true)
            }
        case let .stringList(items, _):
            VStack(spacing: 2) {
                ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                    CommitTextField(text: item, isEnabled: !row.readOnly, onFocus: { selected = index }) { model.commit(itemKey(index), .text(value: $0)) }
                        .padding(1)
                        .background(selectionBackground(index))
                }
                listButtons(count: items.count, canReorder: false)
            }
            // This list alone selects its first row on load, so Remove is ready at once (F54).
            .onAppear {
                if selected == nil, !items.isEmpty {
                    selected = 0
                }
            }
        case let .recordList(style, items):
            VStack(spacing: 2) {
                if style.buttonsAbove, style.hasAddRemove {
                    listButtons(count: items.count, canReorder: false)
                }
                ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                    RecordItemView(item: item, inline: style.inline, model: model, isSelected: selected == index) { selected = index }
                }
                if !style.buttonsAbove, style.hasAddRemove {
                    listButtons(count: items.count, canReorder: false)
                }
            }
        default:
            EmptyView()
        }
    }

    private func itemKey(_ index: Int) -> EeRowKey {
        EeRowKey(param: param, index: UInt32(index), subParam: nil)
    }

    private func pairKey(_ index: Int, _ half: String) -> EeRowKey {
        EeRowKey(param: param, index: UInt32(index), subParam: half)
    }

    private func selectionBackground(_ index: Int) -> some View {
        (selected == index ? Color.accentColor.opacity(0.35) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: 3))
    }

    /// Add, Remove, Up and Down. Clicking Add clears the selected row, because Add moves the focus (F56); the others keep it (F58).
    private func listButtons(count: Int, canReorder: Bool) -> some View {
        let wanted = selected.map { UInt32($0) }
        return HStack(spacing: 6) {
            Button("Add") {
                selected = nil
                model.listAdd(param)
            }
            Button("Remove") {
                if let selected {
                    model.listRemove(param, index: selected)
                }
                selected = nil
            }
            .disabled(!model.api.listCanRemove(effects: model.selection, param: param, selected: wanted))
            if canReorder {
                Button("Up") {
                    if let selected {
                        model.listMove(param, from: selected, to: selected - 1)
                        self.selected = selected - 1
                    }
                }
                .disabled(!model.api.listCanMove(effects: model.selection, param: param, selected: wanted, delta: -1))
                Button("Down") {
                    if let selected {
                        model.listMove(param, from: selected, to: selected + 1)
                        self.selected = selected + 1
                    }
                }
                .disabled(!model.api.listCanMove(effects: model.selection, param: param, selected: wanted, delta: 1))
            }
            Spacer()
        }
        .padding(.top, 3)
    }
}

/// Marks where a list is, for `EffectEditorModel.watchPresses`.
private struct ListRegionMarker: NSViewRepresentable {
    let param: String
    let model: EffectEditorModel

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        model.listRegions[param] = WeakListRegion(view: view)
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        model.listRegions[param] = WeakListRegion(view: view)
    }
}

private struct ColorListSwatch: View {
    let rowKey: EeRowKey
    let title: String
    let argb: UInt32
    let valid: [UInt32]
    let readOnly: Bool
    let model: EffectEditorModel
    let onPressed: () -> Void
    @State private var live: UInt32?

    var body: some View {
        ColorSwatch(
            argb: live ?? argb,
            isEnabled: !readOnly,
            opensOnReturn: true,
            onOpen: { openColorEditor(title: title, argb: argb, valid: valid, model: model, live: { live = $0 }) { model.commit(rowKey, .color(argb: $0)) } },
            acceptsDrop: { model.canDrop(rowKey, $0) },
            performDrop: { model.drop(rowKey, $0) },
            onPressed: onPressed
        )
        .frame(width: 25, height: 25)
    }
}

/// F55: an item's own rows, with the same layout and editors. A custom State item is one line, with no labels (F52).
private struct RecordItemView: View {
    let item: EeRecordItem
    let inline: Bool
    let model: EffectEditorModel
    let isSelected: Bool
    let onSelect: () -> Void

    var body: some View {
        VStack(spacing: 2) {
            if !item.header.isEmpty {
                HStack(spacing: 6) {
                    rule
                    Text(item.header).fontWeight(.bold)
                    rule
                }
                .contentShape(Rectangle())
                .onTapGesture(perform: onSelect)
            }
            if inline {
                HStack(spacing: 6) {
                    ForEach(item.rows, id: \.key) { row in
                        RowEditor(rowKey: row.key, label: row.label, control: row.control, readOnly: row.readOnly, model: model)
                    }
                }
                .contentShape(Rectangle())
                .simultaneousGesture(TapGesture().onEnded(onSelect))
            } else {
                ForEach(item.rows, id: \.key) { row in
                    HStack(spacing: 6) {
                        NameLabel(text: row.label, tooltip: row.tooltip, width: model.labelWidth)
                        RowEditor(rowKey: row.key, label: row.label, control: row.control, readOnly: row.readOnly, model: model)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .padding(.vertical, 3)
                }
            }
        }
        .padding(2)
        .background((isSelected ? Color.accentColor.opacity(0.25) : Color.clear).clipShape(RoundedRectangle(cornerRadius: 3)))
    }

    private var rule: some View {
        Rectangle()
            .fill(item.ruleArgb.map { Color(argb: $0) } ?? Color.secondary.opacity(0.5))
            .frame(height: item.ruleArgb == nil ? 1 : 2)
    }
}
