import SwiftUI
import VixenBridge
import VixenKit

/// Opens the Display Setup sheets on their own, so each can be exercised and photographed through the UI harness. Window id `setup-sheets-test`.
/// Add a section of your own to `body` rather than editing someone else's.
struct SetupSheetsTestView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            NamingAndTemplateSheetsTestSection()
            WireDiagramSheetTestSection()
            FixtureWizardTestSection()
        }
        .padding(16)
        .frame(minWidth: 640, minHeight: 360, alignment: .topLeading)
    }
}

/// Wire Diagram Export Options. What it hands back goes to standard error and beside the button.
private struct WireDiagramSheetTestSection: View {
    @State private var showing = false
    @State private var result = "-"

    var body: some View {
        HStack {
            Button("Open Wire Diagram") { showing = true }
            Text(result)
                .font(.callout.monospaced())
                .textSelection(.enabled)
        }
        .sheet(isPresented: $showing) {
            WireDiagramSheet { options in
                showing = false
                result = options.map { "all \($0.allNumbers), interval \($0.intervalText), marker \($0.markerSize), label \($0.labelSize), scale \($0.scalePercent)" } ?? "cancelled"
                FileHandle.standardError.write(Data("SHEET wire: \(result)\n".utf8))
            }
        }
    }
}
