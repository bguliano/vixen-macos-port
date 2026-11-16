// Read actual window-snapshot pixels when SwiftUI omits in-process AX text.
// Standalone verification helper; never linked into Vixen.
import Foundation
import ImageIO
import Vision

struct Line: Encodable {
    let text: String
    let confidence: Float
    let x: CGFloat
    let y: CGFloat
    let width: CGFloat
    let height: CGFloat
}
guard CommandLine.arguments.count == 2,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    fatalError("needs a readable window snapshot path")
}
let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.recognitionLanguages = ["en-US"]
request.usesLanguageCorrection = false
request.usesCPUOnly = true
do {
    try VNImageRequestHandler(cgImage: image).perform([request])
} catch {
    FileHandle.standardError.write(Data("Snapshot OCR failed: \(error)\n".utf8))
    exit(1)
}
let lines = (request.results ?? []).compactMap { observation -> Line? in
    guard let candidate = observation.topCandidates(1).first else { return nil }
    let box = observation.boundingBox
    return Line(text: candidate.string, confidence: candidate.confidence,
                x: box.minX, y: box.midY, width: box.width, height: box.height)
}
let encoder = JSONEncoder()
encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
FileHandle.standardOutput.write(try encoder.encode(lines))
