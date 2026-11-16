import SwiftUI
import VixenBridge
import VixenDialogs
import VixenKit

/// The wizard's whole window (F139): the steps at left, then the page with its title and description, and the buttons at the bottom. It sizes itself to upstream's
/// 1100 to 1400 wide and 605 high, so its `Window` wants `.windowResizability(.contentSize)`.
struct FixtureWizardView: View {
    private let model: FixtureWizardModel

    init(model: FixtureWizardModel) {
        self.model = model
    }

    private func fixtureColor(_ page: SetupFixturePage) -> UInt32 {
        switch page { case .automation: 0xff800080; case .colorSupport: 0xff0000ff; case .dimmingCurves: 0xffffc0cb; case .grouping: 0xff008000; default: 0xff1e90ff }
    }

    var body: some View {
        Group {
            if let state = model.state {
                HStack(spacing: 0) {
                    FixtureWizardSteps(model: model, state: state)
                    Divider()
                    VStack(alignment: .leading, spacing: 0) {
                        FixtureWizardHeader(state: state)
                        Divider()
                        HStack(spacing: 10) {
                            if state.page != .editFunctions && state.page != .editChannels {
                                MovingHeadView(pan: 35, color: fixtureColor(state.page)).frame(minWidth: 90, idealWidth: 140, maxWidth: 170)
                            }
                            FixtureWizardPage(model: model, state: state)
                            if state.page != .editFunctions && state.page != .editChannels {
                                MovingHeadView(pan: 315, color: fixtureColor(state.page)).frame(minWidth: 90, idealWidth: 140, maxWidth: 170)
                            }
                        }
                            .padding(20)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                        Divider()
                        FixtureWizardButtons(model: model, state: state)
                            .padding(12)
                    }
                }
            } else {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minWidth: model.state?.page == .editFunctions ? 1400 : 1100, maxWidth: 1400, minHeight: 605, maxHeight: 605)
        .navigationTitle("Intelligent Fixture Wizard")
        .task {
            await model.load()
        }
        .onDisappear {
            model.windowClosed()
        }
        .confirmationDialog("Replace Existing Fixture Profile?", isPresented: Binding(get: { model.confirmOverwrite }, set: { model.confirmOverwrite = $0 })) {
            Button("Replace Profile", role: .destructive) { model.finish(allowOverwrite: true) }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("A profile with this name already exists. Finishing will replace its saved definition.")
        }
        .alert(
            model.alert?.title ?? "",
            isPresented: Binding(get: { model.alert != nil }, set: { if !$0 { model.alert = nil } })
        ) {
            Button("OK") { model.alert = nil }
        } message: {
            Text(model.alert?.message ?? "")
        }
    }
}

/// Upstream's bubbles down the left: every page in order, and a click goes back to one already visited.
private struct FixtureWizardSteps: View {
    let model: FixtureWizardModel
    let state: SetupFixtureWizardState

    /// DodgerBlue, which upstream sets for these bubbles.
    private static let accent = Color(red: 30 / 255, green: 144 / 255, blue: 1)

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(Array(state.steps.enumerated()), id: \.element.page) { index, step in
                let isCurrent = step.page == state.page
                Button {
                    model.apply { $0.goTo(page: step.page) }
                } label: {
                    Label {
                        Text(step.title)
                            .foregroundStyle(step.visited ? Color.primary : Color.secondary)
                    } icon: {
                        Image(systemName: "\(index + 1).circle\(step.visited ? ".fill" : "")")
                            .foregroundStyle(step.visited ? Self.accent : Color.secondary)
                    }
                    .padding(.vertical, 6)
                    .padding(.horizontal, 10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(isCurrent ? Self.accent.opacity(0.3) : Color.clear, in: RoundedRectangle(cornerRadius: 8))
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(!step.canGo)
                .accessibilityAddTraits(isCurrent ? .isSelected : [])
            }
            Spacer(minLength: 0)
        }
        .padding(10)
        .frame(width: 210)
        .background(Color.primary.opacity(0.05))
    }
}

private struct FixtureWizardHeader: View {
    let state: SetupFixtureWizardState

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(state.title)
                .font(.title2.weight(.semibold))
            Text(state.description)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if let problem = state.problem {
                Label(problem, systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.yellow)
            }
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct FixtureWizardButtons: View {
    let model: FixtureWizardModel
    let state: SetupFixtureWizardState

    var body: some View {
        HStack(spacing: 8) {
            HelpLink { model.openHelp() }
            Spacer(minLength: 0)
            if model.isFinishing {
                ProgressView()
                    .controlSize(.small)
            }
            Button("Back") { model.apply { $0.goBack() } }
                .disabled(!state.canGoBack || model.isFinishing)
            if state.page == .summary {
                Button("Finish") { model.finish() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!state.canFinish || model.isFinishing)
            } else {
                Button("Next") { model.apply { $0.goNext() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!state.canGoNext)
            }
            Button("Cancel", role: .cancel) { model.cancel() }
                .keyboardShortcut(.cancelAction)
                .disabled(model.isFinishing)
        }
    }
}

private struct FixtureWizardPage: View {
    let model: FixtureWizardModel
    let state: SetupFixtureWizardState

    var body: some View {
        switch state.page {
        case .selectProfile:
            FixtureSelectProfilePage(model: model, state: state)
        case .editFunctions:
            if let editor = model.editor { FixtureFunctionsView(model: editor) }
        case .editChannels:
            if let editor = model.editor { FixtureChannelsView(model: editor, showProfile: false) }
        case .colorSupport:
            FixtureColorSupportPage(model: model, state: state)
        case .automation:
            FixtureAutomationPage(model: model, state: state)
        case .dimmingCurves:
            FixtureDimmingCurvesPage(model: model, state: state)
        case .grouping:
            FixtureGroupingPage(model: model, state: state)
        case .summary:
            FixtureSummaryPage(state: state)
        }
    }
}

