import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// F111/F112: a local draft; Cancel never updates the profile or listener.
struct WebServerSheet: View {
    @Environment(AdminModel.self) private var admin
    let server: WebServerSession
    @State private var port: Int
    @State private var enabled: Bool
    @State private var error: String?
    @State private var saving = false

    init(server: WebServerSession) {
        self.server = server
        let settings = server.settings()
        _port = State(initialValue: Int(settings.port))
        _enabled = State(initialValue: settings.enabled)
        _error = State(initialValue: settings.error)
    }

    private var address: String { "http://\(ProcessInfo.processInfo.hostName):\(port)/" }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Configuration").font(.headline)
            if let image = VixenResources.image("images/WebServerLogo.png") {
                Image(nsImage: image).resizable().frame(width: 358, height: 149)
                    .accessibilityHidden(true)
            }
            Text("Web Server").font(.headline)
            HStack {
                Text("Http Port:").frame(width: 85, alignment: .leading)
                SheetNumberField("Http Port", value: $port, range: 80...64000, width: 85)
                    .textFieldStyle(.roundedBorder)
                Spacer()
            }
            HStack {
                Text("IsEnabled").frame(width: 85, alignment: .leading)
                Toggle("IsEnabled", isOn: $enabled).labelsHidden()
                    .accessibilityLabel("IsEnabled")
            }
            if enabled {
                Text("Point your browser here:")
                Button(address) {
                    if let url = URL(string: address) { admin.openExternally(url) }
                }.buttonStyle(.link)
            }
            if let error { Text(error).foregroundStyle(.red).textSelection(.enabled) }
            Text("Turn off the web server when not in use to conserve resources.")
                .font(.callout).fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("Cancel") { admin.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("OK", action: save).keyboardShortcut(.defaultAction)
            }
        }
        .padding(20).frame(width: 398)
        .disabled(saving)
        .onAppear {
            if UITestHarness.isActive {
                UITestHarness.hooks["webSettings"] = { args in
                    if args.first == "port", args.count > 1, let value = Int(args[1]) { port = min(64000, max(80, value)) }
                    if args.first == "enabled", args.count > 1 { enabled = args[1] == "true" }
                    return "port=\(port) enabled=\(enabled) address=\(enabled ? address : "hidden") saving=\(saving) error=\(error ?? "")"
                }
            }
        }
        .onDisappear { if UITestHarness.isActive { UITestHarness.hooks.removeValue(forKey: "webSettings") } }
    }

    private func save() {
        let port = UInt16(min(64000, max(80, port))), enabled = enabled
        saving = true
        Task {
            do {
                try await Task.detached { try server.save(port: port, enabled: enabled) }.value
                admin.sheet = nil
            } catch { self.error = error.userMessage }
            saving = false
        }
    }
}
