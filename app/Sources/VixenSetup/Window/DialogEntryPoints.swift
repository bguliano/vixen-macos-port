import SwiftUI
import VixenBridge
import VixenKit
#if canImport(VixenDialogs)
import VixenDialogs
#endif

/// Every property, filter and controller dialog the window opens comes through here (`SetupDialogRequest`). With the `VixenDialogs` area in the build it shows the real dialog, working on this
/// window's working copy, so Cancel on the window drops what it did. Otherwise it's a sheet naming what will go here.
struct DialogEntryPoint: View {
    let request: SetupDialogRequest
    let session: DisplaySetupSession?
    let onFinish: (Bool) -> Void

    var body: some View {
        #if canImport(VixenDialogs)
        if let session {
            real(session.dialogContext())
        } else {
            placeholder
        }
        #else
        placeholder
        #endif
    }

    #if canImport(VixenDialogs)
    @ViewBuilder
    private func real(_ context: DialogContext) -> some View {
        switch request {
        case .controller(let id, _, _):
            ControllerSetupSheet(controller: id, context: context, onFinish: onFinish)
        case .filter(let instanceID, _):
            FilterSetupSheet(filter: instanceID, context: context, onFinish: onFinish)
        case .property(let element, let selection, let typeID, _):
            PropertySetupSheet(element: element, selection: selection, property: typeID, context: context, onFinish: onFinish)
        case .helper(let name, let nodes, let simple, let startFullColor):
            SetupHelperSheet(
                helper: Self.helper(named: name),
                nodes: nodes,
                simple: simple,
                colorType: startFullColor ? .fullColor : nil,
                context: context,
                onFinish: onFinish
            )
        }
    }

    private static func helper(named name: String) -> DialogSetupHelper {
        switch name {
        case "Dimming Curve": .dimmingCurve
        case "Face Mapping": .faceMapping
        case "Orientation": .orientation
        case "Patching Order": .patchingOrder
        case "State Mapping": .stateMapping
        default: .colorHandling
        }
    }
    #endif

    private var placeholder: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.headline)
            Text(detail)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("OK") { onFinish(false) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 420)
    }

    private var title: String {
        switch request {
        case .controller(_, let name, let typeName): "\(typeName) Setup: \(name)"
        case .filter(_, let typeName): "\(typeName) Setup"
        case .property(_, _, _, let typeName): "\(typeName) Setup"
        case .helper(let name, _, _, _): "\(name) Helper"
        }
    }

    private var detail: String {
        switch request {
        case .controller:
            "The controller setup dialog goes here. It is specified in docs/spec/property-filter-controller-setup.md, and this build doesn't have the VixenDialogs area."
        case .filter:
            "The output filter setup dialog goes here. It is specified in docs/spec/property-filter-controller-setup.md, and this build doesn't have the VixenDialogs area."
        case .property:
            "The element property setup dialog goes here. It is specified in docs/spec/property-filter-controller-setup.md, and this build doesn't have the VixenDialogs area."
        case .helper(_, let nodes, _, _):
            "The setup helper for \(nodes.count) element(s) goes here. It is specified in docs/spec/property-filter-controller-setup.md, and this build doesn't have the VixenDialogs area."
        }
    }
}
