import SwiftUI

/// The Clipframes mark: a tangerine squircle holding a viewfinder frame around a capture dot.
struct LogoMark: View {
    var size: CGFloat = 64
    var glow = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: size * 0.27, style: .continuous)
                .fill(LinearGradient(colors: [Color(red: 1.0, green: 0.58, blue: 0.28), Brand.accentDeep],
                                     startPoint: .topLeading, endPoint: .bottomTrailing))
            // Soft top light
            RoundedRectangle(cornerRadius: size * 0.27, style: .continuous)
                .fill(LinearGradient(colors: [.white.opacity(0.28), .clear], startPoint: .top, endPoint: .center))
                .padding(size * 0.02)
                .blendMode(.softLight)
            RoundedRectangle(cornerRadius: size * 0.27, style: .continuous)
                .strokeBorder(.white.opacity(0.22), lineWidth: max(0.5, size * 0.012))
            FrameBrackets()
                .stroke(.white, style: StrokeStyle(lineWidth: size * 0.075, lineCap: .round, lineJoin: .round))
                .padding(size * 0.24)
            Circle()
                .fill(.white)
                .frame(width: size * 0.2, height: size * 0.2)
                .offset(x: size * 0.035, y: size * 0.035)
        }
        .frame(width: size, height: size)
        .shadow(color: Brand.accent.opacity(glow ? 0.55 : 0), radius: size * 0.35, y: size * 0.08)
        .shadow(color: .black.opacity(0.25), radius: size * 0.04, y: size * 0.03)
    }
}

/// Four rounded corner brackets.
struct FrameBrackets: Shape {
    func path(in r: CGRect) -> Path {
        let l = r.width * 0.3, k = r.width * 0.1
        var p = Path()
        // top-left
        p.move(to: CGPoint(x: r.minX, y: r.minY + l)); p.addLine(to: CGPoint(x: r.minX, y: r.minY + k))
        p.addQuadCurve(to: CGPoint(x: r.minX + k, y: r.minY), control: CGPoint(x: r.minX, y: r.minY))
        p.addLine(to: CGPoint(x: r.minX + l, y: r.minY))
        // top-right
        p.move(to: CGPoint(x: r.maxX - l, y: r.minY)); p.addLine(to: CGPoint(x: r.maxX - k, y: r.minY))
        p.addQuadCurve(to: CGPoint(x: r.maxX, y: r.minY + k), control: CGPoint(x: r.maxX, y: r.minY))
        p.addLine(to: CGPoint(x: r.maxX, y: r.minY + l))
        // bottom-right
        p.move(to: CGPoint(x: r.maxX, y: r.maxY - l)); p.addLine(to: CGPoint(x: r.maxX, y: r.maxY - k))
        p.addQuadCurve(to: CGPoint(x: r.maxX - k, y: r.maxY), control: CGPoint(x: r.maxX, y: r.maxY))
        p.addLine(to: CGPoint(x: r.maxX - l, y: r.maxY))
        // bottom-left
        p.move(to: CGPoint(x: r.minX + l, y: r.maxY)); p.addLine(to: CGPoint(x: r.minX + k, y: r.maxY))
        p.addQuadCurve(to: CGPoint(x: r.minX, y: r.maxY - k), control: CGPoint(x: r.minX, y: r.maxY))
        p.addLine(to: CGPoint(x: r.minX, y: r.maxY - l))
        return p
    }
}

/// The name set in the brand's rounded face.
struct Wordmark: View {
    var size: CGFloat = 20
    var body: some View {
        Text("Clipframes").font(Brand.display(size, .bold)).tracking(-size * 0.02)
    }
}
