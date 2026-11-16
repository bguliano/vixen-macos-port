import AppKit

/// `MessageBoxForm` as `NSAlert` sheets, one at a time, with verbs on the buttons.
enum SchedulerAlerts {
    enum Style {
        case information
        case warning
        case error
    }

    private static func show(_ alert: NSAlert, window: NSWindow?) async -> NSApplication.ModalResponse {
        guard let window, window.isVisible || window.sheetParent != nil else {
            return alert.runModal()
        }
        while window.attachedSheet != nil {
            try? await Task.sleep(for: .milliseconds(100))
        }
        return await withCheckedContinuation { continuation in
            alert.beginSheetModal(for: window) { response in
                continuation.resume(returning: response)
            }
        }
    }

    private static func alert(_ title: String, _ message: String, _ style: Style) -> NSAlert {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        switch style {
        case .information: alert.alertStyle = .informational
        case .warning: alert.alertStyle = .warning
        case .error: alert.alertStyle = .critical
        }
        return alert
    }

    static func tell(_ title: String, _ message: String, style: Style, window: NSWindow?) async {
        let alert = alert(title, message, style)
        alert.addButton(withTitle: "OK")
        _ = await show(alert, window: window)
    }

    /// True for `confirm`, the default button.
    static func confirm(_ title: String, _ message: String, style: Style, confirm: String, cancel: String, destructive: Bool = false, window: NSWindow?) async -> Bool {
        let alert = alert(title, message, style)
        alert.addButton(withTitle: confirm).hasDestructiveAction = destructive
        alert.addButton(withTitle: cancel)
        return await show(alert, window: window) == .alertFirstButtonReturn
    }
}
