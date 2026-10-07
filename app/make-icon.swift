// Draws the app icon: the Clipframes mark, a tangerine squircle holding a
// viewfinder frame around a capture dot. Matches LogoMark in Sources/Logo.swift.
import AppKit

let out = CommandLine.arguments[1]
let sizes: [(Int, String)] = [
    (16, "16x16"), (32, "16x16@2x"), (32, "32x32"), (64, "32x32@2x"),
    (128, "128x128"), (256, "128x128@2x"), (256, "256x256"), (512, "256x256@2x"),
    (512, "512x512"), (1024, "512x512@2x"),
]

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
    let shape = NSBezierPath(roundedRect: body, xRadius: r, yRadius: r)

    let shadow = NSShadow()
    shadow.shadowColor = NSColor(red: 0.45, green: 0.15, blue: 0.02, alpha: 0.35)
    shadow.shadowOffset = NSSize(width: 0, height: -s * 0.014)
    shadow.shadowBlurRadius = s * 0.035
    NSGraphicsContext.saveGraphicsState()
    shadow.set()
    NSColor.black.setFill()
    shape.fill()
    NSGraphicsContext.restoreGraphicsState()

    NSGradient(colors: [
        NSColor(red: 1.0, green: 0.60, blue: 0.30, alpha: 1),
        NSColor(red: 0.95, green: 0.33, blue: 0.12, alpha: 1),
    ])!.draw(in: shape, angle: -60)
    // Soft top light
    NSGradient(colors: [NSColor.white.withAlphaComponent(0.22), NSColor.white.withAlphaComponent(0)])!
        .draw(in: shape, angle: -90)
    NSColor.white.withAlphaComponent(0.25).setStroke()
    shape.lineWidth = max(1, s * 0.006)
    shape.stroke()

    // Brackets
    let f = body.insetBy(dx: body.width * 0.24, dy: body.width * 0.24)
    let l = f.width * 0.3, k = f.width * 0.1
    let b = NSBezierPath()
    b.lineWidth = body.width * 0.075
    b.lineCapStyle = .round
    b.lineJoinStyle = .round
    func corner(_ a: NSPoint, _ c: NSPoint, _ e: NSPoint, _ s1: NSPoint, _ s2: NSPoint) {
        b.move(to: a); b.line(to: s1); b.curve(to: s2, controlPoint1: c, controlPoint2: c); b.line(to: e)
    }
    corner(NSPoint(x: f.minX, y: f.maxY - l), NSPoint(x: f.minX, y: f.maxY), NSPoint(x: f.minX + l, y: f.maxY),
           NSPoint(x: f.minX, y: f.maxY - k), NSPoint(x: f.minX + k, y: f.maxY))
    corner(NSPoint(x: f.maxX - l, y: f.maxY), NSPoint(x: f.maxX, y: f.maxY), NSPoint(x: f.maxX, y: f.maxY - l),
           NSPoint(x: f.maxX - k, y: f.maxY), NSPoint(x: f.maxX, y: f.maxY - k))
    corner(NSPoint(x: f.maxX, y: f.minY + l), NSPoint(x: f.maxX, y: f.minY), NSPoint(x: f.maxX - l, y: f.minY),
           NSPoint(x: f.maxX, y: f.minY + k), NSPoint(x: f.maxX - k, y: f.minY))
    corner(NSPoint(x: f.minX + l, y: f.minY), NSPoint(x: f.minX, y: f.minY), NSPoint(x: f.minX, y: f.minY + l),
           NSPoint(x: f.minX + k, y: f.minY), NSPoint(x: f.minX, y: f.minY + k))
    NSColor.white.setStroke()
    b.stroke()

    // Capture dot, a touch right of and below centre
    let d = body.width * 0.2
    let cx = body.midX + body.width * 0.035, cy = body.midY - body.width * 0.035
    NSColor.white.setFill()
    NSBezierPath(ovalIn: NSRect(x: cx - d / 2, y: cy - d / 2, width: d, height: d)).fill()

    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

try? FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
for (px, name) in sizes {
    try! draw(px).write(to: URL(fileURLWithPath: "\(out)/icon_\(name).png"))
}
