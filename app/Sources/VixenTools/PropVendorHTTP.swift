import Foundation
import VixenBridge
import VixenKit

nonisolated final class PropVendorHTTP: PropVendorClient, @unchecked Sendable {
    let session: URLSession
    let tests: Bool
    init(tests: Bool) {
        self.tests = tests
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 20
        configuration.timeoutIntervalForResource = 30
        session = URLSession(configuration: configuration, delegate: RedirectGuard(tests: tests), delegateQueue: nil)
    }
    func cancel() { session.invalidateAndCancel() }
    static func allowed(_ url: URL?, tests: Bool) -> Bool {
        guard let url, ["http", "https"].contains(url.scheme?.lowercased() ?? "") else { return false }
        return !tests || ["127.0.0.1", "localhost", "[::1]", "::1"].contains(url.host ?? "")
    }
    func get(url text: String) -> PropVendorFetch {
        guard let url = URL(string: text), Self.allowed(url, tests: tests) else { return PropVendorFetch(bytes: Data(), error: tests ? "External network is disabled in UI tests." : "Invalid download URL.") }
        var request = URLRequest(url: url)
        request.setValue("Mozilla/5.0 (compatible; Vixen/1.0)", forHTTPHeaderField: "User-Agent")
        let completed = DispatchSemaphore(value: 0), box = ResponseBox()
        let task = session.dataTask(with: request) { data, response, error in
            if let error { box.value = PropVendorFetch(bytes: Data(), error: error.localizedDescription) }
            else if let response = response as? HTTPURLResponse, (200..<300).contains(response.statusCode), let data {
                box.value = data.count <= 128 * 1024 * 1024 ? PropVendorFetch(bytes: data, error: nil) : PropVendorFetch(bytes: Data(), error: "The downloaded file is too large.")
            } else { box.value = PropVendorFetch(bytes: Data(), error: "The server returned an unsuccessful response.") }
            completed.signal()
        }
        task.resume(); completed.wait(); return box.value
    }
    private final class ResponseBox: @unchecked Sendable { var value = PropVendorFetch(bytes: Data(), error: "No response.") }
    private final class RedirectGuard: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
        let tests: Bool
        init(tests: Bool) { self.tests = tests }
        func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping @Sendable (URLRequest?) -> Void) {
            completionHandler(PropVendorHTTP.allowed(request.url, tests: tests) ? request : nil)
        }
    }
}
