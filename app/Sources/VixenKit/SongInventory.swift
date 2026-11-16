import AppKit

/// Selected timing XML, an import error, or (nil,nil) for cancellation.
public enum SongInventory {
    public static var present: ((_ window: NSWindow?, _ completion: @escaping (String?, String?) -> Void) -> Void)?
}
