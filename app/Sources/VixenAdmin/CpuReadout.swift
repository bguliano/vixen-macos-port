import Observation
import VixenBridge

/// The status bar's `CPU: n%`, in its own object, so a tick once a second only touches the one `Text` that reads it. It shows `Resource Usage`
/// until there's a real reading.
@Observable
final class CpuReadout {
    private(set) var text = "Resource Usage"
    private var task: Task<Void, Never>?

    func start() {
        guard task == nil else { return }
        let reader = CpuReader()
        task = Task { [weak self] in
            while !Task.isCancelled {
                if let usage = reader.usage() {
                    self?.text = "CPU: \(usage)%"
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    func stop() {
        task?.cancel()
        task = nil
    }
}
