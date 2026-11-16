import AppKit
import VixenBridge

/// The Export Sequence dialog, which the sequencer's Sequence > Export item and its Export toolbar button open (`docs/spec/shows-scheduler-export.md`
/// F1). The scheduler and export area fills this in; it stays nil in a build without that area, and the sequencer then leaves its Export
/// items off.
///
/// Pass the window's own session, so the export takes the sequence as it stands, unsaved edits included, and the window, which the dialog
/// attaches to as a sheet. The Effect Preview stays stopped while the dialog is up.
public enum SequenceExport {
    public static var present: ((_ session: SequencerSession, _ window: NSWindow?) -> Void)?
}
