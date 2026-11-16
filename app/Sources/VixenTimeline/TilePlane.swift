import AppKit
import IOSurface
import QuartzCore

/// A rectangle of a surface's canvas to paint, in points.
nonisolated struct TileRequest: @unchecked Sendable {
    let rect: CGRect
    let scale: CGFloat
    let timePerPixel: Int64
    /// Only the grid uses these: selected-row borders belong at the viewport edges, not at tile boundaries.
    let viewportEdges: (left: CGFloat, right: CGFloat)?
    /// The window's. Painting converts into it, so the tile is ready to composite as it is.
    let colorSpace: CGColorSpace
}

/// Hands a painted tile from a paint thread to the main thread. A surface, because Core Animation color matches a `CGImage` on the main thread when it's committed,
/// which took 4 ms a tile on this machine's wide gamut display. A surface goes to the render server as it is. It's painted in the window's colour space, so
/// there's nothing left to match.
nonisolated struct PaintedTile: @unchecked Sendable {
    let image: IOSurface?
    let waitingFor: [UInt64]
    /// F151: the frame failed, so the tile keeps what it shows.
    var failed = false
}

/// Paints one tile, off the main thread, and says which effects it drew with a picture that wasn't up to date. `nil` means the frame failed.
typealias TilePainter = @Sendable (TileRequest, CGContext) -> [UInt64]?

/// Shared by every plane, so a fast scroll in one can't queue work without limit.
nonisolated enum TileWork {
    static let queue = DispatchQueue(label: "vixen.timeline.tiles", qos: .userInteractive, attributes: .concurrent)
    static let limit = max(2, ProcessInfo.processInfo.activeProcessorCount - 3)
}

/// A layer with no implicit animations: everything here follows the pointer or the display.
func plainLayer(_ layer: CALayer = CALayer()) -> CALayer {
    layer.actions = noActions
    layer.anchorPoint = .zero
    return layer
}

nonisolated(unsafe) let noActions: [String: any CAAction] = [
    "position": NSNull(), "bounds": NSNull(), "contents": NSNull(), "transform": NSNull(), "hidden": NSNull(), "sublayers": NSNull(),
    "onOrderIn": NSNull(), "onOrderOut": NSNull(), "backgroundColor": NSNull(), "borderWidth": NSNull(), "borderColor": NSNull(),
    "path": NSNull(), "strokeColor": NSNull(), "fillColor": NSNull(), "opacity": NSNull(), "frame": NSNull(), "string": NSNull(),
    "lineDashPhase": NSNull(), "lineDashPattern": NSNull(), "lineWidth": NSNull(),
]

/// Fixed-size tiles of a surface's canvas, each holding a painted image. Scrolling moves one layer and paints what's newly exposed, off the
/// main thread. Canvas coordinates never reach Core Animation: tiles sit relative to a base near the view, so an hour at the zoom floor is
/// no different from a minute.
///
/// A zoom only stretches what's there, which is one transform per host, so it keeps up with the display however big the sequence is. A new set of
/// tiles starts when a zoom begins, at most every 100 ms while it runs, and when it stops. Each is painted for the zoom it started at, and stretches
/// in its turn if the zoom has moved on.
@MainActor
final class TilePlane {
    struct Key: Hashable {
        let col: Int32
        let row: Int32
    }

    final class Tile {
        let layer: CALayer
        var generation: UInt64 = 1
        var paintedGeneration: UInt64 = 0
        var inFlight = false
        var dirty = true
        init(layer: CALayer) {
            self.layer = layer
        }
    }

    final class Host {
        let layer = plainLayer()
        var tiles: [Key: Tile] = [:]
        var timePerPixel: Int64
        init(timePerPixel: Int64) {
            self.timePerPixel = timePerPixel
        }
    }

    let root = plainLayer()
    private(set) var tileWidth: CGFloat
    private(set) var tileHeight: CGFloat
    private var live: Host
    private var stale: Host?
    /// The zoom on screen. `live` lags it while a zoom runs.
    private var viewTimePerPixel: Int64
    private var lastZoomEvent = 0.0
    private var lastTileSet = 0.0
    private var zoomTickScheduled = false
    private var pool: [CALayer] = []
    private let painter: TilePainter
    private var opaque: Bool

    private var scrollX: CGFloat = 0
    private var scrollY: CGFloat = 0
    private var size: CGSize = .zero
    private var baseX: CGFloat = 0
    private var baseY: CGFloat = 0
    /// Whether the canvas has a vertical axis. Strips are one tile high and don't move vertically.
    private let scrollsVertically: Bool
    private let scrollsHorizontally: Bool
    var paintsViewportEdges = false
    var scale: CGFloat = 2 {
        didSet { if scale != oldValue { invalidateAll() } }
    }
    var colorSpace: CGColorSpace = sRGBSpace {
        didSet { if colorSpace != oldValue { invalidateAll() } }
    }

    private static var running = 0
    private static var planes: [Weak] = []
    private struct Weak {
        weak var plane: TilePlane?
    }

    /// Effects a tile drew without an up-to-date picture, and the tiles waiting on each.
    private var waiting: [UInt64: Set<Key>] = [:]
    /// Pictures that arrived while tiles were painting, which those tiles may have just missed. Cleared once nothing is in flight.
    private var arrivedWhilePainting = Set<UInt64>()
    /// How many tiles have been painted, for tests and benchmarks.
    private(set) var paintedCount = 0
    var onSettled: (() -> Void)?
    /// Called with tiles' lists of effects whose pictures they need.
    var onNeedPictures: (([UInt64]) -> Void)?

    init(tileWidth: CGFloat, tileHeight: CGFloat, timePerPixel: Int64, opaque: Bool, vertical: Bool, horizontal: Bool, painter: @escaping TilePainter) {
        self.tileWidth = tileWidth
        self.tileHeight = tileHeight
        self.painter = painter
        self.opaque = opaque
        scrollsVertically = vertical
        scrollsHorizontally = horizontal
        viewTimePerPixel = timePerPixel
        live = Host(timePerPixel: timePerPixel)
        root.masksToBounds = true
        root.addSublayer(live.layer)
        TilePlane.planes.append(Weak(plane: self))
    }

    /// Where a canvas rectangle sits inside `contentHost`, for layers that scroll with the tiles. Up is up.
    func localFrame(canvas r: CGRect) -> CGRect {
        CGRect(x: r.minX - baseX, y: -(r.maxY - baseY), width: r.width, height: r.height)
    }

    /// Position of a host layer given the scroll, so anything placed with `localFrame` lines up.
    func hostPosition(viewportHeight: CGFloat) -> CGPoint {
        CGPoint(x: baseX - scrollX, y: viewportHeight + scrollY - baseY)
    }

    var currentBase: CGPoint { CGPoint(x: baseX, y: baseY) }

    func setTileSize(width: CGFloat, height: CGFloat) {
        guard width != tileWidth || height != tileHeight else { return }
        tileWidth = width
        tileHeight = height
        dropTiles(of: live)
        if let stale {
            release(stale)
        }
        stale = nil
        live.timePerPixel = viewTimePerPixel
        update()
    }

    /// Moves the view. Cheap when nothing new is exposed.
    func setView(scrollX: CGFloat, scrollY: CGFloat, size: CGSize, timePerPixel: Int64) {
        let oldEdges = viewportEdges
        self.scrollX = scrollsHorizontally ? scrollX : 0
        self.scrollY = scrollsVertically ? scrollY : 0
        if size != self.size {
            self.size = size
            root.bounds = CGRect(origin: .zero, size: size)
        }
        if timePerPixel != viewTimePerPixel {
            viewTimePerPixel = timePerPixel
            zoomed()
        }
        if paintsViewportEdges {
            let newEdges = viewportEdges
            if oldEdges.left != newEdges.left || oldEdges.right != newEdges.right {
                // Retire the old border even in cached offscreen rows. Repaint just the old/new edge columns;
                // the rest of the grid still scrolls by translating its existing tiles.
                let columns = Set([oldEdges.left, oldEdges.right, newEdges.left, newEdges.right].map { tileIndex($0, span: tileWidth) })
                for (key, tile) in live.tiles where columns.contains(key.col) {
                    tile.generation &+= 1
                    tile.dirty = true
                }
            }
        }
        rebaseIfNeeded()
        update()
    }

    private var viewportEdges: (left: CGFloat, right: CGFloat) {
        let zoom = CGFloat(viewTimePerPixel) / CGFloat(max(1, live.timePerPixel))
        return (floor(scrollX * zoom), floor((scrollX + max(0, size.width - 1)) * zoom))
    }

    private func rebaseIfNeeded() {
        // Far enough that single precision in the render server stays exact to a fraction of a point.
        let far: CGFloat = 32768
        guard abs(scrollX - baseX) > far || abs(scrollY - baseY) > far else { return }
        baseX = (scrollX / far).rounded(.down) * far
        baseY = (scrollY / far).rounded(.down) * far
        for (key, tile) in live.tiles {
            place(tile, key)
        }
        if let stale {
            for (key, tile) in stale.tiles {
                place(tile, key)
            }
        }
    }

    // MARK: Zooming

    private func zoomed() {
        let now = ProcessInfo.processInfo.systemUptime
        lastZoomEvent = now
        // Back at the zoom these tiles are for. Pictures asked for here may have been forgotten on the way.
        guard live.timePerPixel != viewTimePerPixel else {
            invalidateAll()
            return
        }
        if now - lastTileSet >= 0.1 {
            startTileSet(now: now)
        }
        scheduleZoomTick()
    }

    private func scheduleZoomTick() {
        guard live.timePerPixel != viewTimePerPixel, !zoomTickScheduled else { return }
        zoomTickScheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0 / 60) { [weak self] in
            MainActor.assumeIsolated {
                self?.zoomTickScheduled = false
                self?.zoomTick()
            }
        }
    }

    private func zoomTick() {
        guard live.timePerPixel != viewTimePerPixel else { return }
        let now = ProcessInfo.processInfo.systemUptime
        let stopped = now - lastZoomEvent > 0.05
        let age = now - lastTileSet
        // While it runs, a set that's nearly done is worth more than one that's just begun. Once it stops, only the final zoom matters.
        if stopped || (age >= 0.1 && (allPainted(live) || age >= 0.25)) {
            startTileSet(now: now)
        }
        scheduleZoomTick()
    }

    /// New tiles for the zoom on screen. Stretched meanwhile: the last set that got all its tiles, or whatever there is if none has.
    private func startTileSet(now: Double) {
        lastTileSet = now
        // These name tiles of the set that's going.
        waiting.removeAll()
        arrivedWhilePainting.removeAll()
        let hasTiles = live.tiles.values.contains { $0.paintedGeneration > 0 }
        if !hasTiles && stale == nil {
            dropTiles(of: live)
            live.timePerPixel = viewTimePerPixel
            update()
            return
        }
        if stale == nil || allPainted(live) {
            if let stale {
                release(stale)
            }
            stale = live
        } else {
            release(live)
        }
        let next = Host(timePerPixel: viewTimePerPixel)
        root.addSublayer(next.layer)
        live = next
        update()
    }

    private func allPainted(_ host: Host) -> Bool {
        !host.tiles.isEmpty && !host.tiles.values.contains { $0.paintedGeneration == 0 }
    }

    /// The view in a host's own canvas, which is at the zoom it was painted for.
    private func viewRect(in host: Host) -> CGRect {
        let s = CGFloat(viewTimePerPixel) / CGFloat(max(host.timePerPixel, 1))
        return CGRect(x: scrollX * s, y: scrollY, width: size.width * s, height: size.height)
    }

    // MARK: Tiles

    /// Rust's paint rectangles use i32 coordinates. A one-point edge strip reaches that limit much sooner than a
    /// 512-point tile index does; include its prefetch margin without trapping during Swift's floating-point cast.
    private func tileIndex(_ position: CGFloat, span: CGFloat) -> Int32 {
        let lower = ceil(CGFloat(Int32.min) / span)
        let upper = floor((CGFloat(Int32.max) - span) / span)
        let index = floor(position / span)
        if index.isNaN { return 0 }
        return Int32(min(upper, max(lower, index)))
    }

    private func dropTiles(of host: Host) {
        for tile in host.tiles.values {
            recycle(tile.layer)
        }
        host.tiles.removeAll()
    }

    private func release(_ host: Host) {
        dropTiles(of: host)
        host.layer.removeFromSuperlayer()
    }

    private func recycle(_ layer: CALayer) {
        layer.removeFromSuperlayer()
        layer.contents = nil
        if pool.count < 64 {
            pool.append(layer)
        }
    }

    private func positionHosts() {
        position(live)
        if let stale {
            position(stale)
        }
    }

    /// Tiles painted at another zoom stretch to where they'd be at this one.
    private func position(_ host: Host) {
        let s = CGFloat(host.timePerPixel) / CGFloat(max(viewTimePerPixel, 1))
        host.layer.transform = s == 1 ? CATransform3DIdentity : CATransform3DMakeScale(s, 1, 1)
        host.layer.position = CGPoint(x: baseX * s - scrollX, y: size.height + scrollY - baseY)
    }

    private func place(_ tile: Tile, _ key: Key) {
        let canvas = CGRect(x: CGFloat(key.col) * tileWidth, y: CGFloat(key.row) * tileHeight, width: tileWidth, height: tileHeight)
        tile.layer.frame = localFrame(canvas: canvas)
    }

    /// Tiles for the view plus a margin, so they're usually painted before they show.
    private func keys(margin: CGSize) -> [Key] {
        guard size.width > 0, size.height > 0, tileWidth > 0, tileHeight > 0 else { return [] }
        let x0 = tileIndex(scrollX - margin.width, span: tileWidth)
        let x1 = tileIndex(scrollX + size.width + margin.width - 1, span: tileWidth)
        let y0 = scrollsVertically ? max(0, tileIndex(scrollY - margin.height, span: tileHeight)) : 0
        let y1 = scrollsVertically ? tileIndex(scrollY + size.height + margin.height - 1, span: tileHeight) : 0
        var out: [Key] = []
        out.reserveCapacity((Int(x1) - Int(x0) + 1) * (Int(y1) - Int(y0) + 1))
        for row in y0...max(y0, y1) {
            for col in max(0, x0)...max(max(0, x0), x1) {
                out.append(Key(col: col, row: row))
            }
        }
        return out
    }

    private func update() {
        positionHosts()
        guard live.timePerPixel == viewTimePerPixel else {
            pump()
            return
        }
        let wanted = keys(margin: CGSize(width: scrollsHorizontally ? tileWidth : 0, height: scrollsVertically ? tileHeight : 0))
        for key in wanted where live.tiles[key] == nil {
            let layer = pool.popLast() ?? plainLayer()
            layer.isOpaque = opaque
            layer.contentsScale = scale
            layer.contentsGravity = .resize
            let tile = Tile(layer: layer)
            live.tiles[key] = tile
            place(tile, key)
            live.layer.addSublayer(layer)
        }
        // Keep a further margin so a little back and forth doesn't repaint, then let go.
        let keep = Set(keys(margin: CGSize(width: scrollsHorizontally ? tileWidth * 3 : 0, height: scrollsVertically ? tileHeight * 3 : 0)))
        if live.tiles.count > keep.count {
            for (key, tile) in live.tiles where !keep.contains(key) {
                recycle(tile.layer)
                live.tiles[key] = nil
            }
        }
        if live.tiles.isEmpty {
            retireStaleIfCovered()
        }
        pump()
    }

    func invalidateAll() {
        for tile in live.tiles.values {
            tile.generation &+= 1
            tile.dirty = true
        }
        pump()
    }

    /// Repaints the tiles under a canvas rectangle. They keep showing the old image until the new one arrives.
    func invalidate(_ r: CGRect) {
        guard tileWidth > 0, tileHeight > 0 else { return }
        for (key, tile) in live.tiles {
            let x = CGFloat(key.col) * tileWidth
            let y = CGFloat(key.row) * tileHeight
            if r.maxX > x && r.minX < x + tileWidth && r.maxY > y && r.minY < y + tileHeight {
                tile.generation &+= 1
                tile.dirty = true
            }
        }
    }

    /// Pictures arrived: the tiles that were waiting for them repaint.
    func picturesArrived(_ ids: Set<UInt64>) {
        let anyInFlight = live.tiles.values.contains { $0.inFlight }
        for id in ids {
            if anyInFlight {
                arrivedWhilePainting.insert(id)
            }
            guard let tiles = waiting.removeValue(forKey: id) else { continue }
            for key in tiles {
                if let tile = live.tiles[key] {
                    tile.generation &+= 1
                    tile.dirty = true
                }
            }
        }
    }

    func pump() {
        TilePlane.pumpAll()
    }

    private static func pumpAll() {
        guard running < TileWork.limit else { return }
        // The grid and strips share the limit. Each gets a turn, tiles in view first.
        planes.removeAll { $0.plane == nil }
        var progress = true
        while progress && running < TileWork.limit {
            progress = false
            for entry in planes {
                guard running < TileWork.limit, let plane = entry.plane else { continue }
                if plane.startNextTile() {
                    progress = true
                }
            }
        }
    }

    private func nextTile() -> (Key, Tile)? {
        let view = viewRect(in: live)
        var fallback: (Key, Tile)?
        for (key, tile) in live.tiles where tile.dirty && !tile.inFlight {
            let rect = CGRect(x: CGFloat(key.col) * tileWidth, y: CGFloat(key.row) * tileHeight, width: tileWidth, height: tileHeight)
            if rect.intersects(view) {
                return (key, tile)
            }
            fallback = (key, tile)
        }
        return fallback
    }

    private func startNextTile() -> Bool {
        guard let (key, tile) = nextTile() else { return false }
        tile.dirty = false
        tile.inFlight = true
        TilePlane.running += 1
        let host = live
        let generation = tile.generation
        let request = TileRequest(
            rect: CGRect(x: CGFloat(key.col) * tileWidth, y: CGFloat(key.row) * tileHeight, width: tileWidth, height: tileHeight),
            scale: scale,
            timePerPixel: host.timePerPixel,
            viewportEdges: paintsViewportEdges ? viewportEdges : nil,
            colorSpace: colorSpace
        )
        let painter = self.painter
        let isOpaque = opaque
        TileWork.queue.async {
            let painted = renderTile(request, opaque: isOpaque, painter: painter)
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    self.finish(key: key, tile: tile, host: host, generation: generation, painted: painted)
                }
            }
        }
        return true
    }

    private func finish(key: Key, tile: Tile, host: Host, generation: UInt64, painted: PaintedTile) {
        TilePlane.running -= 1
        tile.inFlight = false
        paintedCount += 1
        // A newer image never gets overwritten by an older one that finished late.
        if host === live, live.tiles[key] === tile, generation >= tile.paintedGeneration, !painted.failed {
            // No transaction of its own: tiles finishing together go to the render server in one commit, at the end of this turn of the run loop.
            tile.layer.contents = painted.image
            tile.paintedGeneration = generation
            if !painted.waitingFor.isEmpty {
                // A picture this tile drew without may have arrived after it looked, and before now. It repaints once for that, since this set is
                // cleared before the repaint can see it again.
                if painted.waitingFor.contains(where: { arrivedWhilePainting.contains($0) }) {
                    tile.generation &+= 1
                    tile.dirty = true
                }
                for id in painted.waitingFor where !arrivedWhilePainting.contains(id) {
                    waiting[id, default: []].insert(key)
                }
                onNeedPictures?(painted.waitingFor)
            }
        }
        if !arrivedWhilePainting.isEmpty, !live.tiles.values.contains(where: { $0.inFlight }) {
            arrivedWhilePainting.removeAll()
        }
        retireStaleIfCovered()
        TilePlane.pumpAll()
        if isSettled {
            onSettled?()
        }
    }

    private func retireStaleIfCovered() {
        guard let old = stale, live.timePerPixel == viewTimePerPixel else { return }
        let view = viewRect(in: live)
        for (key, tile) in live.tiles where tile.paintedGeneration == 0 {
            let rect = CGRect(x: CGFloat(key.col) * tileWidth, y: CGFloat(key.row) * tileHeight, width: tileWidth, height: tileHeight)
            if rect.intersects(view) {
                return
            }
        }
        release(old)
        stale = nil
    }

    /// Nothing waiting to paint in this plane.
    var isSettled: Bool {
        live.timePerPixel == viewTimePerPixel && !live.tiles.values.contains { $0.dirty || $0.inFlight } && stale == nil
    }

    /// Tiles in view that have an up-to-date image. For benchmarks.
    var visibleTilesReady: Bool {
        guard live.timePerPixel == viewTimePerPixel else { return false }
        let view = viewRect(in: live)
        for (key, tile) in live.tiles {
            let rect = CGRect(x: CGFloat(key.col) * tileWidth, y: CGFloat(key.row) * tileHeight, width: tileWidth, height: tileHeight)
            if rect.intersects(view), tile.paintedGeneration == 0 || tile.dirty || tile.inFlight {
                return false
            }
        }
        return true
    }
}

/// The bitmap and its coordinate system: y down in canvas points, top row of the image at the top of the tile.
nonisolated func renderTile(_ request: TileRequest, opaque: Bool, painter: TilePainter) -> PaintedTile {
    let width = Int((request.rect.width * request.scale).rounded(.up))
    let height = Int((request.rect.height * request.scale).rounded(.up))
    guard width > 0, height > 0, let surface = makeTileSurface(width: width, height: height, colorSpace: request.colorSpace) else {
        return PaintedTile(image: nil, waitingFor: [])
    }
    surface.lock(options: [], seed: nil)
    defer { surface.unlock(options: [], seed: nil) }
    guard
        let ctx = CGContext(
            data: surface.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: surface.bytesPerRow, space: request.colorSpace,
            bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
    else { return PaintedTile(image: nil, waitingFor: []) }
    ctx.translateBy(x: 0, y: CGFloat(height))
    ctx.scaleBy(x: request.scale, y: -request.scale)
    ctx.translateBy(x: -request.rect.minX, y: -request.rect.minY)
    ctx.setShouldAntialias(false)
    guard let waitingFor = painter(request, ctx) else { return PaintedTile(image: nil, waitingFor: [], failed: true) }
    return PaintedTile(image: surface, waitingFor: waitingFor)
}

/// 32BGRA, the layout of the context above, tagged with the colour space its values are in.
nonisolated private func makeTileSurface(width: Int, height: Int, colorSpace: CGColorSpace) -> IOSurface? {
    let surface = IOSurface(properties: [
        .width: width, .height: height, .bytesPerElement: 4, .pixelFormat: 0x4247_5241 as UInt32,
    ])
    if let list = colorSpace.copyPropertyList() {
        surface?.setAttachment(list, forKey: "IOSurfaceColorSpace")
    }
    return surface
}

nonisolated let sRGBSpace = CGColorSpace(name: CGColorSpace.sRGB)!
