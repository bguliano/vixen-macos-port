import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// `Form_Effects`. F320: a tree with "Basic Lighting", "Pixel Lighting", "Device Action" and "Intelligent Fixture" as roots, the first two open. A click chooses the effect to draw, or
/// clears it; dragging places it. About sixty rows, so SwiftUI is fine here.
struct EffectsPanel: View {
    let model: SequencerModel
    @State private var expanded: Set<ShellEffectGroup> = [.basic, .pixel]

    private static let groups: [(ShellEffectGroup, String)] = [
        (.basic, "Basic Lighting"),
        (.pixel, "Pixel Lighting"),
        (.device, "Device Action"),
        (.intelligentFixture, "Intelligent Fixture"),
    ]

    var body: some View {
        List {
            ForEach(Self.groups, id: \.0) { group, title in
                DisclosureGroup(
                    isExpanded: Binding(
                        get: { expanded.contains(group) },
                        set: { open in
                            if open {
                                expanded.insert(group)
                            } else {
                                expanded.remove(group)
                            }
                        }
                    )
                ) {
                    ForEach(model.effects.filter { $0.group == group }, id: \.typeId) { effect in
                        EffectRow(model: model, effect: effect, group: group)
                    }
                } label: {
                    // Full-row click on a group toggles it.
                    Text(title)
                        .fontWeight(.medium)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if expanded.contains(group) {
                                expanded.remove(group)
                            } else {
                                expanded.insert(group)
                            }
                        }
                }
            }
        }
        .listStyle(.sidebar)
        .scrollContentBackground(.hidden)
        .environment(\.defaultMinListRowHeight, 22)
    }
}

/// One effect, with its picture at 18 points.
private struct EffectRow: View {
    let model: SequencerModel
    let effect: ShellEffect
    let group: ShellEffectGroup

    private var chosen: Bool {
        model.ui?.selectedEffect == effect.typeId
    }

    var body: some View {
        HStack(spacing: 6) {
            ToolIcon(image: .effect(effect.icon), size: 18)
            Text(effect.name)
                .lineLimit(1)
            Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
        .padding(.vertical, 1)
        .padding(.horizontal, 3)
        // Upstream's chosen row is a blue back colour.
        .background(chosen ? Color.blue.opacity(0.6) : Color.clear, in: RoundedRectangle(cornerRadius: 3))
        .overlay { EffectDragSource(typeId: effect.typeId, icon: effect.icon, clicked: click) }
        .help(effect.name)
        .accessibilityAddTraits(chosen ? [.isSelected, .isButton] : .isButton)
        .accessibilityLabel(effect.name)
    }

    private func click() {
        if group == .device {
            Task {
                await SequencerAlerts.tell(
                    title: "Effect Selection",
                    message: "Currently, you must drag this item to the grid to place it.",
                    style: .information,
                    window: model.window
                )
            }
            return
        }
        // It only selects; it doesn't switch Draw Mode on.
        _ = model.shell?.toggleSelectedEffect(typeId: effect.typeId)
        model.refresh()
    }
}
