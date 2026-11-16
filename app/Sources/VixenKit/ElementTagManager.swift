import AppKit

/// The element tag manager, which the sequencer's row menu opens with Manage Tags (`docs/spec/sequencer-shell.md` F378, timeline F38). Display Setup
/// owns the tag manager and fills this in; it stays nil in a build without that area.
///
/// Upstream calls `ElementTagManagerWindow.ShowAsDialog(saveOnClose: true)`: the sequencer has no OK or Cancel of its own, so the manager saves the
/// system configuration itself when it closes. Present it over `window`, and call `finished(true)` once tags were saved, so the timeline repaints its
/// labels and applies the hidden row filter again.
public enum ElementTagManager {
    public static var present: ((_ window: NSWindow?, _ finished: @escaping @MainActor (Bool) -> Void) -> Void)?
}
