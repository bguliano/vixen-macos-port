import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// Tools > Audio > Beat/Bar Detection (`docs/spec/marks-layers-analysis.md` F67 to F75) as one sheet: a spinner while the first ten seconds
/// analyse, the Beats and Bars dialog, then the progress with a Cancel. `onFinish` runs once it's over, whatever the outcome; the
/// collections are already in the sequence by then.
public struct BeatsAndBarsSheet: View {
    private let session: SequencerSession
    private let onFinish: () -> Void
    @State private var model: BeatsSheetModel?

    public init(session: SequencerSession, onFinish: @escaping () -> Void) {
        self.session = session
        self.onFinish = onFinish
    }

    public var body: some View {
        Group {
            if let model {
                BeatsSheetContent(model: model, onFinish: onFinish)
            } else {
                Color.clear.frame(width: 320, height: 120)
            }
        }
        .onAppear {
            if model == nil {
                model = BeatsSheetModel(session: session)
            }
        }
    }
}

@Observable
final class BeatsSheetModel {
    let detector: BeatDetector
    private(set) var job: BeatJobState = .preparing
    private(set) var dialog: BeatDialogState
    private(set) var preview: BeatPreview?
    var failure: String?

    init(session: SequencerSession) {
        detector = session.startBeatDetection()
        dialog = detector.dialog()
        BeatsTestHooks.current = self
    }

    /// Follows the worker ten times a second until it's over.
    func follow(onFinish: @escaping () -> Void) async {
        while !Task.isCancelled {
            let now = detector.job()
            if now != job {
                job = now
                if case .ready = now {
                    refresh()
                }
            }
            switch now {
            case .finished, .cancelled:
                onFinish()
                return
            case let .failed(message):
                failure = message
                return
            default:
                break
            }
            try? await Task.sleep(for: .milliseconds(100))
        }
    }

    func refresh() {
        dialog = detector.dialog()
        preview = detector.preview()
    }

    func setOutput(_ output: BeatOutput, _ on: Bool) {
        detector.setOutput(output: output, on: on)
        refresh()
    }

    func setSplitBeats(_ on: Bool) {
        detector.setSplitBeats(on: on)
        refresh()
    }

    func setBeatsPerBar(_ bpb: Int32) {
        detector.setBeatsPerBar(beatsPerBar: bpb)
        refresh()
    }

    func setBaseName(_ name: String) {
        detector.setBaseName(name: name)
        dialog = detector.dialog()
    }

    func chooseColor(_ output: BeatOutput, current: UInt32, title: String) {
        ColorPanelSession.shared.begin(title: title, argb: current) { [weak self] argb in
            self?.detector.setOutputColor(output: output, argb: argb)
            self?.refresh()
        }
    }

    func generate() {
        ColorPanelSession.shared.finish()
        detector.generate()
        job = detector.job()
    }

    func cancel() {
        ColorPanelSession.shared.finish()
        detector.cancel()
    }
}

struct BeatsSheetContent: View {
    @Bindable var model: BeatsSheetModel
    let onFinish: () -> Void

    var body: some View {
        Group {
            switch model.job {
            case .preparing:
                VStack(spacing: 14) {
                    ProgressView()
                    Text("Analysing the first ten seconds…")
                    Button("Cancel") { model.cancel() }
                        .keyboardShortcut(.cancelAction)
                }
                .padding(24)
                .frame(width: 320)
            case let .running(percent, finalizing):
                BeatsProgressView(percent: percent, finalizing: finalizing) { model.cancel() }
            default:
                BeatsDialogView(model: model)
            }
        }
        .task { await model.follow(onFinish: onFinish) }
        .alert("Beats and Bars", isPresented: Binding(get: { model.failure != nil }, set: { if !$0 { model.failure = nil; onFinish() } })) {
            Button("OK") {}
        } message: {
            Text(model.failure ?? "")
        }
    }
}

/// F69: three groups, Generate and Cancel. Return does nothing, as upstream, so a Return in the name field can't start an analysis; Escape
/// cancels (FIX).
struct BeatsDialogView: View {
    @Bindable var model: BeatsSheetModel
    @State private var baseName = ""

    var body: some View {
        let d = model.dialog
        VStack(alignment: .leading, spacing: 10) {
            Text("Beats and Bars").font(.headline)
            GroupBox("Divisions") {
                VStack(alignment: .leading, spacing: 6) {
                    HStack(spacing: 14) {
                        readout("BPM:", d.bpmText)
                        readout("Bar Period (sec):", d.barPeriodText)
                        readout("Div Time (ms):", d.divTimeText)
                        Spacer()
                    }
                    MusicStaffMenu(model: model)
                    HStack {
                        Text("Click staff to change time signature").foregroundStyle(.secondary)
                        Spacer()
                        Toggle("Split Beats", isOn: Binding(get: { d.splitBeats }, set: { model.setSplitBeats($0) }))
                            .toggleStyle(.checkbox)
                    }
                }
                .padding(6)
            }
            GroupBox("Preview") {
                BeatsPreviewStrip(preview: model.preview)
                    .frame(height: 70)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 30)
            }
            GroupBox("Output") {
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Text("Collection Base Name:")
                        TextField("Base name", text: $baseName)
                            .labelsHidden()
                            .frame(width: 220)
                            .help("Base name of each collection")
                            .onChange(of: baseName) { model.setBaseName(baseName) }
                        Spacer()
                    }
                    HStack(spacing: 18) {
                        output("Bars", .bars, d.barsChecked, d.barsArgb, enabled: true, swatchEnabled: d.barsColorEnabled,
                               tip: "Single Collection containing starting location of each measure/bar", swatchTip: "Color of Bars Collection")
                        output("Beat Counts", .beatCounts, d.beatCountsChecked, d.beatCountsArgb, enabled: true, swatchEnabled: d.beatCountsColorEnabled,
                               tip: "Generates a beat collection for each beat count", swatchTip: "Color of Beat Counts Collection")
                        output("Beat Splits", .beatSplits, d.beatSplitsChecked, d.beatSplitsArgb, enabled: d.beatSplitsCheckboxEnabled, swatchEnabled: d.beatSplitsColorEnabled,
                               tip: "Generates a beat collection for each beat count and each beat count split", swatchTip: "Color of Beat Splits Collection")
                        output("All Features", .allFeatures, d.allFeaturesChecked, d.allArgb, enabled: true, swatchEnabled: d.allColorEnabled,
                               tip: "Single Collection containing all features", swatchTip: "Color of All Features Collection")
                    }
                }
                .padding(6)
            }
            HStack {
                Spacer()
                Button("Generate") { model.generate() }
                    .disabled(!d.generateEnabled)
                Button("Cancel") { model.cancel() }
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(16)
        .frame(width: 740)
        .onAppear { baseName = model.dialog.baseName }
    }

    private func readout(_ label: String, _ value: String) -> some View {
        HStack(spacing: 4) {
            Text(label).bold()
            Text(value).monospacedDigit()
        }
    }

    private func output(_ title: String, _ output: BeatOutput, _ on: Bool, _ argb: UInt32, enabled: Bool, swatchEnabled: Bool, tip: String, swatchTip: String) -> some View {
        HStack(spacing: 6) {
            Toggle(title, isOn: Binding(get: { on }, set: { model.setOutput(output, $0) }))
                .toggleStyle(.checkbox)
                .disabled(!enabled)
                .help(tip)
            Button { model.chooseColor(output, current: argb, title: swatchTip) } label: {
                RoundedRectangle(cornerRadius: 3)
                    .fill(Color(argb: argb))
                    .overlay(RoundedRectangle(cornerRadius: 3).stroke(Color.secondary, lineWidth: 1))
                    .frame(width: 28, height: 24)
                    .opacity(swatchEnabled ? 1 : 0.35)
            }
            .buttonStyle(.plain)
            .disabled(!swatchEnabled)
            .help(swatchTip)
            .accessibilityLabel(swatchTip)
        }
    }
}

/// F70: the staff, whose click opens the time signature menu, and the same menu on a right-click (FIX: from the start, always current).
struct MusicStaffMenu: View {
    let model: BeatsSheetModel

    var body: some View {
        let d = model.dialog
        Menu {
            signatureItems(current: d.beatsPerBar)
        } label: {
            MusicStaffView(beatsPerBar: Int(d.beatsPerBar), noteSize: Int(d.noteSize), split: d.splitBeats)
                .frame(height: 96)
                .frame(maxWidth: .infinity)
                .contentShape(Rectangle())
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .menuIndicator(.hidden)
        .contextMenu { signatureItems(current: d.beatsPerBar) }
        .accessibilityLabel("Time signature \(d.beatsPerBar)/\(d.noteSize)")
    }

    @ViewBuilder private func signatureItems(current: Int32) -> some View {
        ForEach(beatTimeSignatures(), id: \.self) { pair in
            Toggle("\(pair[0])/\(pair[1])", isOn: Binding(get: { pair[0] == current }, set: { _ in model.setBeatsPerBar(pair[0]) }))
        }
    }
}

/// The staff picture with the time signature on the left and a note per division above it, as `MusicStaff` paints them.
struct MusicStaffView: View {
    let beatsPerBar: Int
    let noteSize: Int
    let split: Bool

    var body: some View {
        Canvas { ctx, size in
            let staffWidth = size.width - 17
            let staffHeight: CGFloat = 52
            let staffTop = size.height - staffHeight - 4
            if let staff = MarksImage.load("beats/fullstaff") {
                ctx.draw(Image(nsImage: staff).resizable(), in: CGRect(x: 0, y: staffTop, width: staffWidth, height: staffHeight))
            }
            for (n, y) in [(beatsPerBar, staffTop + staffHeight * 0.33), (noteSize, staffTop + staffHeight * 0.72)] {
                ctx.draw(Text("\(n)").font(.system(size: 15, weight: .bold)).foregroundStyle(.black), at: CGPoint(x: 80, y: y), anchor: .center)
            }
            let marksInBar = beatsPerBar * (split ? 2 : 1)
            let glyph = Self.noteImage(noteSize * (split ? 2 : 1))
            let firstBarOffset: CGFloat = 125
            guard marksInBar > 0, let glyph else { return }
            let interval = (staffWidth - 16 - firstBarOffset) / CGFloat(marksInBar)
            let noteHeight: CGFloat = 34
            let noteWidth = noteHeight * glyph.size.width / max(glyph.size.height, 1)
            for j in 0..<marksInBar {
                let x = firstBarOffset + interval * CGFloat(j)
                ctx.draw(Image(nsImage: glyph).resizable(), in: CGRect(x: x, y: staffTop - 20, width: noteWidth, height: noteHeight))
            }
        }
    }

    static func noteImage(_ size: Int) -> NSImage? {
        switch size {
        case 2: MarksImage.load("beats/halfnote")
        case 4: MarksImage.load("beats/quarternote")
        case 8: MarksImage.load("beats/eigthnote")
        case 16: MarksImage.load("beats/sixteenthnote")
        case 32: MarksImage.load("beats/32ndNote")
        default: nil
        }
    }
}

/// F73: the waveform from 0 over the preview span in `Full` style on light grey, and a yellow line per preview mark, each at the first pixel
/// whose time reaches it, one mark per pixel at most, as `PreviewWaveform` places them. Not interactive.
struct BeatsPreviewStrip: View {
    let preview: BeatPreview?

    var body: some View {
        Canvas { ctx, size in
            let rect = CGRect(origin: .zero, size: size)
            ctx.fill(Path(rect), with: .color(Color(white: 0.83)))
            ctx.stroke(Path(rect.insetBy(dx: 0.5, dy: 0.5)), with: .color(.black), lineWidth: 1)
            guard let preview else { return }
            let width = Int(size.width)
            let columns = preview.peaks.count / 2
            if columns > 0, width > 0 {
                var wave = Path()
                let mid = size.height / 2
                for x in 0..<width {
                    let i = min(columns - 1, x * columns / width)
                    let lo = CGFloat(preview.peaks[i * 2]), hi = CGFloat(preview.peaks[i * 2 + 1])
                    wave.move(to: CGPoint(x: CGFloat(x) + 0.5, y: mid - hi * mid))
                    wave.addLine(to: CGPoint(x: CGFloat(x) + 0.5, y: mid - lo * mid))
                }
                ctx.stroke(wave, with: .color(Color(red: 0.1, green: 0.1, blue: 0.4)), lineWidth: 1)
            }
            // `timePerPixel` in whole ticks, as upstream divides them.
            let periodTicks = Int64(preview.periodMs * 10_000)
            let timePerPixel = width > 0 ? periodTicks / Int64(width) : 0
            let marks = preview.linesMs.map { Int64($0 * 10_000) }
            var next = 0
            var lines = Path()
            for j in 0..<max(width, 0) where next < marks.count {
                if marks[next] <= timePerPixel * Int64(j) {
                    lines.addRect(CGRect(x: CGFloat(j), y: 0, width: 2, height: size.height))
                    next += 1
                }
            }
            ctx.fill(lines, with: .color(.yellow))
        }
    }
}

/// F74: "Generating Marks:" with a percentage, then "Finalizing beat analysis..." with an indeterminate bar. GUARD: a Cancel, which upstream
/// lacks.
struct BeatsProgressView: View {
    let percent: Int32
    let finalizing: Bool
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Beats and Bars").font(.headline)
            if finalizing {
                Text("Finalizing beat analysis...")
                ProgressView().progressViewStyle(.linear)
            } else {
                Text("Generating Marks:")
                HStack {
                    ProgressView(value: Double(percent), total: 100).progressViewStyle(.linear)
                    Text("\(percent)%").monospacedDigit().frame(width: 44, alignment: .trailing)
                }
            }
            HStack {
                Spacer()
                Button("Cancel", action: onCancel)
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(20)
        .frame(width: 380)
    }
}

/// What test scripts can reach while the sheet is up.
enum BeatsTestHooks {
    static weak var current: BeatsSheetModel?
}
