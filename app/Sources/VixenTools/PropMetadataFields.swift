import AppKit
import SwiftUI
import VixenBridge

/// The source converter suggests bulb types but accepts arbitrary text.
struct PropBulbType: NSViewRepresentable {
    @Binding var text: String
    func makeCoordinator() -> Coordinator { Coordinator($text) }
    func makeNSView(context: Context) -> NSComboBox {
        let combo = NSComboBox(); combo.addItems(withObjectValues: propEdBulbTypes())
        combo.isEditable = true; combo.completes = false; combo.delegate = context.coordinator
        combo.setAccessibilityLabel("Bulb Type")
        combo.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return combo
    }
    func updateNSView(_ combo: NSComboBox, context: Context) {
        context.coordinator.text = $text
        if combo.stringValue != text { combo.stringValue = text }
    }
    final class Coordinator: NSObject, NSComboBoxDelegate {
        var text: Binding<String>
        init(_ text: Binding<String>) { self.text = text }
        func controlTextDidChange(_ notification: Notification) {
            guard let combo = notification.object as? NSComboBox else { return }
            text.wrappedValue = combo.stringValue
        }
        func comboBoxSelectionDidChange(_ notification: Notification) {
            guard let combo = notification.object as? NSComboBox, let value = combo.objectValueOfSelectedItem as? String else { return }
            text.wrappedValue = value
        }
    }
}

/// Notes retain literal Return and Tab and use the native spelling checker.
struct PropNotes: NSViewRepresentable {
    @Binding var text: String
    func makeCoordinator() -> Coordinator { Coordinator($text) }
    func makeNSView(context: Context) -> NSScrollView {
        let view = NSTextView(frame: NSRect(x: 0, y: 0, width: 100, height: 100))
        view.minSize = NSSize(width: 0, height: 100)
        view.isRichText = false; view.isContinuousSpellCheckingEnabled = true
        view.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        view.textColor = .labelColor; view.backgroundColor = .textBackgroundColor
        view.isVerticallyResizable = true; view.isHorizontallyResizable = false
        view.autoresizingMask = [.width]; view.textContainer?.widthTracksTextView = true
        view.textContainerInset = NSSize(width: 4, height: 4)
        view.delegate = context.coordinator; view.setAccessibilityLabel("Notes")
        let scroll = NSScrollView(); scroll.hasVerticalScroller = true; scroll.documentView = view
        return scroll
    }
    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.text = $text
        if let view = scroll.documentView as? NSTextView, view.string != text { view.string = text }
    }
    final class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>
        init(_ text: Binding<String>) { self.text = text }
        func textDidChange(_ notification: Notification) {
            guard let view = notification.object as? NSTextView else { return }
            text.wrappedValue = view.string
        }
    }
}
