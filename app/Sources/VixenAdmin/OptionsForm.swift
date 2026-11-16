import SwiftUI
import VixenBridge
import VixenKit

/// Options, as the app's Settings window: changes apply as they're made, and the system config saves when the window closes, if something changed.
/// Latency saves at once, as upstream.
struct OptionsForm: View {
    @Environment(AdminModel.self) private var admin
    @State private var options: OptionsRecord?

    private var core: VixenCore? { admin.app.core }

    var body: some View {
        Group {
            if let options {
                form(options)
            } else {
                ContentUnavailableView("Vixen is still starting", systemImage: "hourglass")
            }
        }
        .frame(width: 440)
        // A Settings window left open stays out of a dialog's way, as upstream's modal Options can't be open alongside one.
        .disabled(admin.isBusy)
        .navigationTitle("Options")
        .onAppear { options = core?.options() }
        .onChange(of: core == nil) { options = core?.options() }
        .onDisappear { admin.saveOptionsIfChanged() }
    }

    private func form(_ current: OptionsRecord) -> some View {
        Form {
            Section {
                LabeledContent("Update Interval:") {
                    NumberStepper(
                        value: Binding(get: { Int(current.updateIntervalMs) }, set: { change(\.updateIntervalMs, Int32($0)) }),
                        range: 0...500,
                        step: 23
                    )
                }
                Toggle("Clear Effect Cache on Exit", isOn: Binding(get: { current.clearEffectCacheOnExit }, set: { change(\.clearEffectCacheOnExit, $0) }))
                    .help("Frees disk space, but adds some\ntime to rebuild at sequence open")
            }
            Section("Audio") {
                LabeledContent("Latency:") {
                    NumberStepper(
                        value: Binding(get: { Int(current.audioLatencyMs) }, set: { change(\.audioLatencyMs, Int32($0)) }),
                        range: 5...500,
                        step: 10
                    )
                }
            }
            Section("Video Effect Cache") {
                Picker("Cache File Type:", selection: Binding(get: { current.videoEffectCacheFileType }, set: { change(\.videoEffectCacheFileType, $0) })) {
                    // A value from the file that isn't one of the two stays on offer, so it survives.
                    ForEach(fileTypes(including: current.videoEffectCacheFileType), id: \.self) { type in
                        Text(type).tag(type)
                    }
                }
                .help("bmp - Fastest/Bigger\npng - Fast/Smaller")
            }
            Section {
                Text("This form lets you set various options that control V3's operation")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .scrollDisabled(true)
        .fixedSize(horizontal: false, vertical: true)
    }

    private func fileTypes(including stored: String) -> [String] {
        ["bmp", "png"].contains(stored) ? ["bmp", "png"] : ["bmp", "png", stored]
    }

    private func change<Value>(_ member: WritableKeyPath<OptionsRecord, Value>, _ value: Value) {
        guard var updated = options else { return }
        updated[keyPath: member] = value
        options = updated
        do {
            try core?.setOptions(options: updated)
        } catch {
            logError(logger: "VixenApplication.OptionsDialog", message: "Couldn't apply the options.", error: error.userMessage)
        }
    }

}

/// `NumericUpDown`: a field and a stepper, kept within its range.
private struct NumberStepper: View {
    @Binding var value: Int
    let range: ClosedRange<Int>
    let step: Int

    var body: some View {
        HStack(spacing: 4) {
            TextField("", value: Binding(get: { value }, set: { value = min(max($0, range.lowerBound), range.upperBound) }), format: .number.grouping(.never))
                .labelsHidden()
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .frame(width: 56)
            Stepper("", value: $value, in: range, step: step)
                .labelsHidden()
        }
    }
}
