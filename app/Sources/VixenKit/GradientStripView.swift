import AppKit
import SwiftUI
import VixenBridge

/// The inline gradient row, F44 to F50: a 32 pt strip, then a 10 pt band with a handle per distinct stop position.
public struct GradientStrip: NSViewRepresentable {
    @Environment(\.panelIsShowing) private var showing
    public var value: GradientValue
    public var discrete: Bool
    public var validColors: [UInt32]
    public var isEnabled: Bool
    public var onCommit: (GradientValue) -> Void
    public var onOpenEditor: () -> Void
    /// A handle was double-clicked. `GradientStop.editColor` does what upstream does with it.
    public var onEditStop: (GradientHandleInfo) -> Void
    public var acceptsDrop: ((VixenDragValue) -> Bool)?
    public var performDrop: ((VixenDragValue) -> Void)?
    public var onPressed: (() -> Void)?
    /// A gradient and level pair's gradient half has no Return binding (F53); every other strip opens its dialog on Return (F50).
    public var opensOnReturn = true

    public init(
        value: GradientValue,
        discrete: Bool = false,
        validColors: [UInt32] = [],
        isEnabled: Bool = true,
        opensOnReturn: Bool = true,
        onCommit: @escaping (GradientValue) -> Void,
        onOpenEditor: @escaping () -> Void = {},
        onEditStop: @escaping (GradientHandleInfo) -> Void = { _ in },
        acceptsDrop: ((VixenDragValue) -> Bool)? = nil,
        performDrop: ((VixenDragValue) -> Void)? = nil,
        onPressed: (() -> Void)? = nil
    ) {
        self.value = value
        self.discrete = discrete
        self.validColors = validColors
        self.isEnabled = isEnabled
        self.opensOnReturn = opensOnReturn
        self.onCommit = onCommit
        self.onOpenEditor = onOpenEditor
        self.onEditStop = onEditStop
        self.acceptsDrop = acceptsDrop
        self.performDrop = performDrop
        self.onPressed = onPressed
    }

    public func makeNSView(context: Context) -> GradientStripNSView {
        GradientStripNSView(frame: .zero)
    }

    public func updateNSView(_ view: GradientStripNSView, context: Context) {
        view.setPanelVisible(showing)
        view.onCommit = onCommit
        view.onOpenEditor = onOpenEditor
        view.onEditStop = onEditStop
        view.acceptsDrop = acceptsDrop ?? { if case .curve = $0 { return false } else { return true } }
        view.performDrop = performDrop ?? { [onCommit] drop in
            switch drop {
            case let .gradient(gradient): onCommit(gradient)
            case let .color(argb): onCommit(solidGradientValue(argb: argb))
            case .curve: break
            }
        }
        view.onPressed = onPressed
        view.isEnabled = isEnabled
        view.opensOnReturn = opensOnReturn
        view.validColors = validColors
        view.setValue(value, discrete: discrete)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: GradientStripNSView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 200, height: GradientStripNSView.totalHeight)
    }
}

public final class GradientStripNSView: ValueDragView {
    public static let stripHeight: CGFloat = 32
    public static let bandHeight: CGFloat = 10
    public static let totalHeight: CGFloat = stripHeight + bandHeight

    public private(set) var value = GradientValue(stops: [], alphas: [], gammacorrected: false, title: nil, libraryName: "", isCurrentLibrary: false)
    public private(set) var discrete = false
    public var validColors: [UInt32] = []
    public var onCommit: ((GradientValue) -> Void)?
    public var onOpenEditor: (() -> Void)?
    public var onEditStop: ((GradientHandleInfo) -> Void)?
    public var isEnabled = true {
        didSet { if oldValue != isEnabled { needsDisplay = true } }
    }
    public var opensOnReturn = true

    private var bands: [GradientBand] = []
    private var handles: [GradientHandleInfo] = []
    private var focusedHandle: Int?
    private var pressedHandle: Int?
    private var dragging = false
    private var hold: GradientValue?
    private var pressEvent: NSEvent?
    private var tracking: NSTrackingArea?

    private var isLinked: Bool { !value.libraryName.isEmpty }

    /// F67: the row's gradient, link included.
    public override var dragValue: VixenDragValue? { .gradient(value) }

    /// A handle has the focus, so Delete is the strip's.
    var hasFocusedHandle: Bool {
        focusedHandle.map { $0 < handles.count } == true && !isLinked
    }

    public override var isFlipped: Bool { true }
    public override var acceptsFirstResponder: Bool { isEnabled }
    public override var focusRingMaskBounds: NSRect { stripRect }

    public override func drawFocusRingMask() {
        NSBezierPath(rect: stripRect).fill()
    }

    public override var intrinsicContentSize: NSSize {
        NSSize(width: NSView.noIntrinsicMetric, height: Self.totalHeight)
    }

    public func setValue(_ newValue: GradientValue, discrete newDiscrete: Bool) {
        guard pressedHandle == nil else { return }
        guard newValue != value || newDiscrete != discrete || bands.isEmpty else { return }
        value = newValue
        discrete = newDiscrete
        reload(newValue)
        toolTip = isLinked ? "Library: \(value.libraryName)" : nil
        setAccessibilityLabel(isLinked ? "Library: \(value.libraryName)" : "Gradient, \(value.stops.count) stops")
    }

    /// The handle on the row isn't the value's own any more, so focus goes when it's gone.
    private func reload(_ showing: GradientValue) {
        bands = gradientStripBands(value: showing, discrete: discrete)
        handles = gradientStripHandles(value: showing)
        if let f = focusedHandle, f >= handles.count {
            focusedHandle = nil
        }
        needsDisplay = true
    }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        setAccessibilityRole(.image)
        setAccessibilityCustomActions([
            NSAccessibilityCustomAction(name: "Edit") { [weak self] in
                self?.onOpenEditor?()
                return true
            },
            NSAccessibilityCustomAction(name: "Reverse") { [weak self] in
                guard let self, let reversed = gradientReversed(value: self.value) else { return false }
                self.commit(reversed)
                return true
            },
        ])
    }

    public override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking {
            removeTrackingArea(tracking)
        }
        let area = NSTrackingArea(rect: bounds, options: [.mouseMoved, .mouseEnteredAndExited, .cursorUpdate, .activeInKeyWindow, .inVisibleRect], owner: self)
        addTrackingArea(area)
        tracking = area
    }

    private var stripRect: NSRect {
        NSRect(x: 0, y: 0, width: bounds.width, height: Self.stripHeight)
    }

    private var bandRect: NSRect {
        NSRect(x: 0, y: Self.stripHeight, width: bounds.width, height: Self.bandHeight)
    }

    public override func draw(_ dirtyRect: NSRect) {
        let strip = stripRect
        NSColor(white: 0.1, alpha: 1).setFill()
        strip.fill()
        GradientDrawing.draw(bands: bands, in: strip.insetBy(dx: 1, dy: 1))
        NSColor(srgbRed: 136 / 255, green: 136 / 255, blue: 136 / 255, alpha: 1).setStroke()
        NSBezierPath.defaultLineWidth = 1
        NSBezierPath(rect: strip.insetBy(dx: 0.5, dy: 0.5)).stroke()
        if isLinked {
            LibraryBadge.draw(at: NSPoint(x: 2, y: 2))
            return
        }
        for (i, handle) in handles.enumerated() {
            let centre = handle.position * bounds.width
            if centre >= 0, centre <= bounds.width {
                StripHandle.draw(centerX: centre, top: Self.stripHeight, focused: i == focusedHandle)
            }
        }
    }

    private func local(_ event: NSEvent) -> NSPoint {
        convert(event.locationInWindow, from: nil)
    }

    private func handle(at p: NSPoint) -> Int? {
        guard !isLinked, bandRect.contains(p) else { return nil }
        // The last drawn is on top.
        return handles.indices.reversed().first { StripHandle.hit(p, centerX: handles[$0].position * bounds.width, top: Self.stripHeight) }
    }

    public override func mouseMoved(with event: NSEvent) {
        updateCursor(with: event)
    }

    public override func cursorUpdate(with event: NSEvent) {
        updateCursor(with: event)
    }

    private func updateCursor(with event: NSEvent) {
        guard isEnabled else { NSCursor.arrow.set(); return }
        if pressedHandle != nil { NSCursor.resizeLeftRight.set(); return }
        (handle(at: local(event)) != nil ? NSCursor.resizeLeftRight : NSCursor.arrow).set()
    }

    public override func mouseExited(with event: NSEvent) {
        (pressedHandle != nil ? NSCursor.resizeLeftRight : NSCursor.arrow).set()
    }

    public override func mouseDown(with event: NSEvent) {
        guard isEnabled else { return }
        window?.makeFirstResponder(self)
        onPressed?()
        let p = local(event)
        let flags = event.modifierFlags
        if bandRect.contains(p), !isLinked {
            if let h = handle(at: p) {
                focusedHandle = h
                needsDisplay = true
                if event.clickCount >= 2 {
                    onEditStop?(handles[h])
                    return
                }
                pressedHandle = h
                dragging = false
                pressEvent = event
                updateCursor(with: event)
                return
            }
            if event.clickCount == 1, bounds.width > 0 {
                commit(gradientAddStop(value: value, position: p.x / bounds.width, discrete: discrete, validColors: validColors))
            }
            return
        }
        // ⇧⌥ on the strip reverses at once (F49).
        if flags.contains(.shift), flags.contains(.option) {
            if let reversed = gradientReversed(value: value) {
                commit(reversed)
            }
            return
        }
        if event.clickCount == 2 {
            pressEvent = nil
            onOpenEditor?()
            return
        }
        pressEvent = event
    }

    public override func mouseDragged(with event: NSEvent) {
        guard isEnabled else { return }
        defer { if pressedHandle != nil { NSCursor.resizeLeftRight.set() } }
        guard let start = pressEvent else { return }
        let travelled = hypot(event.locationInWindow.x - start.locationInWindow.x, event.locationInWindow.y - start.locationInWindow.y)
        guard let h = pressedHandle else {
            if travelled > 4 {
                pressEvent = nil
                startValueDrag(.gradient(value), event: start)
            }
            return
        }
        if !dragging {
            if travelled > 2 {
                dragging = true
            }
            return
        }
        // FIX, F46: every move applies. Upstream's `Center` setter only raises its event when the value didn't change.
        let p = local(event)
        let centre = p.x.rounded(.toNearestOrEven)
        guard centre >= 0, centre <= bounds.width, bounds.width > 0 else { return }
        let position = centre / bounds.width
        let moved = gradientMoveHandle(value: value, stops: handles[h].stops, position: position)
        hold = moved
        bands = gradientStripBands(value: moved, discrete: discrete)
        handles[h].position = position
        ValueTip.shared.show(curveLevelTooltip(position: position), near: self, at: NSPoint(x: p.x, y: bounds.height))
        needsDisplay = true
    }

    public override func mouseUp(with event: NSEvent) {
        defer { updateCursor(with: event) }
        ValueTip.shared.hide()
        pressEvent = nil
        guard let h = pressedHandle else { return }
        pressedHandle = nil
        if dragging {
            dragging = false
            if let hold {
                self.hold = nil
                commit(hold)
            } else {
                reload(value)
            }
        } else if KitMods(event.modifierFlags).option, !KitMods(event.modifierFlags).shift, !KitMods(event.modifierFlags).command {
            commit(gradientDeleteHandle(value: value, stops: handles[h].stops))
        }
    }

    private func commit(_ done: GradientValue) {
        value = done
        reload(done)
        onCommit?(done)
    }

    public override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 36, 76 where opensOnReturn:
            onOpenEditor?()
        case 51, 117:
            if let f = focusedHandle, f < handles.count, !isLinked {
                focusedHandle = nil
                commit(gradientDeleteHandle(value: value, stops: handles[f].stops))
            }
        default:
            super.keyDown(with: event)
        }
    }

    public override func resignFirstResponder() -> Bool {
        focusedHandle = nil
        needsDisplay = true
        return super.resignFirstResponder()
    }
}

extension GradientStripNSView: EditorKeyTaking {
    /// Return opens the Color Gradient Editor (F50); Delete removes the focused handle's stops (F47).
    public func takesKey(_ event: NSEvent) -> Bool {
        guard isEnabled else { return false }
        return (EditorKeys.isReturn(event) && opensOnReturn) || (EditorKeys.isDelete(event) && hasFocusedHandle)
    }
}

/// Draws a gradient's bands: `CGGradient`, equal height per band, at real size.
public enum GradientDrawing {
    public static func draw(bands: [GradientBand], in rect: NSRect) {
        guard !bands.isEmpty, let context = NSGraphicsContext.current?.cgContext, let space = CGColorSpace(name: CGColorSpace.sRGB) else { return }
        let height = rect.height / CGFloat(bands.count)
        for (i, band) in bands.enumerated() {
            guard band.colors.count == band.positions.count, !band.colors.isEmpty else { continue }
            let colors = band.colors.map { NSColor(argb: $0).cgColor } as CFArray
            let locations = band.positions.map { CGFloat($0) }
            guard let gradient = CGGradient(colorsSpace: space, colors: colors, locations: locations) else { continue }
            let slice = NSRect(x: rect.minX, y: rect.minY + CGFloat(i) * height, width: rect.width, height: height)
            context.saveGState()
            context.clip(to: slice)
            context.drawLinearGradient(gradient, start: CGPoint(x: slice.minX, y: slice.midY), end: CGPoint(x: slice.maxX, y: slice.midY), options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
            context.restoreGState()
        }
    }
}

/// A gradient as a picture, for library lists: `GenerateColorGradientImage`.
public enum GradientThumbnail {
    public static func image(value: GradientValue, size: CGSize, discrete: Bool = false) -> NSImage {
        let bands = gradientStripBands(value: value, discrete: discrete)
        return NSImage(size: size, flipped: true) { rect in
            GradientDrawing.draw(bands: bands, in: rect)
            return true
        }
    }
}
