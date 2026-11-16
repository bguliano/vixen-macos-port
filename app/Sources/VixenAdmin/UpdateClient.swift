import Foundation
import VixenBridge

/// The HTTP client the update code uses, on `URLSession`. Rust calls it off the main actor and waits, so it blocks.
nonisolated final class URLSessionUpdateClient: UpdateHttpClient, @unchecked Sendable {
    private let base: String
    private let session: URLSession
    private let settings = updateHttpSettings()

    init(base: String) {
        self.base = base
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = TimeInterval(settings.timeoutSeconds)
        configuration.timeoutIntervalForResource = TimeInterval(settings.timeoutSeconds)
        session = URLSession(configuration: configuration)
    }

    func get(relativeUri: String) -> UpdateHttpOutcome {
        guard let url = URL(string: base + relativeUri) else {
            return .failed(reason: "Not an address: \(relativeUri)")
        }
        var request = URLRequest(url: url)
        request.setValue(settings.userAgent, forHTTPHeaderField: "User-Agent")
        request.setValue(settings.accept, forHTTPHeaderField: "Accept")
        request.setValue(settings.apiVersion, forHTTPHeaderField: settings.apiVersionHeader)

        let finished = DispatchSemaphore(value: 0)
        let outcome = OutcomeBox()
        let task = session.dataTask(with: request) { data, response, error in
            if let error = error as? URLError, error.code == .timedOut {
                outcome.value = .timedOut
            } else if let error {
                outcome.value = .failed(reason: error.localizedDescription)
            } else if let response = response as? HTTPURLResponse {
                outcome.value = .response(
                    status: UInt16(clamping: response.statusCode),
                    body: String(data: data ?? Data(), encoding: .utf8) ?? "",
                    rateLimitRemaining: response.value(forHTTPHeaderField: "X-RateLimit-Remaining")
                )
            } else {
                outcome.value = .failed(reason: "No response")
            }
            finished.signal()
        }
        task.resume()
        finished.wait()
        return outcome.value
    }

    private final class OutcomeBox: @unchecked Sendable {
        var value = UpdateHttpOutcome.failed(reason: "No response")
    }
}
