import AppKit
import SwiftUI
import VixenBridge

extension Notification.Name { static let propPreferencesChanged = Notification.Name("VixenPropPreferencesChanged") }

struct PropPreferencesSheet: View {
    let model: PropEditorModel
    @Environment(\.dismiss) private var dismiss
    @State private var light = Color.white
    @State private var selected = Color.pink
    @State private var base = Color.gray
    @State private var size = "3"
    @State private var saving = false
    @State private var error = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Preferences").font(.headline)
            Form {
                ColorPicker("Light Color", selection: $light, supportsOpacity: false)
                ColorPicker("Selected Color", selection: $selected, supportsOpacity: false)
                ColorPicker("State Preview Base Color", selection: $base, supportsOpacity: false)
                TextField("Default Light Size", text: $size)
            }
            if !error.isEmpty { Text(error).foregroundStyle(.red) }
            HStack {
                Button("Defaults") { load(propEdDefaultPreferences()) }
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("OK") { save() }.keyboardShortcut(.defaultAction).disabled(UInt32(size) == nil || saving)
            }
        }.padding(20).frame(width: 350)
            .onAppear { load(model.app?.core?.propPreferences() ?? propEdDefaultPreferences()) }
    }
    private func load(_ p: PropEdPreferences) {
        light = Color(nsColor: NSColor(argb: p.lightColor)); selected = Color(nsColor: NSColor(argb: p.selectedColor)); base = Color(nsColor: NSColor(argb: p.stateBaseColor)); size = String(p.lightSize)
    }
    private func argb(_ value: Color) -> UInt32 {
        let c = NSColor(value).usingColorSpace(.deviceRGB) ?? .white
        return 0xff000000 | UInt32((c.redComponent * 255).rounded()) << 16 | UInt32((c.greenComponent * 255).rounded()) << 8 | UInt32((c.blueComponent * 255).rounded())
    }
    private func save() {
        guard let size = UInt32(size) else { return }
        let p = PropEdPreferences(lightColor: argb(light), selectedColor: argb(selected), stateBaseColor: argb(base), lightSize: size)
        saving = true
        Task {
            do {
                if let core = model.app?.core { try await Task.detached { try core.savePropPreferences(preferences: p) }.value }
                model.editor.applyPreferences(preferences: p); model.refresh()
                NotificationCenter.default.post(name: .propPreferencesChanged, object: nil); dismiss()
            } catch { self.error = error.userMessage }
            saving = false
        }
    }
}

extension PropEditorModel {
    func reloadPreferences() {
        if let p = app?.core?.propPreferences() { editor.applyPreferences(preferences: p); refresh() }
    }
}
