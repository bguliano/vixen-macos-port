import SwiftUI

/// Colours for canvases and custom-drawn views, from upstream's ThemeColorTable. Standard controls use the
/// system's dark appearance instead of these.
public enum Theme {
    public static let background = Color(red: 68 / 255, green: 68 / 255, blue: 68 / 255)
    public static let foreground = Color(red: 235 / 255, green: 235 / 255, blue: 235 / 255)
    public static let foregroundDisabled = Color(red: 119 / 255, green: 119 / 255, blue: 119 / 255)
    public static let inputBackground = Color(red: 90 / 255, green: 90 / 255, blue: 90 / 255)
    public static let border = Color(red: 136 / 255, green: 136 / 255, blue: 136 / 255)
    public static let link = Color(red: 38 / 255, green: 160 / 255, blue: 218 / 255)

    public enum Timeline {
        public static let panelBackground = NSColor(red: 40 / 255, green: 40 / 255, blue: 40 / 255, alpha: 1)
        public static let grid = NSColor.black
        public static let effectsBackground = NSColor(red: 68 / 255, green: 68 / 255, blue: 68 / 255, alpha: 1)
        public static let foreground = NSColor(red: 235 / 255, green: 235 / 255, blue: 235 / 255, alpha: 1)
        public static let labelBackground = NSColor(red: 68 / 255, green: 68 / 255, blue: 68 / 255, alpha: 1)
    }

    /// Display design colours: Selected, Linked, Unlinked, Element Selected, Locked, Pixel #1.
    public enum Preview {
        public static let selected = Color(red: 50 / 255, green: 205 / 255, blue: 50 / 255)
        public static let linked = Color(red: 64 / 255, green: 224 / 255, blue: 208 / 255)
        public static let unlinked = Color.white
        public static let elementSelected = Color(red: 255 / 255, green: 105 / 255, blue: 180 / 255)
        public static let locked = Color(red: 139 / 255, green: 0, blue: 0)
        public static let pixel1 = Color.yellow
    }
}

extension Color {
    /// From ARGB as Rust passes colours.
    public init(argb: UInt32) {
        self.init(
            .sRGB,
            red: Double((argb >> 16) & 0xFF) / 255,
            green: Double((argb >> 8) & 0xFF) / 255,
            blue: Double(argb & 0xFF) / 255,
            opacity: Double((argb >> 24) & 0xFF) / 255
        )
    }
}

extension NSColor {
    public convenience init(argb: UInt32) {
        self.init(
            srgbRed: CGFloat((argb >> 16) & 0xFF) / 255,
            green: CGFloat((argb >> 8) & 0xFF) / 255,
            blue: CGFloat(argb & 0xFF) / 255,
            alpha: CGFloat((argb >> 24) & 0xFF) / 255
        )
    }

    /// As ARGB for passing to Rust, converted to sRGB first.
    public var argb: UInt32 {
        let c = usingColorSpace(.sRGB) ?? self
        let byte = { (v: CGFloat) in UInt32((v * 255).rounded().clamped(to: 0...255)) }
        return (byte(c.alphaComponent) << 24) | (byte(c.redComponent) << 16) | (byte(c.greenComponent) << 8) | byte(c.blueComponent)
    }
}

extension Comparable {
    public func clamped(to range: ClosedRange<Self>) -> Self {
        min(max(self, range.lowerBound), range.upperBound)
    }
}

/// Formats a time the way Vixen shows it: `m:ss.fff`.
public func formatTime(ms: Double) -> String {
    let total = max(0, Int((ms).rounded()))
    let minutes = total / 60_000
    let seconds = (total % 60_000) / 1000
    let millis = total % 1000
    return String(format: "%d:%02d.%03d", minutes, seconds, millis)
}
