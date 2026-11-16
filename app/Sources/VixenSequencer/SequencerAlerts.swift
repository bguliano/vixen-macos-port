import AppKit
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// `MessageBoxForm` as `NSAlert` sheets, with verbs on the buttons.
enum SequencerAlerts {
    enum SaveChangesAnswer {
        case save
        case dontSave
        case cancel
    }

    private static func style(_ style: ShellAlertStyle) -> NSAlert.Style {
        switch style {
        case .information: .informational
        case .warning: .warning
        case .error: .critical
        }
    }

    private static func show(_ alert: NSAlert, window: NSWindow?) async -> NSApplication.ModalResponse {
        guard let window, window.isVisible else {
            return alert.runModal()
        }
        // One sheet at a time: a second alert waits its turn.
        while window.attachedSheet != nil {
            try? await Task.sleep(for: .milliseconds(100))
        }
        return await withCheckedContinuation { continuation in
            alert.beginSheetModal(for: window) { response in
                continuation.resume(returning: response)
            }
        }
    }

    static func tell(title: String, message: String, style: ShellAlertStyle, window: NSWindow?) async {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = Self.style(style)
        alert.addButton(withTitle: "OK")
        _ = await show(alert, window: window)
    }

    /// True for the default button.
    static func confirm(title: String, message: String, style: ShellAlertStyle, confirm: String, cancel: String, window: NSWindow?, destructive: Bool = false) async -> Bool {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = Self.style(style)
        let button = alert.addButton(withTitle: confirm)
        button.hasDestructiveAction = destructive
        alert.addButton(withTitle: cancel)
        return await show(alert, window: window) == .alertFirstButtonReturn
    }

    enum ThreeWay {
        case first
        case second
        case third
    }

    /// A question with three verbs. The first is the default.
    static func ask(title: String, message: String, first: String, second: String, third: String, window: NSWindow?) async -> ThreeWay {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: first)
        alert.addButton(withTitle: second)
        alert.addButton(withTitle: third)
        switch await show(alert, window: window) {
        case .alertFirstButtonReturn: return .first
        case .alertSecondButtonReturn: return .second
        default: return .third
        }
    }

    /// F16's prompt: `Save changes to the sequence <name>?`, without Cancel when the app itself is closing.
    static func saveChanges(name: String, canCancel: Bool, window: NSWindow?) async -> SaveChangesAnswer {
        let alert = NSAlert()
        alert.messageText = "Save Changes?"
        alert.informativeText = "Save changes to the sequence \(name)?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Don't Save")
        if canCancel {
            alert.addButton(withTitle: "Cancel")
        }
        switch await show(alert, window: window) {
        case .alertFirstButtonReturn: return .save
        case .alertSecondButtonReturn: return .dontSave
        default: return .cancel
        }
    }
}

/// `saveFileDialog`: "Save Timed Sequence", filter `Timed Sequence Editor (*.tim)|*.tim`, starting in the sequence folder. F13. The extension rule is Rust's, so
/// the panel accepts anything typed.
enum SaveSequencePanel {
    static func run(directory: String, suggestedName: String, window: NSWindow?) async -> URL? {
        let panel = NSSavePanel()
        panel.title = "Save Timed Sequence"
        panel.directoryURL = URL(fileURLWithPath: directory, isDirectory: true)
        panel.nameFieldStringValue = suggestedName
        panel.allowsOtherFileTypes = true
        panel.isExtensionHidden = false
        if let type = UTType("com.vixenlights.timed-sequence") ?? UTType(filenameExtension: "tim") {
            panel.allowedContentTypes = [type]
        }
        // The harness keeps its windows off screen, where a sheet can't be seen or answered.
        if let window, window.isVisible, !UITestHarness.isActive {
            let response = await panel.beginSheetModal(for: window)
            return response == .OK ? panel.url : nil
        }
        if let scripted = ProcessInfo.processInfo.environment["VIXEN_TEST_SAVE_AS"] {
            return URL(fileURLWithPath: scripted)
        }
        if UITestHarness.isActive {
            return nil
        }
        return panel.runModal() == .OK ? panel.url : nil
    }
}

/// F315: `org.vixenlights.timeline-elements`, with the count as its own small item so reading it never decodes the effects. Tests use a private pasteboard so a
/// script never touches the owner's clipboard.
enum SequencerPasteboard {
    private static let types = sequencerPasteboardTypes()
    private static let dataType = NSPasteboard.PasteboardType(types.data)
    private static let countType = NSPasteboard.PasteboardType(types.count)

    static var board: NSPasteboard {
        UITestHarness.isActive ? NSPasteboard(name: NSPasteboard.Name("org.vixenlights.test")) : .general
    }

    static func write(json: String, count: Int) {
        let board = board
        board.clearContents()
        board.declareTypes([dataType, countType], owner: nil)
        board.setString(json, forType: dataType)
        board.setString(String(count), forType: countType)
    }

    static func count() -> Int {
        Int(board.string(forType: countType) ?? "") ?? 0
    }

    static func readJSON() -> String? {
        board.string(forType: dataType)
    }
}
