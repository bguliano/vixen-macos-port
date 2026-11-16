import AppKit
import SwiftUI
import VixenBridge

/// F36's 25 by 25 swatch: double-click opens, and it's a drag source and a drop target. `nil` draws DimGray, upstream's null.
public struct ColorSwatch: NSViewRepresentable {
    @Environment(\.panelIsShowing) private var showing
    public var argb: UInt32?
    public var isEnabled: Bool
    /// F52's colour list takes Return too; a plain colour row doesn't.
    public var opensOnReturn: Bool
    public var onOpen: () -> Void
    public var acceptsDrop: ((VixenDragValue) -> Bool)?
    public var performDrop: ((VixenDragValue) -> Void)?
    public var onPressed: (() -> Void)?

    public init(
        argb: UInt32?,
        isEnabled: Bool = true,
        opensOnReturn: Bool = false,
        onOpen: @escaping () -> Void,
        acceptsDrop: ((VixenDragValue) -> Bool)? = nil,
        performDrop: ((VixenDragValue) -> Void)? = nil,
        onPressed: (() -> Void)? = nil
    ) {
        self.argb = argb
        self.isEnabled = isEnabled
        self.opensOnReturn = opensOnReturn
        self.onOpen = onOpen
        self.acceptsDrop = acceptsDrop
        self.performDrop = performDrop
        self.onPressed = onPressed
    }

    public func makeNSView(context: Context) -> ColorSwatchNSView {
        ColorSwatchNSView(frame: NSRect(x: 0, y: 0, width: 25, height: 25))
    }

    public func updateNSView(_ view: ColorSwatchNSView, context: Context) {
        view.setPanelVisible(showing)
        view.argb = argb
        view.isEnabled = isEnabled
        view.opensOnReturn = opensOnReturn
        view.onOpen = onOpen
        view.acceptsDrop = acceptsDrop
        view.performDrop = performDrop
        view.onPressed = onPressed
    }

    public func sizeThatFits(_ proposal: ProposedViewSize, nsView: ColorSwatchNSView, context: Context) -> CGSize? {
        CGSize(width: 25, height: 25)
    }
}

public final class ColorSwatchNSView: ValueDragView {
    public var argb: UInt32? {
        didSet {
            if oldValue != argb {
                needsDisplay = true
                if let argb {
                    setAccessibilityLabel("Color \(kitColorText(argb: argb))")
                }
            }
        }
    }
    public var isEnabled = true
    public var opensOnReturn = false

    /// F67: a colour; a null one drags nothing.
    public override var dragValue: VixenDragValue? { argb.map { .color($0) } }
    public var onOpen: (() -> Void)?
    private var pressEvent: NSEvent?

    public override var isFlipped: Bool { true }
    public override var acceptsFirstResponder: Bool { isEnabled }
    public override var intrinsicContentSize: NSSize { NSSize(width: 25, height: 25) }

    public override func draw(_ dirtyRect: NSRect) {
        let rect = bounds.insetBy(dx: 0.5, dy: 0.5)
        // A transparent colour shows the row behind it (F56), and `nil` is upstream's null, DimGray.
        (argb.map { NSColor(argb: $0) } ?? NSColor(srgbRed: 105 / 255, green: 105 / 255, blue: 105 / 255, alpha: 1)).setFill()
        rect.fill(using: .sourceOver)
        NSColor.gray.setStroke()
        NSBezierPath.defaultLineWidth = 1
        NSBezierPath(rect: rect).stroke()
    }

    public override func mouseDown(with event: NSEvent) {
        guard isEnabled else { return }
        window?.makeFirstResponder(self)
        onPressed?()
        if event.clickCount == 2 {
            pressEvent = nil
            onOpen?()
        } else {
            pressEvent = event
        }
    }

    public override func mouseDragged(with event: NSEvent) {
        guard let start = pressEvent, let argb else { return }
        if hypot(event.locationInWindow.x - start.locationInWindow.x, event.locationInWindow.y - start.locationInWindow.y) > 4 {
            pressEvent = nil
            startValueDrag(.color(argb), event: start)
        }
    }

    public override func mouseUp(with event: NSEvent) {
        pressEvent = nil
    }

    public override func keyDown(with event: NSEvent) {
        if opensOnReturn, event.keyCode == 36 || event.keyCode == 76 {
            onOpen?()
        } else {
            super.keyDown(with: event)
        }
    }
}

extension ColorSwatchNSView: EditorKeyTaking {
    /// A colour list's swatch opens on Return; a colour row's doesn't (F52).
    public func takesKey(_ event: NSEvent) -> Bool {
        isEnabled && opensOnReturn && EditorKeys.isReturn(event)
    }
}

@MainActor
private final class SessionColorWell: NSColorWell {
    var onDeactivate: (() -> Void)?
    override func deactivate() {
        let wasActive = isActive
        super.deactivate()
        if wasActive { onDeactivate?() }
    }
}

/// The system colour panel for one edit at a time (F36, F103). The panel isn't modal: the commit comes when it closes, or when another edit takes it over, so one edit is one undo step.
/// Quick Picks stay, as a small accessory, because the owner will miss them. Alpha is off, since upstream's result is always opaque.
@MainActor
public final class ColorPanelSession: NSObject {
    public static let shared = ColorPanelSession()

    private var onFinish: ((UInt32) -> Void)?
    private var onLive: ((UInt32) -> Void)?
    private var onEnd: (() -> Void)?
    private var notifyUnchanged = false
    private var startColor: UInt32 = 0
    private var current: UInt32 = 0
    private var observing = false
    // Native and SwiftUI color wells participate in AppKit's exclusive-owner
    // protocol. Merely replacing NSColorPanel.target leaves those wells active.
    private lazy var ownershipWell: SessionColorWell = {
        let well = SessionColorWell()
        well.target = nil; well.action = nil
        well.onDeactivate = { [weak self] in self?.finish() }
        return well
    }()

    /// `onEnd` clears temporary previews even after a change is reverted. Creating
    /// a new colour opts into `notifyUnchanged`; ordinary edits still skip no-ops.
    public func begin(title: String, argb: UInt32, onLive: ((UInt32) -> Void)? = nil, notifyUnchanged: Bool = false, onEnd: (() -> Void)? = nil, onFinish: @escaping (UInt32) -> Void) {
        finish()
        self.onFinish = onFinish
        self.onLive = onLive
        self.onEnd = onEnd
        self.notifyUnchanged = notifyUnchanged
        startColor = argb | 0xFF00_0000
        current = startColor
        let panel = NSColorPanel.shared
        let testPresentation = UITestHarness.isActive
        let previousAlpha = panel.alphaValue
        if testPresentation {
            // AppKit may constrain a requested offscreen frame when activating
            // a well. Keep test presentation transparent until it is hidden.
            panel.alphaValue = 0
            panel.setFrameAutosaveName("")
            panel.setFrameOrigin(NSPoint(x: -12000, y: -12000))
        }
        defer {
            if testPresentation {
                panel.orderOut(nil)
                panel.setFrameOrigin(NSPoint(x: -12000, y: -12000))
                panel.alphaValue = previousAlpha
            }
        }
        ownershipWell.color = NSColor(argb: startColor)
        ownershipWell.activate(true)
        panel.showsAlpha = false
        panel.isContinuous = true
        panel.title = title
        panel.setTarget(self)
        panel.setAction(#selector(colorChanged(_:)))
        panel.color = NSColor(argb: startColor)
        // The flush rule: an edit in the panel belongs to the selection it was started on.
        PendingCommits.set(self) { [weak self] in self?.finish() }
        if panel.accessoryView == nil {
            panel.accessoryView = NSHostingView(rootView: QuickPicks { [weak self] argb in self?.setQuickPick(argb) })
            panel.accessoryView?.frame = NSRect(x: 0, y: 0, width: 240, height: 54)
        }
        if !observing {
            observing = true
            NotificationCenter.default.addObserver(self, selector: #selector(panelClosing), name: NSWindow.willCloseNotification, object: panel)
        }
        if !testPresentation { panel.orderFront(nil) }
    }

    /// Ends the current edit, committing it.
    public func finish() {
        guard let onFinish else { return }
        let onEnd = self.onEnd
        let notify = notifyUnchanged || current != startColor
        let finishedColor = current
        self.onFinish = nil
        onLive = nil
        self.onEnd = nil
        notifyUnchanged = false
        PendingCommits.clear(self)
        ownershipWell.deactivate()
        NSColorPanel.shared.setTarget(nil)
        onEnd?()
        if notify {
            onFinish(finishedColor)
        }
    }

    /// For the test harness, which can't reach the system panel.
    public func testSetColor(_ argb: UInt32) {
        current = argb | 0xFF00_0000
        onLive?(current)
    }

    private func setQuickPick(_ argb: UInt32) {
        NSColorPanel.shared.color = NSColor(argb: argb)
        current = argb
        onLive?(argb)
    }

    @objc private func colorChanged(_ sender: NSColorPanel) {
        current = sender.color.argb | 0xFF00_0000
        onLive?(current)
    }

    @objc private func panelClosing(_ note: Notification) {
        finish()
    }
}

/// F103's Quick Picks: each sets the colour at once and doesn't close.
private struct QuickPicks: View {
    let onPick: (UInt32) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Quick Picks").font(.caption).foregroundStyle(.secondary)
            HStack(spacing: 6) {
                ForEach([UInt32(0xFFFF_FFFF), 0xFFFF_0000, 0xFF00_FF00, 0xFF00_00FF], id: \.self) { argb in
                    Button {
                        onPick(argb)
                    } label: {
                        RoundedRectangle(cornerRadius: 3).fill(Color(argb: argb)).frame(width: 28, height: 20)
                            .overlay(RoundedRectangle(cornerRadius: 3).stroke(.secondary.opacity(0.5)))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(8)
    }
}

/// F100: only the colours an element can show, one to choose. `current` starts selected if it's in the set. **Ok** is off with nothing selected.
struct SingleDiscreteColorPicker: View {
    let colors: [UInt32]
    let onFinish: (UInt32?) -> Void
    @State private var selected: UInt32?

    init(colors: [UInt32], current: UInt32?, onFinish: @escaping (UInt32?) -> Void) {
        self.colors = colors
        self.onFinish = onFinish
        _selected = State(initialValue: current.flatMap { colors.contains($0) ? $0 : nil })
    }

    private var pickerSize: [UInt32] { discretePickerSize(count: UInt32(colors.count), itemWidth: 72, threshold: 4) }

    var body: some View {
        VStack(spacing: 12) {
            Text("Select Color").font(.headline).frame(maxWidth: .infinity, alignment: .leading)
            ScrollView(.vertical) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 62, maximum: 72))], spacing: 10) {
                    ForEach(colors, id: \.self) { color in
                        Button {
                            selected = color
                        } label: {
                            RoundedRectangle(cornerRadius: 4)
                                .fill(Color(argb: color))
                                .frame(width: 50, height: 50)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 4)
                                        .stroke(selected == color ? Color.accentColor : Color.secondary.opacity(0.5), lineWidth: selected == color ? 3 : 1)
                                )
                                .padding(3)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(2)
            }
            HStack {
                Spacer()
                Button("Cancel") { onFinish(nil) }
                    .keyboardShortcut(.cancelAction)
                Button("Ok") { onFinish(selected) }
                    .keyboardShortcut(.defaultAction)
                    .disabled(selected == nil)
            }
        }
        .padding(16)
        .frame(width: CGFloat(pickerSize[0]) + 20, height: CGFloat(pickerSize[1]) + 40)
    }
}

/// F99: tick any of the colours. OK is always enabled; none ticked means the caller's "delete".
struct MultipleDiscreteColorPicker: View {
    let colors: [UInt32]
    let onFinish: ([UInt32]?) -> Void
    @State private var ticked: Set<UInt32>

    init(colors: [UInt32], selected: [UInt32], onFinish: @escaping ([UInt32]?) -> Void) {
        self.colors = colors
        self.onFinish = onFinish
        _ticked = State(initialValue: Set(selected))
    }

    private var pickerSize: [UInt32] { discretePickerSize(count: UInt32(colors.count), itemWidth: 92, threshold: 3) }

    var body: some View {
        VStack(spacing: 12) {
            Text("Select Color(s)").font(.headline).frame(maxWidth: .infinity, alignment: .leading)
            ScrollView(.vertical) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 84, maximum: 92))], spacing: 10) {
                    ForEach(colors, id: \.self) { color in
                        HStack(spacing: 4) {
                            Toggle(
                                "",
                                isOn: Binding(
                                    get: { ticked.contains(color) },
                                    set: { on in
                                        if on { ticked.insert(color) } else { ticked.remove(color) }
                                    }
                                )
                            )
                            .toggleStyle(.checkbox)
                            .labelsHidden()
                            RoundedRectangle(cornerRadius: 4)
                                .fill(Color(argb: color))
                                .frame(width: 50, height: 50)
                                .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.secondary.opacity(0.5)))
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if ticked.contains(color) { ticked.remove(color) } else { ticked.insert(color) }
                        }
                    }
                }
                .padding(2)
            }
            HStack {
                Spacer()
                Button("Cancel") { onFinish(nil) }
                    .keyboardShortcut(.cancelAction)
                Button("OK") { onFinish(colors.filter(ticked.contains)) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: CGFloat(pickerSize[0]) + 20, height: CGFloat(pickerSize[1]) + 40)
    }
}

/// The colour pickers restricted to discrete colours, as sheets on a window.
@MainActor
public enum DiscreteColorPicker {
    /// F100. `onChosen` isn't called on Cancel.
    public static func pickOne(on window: NSWindow, colors: [UInt32], current: UInt32?, onChosen: @escaping (UInt32) -> Void) {
        KitSheet.present(on: window) { dismiss in
            SingleDiscreteColorPicker(colors: colors, current: current) { chosen in
                dismiss()
                if let chosen {
                    onChosen(chosen)
                }
            }
        }
    }

    /// F99. `onChosen` isn't called on Cancel; an empty array means OK with none ticked.
    public static func pickSeveral(on window: NSWindow, colors: [UInt32], selected: [UInt32], onChosen: @escaping ([UInt32]) -> Void) {
        KitSheet.present(on: window) { dismiss in
            MultipleDiscreteColorPicker(colors: colors, selected: selected) { chosen in
                dismiss()
                if let chosen {
                    onChosen(chosen)
                }
            }
        }
    }
}

/// F48 and F90's colour edit on a gradient stop, whichever way it goes.
@MainActor
public enum GradientStopColorEdit {
    /// The handle's colour edit on a strip. `onCommit` gets the changed gradient.
    public static func edit(
        handle: GradientHandleInfo,
        value: GradientValue,
        discrete: Bool,
        validColors: [UInt32],
        window: NSWindow?,
        onCommit: @escaping (GradientValue) -> Void
    ) {
        guard let window else { return }
        if discrete {
            let current = handle.stops.compactMap { Int($0) < value.stops.count ? value.stops[Int($0)].argb : nil }
            DiscreteColorPicker.pickSeveral(on: window, colors: validColors, selected: current) { chosen in
                onCommit(gradientSetHandleColors(value: value, stops: handle.stops, colors: chosen))
            }
            return
        }
        guard let first = handle.stops.first, Int(first) < value.stops.count else { return }
        Task { @MainActor in
            if handle.stops.count > 1 {
                _ = await askInSheet(
                    on: window, message: "Non-discrete color gradient, >1 selected point. oops! please report it.", title: "Delete library gradient?", buttons: ["OK"], style: .critical
                )
            }
            ColorPanelSession.shared.begin(title: "Color Picker", argb: value.stops[Int(first)].argb) { argb in
                onCommit(gradientSetStopColor(value: value, stop: first, argb: argb))
            }
        }
    }
}
