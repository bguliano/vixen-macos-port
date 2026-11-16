import AppKit
import Observation
import VixenBridge
import VixenKit

/// Called on whichever thread made the change.
private final class PreviewChangeListener: PreviewListener, @unchecked Sendable {
    func changed() {
        Task { @MainActor in LivePreviewCoordinator.shared?.sync() }
    }
}

private final class PreviewPlayerActivityListener: PreviewPlayerListener, @unchecked Sendable {
    func playerActive(active: Bool) {
        Task { @MainActor in LivePreviewCoordinator.shared?.playerActive(active) }
    }
}

/// One live window per running preview. These are AppKit windows, not a SwiftUI group, because they have no close button, can drop their borders and
/// still take keys, and open at the bounds the show's settings say.
@Observable
final class LivePreviewCoordinator {
    @ObservationIgnored private(set) static var shared: LivePreviewCoordinator?

    /// Whether any window is transparent, for the Window menu's way back (F70).
    private(set) var anyTransparent = false

    @ObservationIgnored private let model: AppModel
    @ObservationIgnored private(set) var controllers: [String: LivePreviewController] = [:]
    @ObservationIgnored private var opening = Set<String>()
    @ObservationIgnored private var subscribed = false

    private init(model: AppModel) {
        self.model = model
    }

    /// The scene's value is rebuilt often, and this only runs once.
    static func start(model: AppModel) {
        guard shared == nil else { return }
        let coordinator = LivePreviewCoordinator(model: model)
        shared = coordinator
        coordinator.watchForCore()
    }

    private func watchForCore() {
        let core = withObservationTracking {
            model.core
        } onChange: {
            Task { @MainActor in LivePreviewCoordinator.shared?.watchForCore() }
        }
        guard let core, !subscribed else { return }
        subscribed = true
        core.subscribePreviewChanges(listener: PreviewChangeListener())
        core.subscribePreviewPlayerActive(listener: PreviewPlayerActivityListener())
        sync()
    }

    func sync() {
        guard let core = model.core else { return }
        let running = Set(core.runningPreviewIds())
        for (id, controller) in controllers where !running.contains(id) {
            controller.shutDown()
            controllers[id] = nil
        }
        transparencyChanged()
        for id in running where controllers[id] == nil && !opening.contains(id) {
            open(id: id, core: core)
        }
    }

    private func open(id: String, core: VixenCore) {
        opening.insert(id)
        Task {
            do {
                let preparation = try await Task.detached { try core.prepareLivePreview(id: id) }.value
                opening.remove(id)
                // It may have stopped while that ran.
                guard core.runningPreviewIds().contains(id), controllers[id] == nil else { return }
                controllers[id] = LivePreviewController(id: id, preparation: preparation)
                transparencyChanged()
            } catch {
                opening.remove(id)
                PreviewLog.problem("Couldn't open a preview window: \(error.userMessage)")
            }
        }
    }

    func playerActive(_ active: Bool) {
        for controller in controllers.values {
            controller.playerActive(active)
        }
    }

    func transparencyChanged() {
        let any = controllers.values.contains { $0.flags.transparent }
        if anyTransparent != any {
            anyTransparent = any
        }
    }

    /// The Window menu's command, for a window that may not be right-clickable any more.
    func turnOffTransparency() {
        for controller in controllers.values where controller.flags.transparent {
            controller.setTransparent(false)
        }
    }

    /// F1: after the designer closes the window carries on with what it left. Upstream only swaps its form for GDI to OpenGL, and one window does both here, so both
    /// directions apply now (Q8).
    func designerClosed(id: String) {
        guard let controller = controllers[id] else { return }
        controller.shutDown()
        controllers[id] = nil
        if let core = model.core {
            open(id: id, core: core)
        }
    }
}

enum PreviewLog {
    static func problem(_ text: String) {
        FileHandle.standardError.write(Data((text + "\n").utf8))
    }
}
