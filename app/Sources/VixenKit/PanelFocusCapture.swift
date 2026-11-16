import AppKit
import SwiftUI

/// Clicking blank space in the panel must release the timeline/text editor's focus.
/// Set a baseline responder before normal dispatch; a clicked text field or control
/// then receives the event and takes its own focus normally.
public struct PanelFocusCapture: NSViewRepresentable {
    private let testHook: String
    public init(testHook: String = "") { self.testHook = testHook }
    public func makeNSView(context: Context) -> PanelFocusView { PanelFocusView(testHook: testHook) }
    public func updateNSView(_ nsView: PanelFocusView, context: Context) {}
    public static func dismantleNSView(_ nsView: PanelFocusView, coordinator: ()) { nsView.removeMonitor() }
}

public final class PanelFocusView: NSView {
    private let testHook: String
    init(testHook: String) { self.testHook = testHook; super.init(frame: .zero) }
    public required init?(coder: NSCoder) { fatalError("not used") }
    private var monitor: Any?
    public override var acceptsFirstResponder: Bool { true }
    public override func hitTest(_ point: NSPoint) -> NSView? { nil }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        removeMonitor()
        guard window != nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .leftMouseDown) { [weak self] event in
            guard let self, let window = self.window, event.window === window,
                  !self.isHiddenOrHasHiddenAncestor,
                  self.bounds.contains(self.convert(event.locationInWindow, from: nil)) else { return event }
            window.makeFirstResponder(self)
            return event
        }
        if UITestHarness.isActive, !testHook.isEmpty {
            UITestHarness.hooks[testHook] = { [weak self] args in
                guard let self, let window = self.window else { return "FAIL no panel" }
                if args.first == "clickBlank" {
                    let point = self.convert(NSPoint(x: self.bounds.midX, y: self.bounds.minY + 30), to: nil)
                    for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                        if let event = NSEvent.mouseEvent(with: type, location: point, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                                         windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 0) {
                            NSApp.postEvent(event, atStart: false)
                        }
                    }
                    try? await Task.sleep(for: .milliseconds(150))
                }
                return "focused=\(window.firstResponder === self) responder=\(String(describing: window.firstResponder))"
            }
        }
    }

    func removeMonitor() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
    }
}

