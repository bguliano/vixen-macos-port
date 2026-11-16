import AppKit
import SwiftUI
import VixenBridge

/// F24 and F25: a text field over a 7 pt wedge you can drag. Nothing commits while typing. Return and leaving the field commit, Escape puts the text back.
public struct DragNumberField: NSViewRepresentable {
    public var text: String
    public var limits: NumberLimits
    public var isEnabled: Bool
    /// The text to commit, and the drag's gesture number when it comes from the strip.
    public var onCommit: (String, UInt64?) -> Void

    public init(text: String, limits: NumberLimits, isEnabled: Bool = true, onCommit: @escaping (String, UInt64?) -> Void) {
        self.text = text
        self.limits = limits
        self.isEnabled = isEnabled
        self.onCommit = onCommit
    }

    public func makeNSView(context: Context) -> NumberFieldNSView {
        NumberFieldNSView(frame: .zero)
    }

    public func updateNSView(_ view: NumberFieldNSView, context: Context) {
        view.limits = limits
        view.onCommit = onCommit
        view.setEnabled(isEnabled)
        view.setText(text)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: NumberFieldNSView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 120, height: nsView.intrinsicContentSize.height)
    }
}

public final class NumberFieldNSView: NSView, NSTextFieldDelegate {
    private static let stripHeight: CGFloat = 7

    public var limits = NumberLimits(isDecimal: false, min: "0", max: "2147483647", step: 1, precision: 0)
    public var onCommit: ((String, UInt64?) -> Void)?
    private let field = NSTextField()
    private let strip = NumberDragStrip(frame: .zero)
    private var valueText = ""
    private var gesture: NumberStripGesture?
    private var gestureNumber: UInt64?
    private var timer: Timer?
    private var committedHold: String?

    public override init(frame: NSRect) {
        super.init(frame: frame)
        field.delegate = self
        field.isBordered = true
        field.isBezeled = true
        field.bezelStyle = .squareBezel
        field.controlSize = .small
        field.font = .monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        field.lineBreakMode = .byClipping
        field.cell?.isScrollable = true
        addSubview(field)
        addSubview(strip)
        strip.owner = self
    }

    public required init?(coder: NSCoder) {
        fatalError("not used")
    }

    public override var isFlipped: Bool { true }

    public override var intrinsicContentSize: NSSize {
        NSSize(width: NSView.noIntrinsicMetric, height: field.intrinsicContentSize.height + Self.stripHeight)
    }

    public override func layout() {
        super.layout()
        let height = field.intrinsicContentSize.height
        field.frame = NSRect(x: 0, y: 0, width: bounds.width, height: height)
        strip.frame = NSRect(x: 0, y: height, width: bounds.width, height: Self.stripHeight)
    }

    func setEnabled(_ enabled: Bool) {
        field.isEnabled = enabled
        strip.isEnabled = enabled
    }

    /// From the model: leaves alone a field being typed in, or a drag in progress.
    func setText(_ text: String) {
        valueText = text
        guard gesture == nil, !isEditing else { return }
        if field.stringValue != text {
            field.stringValue = text
        }
    }

    private var isEditing: Bool {
        field.currentEditor() != nil && window?.firstResponder === field.currentEditor()
    }

    private func commitTyped() {
        let typed = field.stringValue
        guard typed != valueText else { return }
        if let normalized = numberFieldNormalized(limits: limits, text: typed) {
            field.stringValue = normalized
            onCommit?(normalized, nil)
            readBack()
        } else {
            field.stringValue = valueText
        }
    }

    public func control(_ control: NSControl, textView: NSTextView, doCommandBy commandSelector: Selector) -> Bool {
        if commandSelector == #selector(NSResponder.insertNewline(_:)) {
            commitTyped()
            return true
        }
        if commandSelector == #selector(NSResponder.cancelOperation(_:)) {
            field.stringValue = valueText
            window?.makeFirstResponder(nil)
            return true
        }
        return false
    }

    public func controlTextDidEndEditing(_ obj: Notification) {
        commitTyped()
    }

    // The drag strip. Text shows every step; the value commits first after 300 ms, then every 500 ms, and finally on release.

    func stripPressed(at p: NSPoint) {
        window?.makeFirstResponder(self)
        field.stringValue = valueText
        gesture = NumberStripGesture.press(limits: limits, valueText: valueText, x: p.x, y: p.y)
        gestureNumber = KitGesture.next()
        committedHold = valueText
    }

    func stripDragged(to p: NSPoint, flags: NSEvent.ModifierFlags) {
        guard let gesture, let text = gesture.mouseMove(x: p.x, y: p.y, mods: KitMods(flags)) else { return }
        field.stringValue = text
        startTimerIfNeeded()
    }

    func stripReleased() {
        timer?.invalidate()
        timer = nil
        PendingCommits.clear(self)
        commitHold()
        gesture = nil
        gestureNumber = nil
        readBack()
    }

    /// What's stored can read back differently from what went in, and a value that didn't change brings no update, so look again.
    private func readBack() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
            guard let self, self.gesture == nil, !self.isEditing, self.field.stringValue != self.valueText else { return }
            self.field.stringValue = self.valueText
        }
    }

    private func startTimerIfNeeded() {
        PendingCommits.set(self) { [weak self] in self?.commitHold() }
        guard timer == nil else { return }
        timer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.commitHold()
                self.timer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
                    MainActor.assumeIsolated { self?.commitHold() }
                }
            }
        }
    }

    private func commitHold() {
        guard let gesture else { return }
        let hold = gesture.holdText()
        guard hold != committedHold else { return }
        committedHold = hold
        onCommit?(hold, gestureNumber)
    }
}

/// The wedge under a number field: brightens on hover and while dragging, and shows a move cursor.
final class NumberDragStrip: NSView {
    weak var owner: NumberFieldNSView?
    var isEnabled = true {
        didSet { needsDisplay = true }
    }
    private var hovering = false
    private var pressed = false
    private var tracking: NSTrackingArea?

    override var isFlipped: Bool { true }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        UITestHarness.isActive
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking {
            removeTrackingArea(tracking)
        }
        let area = NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .cursorUpdate, .activeInKeyWindow, .inVisibleRect], owner: self)
        addTrackingArea(area)
        tracking = area
    }

    override func cursorUpdate(with event: NSEvent) {
        if isEnabled {
            NSCursor.openHand.set()
        }
    }

    override func mouseEntered(with event: NSEvent) {
        hovering = true
        needsDisplay = true
    }

    override func mouseExited(with event: NSEvent) {
        hovering = false
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        let path = NSBezierPath()
        path.move(to: NSPoint(x: 0, y: bounds.height - 1))
        path.line(to: NSPoint(x: bounds.width, y: 1))
        path.line(to: NSPoint(x: bounds.width, y: bounds.height - 1))
        path.close()
        let alpha: CGFloat = !isEnabled ? 0.12 : (pressed ? 0.75 : (hovering ? 0.5 : 0.28))
        NSColor.white.withAlphaComponent(alpha).setFill()
        path.fill()
    }

    override func mouseDown(with event: NSEvent) {
        guard isEnabled else { return }
        pressed = true
        needsDisplay = true
        NSCursor.closedHand.set()
        owner?.stripPressed(at: convert(event.locationInWindow, from: nil))
    }

    override func mouseDragged(with event: NSEvent) {
        guard pressed else { return }
        owner?.stripDragged(to: convert(event.locationInWindow, from: nil), flags: event.modifierFlags)
    }

    override func mouseUp(with event: NSEvent) {
        guard pressed else { return }
        pressed = false
        needsDisplay = true
        owner?.stripReleased()
    }
}

/// F32: all four of upstream's sliders. Tracking is per frame, and doesn't go through SwiftUI state. The value commits 300 ms after the last change, also in the middle of a drag, and on release.
public struct ValueSlider: NSViewRepresentable {
    public var value: Double
    public var range: ClosedRange<Double>
    public var tick: Double
    public var snap: Bool
    public var tooltip: String?
    public var isEnabled: Bool
    /// The value on the slider, and the gesture number that makes all of one drag's commits one undo step.
    public var onCommit: (Double, UInt64) -> Void

    public init(value: Double, range: ClosedRange<Double>, tick: Double, snap: Bool, tooltip: String? = nil, isEnabled: Bool = true, onCommit: @escaping (Double, UInt64) -> Void) {
        self.value = value
        self.range = range
        self.tick = tick
        self.snap = snap
        self.tooltip = tooltip
        self.isEnabled = isEnabled
        self.onCommit = onCommit
    }

    public func makeNSView(context: Context) -> ValueSliderNSView {
        ValueSliderNSView(frame: .zero)
    }

    public func updateNSView(_ view: ValueSliderNSView, context: Context) {
        view.onCommit = onCommit
        view.isEnabled = isEnabled
        view.toolTip = tooltip
        view.configure(range: range, tick: tick, snap: snap)
        view.setValueFromModel(value)
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: ValueSliderNSView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 120, height: nsView.intrinsicContentSize.height)
    }
}

public final class ValueSliderNSView: NSSlider {
    var onCommit: ((Double, UInt64) -> Void)?
    private var tick = 1.0
    private var snap = false
    private var commitTimer: Timer?
    private var gesture: UInt64?
    private var lastSent: Double?
    private var modelValue: Double?
    private var tracking = false

    public override init(frame: NSRect) {
        super.init(frame: frame)
        isContinuous = true
        controlSize = .small
        target = self
        action = #selector(changed)
        (cell as? NSSliderCell)?.sliderType = .linear
    }

    public required init?(coder: NSCoder) {
        fatalError("not used")
    }

    public override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        UITestHarness.isActive || super.acceptsFirstMouse(for: event)
    }

    /// Upstream's slider takes the focus when pressed, so its keys work at once (F32). A Mac slider only does with Full Keyboard Access on.
    public override var acceptsFirstResponder: Bool { isEnabled }

    func configure(range: ClosedRange<Double>, tick: Double, snap: Bool) {
        if minValue != range.lowerBound { minValue = range.lowerBound }
        if maxValue != range.upperBound { maxValue = range.upperBound }
        self.tick = tick
        self.snap = snap
    }

    func setValueFromModel(_ value: Double) {
        modelValue = value
        guard !tracking, gesture == nil, doubleValue != value else { return }
        doubleValue = value
    }

    public override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        tracking = true
        if gesture == nil {
            gesture = KitGesture.next()
        }
        super.mouseDown(with: event)
        // `super` runs the tracking loop until the button comes up.
        tracking = false
        finishGesture()
    }

    @objc private func changed() {
        let snapped = sliderSnap(value: doubleValue, min: minValue, max: maxValue, tick: tick, snap: snap)
        if snapped != doubleValue {
            doubleValue = snapped
        }
        if gesture == nil {
            gesture = KitGesture.next()
        }
        if tracking {
            ValueTip.shared.show(sliderDragTip(value: snapped), near: self, at: NSPoint(x: knobX(snapped), y: 0))
        }
        commitTimer?.invalidate()
        PendingCommits.set(self) { [weak self] in
            self?.commitTimer?.invalidate()
            self?.sendValue()
        }
        commitTimer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self else { return }
                PendingCommits.clear(self)
                self.sendValue()
                if !self.tracking {
                    self.gesture = nil
                }
            }
        }
    }

    private func knobX(_ value: Double) -> CGFloat {
        guard maxValue > minValue else { return 0 }
        return bounds.width * (value - minValue) / (maxValue - minValue)
    }

    private func finishGesture() {
        ValueTip.shared.hide()
        commitTimer?.invalidate()
        commitTimer = nil
        PendingCommits.clear(self)
        sendValue()
        gesture = nil
        lastSent = nil
    }

    private func sendValue() {
        guard let gesture else { return }
        let value = doubleValue
        guard value != lastSent, value != modelValue else { return }
        lastSent = value
        onCommit?(value, gesture)
        // F33: what's stored may read back differently, so Gain's thumb at 37 jumps to 30. A value that didn't change brings no update, so look again.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
            guard let self, !self.tracking, self.gesture == nil, let model = self.modelValue, self.doubleValue != model else { return }
            self.doubleValue = model
        }
    }

    /// Arrows move by the tick, Page Up and Down by 5, Home and End go to the ends (F32).
    public override func keyDown(with event: NSEvent) {
        let target: Double?
        switch event.keyCode {
        case 123, 125: target = doubleValue - tick
        case 124, 126: target = doubleValue + tick
        case 116: target = doubleValue + 5
        case 121: target = doubleValue - 5
        case 115: target = minValue
        case 119: target = maxValue
        default: target = nil
        }
        guard let target, isEnabled else {
            super.keyDown(with: event)
            return
        }
        doubleValue = min(maxValue, max(minValue, target))
        changed()
    }
}

extension ValueSliderNSView: EditorKeyTaking {
    public func takesKey(_ event: NSEvent) -> Bool {
        isEnabled && EditorKeys.isNavigation(event)
    }
}
