import AppKit

/// What the app delegate asks of whichever area owns it. Without the Administration area the app quits at once and ignores files.
public enum AppHooks {
    /// `applicationShouldTerminate`.
    public static var shouldTerminate: (() -> NSApplication.TerminateReply)?
    /// `application(_:open:)`: Finder's Open With.
    public static var openFiles: (([URL]) -> Void)?
}
