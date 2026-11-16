import AppKit
import QuartzCore
import VixenBridge
import VixenKit

/// Measures what the owner's complaint was about: time per frame while scrolling, zooming and dragging, in the real views on a real sequence. One step per
/// display frame, with everything a pointer event would do on the main thread, through `CATransaction.flush`.
@MainActor
final class TimelineBench: NSObject {
    private struct Scenario {
        let name: String
        let frames: Int
        let setUp: () -> Void
        let frame: (Int) -> Void
        let tearDown: () -> Void
    }

    private struct Result {
        var busy: [Double] = []
        var intervals: [Double] = []
        var notReady = 0
        var settle = 0.0
        /// What Rust said the drag was doing halfway through, so a number can't be of the wrong thing.
        var dragState = ""
        var selected = 0
    }

    private static var running: TimelineBench?
    private let canvas: TimelineCanvas
    private let outDir: URL
    private var scenarios: [Scenario] = []
    private var index = 0
    private var frameIndex = 0
    private var current = Result()
    private var results: [(String, Result)] = []
    private var last = 0.0
    private var link: CADisplayLink?
    private var timer: Timer?
    private var pacedByDisplay = true
    private var settling = false
    private var settleStart = 0.0
    private var started = false
    private var lifecycleLog: [String: Double] = [:]

    private init(canvas: TimelineCanvas, outDir: URL) {
        self.canvas = canvas
        self.outDir = outDir
        super.init()
    }

    static func start(canvas: TimelineCanvas, outDir: URL) {
        guard running == nil else { return }
        let bench = TimelineBench(canvas: canvas, outDir: outDir)
        running = bench
        bench.waitForReady()
    }

    /// Renders and pictures for the first view finish first: a measurement shouldn't be of the load.
    private func waitForReady() {
        let start = ProcessInfo.processInfo.systemUptime
        Task { @MainActor in
            while true {
                try? await Task.sleep(for: .milliseconds(100))
                let waiting = canvas.controller.session().renderStatus().waiting
                canvas.apply(canvas.controller.refresh())
                let ready = waiting == 0 && canvas.pictures.isIdle && canvas.grid.planes.allSatisfy(\.isSettled) && canvas.grid.bounds.width > 0
                if ready || ProcessInfo.processInfo.systemUptime - start > 180 {
                    lifecycleLog["secondsUntilReady"] = ProcessInfo.processInfo.systemUptime - start
                    begin()
                    return
                }
            }
        }
    }

    private func mods(shift: Bool = false, cmd: Bool = false, opt: Bool = false) -> TimelineMods {
        TimelineMods(shift: shift, cmd: cmd, opt: opt)
    }

    private func pointer(_ x: Int32, _ y: Int32, button: TimelineButton?, mods: TimelineMods? = nil) -> TimelinePointer {
        TimelinePointer(x: x, y: y, button: button, clicks: 1, mods: mods ?? self.mods())
    }

    private func begin() {
        let c = canvas.controller
        let apply = { [canvas] (u: TimelineUpdate) in canvas.apply(u) }
        let gridSize = canvas.grid.bounds.size
        let width = Int32(gridSize.width)
        let height = Int32(gridSize.height)
        // A place with effects in view, so the numbers aren't of an empty grid.
        func firstEffectPoint() -> (Int32, Int32)? {
            let v = c.view()
            for info in c.elementsInfo() where !info.rows.isEmpty {
                let top = c.rowFrame(top: 0, bottom: v.canvasHeight).rows.first { $0.id == info.rows[0] }
                guard let top else { continue }
                let x = Double(info.start) / Double(v.timePerPixel) - v.scrollX + Double(info.duration) / Double(v.timePerPixel) / 2
                if x > 40, x < Double(width) - 40, top.top - v.verticalOffset >= 0, top.top - v.verticalOffset + top.height < height {
                    return (Int32(x), top.top - v.verticalOffset + 3)
                }
            }
            return nil
        }
        scenarios = [
            Scenario(
                name: "scroll horizontally, 10 points a frame",
                frames: 600,
                setUp: { apply(c.scrollToStart()) },
                frame: { _ in apply(c.scrollWheel(surface: .grid, dx: -10, dy: 0, precise: true, mods: self.mods(), pointerOffsetX: 400, gridY: 100)) },
                tearDown: {}
            ),
            Scenario(
                name: "scroll vertically, 6 points a frame",
                frames: 300,
                setUp: { apply(c.expandAllRows()); apply(c.scrollToTop()) },
                frame: { _ in apply(c.scrollWheel(surface: .grid, dx: 0, dy: -6, precise: true, mods: self.mods(), pointerOffsetX: 400, gridY: 100)) },
                // Every row stays open from here on, which is what a person editing sees.
                tearDown: { apply(c.scrollToTop()) }
            ),
            Scenario(
                name: "zoom with command wheel, in and out",
                frames: 300,
                setUp: { apply(c.scrollToStart()) },
                frame: { i in
                    let direction = (i / 75) % 2 == 0 ? 3.0 : -3.0
                    apply(c.scrollWheel(surface: .grid, dx: 0, dy: direction, precise: true, mods: self.mods(cmd: true), pointerOffsetX: 400, gridY: 100))
                },
                tearDown: { apply(c.setTimePerPixelForBench(ticks: 100_000)) }
            ),
            Scenario(
                name: "pinch, in and out",
                frames: 300,
                setUp: { apply(c.scrollToStart()) },
                frame: { i in
                    let direction = (i / 75) % 2 == 0 ? 0.01 : -0.01
                    apply(c.magnify(magnification: direction, pointerOffsetX: 400, mods: self.mods()))
                },
                tearDown: { apply(c.setTimePerPixelForBench(ticks: 100_000)) }
            ),
            Scenario(
                name: "fast flick along, 80 points a frame",
                frames: 300,
                setUp: { apply(c.scrollToStart()) },
                frame: { i in
                    let direction = (i / 150) % 2 == 0 ? -80.0 : 80.0
                    apply(c.scrollWheel(surface: .grid, dx: direction, dy: 0, precise: true, mods: self.mods(), pointerOffsetX: 400, gridY: 100))
                },
                tearDown: { apply(c.scrollToStart()) }
            ),
            Scenario(
                name: "whole sequence in view, scroll rows 12 points a frame",
                frames: 300,
                setUp: { apply(c.zoomTime(scale: 1_000_000)); apply(c.scrollToTop()) },
                frame: { _ in apply(c.scrollWheel(surface: .grid, dx: 0, dy: -12, precise: true, mods: self.mods(), pointerOffsetX: 400, gridY: 100)) },
                tearDown: { apply(c.scrollToTop()); apply(c.setTimePerPixelForBench(ticks: 100_000)) }
            ),
            Scenario(
                name: "playback line and follow",
                frames: 600,
                setUp: { apply(c.scrollToStart()) },
                frame: { i in
                    // As `playbackFrame` does, with no sound or show running: a position, the rule, and two layers.
                    let ticks = Int64(i) * 10_000_000 / 120 * 8
                    apply(c.followPlayback(position: ticks))
                    let x = CGFloat(ticks) / CGFloat(max(c.view().timePerPixel, 1))
                    self.canvas.grid.setPlayback(x: x, state: self.canvas.state)
                    self.canvas.ruler.setPlayback(x: x, state: self.canvas.state)
                },
                tearDown: {
                    self.canvas.grid.clearPlayback()
                    self.canvas.ruler.clearPlayback()
                    apply(c.playbackEnded())
                }
            ),
            self.dragScenario(name: "drag one effect", selectAll: false, firstEffectPoint),
            self.dragScenario(name: "drag every effect, selected with select all", selectAll: true, firstEffectPoint),
            Scenario(
                name: "rubber band across the view",
                frames: 240,
                setUp: {
                    apply(c.clearSelection())
                    apply(c.scrollToStart())
                    apply(c.gridMouseDown(ev: self.pointer(width - 2, height - 2, button: .left)))
                },
                frame: { i in
                    let x = width - 2 - Int32(i) * (width - 4) / 240
                    let y = height - 2 - Int32(i) * (height - 4) / 240
                    apply(c.gridMouseMove(ev: self.pointer(x, y, button: .left)))
                },
                tearDown: {
                    apply(c.gridMouseUp(ev: self.pointer(2, 2, button: .left)))
                    apply(c.clearSelection())
                }
            ),
            self.resizeScenario(firstEffectPoint),
        ]
        // `VIXEN_TIMELINE_BENCH=zoom` runs the scenarios with that in their names. `VIXEN_TIMELINE_BENCH_REPEAT` runs each for that many times as long.
        let environment = ProcessInfo.processInfo.environment
        if let only = environment["VIXEN_TIMELINE_BENCH"], only != "1" {
            scenarios = scenarios.filter { $0.name.contains(only) }
        }
        if let repeatCount = environment["VIXEN_TIMELINE_BENCH_REPEAT"].flatMap({ Int($0) }) {
            scenarios = scenarios.map { Scenario(name: $0.name, frames: $0.frames * repeatCount, setUp: $0.setUp, frame: $0.frame, tearDown: $0.tearDown) }
        }
        apply(c.expandAllRows())
        index = 0
        startScenario()
        // With the display asleep or the screen locked a display link never fires. Then a timer at the display's rate stands in, which still shows whether
        // the main thread keeps up, though not what the render server does.
        if CGDisplayIsAsleep(CGMainDisplayID()) != 0 {
            pacedByDisplay = false
            let timer = Timer(timeInterval: 1.0 / 120, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.tick() }
            }
            RunLoop.main.add(timer, forMode: .common)
            self.timer = timer
        } else {
            let link = canvas.grid.window?.screen?.displayLink(target: self, selector: #selector(tick))
                ?? NSScreen.screens.first?.displayLink(target: self, selector: #selector(tick))
            link?.add(to: .main, forMode: .common)
            self.link = link
        }
    }

    private func dragScenario(name: String, selectAll: Bool, _ point: @escaping () -> (Int32, Int32)?) -> Scenario {
        let c = canvas.controller
        let apply = { [canvas] (u: TimelineUpdate) in canvas.apply(u) }
        var origin: (Int32, Int32) = (0, 0)
        return Scenario(
            name: name,
            frames: 240,
            setUp: {
                apply(c.scrollToStart())
                apply(c.clearSelection())
                origin = point() ?? (100, 10)
                apply(c.gridMouseMove(ev: self.pointer(origin.0, origin.1, button: nil)))
                if selectAll {
                    apply(c.selectAll())
                }
                apply(c.gridMouseDown(ev: self.pointer(origin.0, origin.1, button: .left)))
                apply(c.gridMouseMove(ev: self.pointer(origin.0 + 9, origin.1, button: .left)))
            },
            frame: { i in
                // Along and back, within the view, so it doesn't turn into an autoscroll.
                let dx = Int32(Double(120) * sin(Double(i) / 240 * 12))
                apply(c.gridMouseMove(ev: self.pointer(origin.0 + 9 + dx, origin.1, button: .left)))
            },
            tearDown: {
                apply(c.gridCancelDrag())
                apply(c.gridMouseUp(ev: self.pointer(origin.0, origin.1, button: .left)))
                apply(c.clearSelection())
            }
        )
    }

    private func resizeScenario(_ point: @escaping () -> (Int32, Int32)?) -> Scenario {
        let c = canvas.controller
        let apply = { [canvas] (u: TimelineUpdate) in canvas.apply(u) }
        var origin: (Int32, Int32) = (0, 0)
        return Scenario(
            name: "resize one effect",
            frames: 240,
            setUp: {
                apply(c.scrollToStart())
                apply(c.clearSelection())
                guard let p = point() else { return }
                // Find its front edge: walk left to where it stops being under the pointer.
                var x = p.0
                while x > 1, c.elementAtClient(x: x - 1, y: p.1) != nil {
                    x -= 1
                }
                origin = (x + 1, p.1)
                apply(c.gridMouseMove(ev: self.pointer(origin.0, origin.1, button: nil)))
                apply(c.gridMouseDown(ev: self.pointer(origin.0, origin.1, button: .left)))
            },
            frame: { i in
                let dx = Int32(Double(40) * sin(Double(i) / 240 * 12))
                apply(c.gridMouseMove(ev: self.pointer(origin.0 + dx, origin.1, button: .left)))
            },
            tearDown: {
                apply(c.gridCancelDrag())
                apply(c.gridMouseUp(ev: self.pointer(origin.0, origin.1, button: .left)))
                apply(c.clearSelection())
            }
        )
    }

    private func startScenario() {
        current = Result()
        frameIndex = 0
        last = 0
        settling = false
        scenarios[index].setUp()
    }

    @objc private func tick() {
        let now = ProcessInfo.processInfo.systemUptime
        if settling {
            if canvas.grid.planes.allSatisfy(\.isSettled) && canvas.ruler.plane.isSettled && canvas.marksBar.plane.isSettled {
                current.settle = now - settleStart
                settling = false
                results.append((scenarios[index].name, current))
                scenarios[index].tearDown()
                index += 1
                if index < scenarios.count {
                    startScenario()
                } else {
                    finish()
                }
            }
            return
        }
        if last > 0 {
            current.intervals.append(now - last)
        }
        last = now
        let scenario = scenarios[index]
        scenario.frame(frameIndex)
        CATransaction.flush()
        current.busy.append(ProcessInfo.processInfo.systemUptime - now)
        if frameIndex == scenario.frames / 2 {
            current.dragState = canvas.controller.dragStateName()
            current.selected = canvas.controller.selectedElements().count
        }
        if !canvas.grid.planes.allSatisfy(\.visibleTilesReady) {
            current.notReady += 1
        }
        frameIndex += 1
        if frameIndex >= scenario.frames {
            settling = true
            settleStart = ProcessInfo.processInfo.systemUptime
        }
    }

    private func percentile(_ values: [Double], _ p: Double) -> Double {
        guard !values.isEmpty else { return 0 }
        let sorted = values.sorted()
        return sorted[min(sorted.count - 1, Int(Double(sorted.count) * p))]
    }

    private func finish() {
        link?.invalidate()
        link = nil
        timer?.invalidate()
        timer = nil
        let c = canvas.controller
        var report: [[String: Any]] = []
        for (name, r) in results {
            func summary(_ v: [Double]) -> [String: Double] {
                ["p50": percentile(v, 0.5) * 1000, "p95": percentile(v, 0.95) * 1000, "p99": percentile(v, 0.99) * 1000, "max": (v.max() ?? 0) * 1000]
            }
            report.append([
                "scenario": name,
                "frames": r.busy.count,
                "mainThreadBusyMs": summary(r.busy),
                "frameIntervalMs": summary(r.intervals),
                "framesWithATileStillPainting": r.notReady,
                "msUntilAllTilesPaintedAfterwards": r.settle * 1000,
                "framesLongerThanTwoPeriods": r.intervals.filter { $0 > 2.0 / Double(max(NSScreen.main?.maximumFramesPerSecond ?? 60, 1)) }.count,
                "dragStateHalfway": r.dragState,
                "selectedHalfway": r.selected,
            ])
        }
        let out: [String: Any] = [
            "effects": c.elementsInfo().count,
            "rows": c.rowFrame(top: 0, bottom: c.view().canvasHeight).rows.count,
            "scale": canvas.scale,
            "window": canvas.grid.window?.frame.origin.x ?? 0,
            "displayRefreshHz": NSScreen.main?.maximumFramesPerSecond ?? 0,
            "pacedBy": pacedByDisplay ? "display" : "timer, with the display asleep",
            "secondsUntilReady": lifecycleLog["secondsUntilReady"] ?? 0,
            "pictureMegabytes": canvas.pictures.memoryUse / 1_000_000,
            "scenarios": report,
        ]
        if let data = try? JSONSerialization.data(withJSONObject: out, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: outDir.appending(path: "timeline-bench.json"))
        }
        print("Timeline benchmark finished")
        TimelineBench.running = nil
    }
}
