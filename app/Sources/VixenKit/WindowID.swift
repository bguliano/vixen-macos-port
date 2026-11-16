import Foundation

/// Scene ids, so areas can open each other's windows without depending on each other.
public enum WindowID {
    public static let administration = "administration"
    public static let installedModules = "installed-modules"
    public static let profiles = "profiles"
    /// Its window calls `AppModel.beginSystemConfigSession()` when it appears and `endSystemConfigSession(_:)` when it closes.
    public static let displaySetup = "display-setup"
    /// The preview designer. Setup Previews opens it with `value:` the preview's id as a `String`, and expects
    /// `AppModel.previewDesignerClosed(id:)` when it closes.
    public static let previewSetup = "preview-setup"
    public static let livePreview = "live-preview"
    public static let sequencer = "sequencer"
    public static let about = "about"
    public static let releaseNotes = "release-notes"
    public static let instrumentation = "instrumentation"
    /// Each opens with `value: UUID()`, since every click opens another window.
    public static let customPropEditor = "custom-prop-editor"
    public static let exportWizard = "export-wizard"
    public static let sequencePackageImport = "sequence-package-import"
    public static let sequencePackageExport = "sequence-package-export"
    public static let webServer = "web-server"
    public static let shows = "shows"
    public static let schedules = "schedules"
    public static let schedulerStatus = "scheduler-status"
}

/// What a sequencer window edits.
public enum SequenceDocumentRef: Codable, Hashable, Sendable {
    /// A new, unsaved sequence. The id tells two new windows apart.
    case untitled(UUID)
    case existing(path: String)

    public static func new() -> SequenceDocumentRef {
        .untitled(UUID())
    }
}
