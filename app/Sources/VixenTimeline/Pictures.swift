import AppKit
import VixenBridge

/// Where one effect's picture goes. `id` and `picture` say which, and the rest say how big.
nonisolated struct PictureKey: Hashable, Sendable {
    let id: UInt64
    let picture: UInt64
    /// The whole effect's width and the height, in pixels.
    let overall: Int
    let height: Int
}

/// Effects wider than this are rasterized in slices, by the tile.
nonisolated let pictureSliceWidth = 2048

/// Effects' pictures, up to a budget. Painting threads read this, and workers fill it. A picture that isn't up to date is stretched into place
/// until the right one arrives, so nothing ever flashes to a placeholder because of a zoom or a move.
nonisolated final class PictureStore: @unchecked Sendable {
    private struct Slice {
        let image: CGImage
        /// Where it sits along the effect, as fractions of its width.
        let x: CGFloat
        let w: CGFloat
    }

    private struct Entry {
        var key: PictureKey
        var slices: [Int: Slice]
        var bytes: Int
        var lastUse: UInt64
    }

    private struct Job: Sendable {
        let key: PictureKey
        let slice: Int
        /// Which zoom asked. A later one makes this one's width wrong.
        let zoom: UInt64
    }

    private let lock = NSLock()
    private var entries: [UInt64: Entry] = [:]
    private var bytes = 0
    private var clock: UInt64 = 0
    private let budget = 768 * 1024 * 1024
    private var queued: [Job] = []
    private var queuedKeys = Set<String>()
    private var running = 0
    private let limit = max(2, ProcessInfo.processInfo.activeProcessorCount - 2)
    private var holdUntil: Double = 0
    private var wakeScheduled = false
    private var zoom: UInt64 = 0
    private var arrivals = Set<UInt64>()
    private var arrivalsScheduled = false
    private let controller: TimelineController
    private let queue = DispatchQueue(label: "vixen.timeline.pictures", qos: .userInitiated, attributes: .concurrent)
    /// Called on the main thread with the pictures that have arrived, a few at a time, so a tile holding many effects doesn't repaint for each.
    private let arrived: @Sendable (Set<UInt64>) -> Void
    /// Pictures rasterized, for benchmarks.
    private(set) var rasterized = 0

    init(controller: TimelineController, arrived: @escaping @Sendable (Set<UInt64>) -> Void) {
        self.controller = controller
        self.arrived = arrived
    }

    private func jobName(_ key: PictureKey, _ slice: Int) -> String {
        "\(key.id)/\(key.picture)/\(key.overall)/\(key.height)/\(slice)"
    }

    /// While a zoom runs, the widths keep changing, so nothing rasterizes for a moment after each step, and what was asked for at the last width is forgotten.
    func holdRasterizing(for seconds: Double) {
        lock.lock()
        holdUntil = ProcessInfo.processInfo.systemUptime + seconds
        zoom &+= 1
        queued.removeAll()
        queuedKeys.removeAll()
        lock.unlock()
    }

    /// Draws what's there for a quad into a tile's context, stretched if it has to be. Returns whether what's drawn is up to date, and queues
    /// whatever is missing. `clip` is the tile's rectangle.
    func draw(_ q: TimelineElementQuad, in ctx: CGContext, clip: CGRect, scale: CGFloat) -> Bool {
        let rect = CGRect(x: CGFloat(q.x), y: CGFloat(q.y), width: CGFloat(q.w), height: CGFloat(q.h))
        let key = PictureKey(id: q.id, picture: q.picture, overall: Int((CGFloat(q.w) * scale).rounded()), height: Int((CGFloat(q.h) * scale).rounded()))
        let slices = neededSlices(key: key, rect: rect, clip: clip, scale: scale)
        var drawn: [(CGImage, CGRect)] = []
        var upToDate = true
        lock.lock()
        clock &+= 1
        if var entry = entries[q.id] {
            let exact = entry.key == key
            entry.lastUse = clock
            entries[q.id] = entry
            for (index, slice) in entry.slices {
                let dest = CGRect(x: rect.minX + slice.x * rect.width, y: rect.minY, width: slice.w * rect.width, height: rect.height)
                if dest.intersects(clip) {
                    drawn.append((slice.image, dest))
                }
                _ = index
            }
            if exact {
                for s in slices where entry.slices[s] == nil {
                    upToDate = false
                    enqueueLocked(key, s)
                }
            } else {
                upToDate = false
                for s in slices {
                    enqueueLocked(key, s)
                }
            }
        } else {
            upToDate = false
            for s in slices {
                enqueueLocked(key, s)
            }
        }
        lock.unlock()
        if !upToDate {
            startWorkers()
        }
        ctx.saveGState()
        ctx.clip(to: rect)
        ctx.interpolationQuality = .low
        for (image, dest) in drawn {
            ctx.saveGState()
            ctx.translateBy(x: dest.minX, y: dest.maxY)
            ctx.scaleBy(x: 1, y: -1)
            ctx.draw(image, in: CGRect(origin: .zero, size: dest.size))
            ctx.restoreGState()
        }
        ctx.restoreGState()
        return upToDate
    }

    /// The whole picture, if it's one image, for a lifted effect's layer. Whatever's there, however stale.
    func bestImage(id: UInt64) -> CGImage? {
        lock.lock()
        defer { lock.unlock() }
        return entries[id]?.slices.count == 1 ? entries[id]?.slices.values.first?.image : nil
    }

    private func neededSlices(key: PictureKey, rect: CGRect, clip: CGRect, scale: CGFloat) -> [Int] {
        if key.overall <= pictureSliceWidth * 2 {
            return [0]
        }
        let sliceWidth = CGFloat(pictureSliceWidth) / scale
        let first = max(0, Int(((clip.minX - rect.minX) / sliceWidth).rounded(.down)))
        let last = max(first, Int(((clip.maxX - rect.minX) / sliceWidth).rounded(.down)))
        let count = (CGFloat(key.overall) / CGFloat(pictureSliceWidth)).rounded(.up)
        return (first...last).filter { CGFloat($0) < count }
    }

    private func enqueueLocked(_ key: PictureKey, _ slice: Int) {
        let name = jobName(key, slice)
        if queuedKeys.insert(name).inserted {
            queued.append(Job(key: key, slice: slice, zoom: zoom))
        }
    }

    private func startWorkers() {
        lock.lock()
        let wait = holdUntil - ProcessInfo.processInfo.systemUptime
        if wait > 0 {
            let schedule = !wakeScheduled
            wakeScheduled = true
            lock.unlock()
            if schedule {
                queue.asyncAfter(deadline: .now() + wait) {
                    self.lock.lock()
                    self.wakeScheduled = false
                    self.lock.unlock()
                    self.startWorkers()
                }
            }
            return
        }
        var jobs: [Job] = []
        while running + jobs.count < limit, let job = queued.popLast() {
            jobs.append(job)
        }
        running += jobs.count
        lock.unlock()
        for job in jobs {
            queue.async { self.run(job) }
        }
    }

    private func run(_ job: Job) {
        let key = job.key
        let isWhole = key.overall <= pictureSliceWidth * 2
        let sliceX = isWhole ? 0 : job.slice * pictureSliceWidth
        let width = isWhole ? key.overall : min(pictureSliceWidth, key.overall - sliceX)
        let picture = controller.picture(
            id: key.id, overallWidth: Double(key.overall), sliceX: Int32(sliceX), width: UInt32(max(width, 1)), height: UInt32(max(key.height, 1)))
        var image: CGImage?
        if let picture {
            image = makeImage(picture)
        }
        lock.lock()
        running -= 1
        rasterized += 1
        queuedKeys.remove(jobName(key, job.slice))
        var delivered = false
        // A width from before the last zoom step is only worth keeping if there's nothing else to stretch.
        if let image, job.zoom == zoom || entries[key.id] == nil {
            var entry = entries[key.id] ?? Entry(key: key, slices: [:], bytes: 0, lastUse: clock)
            if entry.key != key {
                bytes -= entry.bytes
                entry = Entry(key: key, slices: [:], bytes: 0, lastUse: clock)
            }
            let overall = CGFloat(max(key.overall, 1))
            entry.slices[job.slice] = Slice(image: image, x: CGFloat(sliceX) / overall, w: CGFloat(max(width, 1)) / overall)
            let size = image.bytesPerRow * image.height
            entry.bytes += size
            bytes += size
            entries[key.id] = entry
            delivered = true
            trimLocked()
        }
        var schedule = false
        if delivered {
            arrivals.insert(key.id)
            schedule = !arrivalsScheduled
            arrivalsScheduled = true
        }
        lock.unlock()
        if schedule {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.03) { self.deliverArrivals() }
        }
        startWorkers()
    }

    private func deliverArrivals() {
        lock.lock()
        let ids = arrivals
        arrivals.removeAll()
        arrivalsScheduled = false
        lock.unlock()
        arrived(ids)
    }

    private func trimLocked() {
        guard bytes > budget else { return }
        let oldest = entries.values.sorted { $0.lastUse < $1.lastUse }
        for entry in oldest {
            if bytes <= budget * 3 / 4 {
                break
            }
            entries[entry.key.id] = nil
            bytes -= entry.bytes
        }
    }

    private func makeImage(_ p: TimelinePicture) -> CGImage? {
        guard p.width > 0, p.height > 0, let provider = CGDataProvider(data: p.rgba as CFData) else { return nil }
        return CGImage(
            width: Int(p.width), height: Int(p.height), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: Int(p.width) * 4, space: sRGBSpace,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue), provider: provider, decode: nil, shouldInterpolate: false,
            intent: .defaultIntent)
    }

    /// Nothing queued or rasterizing.
    var isIdle: Bool {
        lock.lock()
        defer { lock.unlock() }
        return queued.isEmpty && running == 0 && arrivals.isEmpty
    }

    var memoryUse: Int {
        lock.lock()
        defer { lock.unlock() }
        return bytes
    }
}
