import SwiftUI
import VixenBridge
import VixenKit

/// The app's main window, the menus it owns, and the small windows that open from them.
public struct AdministrationScene: Scene {
    private let model: AppModel
    @State private var admin: AdminModel

    public init(model: AppModel) {
        self.model = model
        model.startsElsewhere = true
        let admin = AdminModel(app: model)
        // Finder can ask to open a file before the window has appeared. The first wins, because `init` can run again and only the first model stays.
        if AppHooks.openFiles == nil {
            AppHooks.openFiles = { [weak admin] urls in admin?.openFiles(urls) }
        }
        _admin = State(initialValue: admin)
    }

    public var body: some Scene {
        Window("Vixen Administration", id: WindowID.administration) {
            AdministrationView()
                .environment(admin)
                .environment(model)
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 472, height: 578)
        .windowResizability(.contentMinSize)
        // Always, even when the app starts by being asked to open a file, since the rest of the launch happens in this window.
        .defaultLaunchBehavior(.presented)
        .restorationBehavior(.disabled)
        .commands {
            AdministrationCommands(admin: admin, state: MenuState(admin: admin))
        }

        Settings {
            OptionsForm()
                .environment(admin)
                .preferredColorScheme(.dark)
        }

        Window("About Vixen", id: WindowID.about) {
            AboutView()
                .environment(admin)
                .preferredColorScheme(.dark)
        }
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
        .windowResizability(.contentSize)

        Window("Installed Modules", id: WindowID.installedModules) {
            InstalledModulesView()
                .preferredColorScheme(.dark)
        }
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
        .windowResizability(.contentSize)

        Window("Vixen Release Notes", id: WindowID.releaseNotes) {
            ReleaseNotesView()
                .environment(admin)
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 774, height: 571)
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)

        Window("Instrumentation", id: WindowID.instrumentation) {
            InstrumentationView()
                .environment(admin)
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 807, height: 437)
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
    }
}
