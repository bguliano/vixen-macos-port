import SwiftUI
import VixenBridge
import VixenKit

/// The prompts, the template dialogs, the naming dialogs and Manage Tag Colors. What each sheet hands back goes to standard error and under the buttons.
struct NamingAndTemplateSheetsTestSection: View {
    private enum Choice: Identifiable {
        case text, textCaret, textUntitled, number, list
        case template(SetupTemplateKind)
        case names, namesRename(Int), replace(Int), tags

        var id: String { String(describing: self) }
    }

    @State private var choice: Choice?
    @State private var result = "-"
    /// F99: the caller's memory of each template, for the window's life.
    @State private var drafts: [SetupTemplateKind: SetupTemplateDraft] = [:]

    private let templates: [(String, SetupTemplateKind)] = [
        ("Open Group", .numberedGroup), ("Open Arch", .arch), ("Open Cane", .candyCane), ("Open Star", .star), ("Open Burst", .starburst),
        ("Open Icicle", .icicles), ("Open Lip", .lipSync), ("Open Mega", .megatree), ("Open Grid", .pixelGrid), ("Open Single", .singleItem),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                open("Open Text", .text)
                open("Open Caret", .textCaret)
                open("Open Blank", .textUntitled)
                open("Open Number", .number)
                open("Open List", .list)
                open("Open Tags", .tags)
            }
            LazyVGrid(columns: Array(repeating: GridItem(.fixed(110), alignment: .leading), count: 5), alignment: .leading) {
                ForEach(templates, id: \.0) { title, kind in
                    open(title, .template(kind))
                }
            }
            HStack {
                open("Open Names", .names)
                open("Open Rename", .namesRename(12))
                open("Open Replace", .replace(12))
                open("Open Big Rename", .namesRename(1_000_000))
                open("Open Big Replace", .replace(1_000_000))
            }
            Text(result)
                .font(.callout.monospaced())
                .textSelection(.enabled)
                .lineLimit(6)
        }
        .sheet(item: $choice) { choice in
            sheet(for: choice)
        }
    }

    private func open(_ title: String, _ choice: Choice) -> some View {
        Button(title) { self.choice = choice }
    }

    @ViewBuilder private func sheet(for choice: Choice) -> some View {
        switch choice {
        case .text:
            TextPromptSheet(title: "Rename", prompt: "Enter a new name:", initial: "Selected text", selectAll: true) { finish("text", $0 as Any) }
        case .textCaret:
            TextPromptSheet(title: "Rename", prompt: "Enter a new name:", initial: "Caret at the end", selectAll: false) { finish("caret", $0 as Any) }
        case .textUntitled:
            TextPromptSheet(title: "", prompt: "One-argument form, no title", initial: "", selectAll: false) { finish("blank", $0 as Any) }
        case .number:
            NumberPromptSheet(title: "Channel Count", prompt: "Number of channels:", value: 16, min: 8, max: 512) { finish("number", $0 as Any) }
        case .list:
            ListSelectSheet(title: "Add Property", items: setupPropertyTypes()) { finish("list", ($0?.name) as Any) }
        case .template(let kind):
            if let draft = drafts[kind] ?? setupTemplateDraft(kind: kind) {
                TemplateSheet(draft: draft, rememberOnCancel: { drafts[kind] = $0 }) { edited in
                    if let edited { drafts[kind] = edited }
                    finish("template", edited as Any)
                }
            }
        case .names:
            NameGeneratorSheet(seed: "Tree", oldNames: nil) { names in
                finish("names", names.map { "\($0.count): \($0.prefix(3)) … \($0.suffix(1))" } as Any)
            }
        case .namesRename(let count):
            NameGeneratorSheet(seed: "ignored", oldNames: sampleNames(count)) { names in
                finish("rename", names.map { "\($0.count): \($0.prefix(3)) … \($0.suffix(1))" } as Any)
            }
        case .replace(let count):
            SubstitutionRenamerSheet(oldNames: sampleNames(count)) { names in
                finish("replace", names.map { "\($0.count): \($0.prefix(3)) … \($0.suffix(1))" } as Any)
            }
        case .tags:
            ManageTagColorsSheet(rows: [
                SetupTagColor(id: "deprecated", name: "Deprecated", color: 0xFFCC_3333),
                SetupTagColor(id: "hidden", name: "Hidden", color: 0xFF80_8080),
                SetupTagColor(id: "prop", name: "Prop", color: 0xFF33_99FF),
            ]) { rows in
                finish("tags", rows.map { $0.map { "\($0.name)=\(String($0.color, radix: 16))" } } as Any)
            }
        }
    }

    private func finish(_ what: String, _ value: Any) {
        choice = nil
        result = "\(what): \(value)"
        FileHandle.standardError.write(Data("SHEET \(result)\n".utf8))
    }

    private func sampleNames(_ count: Int) -> [String] {
        (0..<count).map { "Tree \($0 + 1)" }
    }
}
