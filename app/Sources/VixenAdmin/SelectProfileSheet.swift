import SwiftUI
import VixenBridge

/// Select a Profile. A sheet has no close box, so `Quit Vixen` stands in for it.
struct SelectProfileSheet: View {
    @Environment(AdminModel.self) private var admin
    @State private var rows: [ProfileRow] = []
    @State private var selection: Int32?
    @State private var showingEditor = false
    @State private var alerts = AlertPresenter()

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Select a Profile").font(.headline)
            Text("Select a Profile to Load:")
            List(rows, id: \.number, selection: $selection) { row in
                Text(row.isLocked ? "\(row.name) - Locked" : row.name)
                    .foregroundStyle(row.isLocked ? .secondary : .primary)
                    .selectionDisabled(row.isLocked)
            }
            .listStyle(.bordered)
            .frame(minHeight: 110)
            .contextMenu(forSelectionType: Int32.self) { _ in
            } primaryAction: { _ in
                load()
            }
            HStack {
                Button("Quit Vixen", role: .cancel) { quit() }
                    .keyboardShortcut(.cancelAction)
                Button("Profile Editor") { showingEditor = true }
                Spacer()
                Button("Load") { load() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(selectedRow == nil)
            }
        }
        .padding(16)
        .frame(width: 340, height: 260)
        .alerts(from: alerts)
        .task { await refresh() }
        .onChange(of: selection) {
            // Selecting a locked row clears the selection.
            if let row = rows.first(where: { $0.number == selection }), row.isLocked {
                selection = nil
            }
        }
        .sheet(isPresented: $showingEditor) {
            DataProfileSheet { saved in
                showingEditor = false
                if saved {
                    Task { await refresh() }
                }
            }
            .interactiveDismissDisabled()
        }
    }

    private var selectedRow: ProfileRow? {
        rows.first { $0.number == selection && !$0.isLocked }
    }

    private func refresh() async {
        rows = await Task.detached { listProfilesForSelection() }.value
        selection = nil
    }

    private func load() {
        if let row = selectedRow {
            admin.profileChosen(row)
        }
    }

    /// F5.
    private func quit() {
        Task {
            let answer = await alerts.ask(
                "Vixen",
                "Vixen cannot continue without a valid profile.\n\nAre you sure you want to exit Vixen?",
                [.init("Keep Selecting", role: .cancel), .init("Exit")],
                critical: true
            )
            if answer == "Exit" {
                exit(0)
            }
        }
    }
}
