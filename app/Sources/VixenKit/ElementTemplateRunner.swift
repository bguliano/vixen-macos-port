import AppKit

/// What an element template made, as node ids: `nodes` are what a drawn shape links to, the first to the shape itself and each further one to a copy of it,
/// and `selection` is what the element tree selects afterwards.
public struct ElementTemplateResult: Sendable {
    public var nodes: [String]
    public var selection: [String]

    public init(nodes: [String], selection: [String]) {
        self.nodes = nodes
        self.selection = selection
    }
}

/// A template's dialog values for as long as its owner lives. Upstream's template objects remember what was last typed, and a window that keeps its
/// templates, like the preview designer's Add row, hands the same one back each time.
public final class ElementTemplateMemory {
    public var values: [String: Any] = [:]

    public init() {}
}

/// The element template wizards belong to Display Setup, and the preview designer runs them too. Display Setup fills this in when it's in the build.
public enum ElementTemplateRunner {
    /// Runs the named template as sheets on `window`, and applies what it made to the running display without saving. Nil when cancelled or when
    /// nothing was made.
    public static var run: ((_ name: String, _ window: NSWindow, _ app: AppModel, _ memory: ElementTemplateMemory?) async -> ElementTemplateResult?)?
}
