import SwiftUI
import VixenBridge
import VixenKit

/// Data Profile. The rules are in `ProfileEditorSession`, so this only binds.
struct DataProfileSheet: View {
    let finished: (Bool) -> Void

    @State private var session = ProfileEditorSession()
    @State private var items: [ProfileRow] = []
    @State private var selected: UInt32?
    @State private var name = ""
    @State private var folder = ""
    @State private var askEveryTime = false
    @State private var loadSelection: UInt32?
    @State private var alerts = AlertPresenter()
    @State private var showingNamePrompt = false
    @State private var newName = "New Profile"
    @State private var showingArchive = false
    @State private var window: NSWindow?
    @FocusState private var nameFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Data Profile").font(.headline)
            GroupBox("Profiles:") {
                Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 8) {
                    GridRow {
                        Text("Profile:").gridColumnAlignment(.trailing)
                        Picker("Profile:", selection: Binding(get: { selected }, set: { choose($0) })) {
                            ForEach(items, id: \.number) { item in
                                Text(item.name).tag(Optional(UInt32(item.number)))
                            }
                        }
                        .labelsHidden()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        HStack(spacing: 4) {
                            Button(action: { commit(); newName = "New Profile"; showingNamePrompt = true }) {
                                Image(systemName: "plus").frame(width: 14, height: 14)
                            }
                            .accessibilityLabel("Add profile")
                            .help("Add a profile")
                            Button(action: delete) {
                                Image(systemName: "minus").frame(width: 14, height: 14)
                            }
                            .accessibilityLabel("Delete profile")
                            .help("Delete this profile")
                            .disabled(selected == nil)
                        }
                    }
                    GridRow {
                        Text("Profile Name:")
                        TextField("Profile Name:", text: $name, prompt: Text(""))
                            .labelsHidden()
                            .focused($nameFocused)
                            .onChange(of: nameFocused) {
                                if !nameFocused { commit() }
                            }
                            .onSubmit { commit() }
                            .gridCellColumns(2)
                            .disabled(selected == nil)
                    }
                    GridRow {
                        Text("Data Folder:")
                        TextField("Data Folder:", text: $folder, prompt: Text(""))
                            .labelsHidden()
                            .disabled(selected == nil)
                        Button(action: chooseDataFolder) {
                            Image(systemName: "folder").frame(width: 14, height: 14)
                        }
                        .accessibilityLabel("Choose data folder")
                        .help("Choose the data folder")
                        .disabled(selected == nil)
                    }
                }
                .padding(4)
            }
            GroupBox("Profile to Use when Loading Vixen") {
                HStack(spacing: 16) {
                    RadioChoice(title: "Ask Me Every Time", isOn: askEveryTime) { setAsk(true) }
                    RadioChoice(title: "Load This Profile:", isOn: !askEveryTime) { setAsk(false) }
                    Picker("Load This Profile:", selection: Binding(get: { loadSelection }, set: { setLoadSelection($0) })) {
                        Text("").tag(UInt32?.none)
                        ForEach(items, id: \.number) { item in
                            Text(item.name).tag(Optional(UInt32(item.number)))
                        }
                    }
                    .labelsHidden()
                    Spacer(minLength: 0)
                }
                .padding(4)
            }
            HStack {
                Button("Zip Wizard") { showingArchive = true }
                Spacer()
                Button("Cancel", role: .cancel) { finished(false) }
                    .keyboardShortcut(.cancelAction)
                Button("OK") { save() }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 520)
        .background(WindowReader { window = $0 })
        .alerts(from: alerts)
        .onAppear(perform: reload)
        .sheet(isPresented: $showingNamePrompt) {
            ProfileNamePrompt(name: $newName) { accepted in
                showingNamePrompt = false
                guard accepted else { return }
                Task { await addProfile() }
            }
        }
        .sheet(isPresented: $showingArchive) {
            ArchiveWizardSheet { showingArchive = false }
                .interactiveDismissDisabled()
        }
    }

    private func reload() {
        items = session.items()
        selected = session.selected()
        askEveryTime = session.askEveryTime()
        loadSelection = session.loadSelection()
        showSelected()
    }

    private func showSelected() {
        let item = selected.flatMap { index in items.first { UInt32($0.number) == index } }
        name = item?.name ?? ""
        folder = item?.dataFolder ?? ""
    }

    /// `SaveCurrentItem`: always both fields, so leaving the name commits the folder too.
    private func commit() {
        session.setName(text: name)
        session.setDataFolder(text: folder)
        items = session.items()
    }

    private func choose(_ index: UInt32?) {
        guard let index, index != selected else { return }
        commit()
        session.select(index: index)
        selected = index
        showSelected()
    }

    private func setAsk(_ ask: Bool) {
        session.setAskEveryTime(ask: ask)
        askEveryTime = ask
    }

    private func setLoadSelection(_ index: UInt32?) {
        session.setLoadSelection(index: index)
        loadSelection = session.loadSelection()
    }

    private func chooseDataFolder() {
        Task {
            if let chosen = await chooseFolder(startingAt: folder, on: window) {
                folder = chosen
            }
        }
    }

    /// F69: back to the prompt with the text kept until the name is good or the user cancels.
    private func addProfile() async {
        try? await Task.sleep(for: .milliseconds(300))
        if let problem = session.add(name: newName) {
            await alerts.tell(problem.title, problem.message, critical: problem.title == "Error")
            showingNamePrompt = true
            return
        }
        reload()
    }

    /// F70.
    private func delete() {
        guard let selected, let item = items.first(where: { UInt32($0.number) == selected }) else { return }
        commit()
        Task {
            let answer = await alerts.ask(
                "Delete a Profile",
                "Are you sure you want to delete the \(item.name) profile? The data folder and all its contents will remain and must be removed manually.",
                [.init("Cancel", role: .cancel), .init("Delete Profile", role: .destructive)],
                critical: true
            )
            if answer == "Delete Profile" {
                session.delete(index: selected)
                reload()
            }
        }
    }

    /// F75, with the fields committed first and nothing written until every question has its answer.
    private func save() {
        commit()
        Task {
            let issues = session.validate()
            for _ in 0..<issues.blankCount {
                await alerts.tell("Warning - Blank Entries", issues.blankMessage, critical: true)
            }
            if let message = issues.duplicatesMessage, await !confirm("Warning - Duplicate Entries", message) {
                return
            }
            if let message = issues.invalidMessage, await !confirm("Warning - Invalid Data Path", message) {
                return
            }
            for prompt in session.foldersToCreate() {
                let answer = await alerts.ask("Vixen", prompt.prompt, [.init("Don't Create", role: .cancel), .init("Create Folder")], critical: true)
                if answer == "Create Folder", !session.createFolder(folder: prompt.folder), await !confirm("Error", prompt.failedMessage) {
                    return
                }
            }
            do {
                try session.save()
                finished(true)
            } catch {
                await alerts.tell("Error", error.userMessage, critical: true)
            }
        }
    }

    private func confirm(_ title: String, _ message: String) async -> Bool {
        await alerts.ask(title, message, [.init("Cancel", role: .cancel), .init("OK")], critical: true) == "OK"
    }
}

/// A radio button on its own, since the second has a drop-down beside it that a `Picker` of radios can't hold.
private struct RadioChoice: View {
    let title: String
    let isOn: Bool
    let choose: () -> Void

    var body: some View {
        Toggle(title, isOn: Binding(get: { isOn }, set: { if $0 { choose() } }))
            .toggleStyle(.radioGroupItem)
    }
}

private extension ToggleStyle where Self == RadioGroupItemToggleStyle {
    static var radioGroupItem: RadioGroupItemToggleStyle { RadioGroupItemToggleStyle() }
}

private struct RadioGroupItemToggleStyle: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        Button {
            configuration.isOn = true
        } label: {
            HStack(spacing: 4) {
                Image(systemName: configuration.isOn ? "circle.inset.filled" : "circle")
                    .foregroundStyle(configuration.isOn ? Color.accentColor : .secondary)
                configuration.label
            }
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(configuration.isOn ? .isSelected : [])
    }
}

/// `TextDialog`, for the + button.
private struct ProfileNamePrompt: View {
    @Binding var name: String
    let finished: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Profile Name").font(.headline)
            Text("Enter a name for the new profile")
            TextField("Enter a name for the new profile", text: $name, prompt: Text(""))
                .labelsHidden()
                .frame(width: 280)
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { finished(false) }
                    .keyboardShortcut(.cancelAction)
                Button("OK") { finished(true) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
    }
}
