import SwiftUI

/// Files copied into the app bundle's Resources folder.
public enum VixenResources {
    public static func url(_ relativePath: String) -> URL? {
        Bundle.main.resourceURL?.appending(path: relativePath)
    }

    public static func image(_ relativePath: String) -> NSImage? {
        guard let url = url(relativePath) else { return nil }
        return NSImage(contentsOf: url)
    }
}
