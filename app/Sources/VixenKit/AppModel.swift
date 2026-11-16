import Observation
import SwiftUI
import VixenBridge

/// The open profile, shared by every window. Each area adds its own observable state on top of `core`.
@Observable
public final class AppModel {
    public enum Phase {
        case starting(progress: Double, message: String)
        case ready
        case failed(String)
    }

    public private(set) var phase: Phase = .starting(progress: 0.1, message: "Starting up")
    public private(set) var core: VixenCore?

    /// The app can't take commands: it's starting up, a dialog is up, or the config is saving or reloading. Menus and buttons that change the
    /// system follow this.
    public var isBusy: Bool {
        administrationBusy || systemConfigSessionActive
    }

    /// The Administration area's part of `isBusy`.
    public var administrationBusy = false

    /// A build with the Administration area sets this: that area does the real launch, with the profile and its lock.
    @ObservationIgnored public var startsElsewhere = false

    public init() {}

    /// `VIXEN_DATA` chooses the data folder for tests. Otherwise it's the default, `~/Documents/Vixen 3`.
    public var dataRoot: String? {
        ProcessInfo.processInfo.environment["VIXEN_DATA"] ?? defaultDataRoot()
    }

    /// True in tests: no sound and no output hardware.
    public static var isMuted: Bool {
        ProcessInfo.processInfo.environment["VIXEN_MUTE"] != nil || UITestHarness.isActive
    }

    public func start() async {
        guard case .starting = phase else { return }
        if startsElsewhere {
            // The Administration area keeps reporting progress after it hands over the core, which puts the phase back to starting, so the
            // core is what says the launch got far enough.
            while core == nil, case .starting = phase {
                try? await Task.sleep(for: .milliseconds(50))
            }
            return
        }
        guard let root = dataRoot else {
            phase = .failed("Couldn't find a folder for Vixen's data.")
            return
        }
        do {
            let muted = Self.isMuted
            let opened = try await Task.detached { try VixenCore.open(dataRoot: root, muted: muted) }.value
            core = opened
            phase = .ready
        } catch {
            phase = .failed(error.userMessage)
        }
    }

    // The launch, as the Administration area runs it.

    public func reportStartup(progress: Double, message: String) {
        phase = .starting(progress: progress, message: message)
    }

    public func adopt(core: VixenCore) {
        self.core = core
        phase = .ready
    }

    public func startupFailed(_ message: String) {
        phase = .failed(message)
    }

    // Open editors. The sequencer area reports each window here.

    public private(set) var openEditors: [any OpenEditor] = []
    public private(set) var activeEditor: (any OpenEditor)?

    /// `_OpenEditor`.
    public func editorOpened(_ editor: any OpenEditor) {
        openEditors.append(editor)
    }

    /// An editor's `Activated` event.
    public func editorActivated(_ editor: any OpenEditor) {
        activeEditor = editor
    }

    /// `_CloseEditor`, after its prompt: forgets the editor and adds its file to the recent list. An unsaved sequence has no path. As upstream,
    /// the active editor is forgotten even if another one was active.
    public func editorClosed(_ editor: any OpenEditor, filePath: String?) {
        openEditors.removeAll { $0 === editor }
        activeEditor = nil
        if let filePath, !filePath.isEmpty {
            sequenceClosed?(filePath)
        }
    }

    /// Set by the Administration area, which keeps the recent list.
    @ObservationIgnored public var sequenceClosed: ((String) -> Void)?

    // Auxiliary documents (props, fixtures, and similar tools) can veto application shutdown.
    // Take a snapshot while asking: a window may close and unregister itself during a prompt.
    @ObservationIgnored private var documentCloseGuards: [(UUID, @MainActor () async -> Bool)] = []

    public func registerDocumentCloseGuard(id: UUID, _ guardClose: @escaping @MainActor () async -> Bool) {
        documentCloseGuards.removeAll { $0.0 == id }
        documentCloseGuards.append((id, guardClose))
    }

    public func removeDocumentCloseGuard(id: UUID) {
        documentCloseGuards.removeAll { $0.0 == id }
    }

    public func prepareDocumentsForShutdown() async -> Bool {
        for (_, guardClose) in documentCloseGuards {
            if await !guardClose() { return false }
        }
        return true
    }

    // A system configuration session: Display Setup or Setup Previews. Their dialogs are modal to the whole app upstream, and Cancel can replace the
    // whole element tree, so while this is true the menus and sequencer editing stay off.

    public private(set) var systemConfigSessionActive = false

    /// Call when the Display Setup window appears.
    public func beginSystemConfigSession() {
        systemConfigSessionActive = true
    }

    /// Call when the Display Setup window closes. The Administration area shows the progress and does the save or reload, and the session lasts
    /// until that's finished.
    public func endSystemConfigSession(_ outcome: SystemConfigOutcome) {
        if let systemConfigSessionEnded {
            systemConfigSessionEnded(outcome)
        } else {
            finishSystemConfigSession()
        }
    }

    public func finishSystemConfigSession() {
        systemConfigSessionActive = false
    }

    @ObservationIgnored public var systemConfigSessionEnded: ((SystemConfigOutcome) -> Void)?

    // The preview designer, opened from Setup Previews.

    /// The designer's window calls this when it closes, so a preview stopped for designing can start again.
    public func previewDesignerClosed(id: String) {
        previewDesignerDidClose?(id)
    }

    @ObservationIgnored public var previewDesignerDidClose: ((String) -> Void)?

    // The Scheduler menu's Enabled toggle. The scheduler area fills this in, and until then the item is off.

    public var schedulerEnabledToggle: SchedulerEnabledToggle?
}

/// An open sequence window, as the app sees it.
public protocol OpenEditor: AnyObject {
    /// `IsModified`, for the prompt at shutdown.
    var isModified: Bool { get }
    /// `_CloseEditor` as the app closes: the save prompt with no Cancel, the rest of closing, then `AppModel.editorClosed`.
    func closeForShutdown() async
}

/// How Display Setup ended.
public enum SystemConfigOutcome: Sendable {
    /// OK. `apply` is its `ApplyConfirmedChanges`, run off the main actor before the Administration area's saves.
    case saved(apply: (@Sendable () -> Void)?)
    /// OK, for a window that does its own applying and saving. `work` runs off the main actor and reports to the Administration window's progress bar.
    case performed(work: @Sendable (_ progress: @escaping @Sendable (Int32, String) -> Void) -> Void)
    /// Cancel. With `changed` false nothing is reloaded, because a reload restarts every controller and preview.
    case discarded(changed: Bool)
}

/// What the Scheduler menu's Enabled item needs.
public struct SchedulerEnabledToggle {
    public var isOn: () -> Bool
    public var setOn: (Bool) -> Void

    public init(isOn: @escaping () -> Bool, setOn: @escaping (Bool) -> Void) {
        self.isOn = isOn
        self.setOn = setOn
    }
}
