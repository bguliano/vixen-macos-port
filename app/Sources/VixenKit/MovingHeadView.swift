import AppKit
import QuartzCore
import SwiftUI
import VixenBridge

/// Every fixture view shares the Rust mesh, camera, and owner-confirmed bottom-mount +180° tilt.
public struct MovingHeadView: NSViewRepresentable {
    let pan: Float
    let tilt: Float
    let color: UInt32
    let animation: String
    public init(pan: Float = 35, tilt: Float = 45, color: UInt32 = 0xff1e90ff, animation: String = "") {
        self.pan = pan; self.tilt = tilt; self.color = color; self.animation = animation
    }
    public func makeNSView(context: Context) -> MovingHeadNSView { MovingHeadNSView() }
    public func updateNSView(_ view: MovingHeadNSView, context: Context) { view.configure(pan: pan, tilt: tilt, color: color, animation: animation) }
    public static func dismantleNSView(_ view: MovingHeadNSView, coordinator: ()) { view.stop() }
}

public final class MovingHeadNSView: NSView {
    private let metal = MetalLayerNSView(frame: .zero)
    private var demo: FixtureDemo?
    private var link: CADisplayLink?
    private var generation = 0
    private var creating = false
    private var pan: Float = 35
    private var tilt: Float = 45
    private var color: UInt32 = 0xff1e90ff
    private var animation = ""
    private var epoch = 0.0
    public init() {
        super.init(frame: .zero)
        addSubview(metal)
        metal.metalLayer?.isOpaque = false
        metal.onResize = { [weak self] w, h, scale in self?.demo?.resize(width: w, height: h, scale: scale) }
        setAccessibilityElement(false)
    }
    required init?(coder: NSCoder) { fatalError("not from a file") }
    public override func layout() { super.layout(); metal.frame = bounds }
    public override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); if window == nil { stop() } else { start() } }
    func configure(pan: Float, tilt: Float, color: UInt32, animation: String) {
        let changed = self.pan != pan || self.tilt != tilt || self.color != color || self.animation != animation
        let animationChanged = self.animation != animation
        self.pan = pan; self.tilt = tilt; self.color = color; self.animation = animation
        if changed { demo?.setPose(pan: pan, tilt: tilt, argb: color) }
        if animationChanged { epoch = CACurrentMediaTime(); updateLink() }
    }
    private func start() {
        guard demo == nil, !creating, window != nil else { return }
        creating = true; generation += 1
        let run = generation
        let address = metal.layerAddress, size = metal.pixelSize, scale = metal.scale
        Task {
            let opened = await Task.detached { try? FixtureDemo.open(layer: address, width: size.width, height: size.height, scale: scale) }.value
            guard run == generation, window != nil else { return }
            creating = false; demo = opened
            let size = metal.pixelSize
            demo?.resize(width: size.width, height: size.height, scale: metal.scale)
            demo?.setPose(pan: pan, tilt: tilt, argb: color)
            epoch = CACurrentMediaTime(); updateLink()
        }
    }
    func stop() { generation += 1; creating = false; link?.invalidate(); link = nil; demo = nil }
    private func updateLink() {
        link?.invalidate(); link = nil
        guard demo != nil, window != nil, !animation.isEmpty else { return }
        let display = displayLink(target: MovingHeadTickTarget(view: self), selector: #selector(MovingHeadTickTarget.tick))
        display.add(to: .main, forMode: .common); link = display
    }
    fileprivate func tick() {
        guard window?.isVisible == true, !isHiddenOrHasHiddenAncestor else { return }
        demo?.advance(elapsed: CACurrentMediaTime() - epoch, mode: animation)
    }
}
private final class MovingHeadTickTarget: NSObject {
    weak var view: MovingHeadNSView?
    init(view: MovingHeadNSView) { self.view = view }
    @objc func tick() { view?.tick() }
}
