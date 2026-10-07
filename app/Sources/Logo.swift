import AppKit
import SwiftUI

/// The Clipframes mark: a square frame with its top-right corner clipped off, and the
/// clipped piece lifted away. Same geometry as site/public/mark.svg and make-icon.swift.
enum ClipGeometry {
    /// The design box is 114 units square; these points are in it, top-left origin.
    static let box: CGFloat = 114
    static let frame: [CGPoint] = [(4, 10.2), (76, 10.2), (104, 38.2), (104, 110.2), (4, 110.2)].map { CGPoint(x: $0.0, y: $0.1) }
    static let hole = CGRect(x: 21, y: 27.2, width: 66, height: 66)
    static let piece: [CGPoint] = [(83.6, 4), (110, 4), (110, 30.4)].map { CGPoint(x: $0.0, y: $0.1) }

    /// The mark as a path in `rect`. `flipped` for AppKit's bottom-left origin.
    static func path(in rect: CGRect, flipped: Bool = false) -> CGPath {
        let k = min(rect.width, rect.height) / box
        let ox = rect.midX - box * k / 2, oy = rect.midY - box * k / 2
        func p(_ q: CGPoint) -> CGPoint {
            CGPoint(x: ox + q.x * k, y: flipped ? oy + (box - q.y) * k : oy + q.y * k)
        }
        let path = CGMutablePath()
        path.addLines(between: frame.map(p)); path.closeSubpath()
        let h = [CGPoint(x: hole.minX, y: hole.minY), CGPoint(x: hole.maxX, y: hole.minY),
                 CGPoint(x: hole.maxX, y: hole.maxY), CGPoint(x: hole.minX, y: hole.maxY)]
        path.addLines(between: h.map(p)); path.closeSubpath()
        path.addLines(between: piece.map(p)); path.closeSubpath()
        return path
    }
}

struct ClipMark: Shape {
    func path(in rect: CGRect) -> Path { Path(ClipGeometry.path(in: rect)) }
}

/// The mark on its off-white tile, like the app icon.
struct LogoMark: View {
    var size: CGFloat = 64
    var glow = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: size * 0.225, style: .continuous)
                .fill(Color(red: 0.984, green: 0.98, blue: 0.969))
            RoundedRectangle(cornerRadius: size * 0.225, style: .continuous)
                .strokeBorder(.black.opacity(0.08), lineWidth: max(0.5, size * 0.008))
            ClipMark()
                .fill(Color(red: 0.08, green: 0.08, blue: 0.08), style: FillStyle(eoFill: true))
                .padding(size * 0.2)
        }
        .frame(width: size, height: size)
        .shadow(color: .black.opacity(glow ? 0.45 : 0.25), radius: glow ? size * 0.22 : size * 0.04, y: size * (glow ? 0.08 : 0.03))
    }
}

extension Brand {
    /// The mark for the menu bar: a template image, so macOS tints it like its own icons.
    static let menuBarImage: NSImage = {
        let image = NSImage(size: NSSize(width: 16, height: 16), flipped: false) { rect in
            let ctx = NSGraphicsContext.current!.cgContext
            ctx.addPath(ClipGeometry.path(in: rect.insetBy(dx: 0.5, dy: 0.5), flipped: true))
            ctx.setFillColor(NSColor.black.cgColor)
            ctx.fillPath(using: .evenOdd)
            return true
        }
        image.isTemplate = true
        return image
    }()
}

/// The name, set the way the logo lockup sets it.
struct Wordmark: View {
    var size: CGFloat = 20
    var body: some View {
        Text("Clipframes").font(.system(size: size, weight: .semibold)).tracking(-size * 0.02)
    }
}
