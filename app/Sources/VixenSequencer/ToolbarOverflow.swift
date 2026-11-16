import AppKit
import SwiftUI
import VixenBridge
import VixenKit

// Upstream's strips sit in a `ToolStripPanel`: rows pack left from X = 3, and a strip that runs past the panel's edge moves the items that don't fit into
// the drop-down at its right end. `NSToolbar` does the same on the Mac with its » menu, so the band does too: nothing is ever cut off on the left, and
// nothing is out of reach on the right.

/// What an item becomes in its strip's overflow menu.
struct StripEntry {
    var title: String
    var image: ToolImage?
    var enabled = true
    var checked = false
    var action: (() -> Void)?
    /// Built when the menu opens, as upstream's drop-downs rebuild on opening.
    var submenu: (() -> [StripEntry])?
}

// SwiftUI runs layouts on the main thread without marking them so, hence `nonisolated` here and on the state below.
private nonisolated struct StripEntryKey: LayoutValueKey {
    static var defaultValue: StripEntry? { nil }
}

extension View {
    /// The item's place in the overflow menu when it doesn't fit.
    func stripEntry(_ entry: StripEntry) -> some View {
        layoutValue(key: StripEntryKey.self, value: entry)
    }
}

/// What's past a strip's right edge. Written by the layout, read when the drop-down opens, so it never causes a redraw.
nonisolated final class StripOverflowState {
    var hidden: [StripEntry] = []
}

/// A strip's items, left to right. When they don't all fit, those that don't go into the drop-down, which is the last subview.
struct StripOverflowLayout: Layout {
    let state: StripOverflowState
    var spacing: CGFloat = 1

    private func widths(_ sizes: [CGSize]) -> CGFloat {
        sizes.map(\.width).reduce(0, +) + spacing * CGFloat(max(sizes.count - 1, 0))
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard let chevron = subviews.last else { return .zero }
        let sizes = subviews.dropLast().map { $0.sizeThatFits(.unspecified) }
        let chevronSize = chevron.sizeThatFits(.unspecified)
        let height = max(sizes.map(\.height).max() ?? 0, chevronSize.height)
        let total = widths(sizes)
        guard let offered = proposal.width, total > offered + 0.5 else {
            return CGSize(width: total, height: height)
        }
        return CGSize(width: max(offered, chevronSize.width), height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        guard let chevron = subviews.last else { return }
        let items = Array(subviews.dropLast())
        let sizes = items.map { $0.sizeThatFits(.unspecified) }
        let chevronSize = chevron.sizeThatFits(.unspecified)
        let fitsAll = widths(sizes) <= bounds.width + 0.5
        let limit = fitsAll ? bounds.maxX : bounds.maxX - chevronSize.width - spacing
        // Out of sight: the strip clips what's outside it.
        let away = CGPoint(x: bounds.maxX + 100_000, y: bounds.minY)
        var x = bounds.minX
        var overflowing = false
        var hidden: [StripEntry] = []
        for (item, size) in zip(items, sizes) {
            if !overflowing, x + size.width <= limit + 0.5 {
                item.place(at: CGPoint(x: x, y: bounds.midY), anchor: .leading, proposal: ProposedViewSize(size))
                x += size.width + spacing
            } else {
                overflowing = true
                item.place(at: away, proposal: ProposedViewSize(size))
                if let entry = item[StripEntryKey.self] {
                    hidden.append(entry)
                }
            }
        }
        if fitsAll {
            chevron.place(at: away, proposal: ProposedViewSize(chevronSize))
        } else {
            chevron.place(at: CGPoint(x: bounds.maxX - chevronSize.width, y: bounds.midY), anchor: .leading, proposal: ProposedViewSize(chevronSize))
        }
        state.hidden = hidden
    }
}

/// Every strip's items go in one of these.
struct StripItems<Content: View>: View {
    let title: String
    private let content: Content
    @State private var overflow = StripOverflowState()

    init(_ context: StripContext, @ViewBuilder content: () -> Content) {
        title = context.info.title
        self.content = content()
    }

    var body: some View {
        StripOverflowLayout(state: overflow) {
            content
            StripChevron(state: overflow, title: title)
        }
        .clipped()
    }
}

/// The » at a strip's end. Its menu is built when it opens, from what the last layout left out.
struct StripChevron: NSViewRepresentable {
    let state: StripOverflowState
    let title: String

    func makeNSView(context: Context) -> ChevronButton {
        let button = ChevronButton()
        button.overflow = state
        button.setAccessibilityLabel("\(title) overflow")
        button.toolTip = "More \(title) items"
        if UITestHarness.isActive {
            // `sequencer.overflow.<strip>` answers with the menu the chevron would show.
            UITestHarness.hooks["sequencer.overflow.\(title)"] = { [weak button] _ in
                guard let button else { return "FAIL no chevron" }
                return SequencerModel.text(of: StripMenus.menu(button.overflow?.hidden ?? [])).replacingOccurrences(of: "\n", with: " | ")
            }
        }
        return button
    }

    func updateNSView(_ button: ChevronButton, context: Context) {
        button.overflow = state
        button.setAccessibilityLabel("\(title) overflow")
    }

    final class ChevronButton: NSButton {
        var overflow: StripOverflowState?
        private var count = 0

        init() {
            super.init(frame: .zero)
            title = ""
            isBordered = false
            imagePosition = .imageOnly
            image = NSImage(systemSymbolName: "chevron.right.2", accessibilityDescription: nil)?
                .withSymbolConfiguration(NSImage.SymbolConfiguration(pointSize: 10, weight: .semibold))
            contentTintColor = .secondaryLabelColor
            target = self
            action = #selector(open)
            setContentHuggingPriority(.required, for: .horizontal)
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) {
            fatalError("init(coder:) is not used")
        }

        override var intrinsicContentSize: NSSize {
            NSSize(width: 14, height: 20)
        }

        @objc private func open() {
            let menu = StripMenus.menu(overflow?.hidden ?? [])
            // A test window can't track a menu: the script reads what it would have shown.
            if UITestHarness.isActive, let out = ProcessInfo.processInfo.environment["VIXEN_UI_TEST_OUT"] {
                count += 1
                let name = accessibilityLabel()?.replacingOccurrences(of: " ", with: "-") ?? "strip"
                try? SequencerModel.text(of: menu).write(to: URL(fileURLWithPath: out).appending(path: "menu-\(name)-\(count).txt"), atomically: true, encoding: .utf8)
                return
            }
            menu.popUp(positioning: nil, at: NSPoint(x: 0, y: isFlipped ? bounds.maxY + 2 : -2), in: self)
        }
    }
}

/// Builds the overflow menu. Each item's closure travels with it, so nothing else needs to stay alive while the menu is up.
@MainActor
enum StripMenus {
    private final class Runner: NSObject {
        let run: () -> Void

        init(_ run: @escaping () -> Void) {
            self.run = run
        }

        @objc func perform(_ sender: NSMenuItem) {
            run()
        }
    }

    static func menu(_ entries: [StripEntry]) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        for entry in entries {
            let item = NSMenuItem(title: entry.title, action: nil, keyEquivalent: "")
            if let action = entry.action {
                let runner = Runner(action)
                item.representedObject = runner
                item.target = runner
                item.action = #selector(Runner.perform(_:))
            }
            item.image = entry.image.flatMap(image(for:))
            item.state = entry.checked ? .on : .off
            item.isEnabled = entry.enabled
            if let submenu = entry.submenu {
                item.submenu = Self.menu(submenu())
            }
            menu.addItem(item)
        }
        return menu
    }

    private static func image(for image: ToolImage) -> NSImage? {
        let found: NSImage? = switch image {
        case .symbol(let name): NSImage(systemSymbolName: name, accessibilityDescription: nil)
        case .resource(let name): VixenResources.image("images/sequencer/\(name).png")
        case .effect(let name): VixenResources.image("effects/\(name).png")
        case .drawn(let draw): draw()
        }
        guard let found, let copy = found.copy() as? NSImage else { return nil }
        copy.size = NSSize(width: 16, height: 16)
        return copy
    }
}

/// One row of the band: strips packed left. When the row is too narrow, the strip that reaches the edge gives up its last items to its drop-down, and the
/// strips after it keep their grip and drop-down, as upstream's panel row does.
struct StripRowLayout: Layout {
    var spacing: CGFloat = 3

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let ideal = subviews.map { $0.sizeThatFits(.unspecified) }
        let total = ideal.map(\.width).reduce(0, +) + spacing * CGFloat(max(subviews.count - 1, 0))
        let height = ideal.map(\.height).max() ?? 0
        return CGSize(width: min(total, proposal.width ?? total), height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let ideal = subviews.map { $0.sizeThatFits(.unspecified) }
        let minimum = subviews.map { $0.sizeThatFits(ProposedViewSize(width: 0, height: nil)).width }
        let away = CGPoint(x: bounds.maxX + 100_000, y: bounds.minY)
        var x = bounds.minX
        for index in subviews.indices {
            let after = index + 1 < subviews.count ? minimum[(index + 1)...].reduce(0, +) + spacing * CGFloat(subviews.count - index - 1) : 0
            guard x + minimum[index] <= bounds.maxX + 0.5 else {
                subviews[index].place(at: away, proposal: ProposedViewSize(ideal[index]))
                continue
            }
            let room = max(minimum[index], bounds.maxX - x - after)
            let width = min(ideal[index].width, room, bounds.maxX - x)
            subviews[index].place(at: CGPoint(x: x, y: bounds.minY), anchor: .topLeading, proposal: ProposedViewSize(width: width, height: ideal[index].height))
            x += width + spacing
        }
    }
}

// MARK: Buttons

/// Upstream's tool buttons: the image with a little room around it, a light box under the pointer, and a darker one pressed or checked.
struct StripButtonStyle: ButtonStyle {
    var checked = false

    func makeBody(configuration: Configuration) -> some View {
        StripButtonBody(label: configuration.label, pressed: configuration.isPressed, checked: checked)
    }
}

private struct StripButtonBody<Label: View>: View {
    let label: Label
    let pressed: Bool
    let checked: Bool
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false

    var body: some View {
        label
            .padding(.horizontal, 3)
            .padding(.vertical, 2)
            .opacity(isEnabled ? 1 : 0.35)
            .background(fill, in: RoundedRectangle(cornerRadius: 4))
            .overlay {
                if checked {
                    RoundedRectangle(cornerRadius: 4).strokeBorder(Color.accentColor.opacity(0.8), lineWidth: 1)
                }
            }
            .contentShape(Rectangle())
            .onHover { hovering = $0 }
    }

    private var fill: Color {
        if pressed, isEnabled {
            return Color.primary.opacity(0.25)
        }
        if checked {
            return Color.accentColor.opacity(0.3)
        }
        if hovering, isEnabled {
            return Color.primary.opacity(0.12)
        }
        return .clear
    }
}

/// A checkable tool button, as upstream's `CheckOnClick` items.
struct StripToggleStyle: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        Button {
            configuration.isOn.toggle()
        } label: {
            configuration.label
        }
        .buttonStyle(StripButtonStyle(checked: configuration.isOn))
        .accessibilityValue(configuration.isOn ? "on" : "off")
    }
}
