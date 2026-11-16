import SwiftUI
import VixenBridge
import VixenKit

/// Check for Updates. It only reaches the network because the user opened it, and the answer is about upstream's releases: this port isn't
/// published there, so Download opens the release page rather than fetching a Windows installer.
struct CheckForUpdatesSheet: View {
    @Environment(AdminModel.self) private var admin
    @State private var result: UpdateDialogRecord?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("\(admin.version.versionName) Installed").font(.headline)
            HStack(alignment: .top, spacing: 16) {
                if let image = VixenResources.image("images/VixenImage.png") {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 120, height: 120)
                        .accessibilityHidden(true)
                }
                VStack(alignment: .leading, spacing: 8) {
                    if let result {
                        Text(result.heading)
                            .font(.system(size: 18, weight: .bold))
                        Text(result.versionText)
                            .font(.system(size: 13))
                        if result.showDownload, let page = result.releasePageUri, let url = URL(string: page) {
                            Button("Download") { admin.openExternally(url) }
                                .help("Opens the release page on GitHub. The installers there are for Windows.")
                        }
                    } else {
                        Text("Checking for updates, please wait.")
                            .font(.system(size: 13))
                    }
                }
                Spacer(minLength: 0)
            }
            if let result, result.showNotes {
                Text("Change Log for latest version:")
                ReadOnlyTextView(text: result.notes)
                    .frame(height: 225)
            }
            HStack {
                Spacer()
                Button("OK") { admin.sheet = nil }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 620)
        .task {
            result = await admin.checkForUpdates()
        }
    }
}
