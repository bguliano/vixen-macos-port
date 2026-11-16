// System WebKit verification of the unchanged page against loopback fixtures.
// Add fourth argument `real` for the private muted vixen-session web_profile_fixture.
// swiftc -parse-as-library -framework AppKit -framework WebKit scripts/test_web_page.swift -o target/test-web-page
// target/test-web-page http://127.0.0.1:PORT target/web-page
import AppKit
import WebKit

@main @MainActor struct WebPageSmoke {
    static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let delegate = Harness()
        app.delegate = delegate
        app.run()
    }
}

@MainActor final class Harness: NSObject, NSApplicationDelegate, WKNavigationDelegate {
    var web: WKWebView!
    var window: NSWindow!
    var started = false
    var results: [String] = []
    let real = CommandLine.arguments.count > 3 && CommandLine.arguments[3] == "real"
    let out = URL(fileURLWithPath: CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : "target/web-page", isDirectory: true)
    func applicationDidFinishLaunching(_ notification: Notification) {
        guard CommandLine.arguments.count > 1, let url = URL(string: CommandLine.arguments[1]), url.host == "127.0.0.1" else { finish("Only a loopback fixture URL is allowed"); return }
        let config = WKWebViewConfiguration(); config.websiteDataStore = .nonPersistent()
        let record = """
        window.testRequests=[];window.testErrors=[];
        addEventListener('error',e=>testErrors.push(e.message||'resource error'));
        addEventListener('unhandledrejection',e=>testErrors.push(String(e.reason)));
        const originalOpen=XMLHttpRequest.prototype.open, originalSend=XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.open=function(method,url,...rest){this.testRecord={method,url};return originalOpen.call(this,method,url,...rest)};
        XMLHttpRequest.prototype.send=function(body){if(this.testRecord){this.testRecord.body=body;testRequests.push(this.testRecord);this.addEventListener('loadend',()=>{this.testRecord.status=this.status;this.testRecord.response=this.responseText})}return originalSend.call(this,body)};
        """
        config.userContentController.addUserScript(WKUserScript(source: record, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        web = WKWebView(frame: NSRect(x:0,y:0,width:1100,height:900), configuration: config); web.navigationDelegate = self
        window = NSWindow(contentRect:web.frame, styleMask:[.titled], backing:.buffered, defer:false); window.contentView = web
        window.title = "Vixen loopback page verification"; window.orderBack(nil)
        web.load(URLRequest(url:url))
        Task { try? await Task.sleep(nanoseconds:35_000_000_000); if !started { finish("Page load timed out") } }
    }
    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        decisionHandler(navigationAction.request.url?.host == "127.0.0.1" ? .allow : .cancel)
    }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard !started else { return }; started = true
        Task { do { try await run(); finish(nil) } catch { finish(error.localizedDescription) } }
    }
    func js(_ source: String) async throws -> Any? {
        try await withCheckedThrowingContinuation { continuation in web.evaluateJavaScript(source) { value,error in if let error { continuation.resume(throwing:error) } else { continuation.resume(returning:value) } } }
    }
    func waitFor(_ expression: String, _ label: String) async throws {
        for _ in 0..<100 { if (try await js(expression)) as? Bool == true { results.append(label); return }; try await Task.sleep(nanoseconds:100_000_000) }
        throw NSError(domain:"VixenWebTest",code:1,userInfo:[NSLocalizedDescriptionKey:"Timed out: \(label)"])
    }
    func run() async throws {
        try await waitFor("document.title==='Vixen 3 Web Interface' && document.querySelector('#accordion').textContent.includes('Loopback Element') && document.querySelector('#select-sequence').options.length===2 && document.querySelector('#Controllers').textContent.includes('Loopback Controller')", "initial page, elements, sequences and controllers")
        if !real { try await waitFor("document.querySelector('#nowplaying-accordian').textContent.includes('Loopback Sequence')", "SignalR Now Playing") }
        _ = try await js("document.querySelector('#Header0 a').click();document.querySelector('[aria-label=On]').click()")
        try await waitFor("testRequests.some(r=>r.url==='/api/element/on' && new URLSearchParams(r.body).get('duration')==='0')", "On sends bound form")
        if real {
            try await waitFor("testRequests.some(r=>r.url==='/api/element/on' && r.status===200 && JSON.parse(r.response).IsSuccessful)", "real engine accepts On")
            try await Task.sleep(nanoseconds:200_000_000)
        }
        if real {
            _ = try await js("document.querySelector('[aria-label=Timed]').click()")
            try await waitFor("testRequests.some(r=>r.url==='/api/element/on' && new URLSearchParams(r.body).get('duration')==='30' && r.status===200)", "Timed request completes")
            _ = try await js("document.querySelector('[aria-label=Off]').click()")
            try await waitFor("testRequests.some(r=>r.url==='/api/element/off' && r.status===200)", "Off clears live effect before sequence playback")
        } else {
            _ = try await js("document.querySelector('[aria-label=Timed]').click();document.querySelector('[aria-label=Off]').click()")
        }
        try await waitFor("testRequests.some(r=>r.url==='/api/element/on' && new URLSearchParams(r.body).get('duration')==='30') && testRequests.some(r=>r.url==='/api/element/off')", "Timed and Off")
        _ = try await js("let select=document.querySelector('#select-sequence');select.selectedIndex=1;select.dispatchEvent(new Event('change',{bubbles:true}));document.querySelector('#Sequences button').click()")
        if real {
            try await waitFor("testRequests.some(r=>r.url==='/api/play/playSequence' && new URLSearchParams(r.body).get('FileName').endsWith('/Loopback Sequence.tim') && r.status===200 && JSON.parse(r.response).State===1)", "real sequence loads and renders")
            try await waitFor("document.querySelector('#nowplaying-accordian').textContent.includes('Loopback Sequence') && !document.querySelector('[data-bind=\"click: $root.pauseSequence, enable: $root.isPlaying($data.State())\"]').disabled", "SignalR reports real Playing context")
            _ = try await js("document.querySelector('[data-bind=\"click: $root.pauseSequence, enable: $root.isPlaying($data.State())\"]').click()")
            try await waitFor("!document.querySelector('[data-bind=\"click: $root.playSequence, enable: $root.isPaused($data.State())\"]').disabled", "SignalR reports Paused context")
            _ = try await js("document.querySelector('[data-bind=\"click: $root.playSequence, enable: $root.isPaused($data.State())\"]').click()")
            try await waitFor("!document.querySelector('[data-bind=\"click: $root.pauseSequence, enable: $root.isPlaying($data.State())\"]').disabled", "SignalR reports resumed Playing context")
            _ = try await js("document.querySelector('[data-bind=\"click: $root.stopSequence\"]').click()")
            try await waitFor("!document.querySelector('#nowplaying-accordian').textContent.includes('Loopback Sequence')", "SignalR removes stopped context")
        } else {
            try await waitFor("testRequests.some(r=>r.url==='/api/play/playSequence' && new URLSearchParams(r.body).get('FileName')==='test.tim')", "sequence selection and Play")
            _ = try await js("document.querySelector('[data-bind=\"click: $root.pauseSequence, enable: $root.isPlaying($data.State())\"]').click();document.querySelector('[data-bind=\"click: $root.stopSequence\"]').click()")
            try await waitFor("testRequests.some(r=>r.url==='/api/play/pauseSequence')&&testRequests.some(r=>r.url==='/api/play/stopSequence')", "Now Playing Pause and Stop")
        }
        _ = try await js("document.querySelector('a[href=\"#controllers-section-accordion\"]').click();document.querySelector('[data-bind=\"click: $root.startController, enable: !$data.IsRunning()\"]').click()")
        try await waitFor("testRequests.some(r=>r.url==='/api/system/setControllerState')&&!document.querySelector('#Controllers button[type=submit]').disabled", "controller start enables Save")
        _ = try await js("document.querySelector('#Controllers button[type=submit]').click();document.querySelector('button[aria-label=Settings].hidden-xs').click();let time=document.querySelector('#elementOnTime');time.value='12';time.dispatchEvent(new Event('change',{bubbles:true}))")
        try await waitFor("testRequests.some(r=>r.url==='/api/system/save')&&String(amplify.store('timeout'))==='12'", "controller Save and persistent time setting")
        _ = try await js("let input=document.querySelector('#elementSearch-input');input.value='Loop';input.dispatchEvent(new Event('input',{bubbles:true}))")
        try await waitFor("testRequests.some(r=>r.url.includes('/api/element/searchElements')&&r.url.includes('Loop'))", "debounced search")
        if real { try await waitFor("testRequests.filter(r=>r.url.startsWith('/api/')).every(r=>r.status===200)", "all real API requests completed successfully") }
        try await waitFor("getComputedStyle(document.querySelector('#loading-indicator')).display==='none'", "loading indicator settles")
        try await waitFor("testErrors.length===0", "no JavaScript errors")
        try FileManager.default.createDirectory(at:out,withIntermediateDirectories:true)
        let state=try await js("JSON.stringify({title:document.title,requests:testRequests,errors:testErrors,settings:{timeout:amplify.store('timeout'),sequence:amplify.store('sequence')},nowPlaying:document.querySelector('#nowplaying-accordian').innerText})") as? String ?? "{}"
        try state.write(to:out.appendingPathComponent("state.json"),atomically:true,encoding:.utf8)
        let image: NSImage = try await withCheckedThrowingContinuation { continuation in web.takeSnapshot(with:nil) { image,error in if let error { continuation.resume(throwing:error) } else if let image { continuation.resume(returning:image) } else { continuation.resume(throwing:NSError(domain:"snapshot",code:1)) } } }
        if let tiff=image.tiffRepresentation,let bitmap=NSBitmapImageRep(data:tiff),let png=bitmap.representation(using:.png,properties:[:]) { try png.write(to:out.appendingPathComponent("page.png")) }
    }
    func finish(_ error: String?) {
        let result:[String:Any]=["passed":error==nil,"checks":results,"error":error as Any]
        if let data=try? JSONSerialization.data(withJSONObject:result,options:.prettyPrinted) { FileHandle.standardOutput.write(data);try? FileManager.default.createDirectory(at:out,withIntermediateDirectories:true);try? data.write(to:out.appendingPathComponent("result.json")) }
        exit(error==nil ? 0:1)
    }
}
