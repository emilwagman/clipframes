// Draws the app icon: the Clipframes mark (a frame with its corner clipped off into the
// window, the piece lifted away, corners rounded) in near-black on a white tile. Same geometry as ClipGeometry
// in Sources/Logo.swift and site/public/mark.svg.
import AppKit

let out = CommandLine.arguments[1]
let sizes: [(Int, String)] = [
    (16, "16x16"), (32, "16x16@2x"), (32, "32x32"), (64, "32x32@2x"),
    (128, "128x128"), (256, "128x128@2x"), (256, "256x256"), (512, "256x256@2x"),
    (512, "512x512"), (1024, "512x512@2x"),
]

// The mark in a 114-unit box, top-left origin: the frame as one outline (the cut opens into
// the window) and the lifted piece, every corner rounded.
let box: CGFloat = 114
let frame: [(CGFloat, CGFloat)] = [(0, 0), (64, 0), (81, 17), (17, 17), (17, 83), (83, 83), (83, 19), (100, 36), (100, 100), (0, 100)]
    .map { ($0.0 + 4, $0.1 + 10.2) }
let piece: [(CGFloat, CGFloat)] = [(75.66, 4), (109.5, 4), (109.5, 37.84)]

func mark(in r: NSRect) -> NSBezierPath {
    let k = r.width / box
    let path = NSBezierPath()
    for (shape, radius) in [(frame, CGFloat(6)), (piece, CGFloat(4))] {
        let pts = shape.map { NSPoint(x: r.minX + $0.0 * k, y: r.maxY - $0.1 * k) }   // flip for AppKit
        let n = pts.count
        path.move(to: NSPoint(x: (pts[n - 1].x + pts[0].x) / 2, y: (pts[n - 1].y + pts[0].y) / 2))
        for i in 0..<n { path.appendArc(from: pts[i], to: pts[(i + 1) % n], radius: radius * k) }
        path.close()
    }
    return path
}

func draw(_ px: Int) -> Data {
    let s = CGFloat(px)
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)

    // macOS icon grid: 824/1024 body with room for the shadow.
    let inset = s * 100 / 1024
    let body = NSRect(x: inset, y: inset, width: s - inset * 2, height: s - inset * 2)
    let r = body.width * 0.225
    let tile = NSBezierPath(roundedRect: body, xRadius: r, yRadius: r)

    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.28)
    shadow.shadowOffset = NSSize(width: 0, height: -s * 0.012)
    shadow.shadowBlurRadius = s * 0.03
    NSGraphicsContext.saveGraphicsState()
    shadow.set()
    NSColor.white.setFill()
    tile.fill()
    NSGraphicsContext.restoreGraphicsState()
    NSColor.black.withAlphaComponent(0.08).setStroke()
    tile.lineWidth = max(1, s * 0.004)
    tile.stroke()

    // The mark takes 60% of the tile, centred.
    let m = body.width * 0.6
    NSColor(red: 0.07, green: 0.07, blue: 0.07, alpha: 1).setFill()
    mark(in: NSRect(x: body.midX - m / 2, y: body.midY - m / 2, width: m, height: m)).fill()

    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

try? FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
for (px, name) in sizes {
    try! draw(px).write(to: URL(fileURLWithPath: "\(out)/icon_\(name).png"))
}
