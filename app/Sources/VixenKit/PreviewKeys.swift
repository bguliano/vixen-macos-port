import AppKit

public extension Notification.Name {
    /// A key pressed in a live preview window that it doesn't use itself, with the `NSEvent` as the object. Upstream's `KeydownSWF`
    /// broadcast: whichever sequencer is active takes it, so transport keys work with a preview in front.
    static let previewForwardedKey = Notification.Name("VixenPreviewForwardedKey")
}
