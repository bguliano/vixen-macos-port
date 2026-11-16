import Foundation

/// The scheduler and export area's windows beyond those `WindowID` already names.
public extension WindowID {
    /// The Export Sequence dialog in a window of its own, for a sequence opened from its file: `openWindow(id:value:)` with the `.tim` path as a
    /// `String`. The sequencer uses `SequenceExport.present` instead, which takes its window's session, unsaved edits included.
    static let exportSequence = "export-sequence"
}
