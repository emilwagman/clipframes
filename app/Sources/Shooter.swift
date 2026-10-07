import AppKit
import ScreenCaptureKit
import UniformTypeIdentifiers
import Vision

struct Shot {
    let image: CGImage
    let rect: CGRect // global, top-left origin
    let scale: CGFloat
}

enum Shooter {
    struct Failure: LocalizedError { let errorDescription: String? }

    static func window(_ ref: ScreenWindow) async throws -> Shot {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard let win = content.windows.first(where: { $0.windowID == ref.id }) else {
            throw Failure(errorDescription: "that window is gone")
        }
        let filter = SCContentFilter(desktopIndependentWindow: win)
        return try await shoot(filter, rect: win.frame)
    }

    static func screen(containing p: CGPoint) async throws -> Shot {
        let (filter, rect) = try await screenFilter(containing: p)
        return try await shoot(filter, rect: rect)
    }

    /// Any area of the screen (clipped to the display it's mostly on), without Clipframes' own windows.
    static func region(_ rect: CGRect) async throws -> Shot {
        let (filter, r, sourceRect) = try await regionFilter(rect)
        let scale = CGFloat(filter.pointPixelScale)
        let config = SCStreamConfiguration()
        config.sourceRect = sourceRect
        config.width = max(2, Int(r.width * scale))
        config.height = max(2, Int(r.height * scale))
        config.showsCursor = false
        config.captureResolution = .best
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
        return Shot(image: image, rect: r, scale: CGFloat(image.width) / max(1, r.width))
    }

    /// The display filter for an area, the area clipped to that display, and the area in display coordinates.
    static func regionFilter(_ rect: CGRect) async throws -> (SCContentFilter, CGRect, CGRect) {
        let content = try await Self.content()
        let center = CGPoint(x: rect.midX, y: rect.midY)
        guard let display = content.displays.first(where: { $0.frame.contains(center) })
                ?? content.displays.first(where: { $0.frame.intersects(rect) }) else {
            throw Failure(errorDescription: "that area isn't on a screen")
        }
        let r = rect.intersection(display.frame).integral
        guard r.width >= 2, r.height >= 2 else { throw Failure(errorDescription: "that area is too small") }
        let me = content.applications.filter { $0.processID == getpid() }
        let filter = SCContentFilter(display: display, excludingApplications: me, exceptingWindows: [])
        let source = CGRect(x: r.minX - display.frame.minX, y: r.minY - display.frame.minY, width: r.width, height: r.height)
        return (filter, r, source)
    }

    /// The display under a point, with Clipframes' own windows left out.
    static func screenFilter(containing p: CGPoint) async throws -> (SCContentFilter, CGRect) {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard let display = content.displays.first(where: { $0.frame.contains(p) }) ?? content.displays.first else {
            throw Failure(errorDescription: "no display found")
        }
        let me = content.applications.filter { $0.processID == getpid() }
        return (SCContentFilter(display: display, excludingApplications: me, exceptingWindows: []), display.frame)
    }

    // Asking for the list of windows and displays is the slow part of starting a
    // recording (~1 s). Fetch it while the user is still choosing, reuse it briefly.
    private static var cached: (SCShareableContent, Date)?
    static func prewarm() { Task { _ = try? await content() } }
    static func content() async throws -> SCShareableContent {
        if let (c, at) = cached, Date().timeIntervalSince(at) < 4 { return c }
        let c = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        cached = (c, Date())
        return c
    }

    static func windowFilter(_ ref: ScreenWindow) async throws -> (SCContentFilter, CGRect) {
        let content = try await Self.content()
        guard let win = content.windows.first(where: { $0.windowID == ref.id }) else {
            throw Failure(errorDescription: "that window is gone")
        }
        return (SCContentFilter(desktopIndependentWindow: win), win.frame)
    }

    private static func shoot(_ filter: SCContentFilter, rect: CGRect) async throws -> Shot {
        let scale = CGFloat(filter.pointPixelScale)
        let config = SCStreamConfiguration()
        config.width = Int(filter.contentRect.width * scale)
        config.height = Int(filter.contentRect.height * scale)
        config.showsCursor = false
        config.captureResolution = .best
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
        let actualScale = CGFloat(image.width) / max(1, rect.width)
        return Shot(image: image, rect: rect, scale: actualScale)
    }

    // MARK: Drawing pins and click rings

    static let pinColor = NSColor(red: 0.95, green: 0.29, blue: 0.36, alpha: 1)

    /// Draws numbered markers. `boxes` outline elements; points get a badge.
    static func annotate(_ image: CGImage, rect: CGRect, scale: CGFloat,
                         marks: [(number: Int, point: CGPoint, box: CGRect?, ring: Bool)]) -> CGImage {
        let w = image.width, h = image.height
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return image }
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
        let ns = NSGraphicsContext(cgContext: ctx, flipped: false)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = ns

        func toImage(_ p: CGPoint) -> CGPoint {
            CGPoint(x: (p.x - rect.minX) * scale, y: CGFloat(h) - (p.y - rect.minY) * scale)
        }
        let u = max(1, scale)

        for m in marks {
            if let box = m.box, box.width > 0, box.width < rect.width * 0.95 {
                let o = toImage(CGPoint(x: box.minX, y: box.maxY))
                let r = NSRect(x: o.x, y: o.y, width: box.width * scale, height: box.height * scale).insetBy(dx: -2 * u, dy: -2 * u)
                let path = NSBezierPath(roundedRect: r, xRadius: 4 * u, yRadius: 4 * u)
                pinColor.withAlphaComponent(0.10).setFill()
                path.fill()
                pinColor.setStroke()
                path.lineWidth = 2 * u
                path.stroke()
            }
            let c = toImage(m.point)
            if m.ring {
                let ring = NSBezierPath(ovalIn: NSRect(x: c.x - 16 * u, y: c.y - 16 * u, width: 32 * u, height: 32 * u))
                ring.lineWidth = 3 * u
                pinColor.setStroke()
                ring.stroke()
            }
            let radius = 11 * u
            let badgeCenter = m.ring ? CGPoint(x: c.x + 16 * u, y: c.y + 16 * u) : c
            let badge = NSBezierPath(ovalIn: NSRect(x: badgeCenter.x - radius, y: badgeCenter.y - radius, width: radius * 2, height: radius * 2))
            NSColor.white.setFill()
            NSBezierPath(ovalIn: NSRect(x: badgeCenter.x - radius - 2 * u, y: badgeCenter.y - radius - 2 * u,
                                        width: (radius + 2 * u) * 2, height: (radius + 2 * u) * 2)).fill()
            pinColor.setFill()
            badge.fill()
            let text = NSAttributedString(string: "\(m.number)", attributes: [
                .font: NSFont.systemFont(ofSize: 13 * u, weight: .bold),
                .foregroundColor: NSColor.white,
            ])
            let size = text.size()
            text.draw(at: NSPoint(x: badgeCenter.x - size.width / 2, y: badgeCenter.y - size.height / 2))
        }
        NSGraphicsContext.restoreGraphicsState()
        return ctx.makeImage() ?? image
    }

    // MARK: Text around a point, for UIs the accessibility tree can't see into

    static func textNear(_ p: CGPoint, in shot: Shot) -> String {
        let s = shot.scale
        let cx = (p.x - shot.rect.minX) * s, cy = (p.y - shot.rect.minY) * s
        let box = CGRect(x: cx - 180 * s, y: cy - 60 * s, width: 360 * s, height: 120 * s)
            .intersection(CGRect(x: 0, y: 0, width: shot.image.width, height: shot.image.height))
        guard !box.isNull, let crop = shot.image.cropping(to: box) else { return "" }
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        try? VNImageRequestHandler(cgImage: crop).perform([request])
        let center = CGPoint(x: (cx - box.minX) / box.width, y: 1 - (cy - box.minY) / box.height)
        let lines = (request.results ?? [])
            .compactMap { obs -> (String, CGFloat)? in
                guard let t = obs.topCandidates(1).first?.string else { return nil }
                let b = obs.boundingBox
                let d = hypot(b.midX - center.x, b.midY - center.y)
                return (t, d)
            }
            .sorted { $0.1 < $1.1 }
            .prefix(2)
            .map(\.0)
        return lines.joined(separator: " / ")
    }

    static func writePNG(_ image: CGImage, to url: URL) throws {
        guard let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else {
            throw Failure(errorDescription: "couldn't write \(url.lastPathComponent)")
        }
        CGImageDestinationAddImage(dest, image, nil)
        guard CGImageDestinationFinalize(dest) else { throw Failure(errorDescription: "couldn't write \(url.lastPathComponent)") }
    }

    static func readImage(_ url: URL) -> CGImage? {
        guard let src = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(src, 0, nil)
    }
}
