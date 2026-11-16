import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// Manage Tag Colors (F132). Edits stay here until Save hands them back.
struct ManageTagColorsSheet: View {
    private let onFinish: ([SetupTagColor]?) -> Void
    @State private var rows: [SetupTagColor]
    @State private var finished = false

    init(rows: [SetupTagColor], onFinish: @escaping ([SetupTagColor]?) -> Void) {
        self.onFinish = onFinish
        _rows = State(initialValue: rows)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Manage Tag Colors").font(.headline)
            ForEach($rows, id: \.id) { $row in
                HStack {
                    Text(row.name)
                    Spacer(minLength: 16)
                    ColorPicker(
                        row.name,
                        selection: Binding(get: { Color(argb: row.color) }, set: { row.color = NSColor($0).argb }),
                        supportsOpacity: false
                    )
                    .labelsHidden()
                }
            }
            SheetButtons(okTitle: "Save", ok: { finish(rows) }, cancel: { finish(nil) })
                .padding(.top, 8)
        }
        .padding(16)
        .frame(width: 360)
        .sheetKeys(ok: { finish(rows) }, cancel: { finish(nil) })
        .onAppear {
            guard UITestHarness.isActive else { return }
            UITestHarness.hooks["tagColors"] = { args in
                switch args.first ?? "state" {
                case "set":
                    guard args.count == 3, let index = Int(args[1]), rows.indices.contains(index), let color = UInt32(args[2], radix: 16) else {
                        return "FAIL needs row and ARGB hex"
                    }
                    rows[index].color = color
                case "save": finish(rows)
                case "cancel": finish(nil)
                case "state": break
                default: return "FAIL unknown tag-color action"
                }
                return rows.map { "\($0.name)=\(String($0.color, radix: 16))" }.joined(separator: ",")
            }
        }
        .onDisappear { if UITestHarness.isActive { UITestHarness.hooks["tagColors"] = nil } }
    }

    private func finish(_ result: [SetupTagColor]?) {
        guard !finished else { return }
        finished = true
        // The panel floats on its own and would stay up after the sheet has gone.
        if NSColorPanel.sharedColorPanelExists {
            NSColorPanel.shared.orderOut(nil)
        }
        onFinish(result)
    }
}
