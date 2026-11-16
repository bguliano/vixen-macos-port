import AppKit
import SwiftUI
import VixenKit

/// Attach on the actual AppKit lifecycle event, including a window first created after SwiftUI's initial update.
struct PropWindowAttachment: NSViewRepresentable {
    let model: PropEditorModel
    let app: AppModel
    func makeNSView(context: Context) -> Attachment {
        let view = Attachment(); view.model = model; view.app = app; return view
    }
    func updateNSView(_ view: Attachment, context: Context) { view.model = model; view.app = app; view.attach() }
    final class Attachment: NSView {
        weak var model: PropEditorModel?
        weak var app: AppModel?
        override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); attach() }
        func attach() {
            guard let window else { return }
            DispatchQueue.main.async { [weak self, weak window] in
                guard let self, let window, let model, let app else { return }
                model.attach(window: window, app: app)
            }
        }
    }
}
