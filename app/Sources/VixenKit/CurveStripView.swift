import AppKit
import SwiftUI
import VixenBridge

/// The inline curve row, F38 to F43: one view that draws the strip, the dots, the library badge and the level handle, and sends its gestures to Rust. Nothing renders during a drag: the
/// value commits on release.
public struct CurveStrip: NSViewRepresentable {
    @Environment(\.panelIsShowing) private var showing
    public var value: CurveValue
    public var isEnabled: Bool
    public var onCommit: (CurveValue) -> Void
    public var onOpenEditor: () -> Void
    public var acceptsDrop: ((VixenDragValue) -> Bool)?
    public var performDrop: ((VixenDragValue) -> Void)?
    public var onPressed: (() -> Void)?

    public init(
        value: CurveValue,
        isEnabled: Bool = true,
        onCommit: @escaping (CurveValue) -> Void,
        onOpenEditor: @escaping () -> Void = {},
        acceptsDrop: ((VixenDragValue) -> Bool)? = nil,
        performDrop: ((VixenDragValue) -> Void)? = nil,
        onPressed: (() -> Void)? = nil
    ) {
        self.value = value
        self.isEnabled = isEnabled
        self.onCommit = onCommit
        self.onOpenEditor = onOpenEditor
        self.acceptsDrop = acceptsDrop
        self.performDrop = performDrop
        self.onPressed = onPressed
    }

    public func makeNSView(context: Context) -> CurveStripNSView {
        CurveStripNSView(frame: .zero)
    }

    public func updateNSView(_ view: CurveStripNSView, context: Context) {
        view.setPanelVisible(showing)
        view.onCommit = onCommit
        view.onOpenEditor = onOpenEditor
        view.acceptsDrop = acceptsDrop ?? { if case .curve = $0 { return true } else { return false } }
        view.performDrop = performDrop ?? { [onCommit] drop in
            if case let .curve(curve) = drop { onCommit(curve) }
        }
        view.onPressed = onPressed
        view.isEnabled = isEnabled
        view.setValue(value)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: CurveStripNSView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 200, height: nsView.intrinsicContentSize.height)
    }
}

public final class CurveStripNSView: ValueDragView {
    public static let stripHeight: CGFloat = 32
    private static let barHeight: CGFloat = 10
    private static let barMargin: CGFloat = 3
    private static let blue = NSColor(srgbRed: 0, green: 128 / 255, blue: 1, alpha: 1)
    private static let grey = NSColor(srgbRed: 136 / 255, green: 136 / 255, blue: 136 / 255, alpha: 1)

    public private(set) var value = CurveValue(points: [], libraryName: "", isCurrentLibrary: false)
    public var onCommit: ((CurveValue) -> Void)?
    public var onOpenEditor: (() -> Void)?
    public var isEnabled = true {
        didSet { if oldValue != isEnabled { needsDisplay = true } }
    }

    private var preview: [KitPoint]?
    private var drawPoints: [CGPoint] = []
    private var drawSize = CGSize.zero
    private var gesture: CurveStripGesture?
    private var pressEvent: NSEvent?
    private var capturedCursor: NSCursor?
    private var handleDrag = false
    private var handleHold: CurveValue?
    private var levelBarVisible = false {
        didSet {
            if oldValue != levelBarVisible {
                invalidateIntrinsicContentSize()
                needsDisplay = true
            }
        }
    }
    private var flagsMonitor: Any?
    private var tracking: NSTrackingArea?

    private var isLinked: Bool { !value.libraryName.isEmpty }

    /// F67: the row's curve, link included.
    public override var dragValue: VixenDragValue? { .curve(value) }

    public override var isFlipped: Bool { true }
    public override var acceptsFirstResponder: Bool { isEnabled }
    public override var focusRingMaskBounds: NSRect { stripRect }

    public override func drawFocusRingMask() {
        NSBezierPath(rect: stripRect).fill()
    }

    public override var intrinsicContentSize: NSSize {
        NSSize(width: NSView.noIntrinsicMetric, height: Self.stripHeight + (levelBarVisible ? Self.barHeight + Self.barMargin * 2 : 0))
    }

    public func setValue(_ newValue: CurveValue) {
        guard newValue != value || preview != nil else { return }
        guard gesture == nil, !handleDrag else { return }
        value = newValue
        preview = nil
        drawSize = .zero
        needsDisplay = true
        toolTip = isLinked ? "Library: \(value.libraryName)" : nil
        setAccessibilityLabel(isLinked ? "Library: \(value.libraryName)" : "Curve, \(value.points.count) points")
        if isLinked {
            levelBarVisible = false
        }
    }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        setAccessibilityRole(.image)
        setAccessibilityCustomActions([
            NSAccessibilityCustomAction(name: "Edit") { [weak self] in
                self?.onOpenEditor?()
                return true
            },
            NSAccessibilityCustomAction(name: "Invert") { [weak self] in
                self?.applyModifierClick(KitMods(shift: true, command: true, option: false)) ?? false
            },
            NSAccessibilityCustomAction(name: "Reverse") { [weak self] in
                self?.applyModifierClick(KitMods(shift: true, command: false, option: true)) ?? false
            },
        ])
    }

    private func applyModifierClick(_ mods: KitMods) -> Bool {
        guard let g = curveStripPress(curve: value, x: 0, y: 0, width: bounds.width, height: Self.stripHeight, mods: mods), let done = g.mouseUp(mods: mods) else {
            return false
        }
        commit(done)
        return true
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

    private var barRect: NSRect {
        NSRect(x: 0, y: Self.stripHeight + Self.barMargin, width: bounds.width, height: Self.barHeight)
    }

    public override func draw(_ dirtyRect: NSRect) {
        let strip = stripRect
        NSColor.black.setFill()
        strip.fill()
        let points = preview ?? value.points
        if drawSize != strip.size || preview != nil {
            drawSize = strip.size
            drawPoints = curveStripPoints(points: points, width: strip.width, height: strip.height).map { CGPoint(x: $0.x, y: $0.y) }
        }
        if drawPoints.count > 1 {
            let path = NSBezierPath()
            path.lineWidth = 2
            path.lineJoinStyle = .round
            path.move(to: drawPoints[0])
            for p in drawPoints.dropFirst() {
                path.line(to: p)
            }
            (isLinked ? Self.grey : Self.blue).setStroke()
            path.stroke()
        }
        Self.grey.setFill()
        for p in drawPoints {
            NSBezierPath(ovalIn: NSRect(x: p.x - 3, y: p.y - 3, width: 6, height: 6)).fill()
        }
        Self.grey.setStroke()
        NSBezierPath.defaultLineWidth = 1
        NSBezierPath(rect: strip.insetBy(dx: 0.5, dy: 0.5)).stroke()
        if isLinked {
            LibraryBadge.draw(at: NSPoint(x: 2, y: 2))
        }
        if levelBarVisible {
            let position = handleHold.map { curveLevelPosition(curve: $0) } ?? curveLevelPosition(curve: value)
            StripHandle.draw(centerX: position * barRect.width, top: barRect.minY, focused: handleDrag)
        }
    }

    private func local(_ event: NSEvent) -> NSPoint {
        convert(event.locationInWindow, from: nil)
    }

    public override func mouseEntered(with event: NSEvent) {
        updateLevelBar(event.modifierFlags)
        flagsMonitor = NSEvent.addLocalMonitorForEvents(matching: .flagsChanged) { [weak self] e in
            self?.updateLevelBar(e.modifierFlags)
            return e
        }
    }

    public override func mouseExited(with event: NSEvent) {
        if let flagsMonitor {
            NSEvent.removeMonitor(flagsMonitor)
        }
        flagsMonitor = nil
        levelBarVisible = false
        (capturedCursor ?? .arrow).set()
    }

    /// The bar shows while ⇧ is held and the pointer is inside (F42).
    private func updateLevelBar(_ flags: NSEvent.ModifierFlags) {
        levelBarVisible = flags.contains(.shift) && !isLinked && isEnabled
    }

    public override func mouseMoved(with event: NSEvent) {
        updateLevelBar(event.modifierFlags)
        updateCursor(with: event)
    }

    public override func cursorUpdate(with event: NSEvent) {
        updateCursor(with: event)
    }

    private func updateCursor(with event: NSEvent) {
        guard isEnabled else { NSCursor.arrow.set(); return }
        if let capturedCursor { capturedCursor.set(); return }
        let p = local(event)
        guard bounds.contains(p) else { NSCursor.arrow.set(); return }
        NSCursor.kit(curveStripHover(
            curve: value, x: p.x, y: p.y, width: bounds.width, height: Self.stripHeight, mods: KitMods(event.modifierFlags), overHandle: overHandle(p)
        )).set()
    }

    private func overHandle(_ p: NSPoint) -> Bool {
        guard levelBarVisible else { return false }
        let centre = curveLevelPosition(curve: value) * barRect.width
        return abs(p.x - centre) <= 5 && p.y >= barRect.minY && p.y <= barRect.maxY
    }

    public override func mouseDown(with event: NSEvent) {
        guard isEnabled else { return }
        window?.makeFirstResponder(self)
        onPressed?()
        let p = local(event)
        pressEvent = event
        if event.clickCount == 2 {
            gesture = nil
            capturedCursor = nil
            pressEvent = nil
            onOpenEditor?()
            return
        }
        if overHandle(p) {
            handleDrag = true
            capturedCursor = .resizeLeftRight
            ValueTip.shared.show(curveLevelTooltip(position: curveLevelPosition(curve: value)), near: self, at: p)
            updateCursor(with: event)
            needsDisplay = true
            return
        }
        gesture = curveStripPress(curve: value, x: p.x, y: p.y, width: bounds.width, height: Self.stripHeight, mods: KitMods(event.modifierFlags))
        // Upstream keeps the hover cursor while the point owns mouse capture.
        // The public value remains unchanged until release, so a moved pointer
        // cannot be hit-tested against that old value to choose its drag cursor.
        capturedCursor = gesture?.captured() == true ? NSCursor.kit(curveStripHover(
            curve: value, x: p.x, y: p.y, width: bounds.width, height: Self.stripHeight, mods: KitMods(event.modifierFlags), overHandle: false
        )) : nil
        if capturedCursor != nil { updateCursor(with: event) }
    }

    public override func mouseDragged(with event: NSEvent) {
        guard isEnabled else { return }
        defer { capturedCursor?.set() }
        let p = local(event)
        if handleDrag {
            // `SliderPoint` only starts dragging once the pointer has gone more than two points from the press.
            if let start = pressEvent {
                guard hypot(event.locationInWindow.x - start.locationInWindow.x, event.locationInWindow.y - start.locationInWindow.y) > 2 else { return }
                pressEvent = nil
            }
            let centre = p.x.rounded(.toNearestOrEven)
            if centre >= 0, centre <= bounds.width, bounds.width > 0 {
                let position = centre / bounds.width
                let hold = curveLevelFromHandle(position: position)
                handleHold = hold
                preview = hold.points
                ValueTip.shared.show(curveLevelTooltip(position: position), near: self, at: p)
                needsDisplay = true
            }
            return
        }
        if let gesture, gesture.captured() {
            if let moved = gesture.mouseMove(x: p.x, y: p.y, mods: KitMods(event.modifierFlags)) {
                preview = moved.points
                if let tip = moved.tooltip {
                    ValueTip.shared.show(tip, near: self, at: NSPoint(x: p.x, y: Self.stripHeight))
                }
                if KitMods(event.modifierFlags).shift {
                    levelBarVisible = !isLinked
                }
                needsDisplay = true
            }
            return
        }
        // Not captured, so past the system's distance this is a value drag.
        if let start = pressEvent, hypot(event.locationInWindow.x - start.locationInWindow.x, event.locationInWindow.y - start.locationInWindow.y) > 4 {
            pressEvent = nil
            gesture = nil
            startValueDrag(.curve(value), event: start)
        }
    }

    public override func mouseUp(with event: NSEvent) {
        defer { capturedCursor = nil; updateCursor(with: event) }
        ValueTip.shared.hide()
        pressEvent = nil
        if handleDrag {
            handleDrag = false
            let hold = handleHold
            handleHold = nil
            needsDisplay = true
            if let hold {
                commit(hold)
            }
            return
        }
        guard let gesture else { return }
        self.gesture = nil
        if let done = gesture.mouseUp(mods: KitMods(event.modifierFlags)) {
            commit(done)
        } else {
            preview = nil
            drawSize = .zero
            needsDisplay = true
        }
    }

    private func commit(_ done: CurveValue) {
        value = done
        preview = nil
        drawSize = .zero
        needsDisplay = true
        onCommit?(done)
    }

    public override func keyDown(with event: NSEvent) {
        if event.keyCode == 36 || event.keyCode == 76 {
            onOpenEditor?()
        } else {
            super.keyDown(with: event)
        }
    }
}

extension CurveStripNSView: EditorKeyTaking {
    /// Return opens the Curve Editor (F43).
    public func takesKey(_ event: NSEvent) -> Bool {
        isEnabled && EditorKeys.isReturn(event)
    }
}

/// `librarylink.png`'s place: the corner mark on a linked curve or gradient.
enum LibraryBadge {
    static func draw(at origin: NSPoint) {
        let rect = NSRect(origin: origin, size: NSSize(width: 18, height: 18))
        NSColor(white: 0.1, alpha: 0.85).setFill()
        NSBezierPath(roundedRect: rect, xRadius: 4, yRadius: 4).fill()
        guard let image = NSImage(systemSymbolName: "link", accessibilityDescription: "Linked to the library") else { return }
        let configured = image.withSymbolConfiguration(.init(pointSize: 11, weight: .semibold).applying(.init(hierarchicalColor: .white))) ?? image
        let size = configured.size
        configured.draw(
            in: NSRect(x: rect.midX - size.width / 2, y: rect.midY - size.height / 2, width: size.width, height: size.height),
            from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil
        )
    }
}

/// `SliderPoint`'s shape: a 10 by 10 pentagon, flat base, peak up, centred on x.
enum StripHandle {
    static func draw(centerX: CGFloat, top: CGFloat, focused: Bool) {
        let path = NSBezierPath()
        let left = centerX - 5
        path.move(to: NSPoint(x: left, y: top + 10))
        path.line(to: NSPoint(x: left, y: top + 5))
        path.line(to: NSPoint(x: left + 5, y: top))
        path.line(to: NSPoint(x: left + 10, y: top + 5))
        path.line(to: NSPoint(x: left + 10, y: top + 10))
        path.close()
        (focused ? NSColor(white: 0.92, alpha: 1) : NSColor(white: 0.72, alpha: 1)).setFill()
        path.fill()
        NSColor(white: 0.15, alpha: 1).setStroke()
        path.lineWidth = 1
        path.stroke()
    }

    static func hit(_ p: NSPoint, centerX: CGFloat, top: CGFloat) -> Bool {
        abs(p.x - centerX) <= 5 && p.y >= top && p.y <= top + 10
    }
}

/// A curve as a picture, for library lists: `GenerateGenericCurveImage`, black, with a grey line and no dots.
public enum CurveThumbnail {
    public static func image(points: [KitPoint], size: CGSize) -> NSImage {
        let drawn = curveStripPoints(points: points, width: size.width, height: size.height).map { CGPoint(x: $0.x, y: $0.y) }
        return NSImage(size: size, flipped: true) { rect in
            NSColor.black.setFill()
            rect.fill()
            guard drawn.count > 1 else { return true }
            let path = NSBezierPath()
            path.lineWidth = 2
            path.move(to: drawn[0])
            for p in drawn.dropFirst() {
                path.line(to: p)
            }
            NSColor(srgbRed: 136 / 255, green: 136 / 255, blue: 136 / 255, alpha: 1).setStroke()
            path.stroke()
            return true
        }
    }
}
