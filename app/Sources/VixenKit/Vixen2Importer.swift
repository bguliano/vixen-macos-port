import AppKit
import VixenBridge

/// The scheduler supplies the legacy importer without a dependency from the sequencer area.
public enum Vixen2Importer {
    public static var present: ((_ core: VixenCore, _ path: String, _ window: NSWindow?, _ completion: @escaping (SequencerSession?) -> Void) -> Void)?
}
