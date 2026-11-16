import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// The profile's scheduler as the app sees it: made as soon as the profile loads so the schedule is checked with no window open (F59), and
/// the owner of the Scheduler Status window, which is on screen exactly while the scheduler is enabled (F55).
@Observable
final class SchedulerHub {
    @ObservationIgnored static var current: SchedulerHub?

    let core: VixenCore
    let session: SchedulerSession
    private(set) var status: SchedulerStatusView
    private(set) var rows: [ScheduleRow]
    private(set) var logShown = false
    @ObservationIgnored private var panel: SchedulerStatusPanel?
    @ObservationIgnored private var heightBeforeLog: CGFloat?

    /// Follows `AppModel.core`, which a profile change replaces.
    static func follow(_ model: AppModel) {
        withObservationTracking {
            attach(model)
        } onChange: {
            Task { @MainActor in follow(model) }
        }
    }

    private static func attach(_ model: AppModel) {
        guard let core = model.core else {
            current?.detach()
            current = nil
            return
        }
        if current?.core === core { return }
        current?.detach()
        let hub = SchedulerHub(core: core)
        current = hub
        // The menu item is drawn from `AppModel`, so this assignment is what redraws it.
        Task { @MainActor in
            model.schedulerEnabledToggle = SchedulerEnabledToggle(
                isOn: { SchedulerHub.current?.status.enabled ?? false },
                setOn: { SchedulerHub.current?.setEnabled($0) }
            )
        }
    }

    private init(core: VixenCore) {
        self.core = core
        session = core.scheduler()
        status = session.status()
        rows = session.rows()
        session.setListener(listener: SchedulerForwarder(hub: self))
        syncPanel()
    }

    private func detach() {
        session.setListener(listener: nil)
        panel?.orderOut(nil)
        panel = nil
    }

    func refresh() {
        let next = session.status()
        if next != status { status = next }
        let nextRows = session.rows()
        if nextRows != rows { rows = nextRows }
        syncPanel()
    }

    /// F54.
    func setEnabled(_ on: Bool) {
        let failure = session.setEnabled(on: on)
        refresh()
        if let failure {
            Task { await SchedulerAlerts.tell("Scheduler", failure, style: .error, window: nil) }
        }
    }

    func start() {
        session.start()
        refresh()
    }

    func stop(gracefully: Bool) {
        session.stop(gracefully: gracefully)
        refresh()
    }

    /// F58: the window's height goes to 420, and back to what it was.
    func toggleLog() {
        logShown.toggle()
        guard let panel else { return }
        var frame = panel.frame
        let top = frame.maxY
        if logShown {
            heightBeforeLog = frame.height
            frame.size.height = 420
        } else {
            frame.size.height = max(heightBeforeLog ?? panel.minSize.height, panel.minSize.height)
        }
        frame.origin.y = top - frame.height
        panel.setFrame(frame, display: true, animate: !UITestHarness.isActive)
    }

    private func syncPanel() {
        if status.enabled {
            if panel == nil { panel = SchedulerStatusPanel(hub: self) }
            if panel?.isVisible == false { panel?.orderFront(nil) }
        } else {
            panel?.orderOut(nil)
        }
    }

    func moved(to frame: NSRect) {
        guard let screen = NSScreen.screens.first else { return }
        session.setStatusPosition(x: Int32(frame.minX.rounded()), y: Int32((screen.frame.maxY - frame.maxY).rounded()))
    }
}

private nonisolated final class SchedulerForwarder: SchedulerListener, @unchecked Sendable {
    private weak var hub: SchedulerHub?

    init(hub: SchedulerHub) {
        self.hub = hub
    }

    func changed() {
        Task { @MainActor [weak self] in self?.hub?.refresh() }
    }
}

/// `StatusForm`: no close button, and a position kept in the module data.
final class SchedulerStatusPanel: NSPanel, NSWindowDelegate {
    private weak var hub: SchedulerHub?
    private var placed = false

    init(hub: SchedulerHub) {
        self.hub = hub
        super.init(contentRect: NSRect(x: 0, y: 0, width: 569, height: 121), styleMask: [.titled, .resizable, .utilityWindow], backing: .buffered, defer: false)
        title = "Scheduler Status"
        identifier = NSUserInterfaceItemIdentifier(WindowID.schedulerStatus)
        isReleasedWhenClosed = false
        hidesOnDeactivate = false
        isFloatingPanel = false
        becomesKeyOnlyIfNeeded = true
        appearance = NSAppearance(named: .darkAqua)
        contentMinSize = NSSize(width: 569, height: 121 - (frame.height - contentLayoutRect.height))
        minSize = NSSize(width: 569, height: 121)
        contentView = NSHostingView(rootView: SchedulerStatusContent(hub: hub))
        setContentSize(NSSize(width: 569, height: 121 - (frame.height - contentLayoutRect.height)))
        place(hub.status)
        placed = true
        delegate = self
    }

    /// The stored top-left corner if it's on a screen's work area, as upstream, and otherwise the centre.
    private func place(_ status: SchedulerStatusView) {
        guard let main = NSScreen.screens.first else { return center() }
        let corner = NSPoint(x: CGFloat(status.positionX), y: main.frame.maxY - CGFloat(status.positionY))
        let onScreen = NSScreen.screens.contains { $0.visibleFrame.insetBy(dx: -1, dy: -1).contains(corner) }
        if onScreen, status.positionX != 0 || status.positionY != 0 {
            setFrameTopLeftPoint(corner)
        } else {
            center()
        }
    }

    func windowDidMove(_ notification: Notification) {
        guard placed, isVisible else { return }
        hub?.moved(to: frame)
    }
}

struct SchedulerStatusContent: View {
    let hub: SchedulerHub

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            GroupBox("Show Status") {
                HStack(spacing: 8) {
                    Text("Status:")
                    Text(hub.status.text)
                        .lineLimit(2)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .accessibilityIdentifier("schedulerStatusText")
                    button("play.fill", "Start/Enable the Scheduler", enabled: hub.status.canStart) { hub.start() }
                    button("stop.fill", "Stop Show Immediately", enabled: hub.status.canStop) { hub.stop(gracefully: false) }
                    button("clock.badge.xmark", "Stop Show Gracefully", enabled: hub.status.canStop) { hub.stop(gracefully: true) }
                    button("doc.text", "View Log", enabled: true) { hub.toggleLog() }
                }
                .padding(4)
            }
            if hub.logShown {
                GroupBox("Log") {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 2) {
                            ForEach(Array(hub.status.log.enumerated()), id: \.offset) { _, line in
                                Text(line)
                                    .font(.system(.callout, design: .monospaced))
                                    .textSelection(.enabled)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                            }
                        }
                        .padding(4)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
        .padding(12)
        .frame(minWidth: 569, maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private func button(_ symbol: String, _ help: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol).frame(width: 18, height: 16)
        }
        .help(help)
        .accessibilityLabel(help)
        .disabled(!enabled)
    }
}
