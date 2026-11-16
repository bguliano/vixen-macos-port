import AppKit
import VixenBridge
import VixenKit

/// The sequencer's Manage Tags command saves on close; Display Setup still stages its
/// own edits until OK. A color-only change leaves the running sequence and controllers alone.
enum ElementTagManagerPresenter {
    static func register(app: AppModel) {
        ElementTagManager.present = { [weak app] window, finished in
            guard let core = app?.core, let window else { finished(false); return }
            Task {
                let rows = await offMain { core.elementTagColors() }
                let colors: [SetupTagColor]? = await TemplateRunner.sheet(on: window) {
                    ManageTagColorsSheet(rows: rows, onFinish: $0)
                }
                guard let colors else { finished(false); return }
                let result = await offMain { Result { try core.saveElementTagColors(colors: colors) } }
                switch result {
                case .success: finished(true)
                case .failure(let error):
                    _ = await TemplateRunner.alert(on: window, "Manage Tag Colors", error.userMessage, ["OK"])
                    finished(false)
                }
            }
        }
    }
}
