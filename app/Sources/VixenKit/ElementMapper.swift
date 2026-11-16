import AppKit
import VixenBridge

nonisolated public struct ElementMappingSource: Sendable {
    public var id: String
    public var name: String
    public init(id: String, name: String) { self.id = id; self.name = name }
}

nonisolated public struct ElementMappingResult: Sendable {
    public var sourceId: String
    public var targetId: String
    public init(sourceId: String, targetId: String) { self.sourceId = sourceId; self.targetId = targetId }
}

/// The scheduler area supplies this sheet without making the sequencer depend on its module.
/// Cancel returns nil. Accepted pairs are applied by the caller, after this private editor closes.
public enum ElementMapper {
    public static var present: ((_ core: VixenCore, _ name: String, _ sources: [ElementMappingSource], _ window: NSWindow?, _ completion: @escaping ([ElementMappingResult]?) -> Void) -> Void)?
}
