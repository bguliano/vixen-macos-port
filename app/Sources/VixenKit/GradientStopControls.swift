import AppKit
import SwiftUI
import VixenBridge

/// F89's colour box, `ColorLabel`: the current colour above the previous one, each with its `RRGGBB` text. Disabled, only the border shows, as upstream paints it. A click opens the stop's colour
/// edit; the colour panel's eyedropper does what dragging out of the box to sample the screen did.
public struct ColorLabelBox: NSViewRepresentable {
    public var color: UInt32
    public var previous: UInt32
    public var isEnabled: Bool
    public var onClick: () -> Void

    public init(color: UInt32, previous: UInt32, isEnabled: Bool, onClick: @escaping () -> Void) {
        self.color = color
        self.previous = previous
        self.isEnabled = isEnabled
        self.onClick = onClick
    }

    public func makeNSView(context: Context) -> ColorLabelNSView {
        ColorLabelNSView(frame: NSRect(x: 0, y: 0, width: 75, height: 29))
    }

    public func updateNSView(_ view: ColorLabelNSView, context: Context) {
        view.onClick = onClick
        view.update(color: color, previous: previous, enabled: isEnabled)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: ColorLabelNSView, context: Context) -> CGSize? {
        CGSize(width: 75, height: 29)
    }
}

public final class ColorLabelNSView: NSView {
    var onClick: (() -> Void)?
    private var color: UInt32 = 0xFF00_0000
    private var previous: UInt32 = 0xFF00_0000
    private var enabled = true

    public override var isFlipped: Bool { true }
    public override var intrinsicContentSize: NSSize { NSSize(width: 75, height: 29) }

    public override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        UITestHarness.isActive
    }

    func update(color: UInt32, previous: UInt32, enabled: Bool) {
        let color = color | 0xFF00_0000
        let previous = previous | 0xFF00_0000
        guard color != self.color || previous != self.previous || enabled != self.enabled else { return }
        self.color = color
        self.previous = previous
        self.enabled = enabled
        setAccessibilityLabel(enabled ? "Color \(Self.hex(color))" : "Color")
        needsDisplay = true
    }

    /// `ColorToHexString`.
    static func hex(_ argb: UInt32) -> String {
        String(format: "%06X", argb & 0xFF_FFFF)
    }

    /// `ColorUtility.MaxContrastTo`.
    static func contrast(_ argb: UInt32) -> NSColor {
        let sum = Int((argb >> 16) & 0xFF) + Int((argb >> 8) & 0xFF) + Int(argb & 0xFF)
        return sum > 381 ? .black : .white
    }

    public override func draw(_ dirtyRect: NSRect) {
        if enabled {
            let half = floor(bounds.height / 2)
            let top = NSRect(x: 0, y: 0, width: bounds.width, height: half)
            let bottom = NSRect(x: 0, y: half, width: bounds.width, height: bounds.height - half)
            for (rect, argb) in [(top, color), (bottom, previous)] {
                NSColor(argb: argb).setFill()
                rect.fill()
                let text = NSAttributedString(
                    string: Self.hex(argb),
                    attributes: [.font: NSFont.monospacedDigitSystemFont(ofSize: 9, weight: .regular), .foregroundColor: Self.contrast(argb)]
                )
                let size = text.size()
                text.draw(at: NSPoint(x: rect.midX - size.width / 2, y: rect.midY - size.height / 2))
            }
        }
        NSColor.gray.setStroke()
        NSBezierPath.defaultLineWidth = 1
        NSBezierPath(rect: bounds.insetBy(dx: 0.5, dy: 0.5)).stroke()
    }

    public override func mouseDown(with event: NSEvent) {
        guard enabled else { return }
        onClick?()
    }
}

/// F89's Location, `ValueUpDown`: a whole number in a range, a field that applies on Return or leaving it, spin buttons that step by 1 and repeat while held, and a third button that drops a
/// small vertical tracker. Each value applies as it changes. Setting the value from outside raises nothing, so selecting a stop doesn't write back.
public struct LocationUpDown: NSViewRepresentable {
    public var value: Int
    public var range: ClosedRange<Int>
    public var isEnabled: Bool
    public var onChange: (Int) -> Void

    public init(value: Int, range: ClosedRange<Int> = 0...100, isEnabled: Bool, onChange: @escaping (Int) -> Void) {
        self.value = value
        self.range = range
        self.isEnabled = isEnabled
        self.onChange = onChange
    }

    public func makeNSView(context: Context) -> LocationUpDownNSView {
        LocationUpDownNSView(frame: NSRect(x: 0, y: 0, width: 96, height: 22))
    }

    public func updateNSView(_ view: LocationUpDownNSView, context: Context) {
        view.onChange = onChange
        view.configure(range: range, enabled: isEnabled)
        view.setValueFromModel(value)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: LocationUpDownNSView, context: Context) -> CGSize? {
        nsView.intrinsicContentSize
    }
}

public final class LocationUpDownNSView: NSView, NSTextFieldDelegate {
    var onChange: ((Int) -> Void)?
    private let field = NSTextField()
    private let stepper = NSStepper()
    private let dropButton = NSButton()
    private var range = 0...100
    private var value = 0
    private var popover: NSPopover?

    public override init(frame: NSRect) {
        super.init(frame: frame)
        field.delegate = self
        field.controlSize = .small
        field.font = .monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        field.alignment = .right
        field.stringValue = "0"
        field.setAccessibilityLabel("Location")
        stepper.controlSize = .small
        stepper.increment = 1
        stepper.autorepeat = true
        stepper.valueWraps = false
        stepper.target = self
        stepper.action = #selector(stepped)
        dropButton.bezelStyle = .smallSquare
        dropButton.isBordered = false
        dropButton.image = NSImage(systemSymbolName: "chevron.down", accessibilityDescription: "Show the location tracker")
        dropButton.imageScaling = .scaleProportionallyDown
        dropButton.target = self
        dropButton.action = #selector(showTracker)
        dropButton.setAccessibilityLabel("Location tracker")
        for view in [field, stepper, dropButton] {
            addSubview(view)
        }
    }

    public required init?(coder: NSCoder) {
        fatalError("not used")
    }

    public override var intrinsicContentSize: NSSize {
        NSSize(width: 96, height: max(22, field.intrinsicContentSize.height))
    }

    public override func layout() {
        super.layout()
        let height = field.intrinsicContentSize.height
        let y = (bounds.height - height) / 2
        field.frame = NSRect(x: 0, y: y, width: bounds.width - 38, height: height)
        let stepperSize = stepper.intrinsicContentSize
        stepper.frame = NSRect(x: bounds.width - 38, y: (bounds.height - stepperSize.height) / 2, width: stepperSize.width, height: stepperSize.height)
        dropButton.frame = NSRect(x: bounds.width - 18, y: (bounds.height - 18) / 2, width: 18, height: 18)
    }

    func configure(range: ClosedRange<Int>, enabled: Bool) {
        self.range = range
        stepper.minValue = Double(range.lowerBound)
        stepper.maxValue = Double(range.upperBound)
        field.isEnabled = enabled
        stepper.isEnabled = enabled
        dropButton.isEnabled = enabled
        if !enabled {
            popover?.close()
        }
    }

    /// From the model: raises nothing, and leaves a field being typed in alone.
    func setValueFromModel(_ newValue: Int) {
        value = newValue
        stepper.integerValue = newValue
        if field.currentEditor() == nil, field.stringValue != String(newValue) {
            field.stringValue = String(newValue)
        }
        (popover?.contentViewController?.view as? NSSlider)?.integerValue = newValue
    }

    /// `SetValueRaise`: clamped, and raises only when it changed.
    private func apply(_ wanted: Int) {
        let clamped = min(range.upperBound, max(range.lowerBound, wanted))
        field.stringValue = String(clamped)
        stepper.integerValue = clamped
        guard clamped != value else { return }
        value = clamped
        onChange?(clamped)
    }

    @objc private func stepped() {
        apply(stepper.integerValue)
    }

    /// Digits and control keys only.
    public func controlTextDidChange(_ obj: Notification) {
        let digits = field.stringValue.filter(\.isASCII).filter(\.isNumber)
        if digits != field.stringValue {
            field.stringValue = digits
        }
    }

    /// `OnTextBoxReturn` and leaving the field: unparseable text goes back to the value.
    public func controlTextDidEndEditing(_ obj: Notification) {
        if let typed = Int(field.stringValue) {
            apply(typed)
        } else {
            field.stringValue = String(value)
        }
    }

    @objc private func showTracker() {
        let slider = NSSlider(value: Double(value), minValue: Double(range.lowerBound), maxValue: Double(range.upperBound), target: self, action: #selector(tracked(_:)))
        slider.isVertical = true
        slider.isContinuous = true
        slider.frame = NSRect(x: 0, y: 0, width: 28, height: 120)
        let controller = NSViewController()
        controller.view = slider
        let popover = NSPopover()
        popover.contentViewController = controller
        popover.behavior = .transient
        popover.contentSize = NSSize(width: 28, height: 120)
        popover.show(relativeTo: dropButton.bounds, of: dropButton, preferredEdge: .maxY)
        self.popover = popover
    }

    @objc private func tracked(_ sender: NSSlider) {
        apply(Int(sender.doubleValue.rounded()))
    }
}
