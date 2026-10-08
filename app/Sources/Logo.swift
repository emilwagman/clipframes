import AppKit
import SwiftUI

/// The Clipframes mark: a square frame with its top-right corner clipped off into the window,
/// the clipped piece lifted away, every corner rounded. Same geometry as site/public/mark.svg
/// and make-icon.swift.
enum ClipGeometry {
    /// The design box is 114 units square; these points are in it, top-left origin.
    static let box: CGFloat = 114
    /// The frame, one outline: the cut runs along x - y = 64 (frame units) and opens into the window.
    static let frame: [CGPoint] = [(0, 0), (64, 0), (81, 17), (17, 17), (17, 83), (83, 83), (83, 19), (100, 36), (100, 100), (0, 100)]
        .map { CGPoint(x: $0.0 + 4, y: $0.1 + 10.2) }
    static let piece: [CGPoint] = [(75.66, 4), (109.5, 4), (109.5, 37.84)].map { CGPoint(x: $0.0, y: $0.1) }
    static let frameRadius: CGFloat = 6
    static let pieceRadius: CGFloat = 4

    /// The mark as a path in `rect`. `flipped` for AppKit's bottom-left origin.
    static func path(in rect: CGRect, flipped: Bool = false) -> CGPath {
        let k = min(rect.width, rect.height) / box
        let ox = rect.midX - box * k / 2, oy = rect.midY - box * k / 2
        func p(_ q: CGPoint) -> CGPoint {
            CGPoint(x: ox + q.x * k, y: flipped ? oy + (box - q.y) * k : oy + q.y * k)
        }
        let path = CGMutablePath()
        add(frame.map(p), radius: frameRadius * k, to: path)
        add(piece.map(p), radius: pieceRadius * k, to: path)
        return path
    }

    /// A closed polygon with every corner rounded.
    private static func add(_ pts: [CGPoint], radius: CGFloat, to path: CGMutablePath) {
        let n = pts.count
        let start = CGPoint(x: (pts[n - 1].x + pts[0].x) / 2, y: (pts[n - 1].y + pts[0].y) / 2)
        path.move(to: start)
        for i in 0..<n {
            path.addArc(tangent1End: pts[i], tangent2End: pts[(i + 1) % n], radius: radius)
        }
        path.closeSubpath()
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
                .fill(Color.white)
            RoundedRectangle(cornerRadius: size * 0.225, style: .continuous)
                .strokeBorder(.black.opacity(0.08), lineWidth: max(0.5, size * 0.008))
            ClipMark()
                .fill(Color(red: 0.07, green: 0.07, blue: 0.07))
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
            ctx.fillPath()
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
