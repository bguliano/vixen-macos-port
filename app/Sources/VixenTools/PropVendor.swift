import AppKit
import SwiftUI
import VixenBridge
import VixenKit

@MainActor @Observable final class PropVendorModel: Identifiable {
    let id = UUID()
    let browser = PropVendorBrowser()
    let client = PropVendorHTTP(tests: UITestHarness.isActive)
    var view: PropVendorView
    var tab = 0
    var revision = 0
    @ObservationIgnored weak var owner: PropEditorModel?
    @ObservationIgnored var testExpand: (() -> Void)?
    init(owner: PropEditorModel) { self.owner = owner; view = browser.view() }
    func refresh() { view = browser.view() }
    func selectVendor(_ index: UInt32) { browser.selectVendor(index: index); refresh(); tab = 0; revision += 1 }
    func selectProduct(_ path: String) { if browser.selectProduct(path: path) { refresh(); tab = 0 } }
    func close() { owner?.vendorRequest = nil; client.cancel() }
    func selectModel(_ index: UInt32) {
        guard let owner else { return }
        owner.vendorRequest = nil
        Task { await owner.downloadVendorModel(self, index: index) }
    }
    func open(_ text: String?) {
        guard let text, let url = URL(string: text) else { return }
        if UITestHarness.isActive { UITestHarness.note("vendor link \(text)") } else { NSWorkspace.shared.open(url) }
    }
}

extension PropEditorModel {
    func openVendorBrowser() {
        Task {
            guard await confirmReplacement() else { return }
            busy = true
            let browser = PropVendorModel(owner: self)
            let core = browser.browser, client = browser.client
            let progress = Task { [weak self = self] in
                while !Task.isCancelled {
                    self?.loadingStatus = core.status()
                    try? await Task.sleep(for: .milliseconds(200))
                }
            }
            let url = UITestHarness.isActive ? ProcessInfo.processInfo.environment["VIXEN_TEST_PROP_VENDORS"] : nil
            let errors = await Task.detached { core.load(client: client, vendorUrl: url) }.value
            progress.cancel(); loadingStatus = nil; busy = false
            browser.refresh()
            for error in errors { await tell(error.title, error.message) }
            if browser.view.vendors.isEmpty { client.cancel() } else { vendorRequest = browser }
        }
    }
    func downloadVendorModel(_ browser: PropVendorModel, index: UInt32) async {
        busy = true
        let core = browser.browser, client = browser.client
        do {
            let download = try await Task.detached { try core.download(index: index, client: client) }.value
            let choices = download.choices()
            client.cancel(); busy = false
            if choices.count > 1 {
                xmodelRequest = PropXModelRequest(source: .vendor(download), choices: choices)
            } else {
                await importXModel(source: .vendor(download), index: choices.first?.index ?? 0)
            }
        } catch {
            client.cancel(); busy = false
            await tell("Error Downloading Model", error.userMessage)
        }
    }
}
