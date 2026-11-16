// Draws the macOS app icon from the Vixen logo: a dark rounded square with the logo centred.
// Usage: swift scripts/make_icon.swift <logo.png> <out.iconset>
import AppKit

let args = CommandLine.arguments
guard args.count == 3, let logo = NSImage(contentsOfFile: args[1]) else {
    fatalError("usage: make_icon logo.png out.iconset")
}
let out = URL(fileURLWithPath: args[2])
try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

func render(_ pixels: Int) -> Data {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels, bitsPerSample: 8, samplesPerPixel: 4,
        hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
    )!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let s = CGFloat(pixels)
    // Apple's icon grid: the tile is about 80% of the canvas, leaving room for the system shadow.
    let tile = NSRect(x: s * 0.1, y: s * 0.1, width: s * 0.8, height: s * 0.8)
    let shape = NSBezierPath(roundedRect: tile, xRadius: s * 0.18, yRadius: s * 0.18)
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.4)
    shadow.shadowOffset = NSSize(width: 0, height: -s * 0.012)
    shadow.shadowBlurRadius = s * 0.03
    NSGraphicsContext.saveGraphicsState()
    shadow.set()
    NSColor(white: 0.15, alpha: 1).setFill()
    shape.fill()
    NSGraphicsContext.restoreGraphicsState()
    NSGradient(colors: [NSColor(white: 0.27, alpha: 1), NSColor(white: 0.12, alpha: 1)])?.draw(in: shape, angle: -90)
    let side = s * 0.62
    let target = NSRect(x: (s - side) / 2, y: (s - side) / 2, width: side, height: side)
    let scale = min(target.width / logo.size.width, target.height / logo.size.height)
    let drawn = NSSize(width: logo.size.width * scale, height: logo.size.height * scale)
    logo.draw(
        in: NSRect(x: target.midX - drawn.width / 2, y: target.midY - drawn.height / 2, width: drawn.width, height: drawn.height),
        from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: [.interpolation: NSImageInterpolation.high]
    )
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

for (name, px) in [("16x16", 16), ("16x16@2x", 32), ("32x32", 32), ("32x32@2x", 64), ("128x128", 128), ("128x128@2x", 256), ("256x256", 256), ("256x256@2x", 512), ("512x512", 512), ("512x512@2x", 1024)] {
    try render(px).write(to: out.appending(path: "icon_\(name).png"))
}
