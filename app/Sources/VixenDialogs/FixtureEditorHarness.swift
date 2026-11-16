import Foundation
import VixenBridge
import VixenKit

/// Small deterministic hooks for the same mutations the table cells invoke. Native sheets/buttons
/// and their validation state remain in the UI path; this avoids coordinate-dependent table editing.
@MainActor enum FixtureEditorHarness {
    static weak var model: FixtureEditorModel?
    static func use(_ value: FixtureEditorModel) {
        guard UITestHarness.isActive else { return }
        model = value
        UITestHarness.customActions["fixtureInfo"] = { _ in
            guard let m = model else { return (false, "no fixture editor") }
            let s = m.state
            return (true, "profile=\(s.name) channels=\(s.channels.map { "\($0.number):\($0.name):\($0.function)" }) functions=\(s.functions.map(\.name)) selected=\(s.selectedFunction.map(String.init) ?? "none") panel=\(s.panel) indices=\(s.indices.count) errors=\(s.errors) notice=\(s.notice ?? "none")")
        }
        UITestHarness.customActions["fixtureEdit"] = { step in
            guard let m = model, let text = step.text else { return (false, "no fixture editor/command") }
            let p = text.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
            switch p.first {
            case "addIndex": m.apply { $0.addIndex() }
            case "addChannel": m.apply { $0.addChannel() }
            case "addFunction": m.apply { $0.addFunction() }
            case "removeFunction": if let row = m.state.selectedFunction { m.apply { $0.removeFunction(row: row) } }
            case "functions" where p.count == 2: m.openFunctions(p[1])
            case "profile" where p.count == 3: m.apply { $0.setProfileField(field: p[1], value: p[2]) }
            case "function" where p.count == 4: guard let row = UInt32(p[1]) else { return (false, "invalid row") }; m.function(row, p[2], p[3])
            case "select" where p.count == 2: m.apply { $0.selectFunction(row: UInt32(p[1])) }
            case "index" where p.count == 4: guard let row = UInt32(p[1]) else { return (false, "invalid row") }; m.index(row, p[2], p[3])
            case "channel" where p.count == 4: guard let row = UInt32(p[1]) else { return (false, "invalid row") }; m.apply { $0.setChannelField(row: row, field: p[2], value: p[3]) }
            case "assert" where p.count == 3:
                switch p[1] {
                case "selected": return (m.state.selectedFunction == UInt32(p[2]), "selected=\(String(describing: m.state.selectedFunction))")
                case "panel": return (m.state.panel == p[2], "panel=\(m.state.panel)")
                case "error": return (m.state.errors.joined(separator: "\n").contains(p[2]), m.state.errors.joined(separator: "\n"))
                case "valid": return (m.state.errors.isEmpty, m.state.errors.joined(separator: "\n"))
                default: return (false, "unknown assertion")
                }
            default: return (false, "unknown fixture command")
            }
            return (true, nil)
        }
    }
}
