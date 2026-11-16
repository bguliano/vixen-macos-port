// Writes the small test videos in tests/assets. Run from this folder with:  swift make_assets.swift
import AVFoundation
import CoreVideo

/// Paints a frame's left half and right half.
func paint(_ buffer: CVPixelBuffer, left: (UInt8, UInt8, UInt8), right: (UInt8, UInt8, UInt8)) {
    CVPixelBufferLockBaseAddress(buffer, [])
    defer { CVPixelBufferUnlockBaseAddress(buffer, []) }
    let base = CVPixelBufferGetBaseAddress(buffer)!.assumingMemoryBound(to: UInt8.self)
    let row = CVPixelBufferGetBytesPerRow(buffer)
    let (w, h) = (CVPixelBufferGetWidth(buffer), CVPixelBufferGetHeight(buffer))
    for y in 0..<h {
        for x in 0..<w {
            let (r, g, b) = x < w / 2 ? left : right
            let p = base + y * row + x * 4
            p[0] = b; p[1] = g; p[2] = r; p[3] = 255
        }
    }
}

func write(_ name: String, seconds: Int, transform: CGAffineTransform, colors: (Int) -> ((UInt8, UInt8, UInt8), (UInt8, UInt8, UInt8))) throws {
    let url = URL(fileURLWithPath: "assets/\(name)")
    try? FileManager.default.removeItem(at: url)
    let writer = try AVAssetWriter(outputURL: url, fileType: .mov)
    let (width, height, fps) = (160, 90, 30)
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height,
        AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 200_000],
    ])
    input.transform = transform
    let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height,
    ])
    writer.add(input)
    guard writer.startWriting() else { throw writer.error! }
    writer.startSession(atSourceTime: .zero)
    for frame in 0..<(seconds * fps) {
        while !input.isReadyForMoreMediaData { Thread.sleep(forTimeInterval: 0.005) }
        var buffer: CVPixelBuffer?
        CVPixelBufferPoolCreatePixelBuffer(nil, adaptor.pixelBufferPool!, &buffer)
        let (left, right) = colors(frame / fps)
        paint(buffer!, left: left, right: right)
        adaptor.append(buffer!, withPresentationTime: CMTime(value: CMTimeValue(frame), timescale: CMTimeScale(fps)))
    }
    input.markAsFinished()
    let done = DispatchSemaphore(value: 0)
    writer.finishWriting { done.signal() }
    done.wait()
    if let error = writer.error { throw error }
}

let red: (UInt8, UInt8, UInt8) = (255, 0, 0)
let green: (UInt8, UInt8, UInt8) = (0, 255, 0)
let blue: (UInt8, UInt8, UInt8) = (0, 0, 255)
// One solid colour per second.
try write("colors.mov", seconds: 3, transform: .identity) { second in
    let c = [red, green, blue][second]
    return (c, c)
}
// Red on the left and blue on the right as stored, tagged to display a quarter turn clockwise, as a phone records.
try write("quarter_turn.mov", seconds: 1, transform: CGAffineTransform(a: 0, b: 1, c: -1, d: 0, tx: 90, ty: 0)) { _ in (red, blue) }
