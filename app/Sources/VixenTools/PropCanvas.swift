import AppKit
import SwiftUI
import VixenBridge
import VixenKit

struct PropCanvas: NSViewRepresentable {
    let model: PropEditorModel
    func makeNSView(context: Context) -> NSScrollView {
        let canvas = PropCanvasView(model: model)
        let scroll = NSScrollView(); scroll.backgroundColor = .black; scroll.drawsBackground = true
        scroll.documentView = canvas; scroll.hasHorizontalScroller = true; scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        model.canvasRefresh = { [weak canvas] in canvas?.refresh() }
        return scroll
    }
    func updateNSView(_ scroll: NSScrollView, context: Context) { (scroll.documentView as? PropCanvasView)?.refresh() }
}

final class PropCanvasView: NSView {
    let model: PropEditorModel
    private var dragRevision: UInt64 = 0
    private var tracking: NSTrackingArea?
    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    init(model: PropEditorModel) {
        self.model = model
        super.init(frame: NSRect(x: 0, y: 0, width: model.state.canvasWidth, height: model.state.canvasHeight))
        setAccessibilityLabel("Custom prop canvas")
    }
    required init?(coder: NSCoder) { fatalError("not used") }
    func refresh() {
        let size = NSSize(width: model.state.canvasWidth, height: model.state.canvasHeight)
        if frame.size != size { setFrameSize(size) }
        needsDisplay = true
    }
    override func draw(_ dirtyRect: NSRect) {
        NSColor.black.setFill(); dirtyRect.fill()
        model.image?.draw(in: bounds, from: .zero, operation: .sourceOver, fraction: model.state.opacity, respectFlipped: true, hints: nil)
        guard let context = NSGraphicsContext.current?.cgContext else { return }
        // Reuse fill state and draw directly, preserving per-circle alpha and
        // overlap order without allocating an AppKit path for every light.
        let normal = model.state.lightColor, selected = model.state.selectedLightColor
        var color: UInt32?
        for light in model.canvas.lights {
            let r = CGFloat(light.size), rect = NSRect(x: CGFloat(light.x) - r, y: CGFloat(light.y) - r, width: r * 2, height: r * 2)
            guard rect.intersects(dirtyRect) else { continue }
            let argb = light.stateColor ?? (light.selected ? selected : normal)
            if color != argb { color = argb; context.setFillColor(NSColor(argb: argb).cgColor) }
            context.fillEllipse(in: rect)
        }
        if let band = model.canvas.band {
            let rect = NSRect(x: band.x, y: band.y, width: band.width, height: band.height)
            NSColor.selectedControlColor.withAlphaComponent(0.2).setFill(); rect.fill()
            NSColor.selectedControlColor.setStroke(); NSBezierPath(rect: rect).stroke()
        }
        if let box = model.canvas.adorner {
            NSGraphicsContext.saveGraphicsState()
            let transform = NSAffineTransform()
            transform.translateX(by: box.centerX, yBy: box.centerY)
            transform.rotate(byDegrees: box.angle)
            transform.translateX(by: -box.centerX, yBy: -box.centerY)
            transform.concat()
            NSColor.white.setStroke()
            NSBezierPath(rect: NSRect(x: box.x, y: box.y, width: box.width, height: box.height)).stroke()
            for p in box.handles {
                let r = NSRect(x: p.x, y: p.y, width: box.handleSize, height: box.handleSize)
                NSColor.white.setFill(); NSBezierPath(rect: r).fill(); NSColor.black.setStroke(); NSBezierPath(rect: r).stroke()
            }
            NSGraphicsContext.restoreGraphicsState()
        }
    }
    override func updateTrackingAreas() {
        if let tracking { removeTrackingArea(tracking) }
        let next = NSTrackingArea(rect: .zero, options: [.activeInKeyWindow, .mouseMoved, .mouseEnteredAndExited, .inVisibleRect], owner: self)
        addTrackingArea(next); tracking = next
        super.updateTrackingAreas()
    }
    private func point(_ event: NSEvent) -> NSPoint { convert(event.locationInWindow, from: nil) }
    override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self); dragRevision = model.state.revision
        let p = point(event)
        model.editor.pointerDown(x: p.x, y: p.y, command: event.modifierFlags.contains(.command), shift: event.modifierFlags.contains(.shift)); model.refresh()
    }
    override func mouseDragged(with event: NSEvent) {
        let p = point(event)
        model.editor.pointerDragged(x: p.x, y: p.y, command: event.modifierFlags.contains(.command), shift: event.modifierFlags.contains(.shift)); model.refresh(full: false)
    }
    override func mouseUp(with event: NSEvent) {
        model.editor.pointerUp(); model.refresh(); model.registerChange(after: dragRevision)
    }
    override func mouseMoved(with event: NSEvent) {
        let p = point(event); model.editor.pointerMoved(x: p.x, y: p.y)
        model.state = model.editor.state()
        switch model.editor.handleAt(x: p.x, y: p.y) {
        case .left, .right: NSCursor.resizeLeftRight.set()
        case .top, .bottom: NSCursor.resizeUpDown.set()
        case .center, .rotate: NSCursor.openHand.set()
        case .topLeft, .topRight, .bottomLeft, .bottomRight: NSCursor.crosshair.set()
        case nil: (model.state.mode == .draw ? NSCursor.crosshair : NSCursor.arrow).set()
        }
    }
    override func mouseExited(with event: NSEvent) { model.editor.pointerExited(); model.state = model.editor.state(); NSCursor.arrow.set() }
    override func keyDown(with event: NSEvent) {
        if event.keyCode == 51 || event.keyCode == 117 { model.deleteLights() } else { super.keyDown(with: event) }
    }
}
