import SwiftUI
import VixenBridge
import VixenKit

/// The wizard on its own, for the UI harness. What Finish made goes to standard error and beside the button: the edit's selection, then the root rows and what's under the selection's parent.
struct FixtureWizardTestSection: View {
    @Environment(AppModel.self) private var app: AppModel?
    @State private var session: DisplaySetupSession?
    @State private var model: FixtureWizardModel?
    @State private var result = "-"

    var body: some View {
        HStack {
            Button("Open Fixture Wizard") {
                Task { await open() }
            }
            Text(result)
                .font(.callout.monospaced())
                .textSelection(.enabled)
                .lineLimit(6)
        }
        .sheet(isPresented: Binding(get: { model != nil }, set: { if !$0 { model = nil } })) {
            if let model {
                FixtureWizardView(model: model)
            }
        }
    }

    private func open() async {
        guard let core = await core() else {
            result = "no core"
            return
        }
        let opened = await Task.detached { core.openDisplaySetup() }.value
        session = opened
        model = FixtureWizardModel(session: opened) { edit in
            model = nil
            report(edit, in: opened)
        }
    }

    /// The app's, or one of its own where this window isn't given the app model.
    private func core() async -> VixenCore? {
        if let core = app?.core {
            return core
        }
        guard let root = ProcessInfo.processInfo.environment["VIXEN_DATA"] else { return nil }
        return await Task.detached { try? VixenCore.open(dataRoot: root, muted: true) }.value
    }

    private func report(_ edit: SetupEditResult?, in session: DisplaySetupSession) {
        guard let edit else {
            result = "cancelled"
            FileHandle.standardError.write(Data("SHEET fixture: cancelled\n".utf8))
            return
        }
        let roots = session.elementRows(parent: nil)
        let under = edit.expand.first.map { session.elementRows(parent: $0) } ?? []
        result = "select \(edit.select) expand \(edit.expand) roots \(roots.map(\.name)) under \(under.map(\.name)) names \(session.namesOf(selection: edit.select))"
        FileHandle.standardError.write(Data("SHEET fixture: \(result)\n".utf8))
    }
}
