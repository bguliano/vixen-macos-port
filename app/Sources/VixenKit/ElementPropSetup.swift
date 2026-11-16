import AppKit

/// What a prop file says about its colours (`PhysicalMetadata.ColorMode`), which decides the colour question asked after an import.
public enum ImportedPropColor: Sendable {
    case fullColor, multiple, single, other
}

/// The questions upstream asks once an imported prop's elements exist, a dimming curve and then colours, are Display Setup's helpers.
/// Display Setup fills this in when it's in the build.
public enum ElementPropSetup {
    /// Asks them as sheets on `window` for the elements `nodes`, and applies the answers to the running display without saving.
    /// True when a helper ran, so the display may have changed.
    public static var run: ((_ nodes: [String], _ color: ImportedPropColor, _ window: NSWindow, _ app: AppModel) async -> Bool)?
}
