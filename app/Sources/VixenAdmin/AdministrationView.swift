import SwiftUI
import VixenBridge
import VixenKit

/// The layout of `VixenApplication`: the title across the top, then Sequences on the left and System Configuration on the right, the progress bar
/// across both, and the status bar along the bottom.
struct AdministrationView: View {
    @Environment(AdminModel.self) private var admin
    @Environment(\.openWindow) private var openWindow
    @State private var selection: String?

    var body: some View {
        VStack(spacing: 0) {
            ProportionalStack(axis: .vertical, fractions: [0.3, 0.7]) {
                titlePanel
                buttonPanel
            }
            statusBar
        }
        .frame(minWidth: 472, minHeight: 578)
        .navigationTitle(admin.title)
        .background(WindowReader { admin.attach(window: $0) })
        .background(WindowOpenerRegistration())
        .alerts(from: admin.alerts)
        .sheet(item: Binding(get: { admin.sheet }, set: { admin.sheet = $0 })) { sheet in
            sheetContent(sheet)
                .interactiveDismissDisabled()
                .environment(admin)
        }
        .onAppear { admin.openWindow = openWindow }
        .task { await admin.start() }
    }

    @ViewBuilder
    private func sheetContent(_ sheet: AdminSheet) -> some View {
        switch sheet {
        case .selectProfile:
            SelectProfileSheet()
        case .dataProfile:
            DataProfileSheet { saved in admin.profilesEdited(saved: saved) }
        case .setupPreviews:
            SetupPreviewsSheet()
        case .checkForUpdates:
            CheckForUpdatesSheet()
        case .webServer:
            if let server = admin.webServer { WebServerSheet(server: server) }
        }
    }

    /// `titlePanel`: 40/25/25/10 across and 70/15/15 down, with the logo spanning the rows and Vixen the last three columns.
    private var titlePanel: some View {
        ProportionalStack(axis: .horizontal, fractions: [0.4, 0.6]) {
            logo
            ProportionalStack(axis: .vertical, fractions: [0.7, 0.15, 0.15]) {
                Text("Vixen")
                    .font(.system(size: 120, weight: .bold))
                    .minimumScaleFactor(0.1)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                versionLabel(admin.version.versionName)
                versionLabel(admin.version.buildName)
            }
        }
    }

    @ViewBuilder
    private var logo: some View {
        if let image = VixenResources.image("images/V3Logo.png") {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .padding(3)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityHidden(true)
        }
    }

    /// In the third of the four columns.
    private func versionLabel(_ text: String) -> some View {
        ProportionalStack(axis: .horizontal, fractions: [25.0 / 60, 35.0 / 60]) {
            Color.clear
            Text(text)
                .font(.system(size: 18))
                .minimumScaleFactor(0.3)
                .lineLimit(1)
                .foregroundStyle(versionColor)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        }
    }

    private var versionColor: Color {
        if admin.version.isDevBuild { return .yellow }
        if admin.version.isTestBuild { return .red }
        return .primary
    }

    /// `buttonPanel`: two equal columns, then the progress bar across both, whose row stays when it's hidden.
    private var buttonPanel: some View {
        ProportionalStack(axis: .vertical, fractions: [0.8836, 0.1164]) {
            ProportionalStack(axis: .horizontal, fractions: [0.5, 0.5]) {
                sequences
                    .padding(.horizontal, 4)
                systemConfiguration
                    .padding(.horizontal, 4)
            }
            progressBar
                .padding(.horizontal, 6)
        }
        .padding(.horizontal, 3)
    }

    private var sequences: some View {
        GroupBox("Sequences") {
            VStack(alignment: .leading, spacing: 8) {
                newSequenceButton
                    .disabled(admin.isBusy)
                wideButton("Open Sequence...") { admin.chooseSequencesToOpen() }
                    .disabled(admin.isBusy)
                Text("Recent Sequences:")
                    .padding(.top, 2)
                recentList
            }
            .padding(2)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .dropDestination(for: URL.self) { urls, _ in
            let files = urls.filter { ["tim", "vix"].contains($0.pathExtension.lowercased()) }
            admin.openFiles(files)
            return !files.isEmpty
        }
    }

    /// With one sequence type that can be created it just opens one. A menu would only appear if a second type ever existed.
    @ViewBuilder
    private var newSequenceButton: some View {
        if admin.creatableTypes.count > 1 {
            Menu("New Sequence...") {
                ForEach(admin.creatableTypes, id: \.typeName) { type in
                    Button(type.typeName) { admin.newSequence() }
                }
            }
            .controlSize(.large)
        } else {
            wideButton("New Sequence...") { admin.newSequence() }
        }
    }

    private var recentList: some View {
        List(admin.recent, id: \.path, selection: $selection) { row in
            Text(row.name)
                .lineLimit(1)
        }
        .listStyle(.plain)
        .contextMenu(forSelectionType: String.self) { _ in
            Button("Clear Recent Sequences") { admin.clearRecents() }
                .help("Clears the Recent Sequence list")
        } primaryAction: { paths in
            if let path = paths.first {
                admin.openRecent(path: path)
            }
        }
    }

    private var systemConfiguration: some View {
        VStack {
            GroupBox("System Configuration") {
                VStack(spacing: 8) {
                    wideButton("Setup Display") { admin.showDisplaySetup() }
                    wideButton("Setup Previews") { admin.showSetupPreviews() }
                }
                .padding(2)
            }
            .disabled(admin.isBusy)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func wideButton(_ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).frame(maxWidth: .infinity)
        }
        .controlSize(.large)
    }

    private var progressBar: some View {
        VStack(spacing: 2) {
            Text(admin.progressText)
                .font(.callout)
                .lineLimit(1)
            ProgressView(value: admin.progressValue)
                .progressViewStyle(.linear)
                .tint(Color(red: 0, green: 100.0 / 255, blue: 0))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .opacity(admin.progressVisible ? 1 : 0)
        .accessibilityHidden(!admin.progressVisible)
    }

    /// `statusStrip`: the light, the state, the update notice, then the CPU reading at the far right.
    private var statusBar: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(spacing: 6) {
                RoundedRectangle(cornerRadius: 3)
                    .fill(Color(
                        red: Double(admin.executionStatus.lightRed) / 255,
                        green: Double(admin.executionStatus.lightGreen) / 255,
                        blue: Double(admin.executionStatus.lightBlue) / 255
                    ))
                    .frame(width: 16, height: 16)
                    .accessibilityLabel(admin.executionStatus.text)
                Text(admin.executionStatus.text)
                    .lineLimit(1)
                Divider()
                Text(admin.updateNotice)
                    .lineLimit(1)
                Spacer(minLength: 8)
                CpuReadoutText(readout: admin.cpu)
            }
            .font(.callout)
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            .fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// One `Text`, so the reading changing once a second touches nothing else.
private struct CpuReadoutText: View {
    let readout: CpuReadout

    var body: some View {
        Text(readout.text)
            .monospacedDigit()
    }
}

/// Registers the SwiftUI window opener with the test harness.
private struct WindowOpenerRegistration: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .onAppear { UITestHarness.openWindow = { openWindow(id: $0) } }
    }
}
