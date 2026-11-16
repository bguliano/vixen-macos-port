import SwiftUI
import VixenBridge
import VixenKit

/// `TextDialog` (F148). An empty title is the one-argument form, whose title bar is blank.
struct TextPromptSheet: View {
    private let title: String
    private let prompt: String
    private let selectAll: Bool
    private let onFinish: (String?) -> Void
    @State private var text: String
    @State private var selection: TextSelection?
    @State private var finished = false
    @FocusState private var focused: Bool

    init(title: String, prompt: String, initial: String, selectAll: Bool, onFinish: @escaping (String?) -> Void) {
        self.title = title
        self.prompt = prompt
        self.selectAll = selectAll
        self.onFinish = onFinish
        _text = State(initialValue: initial)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if !title.isEmpty {
                Text(title).font(.headline)
            }
            Text(prompt)
            TextField(prompt, text: $text, selection: $selection, prompt: Text(""))
                .labelsHidden()
                .focused($focused)
            SheetButtons(ok: { finish(text) }, cancel: { finish(nil) })
                .padding(.top, 4)
        }
        .padding(16)
        .frame(width: 404)
        .sheetKeys(ok: { finish(text) }, cancel: { finish(nil) })
        .task {
            focused = true
            guard !selectAll else { return }
            // Taking focus selects all the text, so the caret can only be placed once that has happened.
            try? await Task.sleep(for: .milliseconds(50))
            selection = TextSelection(insertionPoint: text.endIndex)
        }
    }

    private func finish(_ response: String?) {
        guard !finished else { return }
        finished = true
        onFinish(response)
    }
}

/// `NumberDialog` (F148).
struct NumberPromptSheet: View {
    private let title: String
    private let prompt: String
    private let range: ClosedRange<Int>
    private let onFinish: (Int?) -> Void
    @State private var value: Int
    @State private var finished = false

    init(title: String, prompt: String, value: Int, min: Int, max: Int, onFinish: @escaping (Int?) -> Void) {
        self.title = title
        self.prompt = prompt
        range = min...Swift.max(min, max)
        self.onFinish = onFinish
        _value = State(initialValue: value.clamped(to: min...Swift.max(min, max)))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if !title.isEmpty {
                Text(title).font(.headline)
            }
            HStack {
                SheetFieldLabel(prompt)
                Spacer(minLength: 16)
                SheetNumberField(prompt, value: $value, range: range, width: 90)
            }
            SheetButtons(ok: { finish(value) }, cancel: { finish(nil) })
                .padding(.top, 4)
        }
        .padding(16)
        .frame(width: 364)
        .sheetKeys(ok: { finish(value) }, cancel: { finish(nil) })
    }

    private func finish(_ number: Int?) {
        guard !finished else { return }
        finished = true
        onFinish(number)
    }
}

/// `ListSelectDialog` (F148): no row selected to begin with, and OK waits for one. A double-click or Return takes the row.
struct ListSelectSheet: View {
    private let title: String
    private let items: [SetupModuleType]
    private let onFinish: (SetupModuleType?) -> Void
    @State private var selected: String?
    @State private var finished = false
    @FocusState private var focused: Bool

    init(title: String, items: [SetupModuleType], onFinish: @escaping (SetupModuleType?) -> Void) {
        self.title = title
        self.items = items
        self.onFinish = onFinish
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if !title.isEmpty {
                Text(title).font(.headline)
            }
            List(items, id: \.id, selection: $selected) { item in
                Text(item.name)
            }
            .listStyle(.bordered)
            .focused($focused)
            .contextMenu(forSelectionType: String.self) { _ in
            } primaryAction: { ids in
                if let id = ids.first { finish(items.first { $0.id == id }) }
            }
            .onKeyPress(.return) {
                guard selected != nil else { return .ignored }
                finish(items.first { $0.id == selected })
                return .handled
            }
            .frame(minHeight: 170)
            SheetButtons(
                okDisabled: selected == nil,
                ok: { finish(items.first { $0.id == selected }) },
                cancel: { finish(nil) }
            )
        }
        .padding(16)
        .frame(width: 364)
        .sheetKeys(ok: { finish(items.first { $0.id == selected }) }, cancel: { finish(nil) })
        .onAppear { focused = true }
    }

    private func finish(_ item: SetupModuleType?) {
        guard !finished else { return }
        finished = true
        onFinish(item)
    }
}
