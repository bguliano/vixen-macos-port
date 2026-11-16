import AppKit
import SwiftUI

/// The two buttons at the bottom of every sheet. Return is OK and Esc is Cancel.
public struct SheetButtons: View {
    var okTitle = "OK"
    var okDisabled = false
    let ok: () -> Void
    let cancel: () -> Void
    public init(okTitle: String = "OK", okDisabled: Bool = false, ok: @escaping () -> Void, cancel: @escaping () -> Void) { self.okTitle = okTitle; self.okDisabled = okDisabled; self.ok = ok; self.cancel = cancel }

    public var body: some View {
        HStack(spacing: 8) {
            Spacer(minLength: 0)
            Button(role: .cancel, action: cancel) {
                Text("Cancel").frame(minWidth: 58)
            }
            .keyboardShortcut(.cancelAction)
            Button(action: ok) {
                Text(okTitle).frame(minWidth: 58)
            }
            .keyboardShortcut(.defaultAction)
            .disabled(okDisabled)
        }
    }
}

/// `NumericUpDown`: a field and a stepper, kept within its range. What's typed reaches the value at once, clamped, so OK always sees it. The text itself only
/// snaps to the clamped value when editing ends, or typing "16" into a field whose minimum is 8 would be fought at the "1".
public struct SheetNumberField: View {
    private let label: String
    @Binding private var value: Int
    private let range: ClosedRange<Int>
    private let width: CGFloat
    @State private var text: String
    @FocusState private var focused: Bool

    public init(_ label: String, value: Binding<Int>, range: ClosedRange<Int>, width: CGFloat = 56) {
        self.label = label
        _value = value
        self.range = range
        self.width = width
        _text = State(initialValue: String(value.wrappedValue))
    }

    public var body: some View {
        HStack(spacing: 2) {
            TextField(label, text: $text, prompt: Text(""))
                .labelsHidden()
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .frame(width: width)
                .focused($focused)
                .onSubmit(settle)
            Stepper(label, value: $value, in: range)
                .labelsHidden()
        }
        .onChange(of: text) {
            let cleaned = text.filter { ($0.isASCII && $0.isNumber) || ($0 == "-" && range.lowerBound < 0) }
            if cleaned != text {
                text = cleaned
            } else if let typed = Int(text), typed.clamped(to: range) != value {
                value = typed.clamped(to: range)
            }
        }
        .onChange(of: value) {
            if Int(text)?.clamped(to: range) != value {
                text = String(value)
            }
        }
        .onChange(of: focused) {
            if !focused { settle() }
        }
    }

    private func settle() {
        text = String(value)
    }
}

/// A text field with no placeholder. The label is for VoiceOver: the visible one sits beside it.
public struct SheetTextField: View {
    private let label: String
    @Binding private var text: String

    public init(_ label: String, text: Binding<String>) {
        self.label = label
        _text = text
    }

    public var body: some View {
        TextField(label, text: $text, prompt: Text(""))
            .labelsHidden()
    }
}

/// A radio button of its own, for groups a radio `Picker` can't lay out, such as Pixel Grid's 2 by 2 corners.
public struct SheetRadioButton: NSViewRepresentable {
    let title: String
    let isOn: Bool
    let choose: () -> Void
    public init(title: String, isOn: Bool, choose: @escaping () -> Void) { self.title = title; self.isOn = isOn; self.choose = choose }

    public func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    public func makeNSView(context: Context) -> NSButton {
        let button = NSButton(radioButtonWithTitle: title, target: context.coordinator, action: #selector(Coordinator.clicked))
        button.setContentHuggingPriority(.required, for: .horizontal)
        return button
    }

    public func updateNSView(_ button: NSButton, context: Context) {
        context.coordinator.choose = choose
        button.title = title
        button.state = isOn ? .on : .off
        button.isEnabled = context.environment.isEnabled
    }

    public final class Coordinator: NSObject {
        var choose: () -> Void = {}

        @objc func clicked() {
            choose()
        }
    }
}

/// A field's label, which never wraps or truncates, dimmed along with a field that's disabled.
public struct SheetFieldLabel: View {
    private let text: String
    private let enabled: Bool

    public init(_ text: String, enabled: Bool = true) {
        self.text = text
        self.enabled = enabled
    }

    public var body: some View {
        Text(text)
            .fixedSize()
            .foregroundStyle(enabled ? Color.primary : Color(nsColor: .disabledControlTextColor))
    }
}

/// The small icon buttons beside the naming dialogs' lists. A fixed size, since `plus` and `minus` are different heights.
public struct SheetIconButton: View {
    let systemName: String
    let label: String
    let action: () -> Void
    public init(systemName: String, label: String, action: @escaping () -> Void) { self.systemName = systemName; self.label = label; self.action = action }

    public var body: some View {
        Button(action: action) {
            Image(systemName: systemName)
                .frame(width: 16, height: 16)
        }
        .accessibilityLabel(label)
    }
}

/// SwiftUI leaves a sheet's window fixed, however flexible its content's frame.
private struct ResizableWindowMarker: NSViewRepresentable {
    let minSize: NSSize

    final class Marker: NSView {
        var minSize = NSSize.zero

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            window?.styleMask.insert(.resizable)
        }

        // SwiftUI puts its own minimum back on the window after that, so this has to be later.
        override func layout() {
            super.layout()
            if let window, window.contentMinSize != minSize {
                window.contentMinSize = minSize
            }
        }
    }

    func makeNSView(context: Context) -> Marker {
        let marker = Marker()
        marker.minSize = minSize
        return marker
    }

    func updateNSView(_ marker: Marker, context: Context) {}
}

extension View {
    /// The big naming sheets: flexible content, starting at and going no smaller than these sizes.
    public func sheetResizable(width: CGFloat, height: CGFloat) -> some View {
        frame(minWidth: width, idealWidth: width, maxWidth: .infinity, minHeight: height, idealHeight: height, maxHeight: .infinity)
            .background(ResizableWindowMarker(minSize: NSSize(width: width, height: height)))
    }

    /// Return and Esc for a whole sheet. A key can reach a focused text field before it reaches the buttons' shortcuts, and Esc would then make SwiftUI dismiss the
    /// sheet without Cancel's action ever running, so these catch that route too. The two routes can overlap, so `ok` and `cancel` must be safe to call twice.
    public func sheetKeys(okEnabled: Bool = true, ok: @escaping () -> Void, cancel: @escaping () -> Void) -> some View {
        interactiveDismissDisabled()
            .onSubmit { if okEnabled { ok() } }
            .onExitCommand(perform: cancel)
    }
}

extension Binding where Value: BinaryInteger {
    /// For `SheetNumberField`, whose range keeps the value within what the record's integer type holds.
    public var sheetInt: Binding<Int> {
        Binding<Int>(get: { Int(wrappedValue) }, set: { wrappedValue = Value($0) })
    }
}
