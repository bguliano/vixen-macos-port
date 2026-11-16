import AppKit

/// What an edit to the elements did, for the tree that asked. Rows are node ids joined by "/" from a root down, since an element can sit under two parents.
nonisolated public struct ElementTreeChange: Sendable {
    /// The rows to select afterwards, or nil to leave the selection as it is.
    public var select: [String]?
    public var expand: [String]

    public init(select: [String]? = nil, expand: [String] = []) {
        self.select = select
        self.expand = expand
    }
}

/// What the editor needs from the window whose tree it edits.
public struct ElementTreeHost {
    public var app: AppModel
    /// Where its sheets and alerts go.
    public var window: () -> NSWindow?
    public var collapseAll: () -> Void
    /// The running display has the edit by now. Reload the tree, then select and expand as told.
    public var changed: (ElementTreeChange) async -> Void

    public init(app: AppModel, window: @escaping () -> NSWindow?, collapseAll: @escaping () -> Void, changed: @escaping (ElementTreeChange) async -> Void) {
        self.app = app
        self.window = window
        self.collapseAll = collapseAll
        self.changed = changed
    }
}

/// The element tree's context menu, keys and drags for a tree outside Display Setup: the preview designer's Elements panel, which upstream is the same
/// `ElementTree` control with property editing off and wire export on (`docs/spec/preview.md` F82, F85, F86). Edits reach the running display at once and
/// nothing is saved, as upstream's tree edits the live nodes and leaves saving to the window it sits in.
public protocol ElementTreeEditing: AnyObject {
    /// `rows` in display order, `primary` the row last acted on.
    func menu(rows: [String], primary: String?) -> NSMenu?
    func canPerform(_ command: OutlineCommand, rows: [String]) -> Bool
    func perform(_ command: OutlineCommand, rows: [String], primary: String?)
    func canDrop(items: [String], parent: String?, index: Int?, copy: Bool) -> Bool
    func drop(items: [String], parent: String?, index: Int?, copy: Bool)
    /// The asking window wrote to the display itself, so whatever the editor holds is out of date.
    func invalidate()
}

/// Display Setup owns the element tree's editing and fills this in. Nil in a build without that area, where the tree is read-only.
public enum ElementTreeEditor {
    public static var make: ((ElementTreeHost) -> any ElementTreeEditing)?
}
