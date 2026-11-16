import AppKit
import UniformTypeIdentifiers
import VixenBridge

/// The Open dialog, with the original's file type menu.
@MainActor
final class OpenSequencePanel: NSObject {
    private let panel = NSOpenPanel()
    private let filters = openPanelFilters()

    /// Shows the panel and returns the files chosen. Starts in `directory`.
    func run(directory: URL?, on window: NSWindow?) async -> [URL] {
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.directoryURL = directory
        panel.accessoryView = makeAccessory()
        panel.isAccessoryViewDisclosed = true
        apply(filter: filters[0])
        let response: NSApplication.ModalResponse
        if let window {
            response = await panel.beginSheetModal(for: window)
        } else {
            response = await panel.begin()
        }
        return response == .OK ? panel.urls : []
    }

    private func makeAccessory() -> NSView {
        let label = NSTextField(labelWithString: "File type:")
        let popup = NSPopUpButton(frame: .zero, pullsDown: false)
        popup.addItems(withTitles: filters.map(\.label))
        popup.target = self
        popup.action = #selector(filterChosen(_:))
        let row = NSStackView(views: [label, popup])
        row.orientation = .horizontal
        row.edgeInsets = NSEdgeInsets(top: 8, left: 12, bottom: 8, right: 12)
        return row
    }

    @objc private func filterChosen(_ sender: NSPopUpButton) {
        apply(filter: filters[sender.indexOfSelectedItem])
    }

    private func apply(filter: FileFilterRecord) {
        panel.allowedContentTypes = filter.extensions.compactMap { UTType(filenameExtension: $0) }
    }
}
