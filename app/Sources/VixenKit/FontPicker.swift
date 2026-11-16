import AppKit
import SwiftUI
import VixenBridge

/// What F35 edits: `SerializableFont`, as Rust holds it.
public struct KitFont: Equatable, Sendable {
    public var name: String
    public var size: Float
    public var bold: Bool
    public var italic: Bool

    public init(name: String, size: Float, bold: Bool, italic: Bool) {
        self.name = name
        self.size = size
        self.bold = bold
        self.italic = italic
    }

    var nsFont: NSFont {
        var font = NSFont(name: name, size: CGFloat(size)) ?? NSFontManager.shared.font(withFamily: name, traits: [], weight: 5, size: CGFloat(size)) ?? .systemFont(ofSize: CGFloat(size))
        if bold {
            font = NSFontManager.shared.convert(font, toHaveTrait: .boldFontMask)
        }
        if italic {
            font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask)
        }
        return font
    }

    init(_ font: NSFont) {
        let traits = NSFontManager.shared.traits(of: font)
        name = font.familyName ?? font.fontName
        size = Float(font.pointSize)
        bold = traits.contains(.boldFontMask)
        italic = traits.contains(.italicFontMask)
    }
}

/// The system font panel for one edit at a time (F35). It can't be a sheet: the commit comes when it closes, or when another edit takes over, as one undo step.
@MainActor
public final class FontPanelSession: NSObject, NSFontChanging {
    public static let shared = FontPanelSession()

    private var onFinish: ((KitFont) -> Void)?
    private var start: KitFont?
    private var current: KitFont?
    private var observing = false

    public func begin(font: KitFont, onFinish: @escaping (KitFont) -> Void) {
        finish()
        self.onFinish = onFinish
        start = font
        current = font
        let manager = NSFontManager.shared
        manager.target = self
        manager.action = #selector(changeFont(_:))
        manager.setSelectedFont(font.nsFont, isMultiple: false)
        PendingCommits.set(self) { [weak self] in self?.finish() }
        let panel = NSFontPanel.shared
        if !observing {
            observing = true
            NotificationCenter.default.addObserver(self, selector: #selector(panelClosing), name: NSWindow.willCloseNotification, object: panel)
        }
        if !UITestHarness.isActive {
            panel.orderFront(nil)
        }
    }

    public func finish() {
        guard let onFinish else { return }
        let finished = current
        let notify = finished != start
        self.onFinish = nil
        current = nil
        start = nil
        PendingCommits.clear(self)
        if let finished, notify {
            onFinish(finished)
        }
    }

    /// For the test harness, which can't reach the system panel.
    public func testSetFont(_ font: KitFont) {
        current = font
    }

    @objc public func changeFont(_ sender: NSFontManager?) {
        guard let sender, let current else { return }
        self.current = KitFont(sender.convert(current.nsFont))
    }

    public func validModesForFontPanel(_ fontPanel: NSFontPanel) -> NSFontPanel.ModeMask {
        [.collection, .face, .size]
    }

    @objc private func panelClosing(_ note: Notification) {
        finish()
    }
}
