import SwiftUI

/// `MessageBoxForm`, as an `.alert` a flow can wait on. A sheet has its own, since an alert must be presented from the top of the stack.
@Observable
final class AlertPresenter {
    struct Choice: Identifiable {
        let title: String
        var role: ButtonRole?
        var id: String { title }

        init(_ title: String, role: ButtonRole? = nil) {
            self.title = title
            self.role = role
        }
    }

    struct Request: Identifiable {
        let id = UUID()
        let title: String
        let message: String
        let choices: [Choice]
        let critical: Bool
        let finish: (String) -> Void
    }

    private(set) var current: Request?

    /// Shows an alert and waits. Returns the title of the button pressed.
    func ask(_ title: String, _ message: String, _ choices: [Choice], critical: Bool = false) async -> String {
        await withCheckedContinuation { continuation in
            current = Request(title: title, message: message, choices: choices, critical: critical) { continuation.resume(returning: $0) }
        }
    }

    /// An alert with one OK button.
    func tell(_ title: String, _ message: String, critical: Bool = false) async {
        _ = await ask(title, message, [Choice("OK")], critical: critical)
    }

    fileprivate func answer(_ title: String) {
        guard let request = current else { return }
        current = nil
        // SwiftUI needs the alert fully gone before another can be presented.
        Task {
            try? await Task.sleep(for: .milliseconds(250))
            request.finish(title)
        }
    }
}

private struct AlertsModifier: ViewModifier {
    let presenter: AlertPresenter

    func body(content: Content) -> some View {
        content
            .alert(
                presenter.current?.title ?? "",
                isPresented: Binding(get: { presenter.current != nil }, set: { _ in }),
                presenting: presenter.current
            ) { request in
                ForEach(request.choices) { choice in
                    Button(choice.title, role: choice.role) { presenter.answer(choice.title) }
                }
            } message: { request in
                Text(request.message)
            }
            .dialogSeverity(presenter.current?.critical == true ? .critical : .automatic)
    }
}

extension View {
    func alerts(from presenter: AlertPresenter) -> some View {
        modifier(AlertsModifier(presenter: presenter))
    }
}
