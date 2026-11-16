import SwiftUI
import VixenBridge
import VixenKit

/// An alert with verbs on its buttons. The value of the button pressed comes back from `SetupModel.ask`.
struct SetupAlert: Identifiable {
    struct Choice: Identifiable {
        let title: String
        var role: ButtonRole?
        let value: Int
        var id: Int { value }
    }

    let id = UUID()
    let title: String
    let message: String
    let choices: [Choice]
    let finish: (Int) -> Void
}

/// A property, filter or controller dialog the window asks for. These are the entry points the setup dialogs hang off: `DialogEntryPoint` shows the real sheet when the
/// `VixenDialogs` area is in the build, and a sheet naming the dialog otherwise.
enum SetupDialogRequest {
    /// F87. `name` for the placeholder.
    case controller(id: String, name: String, typeName: String)
    /// F124.
    case filter(instanceID: String, typeName: String)
    /// F19.
    case property(element: String, selection: [String], typeID: String, typeName: String)
    /// F15, F69 (Patching Order), F100 steps 5 and 6.
    case helper(name: String, nodes: [String], simple: Bool, startFullColor: Bool)
}

/// A sheet the window is showing.
struct SetupSheet: Identifiable {
    enum Kind {
        case text(title: String, prompt: String, initial: String, selectAll: Bool, finish: (String?) -> Void)
        case number(title: String, prompt: String, value: Int, min: Int, max: Int, finish: (Int?) -> Void)
        case list(title: String, items: [SetupModuleType], finish: (SetupModuleType?) -> Void)
        case template(draft: SetupTemplateDraft, remember: (SetupTemplateDraft) -> Void, finish: (SetupTemplateDraft?) -> Void)
        case nameGenerator(seed: String, oldNames: [String]?, finish: ([String]?) -> Void)
        case renamer(oldNames: [String], finish: ([String]?) -> Void)
        case tagColors(rows: [SetupTagColor], finish: ([SetupTagColor]?) -> Void)
        case wireDiagram(finish: (SetupWireOptions?) -> Void)
        case dialog(SetupDialogRequest, finish: (Bool) -> Void)
    }

    let id = UUID()
    let kind: Kind
}

/// What the elements panel's actions tell the rest of the window (F4).
enum ElementsChange {
    case add
    case remove
    case rename
    case edit
}

/// The status ball colours. Rust says which, and these are the pictures.
enum SetupBall {
    static let green: UInt32 = 0xFF34_C759
    static let red: UInt32 = 0xFFFF_3B30
    static let white: UInt32 = 0xFFF2_F2F2
    static let grey: UInt32 = 0xFF8E_8E93
}

extension SetupRowIcon {
    var picture: OutlineRowIcon {
        switch self {
        case .group: .folder
        case .patched: .ball(SetupBall.green)
        case .patchedMasked: .ball(SetupBall.red)
        case .unpatched: .ball(SetupBall.white)
        }
    }
}

extension SetupControllerIcon {
    var picture: OutlineRowIcon {
        switch self {
        case .group: .folder
        case .stopped: .ball(SetupBall.red)
        case .outputUnpatched: .ball(SetupBall.white)
        case .outputBroken: .ball(SetupBall.grey)
        case .outputPatched: .ball(SetupBall.green)
        }
    }
}

extension SetupElementRow {
    nonisolated var outlineRow: OutlineRow {
        OutlineRow(id: path, title: name, hasChildren: hasChildren, icon: icon.outlinePicture, trailingDots: tagColors, childCount: Int(childCount))
    }
}

extension SetupRowIcon {
    /// `picture`, for building rows on a background task.
    nonisolated var outlinePicture: OutlineRowIcon {
        switch self {
        case .group: .folder
        case .patched: .ball(0xFF34_C759)
        case .patchedMasked: .ball(0xFFFF_3B30)
        case .unpatched: .ball(0xFFF2_F2F2)
        }
    }
}

extension SetupControllerRow {
    var outlineRow: OutlineRow {
        OutlineRow(id: id, title: title, hasChildren: hasChildren, icon: icon.picture, childCount: Int(childCount))
    }
}

extension SetupElementStats {
    static let none = SetupElementStats(total: 0, connected: 0, unconnected: 0, items: 0, groups: 0, leaves: 0, filters: 0)
}

extension SetupControllerStats {
    static let none = SetupControllerStats(outputs: 0, patched: 0, unpatched: 0, controllers: 0, firstLabel: "", lastLabel: "")
}

extension SetupPatchSummary {
    static let none = SetupPatchSummary(sources: 0, destinations: 0, text: "", warning: "", canPatch: false)
}

extension SetupControllerPanelState {
    static let none = SetupControllerPanelState(
        typeLabel: "", outputsLabel: "", oneController: false, anyController: false, canStart: false, canStop: false, canFindElements: false, hasSetup: false
    )
}
