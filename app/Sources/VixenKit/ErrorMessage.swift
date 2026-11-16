import Foundation
import VixenBridge

public extension Error {
    /// What to show the user: the message Rust wrote for them, rather than a description of the error type.
    var userMessage: String {
        if let failure = self as? FfiError, case .Failed(let message) = failure {
            return message
        }
        return localizedDescription
    }
}
