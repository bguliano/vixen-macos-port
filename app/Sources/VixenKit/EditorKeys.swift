import AppKit

/// A focused editor that uses some keys itself: Delete on a gradient handle, arrows on a slider, Return on a strip.
@MainActor
public protocol EditorKeyTaking: AnyObject {
    func takesKey(_ event: NSEvent) -> Bool
}

/// F66: keys pressed in the Effect Editor go to the timeline's shortcuts, unless a text box or the focused editor uses them. The sequencer's key monitor sees every key before any view does,
/// so it asks here first, the way it already leaves text boxes alone.
@MainActor
public enum EditorKeys {
    /// Arrow, Home, End and Page key codes.
    static let navigation: Set<UInt16> = [123, 124, 125, 126, 115, 119, 116, 121]
    static let returnKeys: Set<UInt16> = [36, 76]
    static let deleteKeys: Set<UInt16> = [51, 117]
    static let space: UInt16 = 49

    /// True when the key window's focused control will use this key itself, so a window-wide shortcut mustn't take it.
    public static func focusedEditorTakes(_ event: NSEvent) -> Bool {
        guard let responder = (event.window ?? NSApp.keyWindow)?.firstResponder else { return false }
        // A ⌘ or ⌃ key is a menu's, wherever the focus is.
        if !event.modifierFlags.intersection([.command, .control]).isEmpty {
            return false
        }
        if responder is NSText {
            return true
        }
        if let taker = responder as? EditorKeyTaking {
            return taker.takesKey(event)
        }
        // A pop-up menu opens with Space and the arrows (F29); a check box toggles with Space.
        if responder is NSPopUpButton {
            return event.keyCode == space || [125, 126].contains(event.keyCode)
        }
        if responder is NSButton {
            return event.keyCode == space
        }
        return false
    }

    /// For the test harness, whose key presses go straight to a window and never reach an app-wide monitor: `{ "action": "hook", "name": "editorTakesKey", "path": ["left"] }` says whether the
    /// key window's focused control would keep that key from the sequencer.
    public static func registerTestHook() {
        guard UITestHarness.isActive, UITestHarness.hooks["editorTakesKey"] == nil else { return }
        UITestHarness.hooks["editorTakesKey"] = { path in
            let codes: [String: (UInt16, String)] = [
                "left": (123, "\u{F702}"), "right": (124, "\u{F703}"), "down": (125, "\u{F701}"), "up": (126, "\u{F700}"),
                "home": (115, "\u{F729}"), "end": (119, "\u{F72B}"), "return": (36, "\r"), "delete": (51, "\u{7f}"), "space": (49, " "), "s": (1, "s"),
            ]
            guard let name = path.first, let (code, chars) = codes[name] else { return "FAIL: no key \(path)" }
            let window = NSApp.keyWindow ?? NSApp.orderedWindows.first { $0.isVisible }
            guard let event = NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window?.windowNumber ?? 0,
                context: nil, characters: chars, charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code
            ) else { return "FAIL: no event" }
            let responder = window?.firstResponder.map { String(describing: type(of: $0)) } ?? "none"
            return "\(name) \(focusedEditorTakes(event) ? "kept by" : "passed on by") \(responder)"
        }
    }

    public static func isReturn(_ event: NSEvent) -> Bool { returnKeys.contains(event.keyCode) }
    public static func isDelete(_ event: NSEvent) -> Bool { deleteKeys.contains(event.keyCode) }
    public static func isNavigation(_ event: NSEvent) -> Bool { navigation.contains(event.keyCode) }
}
