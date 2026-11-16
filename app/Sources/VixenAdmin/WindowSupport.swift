import AppKit
import SwiftUI

/// Hands over the `NSWindow` a view ends up in.
struct WindowReader: NSViewRepresentable {
    let found: (NSWindow) -> Void

    func makeNSView(context: Context) -> NSView {
        WindowReaderView(found: found)
    }

    func updateNSView(_ view: NSView, context: Context) {}

    private final class WindowReaderView: NSView {
        let found: (NSWindow) -> Void

        init(found: @escaping (NSWindow) -> Void) {
            self.found = found
            super.init(frame: .zero)
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) { fatalError("not used") }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            if let window { found(window) }
        }
    }
}

/// Closing the Administration window runs the same flow as quitting, as the main form's close does upstream. Everything else still reaches the
/// delegate SwiftUI installed.
final class CloseQuitsDelegate: NSObject, NSWindowDelegate {
    nonisolated(unsafe) private weak var original: NSWindowDelegate?

    init(original: NSWindowDelegate?) {
        self.original = original
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        NSApp.terminate(nil)
        return false
    }

    nonisolated override func responds(to selector: Selector!) -> Bool {
        super.responds(to: selector) || original?.responds(to: selector) == true
    }

    nonisolated override func forwardingTarget(for selector: Selector!) -> Any? {
        original
    }
}

/// Rows or columns sized by fractions of the space, as the original's `TableLayoutPanel`s are.
struct ProportionalStack: Layout {
    var axis: Axis
    var fractions: [CGFloat]

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        proposal.replacingUnspecifiedDimensions()
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var offset: CGFloat = 0
        for (index, subview) in subviews.enumerated() {
            let fraction = index < fractions.count ? fractions[index] : 0
            switch axis {
            case .horizontal:
                let width = bounds.width * fraction
                subview.place(
                    at: CGPoint(x: bounds.minX + offset, y: bounds.minY),
                    anchor: .topLeading,
                    proposal: ProposedViewSize(width: width, height: bounds.height)
                )
                offset += width
            case .vertical:
                let height = bounds.height * fraction
                subview.place(
                    at: CGPoint(x: bounds.minX, y: bounds.minY + offset),
                    anchor: .topLeading,
                    proposal: ProposedViewSize(width: bounds.width, height: height)
                )
                offset += height
            }
        }
    }
}

/// A sheet's bottom row of buttons, and its title, which a sheet has no title bar to show.
extension View {
    func sheetTitle(_ title: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title).font(.headline)
            self
        }
    }
}

/// `FolderBrowserDialog`.
@MainActor
func chooseFolder(startingAt path: String, on window: NSWindow?) async -> String? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.canCreateDirectories = true
    panel.allowsMultipleSelection = false
    if !path.isEmpty {
        panel.directoryURL = URL(fileURLWithPath: path)
    }
    let response: NSApplication.ModalResponse
    if let window {
        response = await panel.beginSheetModal(for: window)
    } else {
        response = await panel.begin()
    }
    return response == .OK ? panel.url?.path : nil
}
