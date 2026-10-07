import AppKit

/// Turns a finished shot into a saved capture.
@MainActor
enum Captures {
    static func saveElement(_ shot: Shot, element: ElementInfo) async throws {
        var el = element
        if el.isWeak { el.ocrText = Shooter.textNear(CGPoint(x: shot.rect.midX, y: shot.rect.midY), in: shot) }
        let lib = Library.shared
        let created = Date()
        var meta = CaptureMeta(kind: .element, created: created)
        meta.app = el.app
        meta.bundleID = el.bundleID.isEmpty ? nil : el.bundleID
        meta.window = el.window
        meta.url = el.url
        meta.element = el
        let capture = try write(shot, meta: meta, created: created)
        lib.add(capture)
    }

    static func saveScreenshot(_ shot: Shot) async throws {
        let lib = Library.shared
        let created = Date()
        var meta = CaptureMeta(kind: .screenshot, created: created)
        let center = CGPoint(x: shot.rect.midX, y: shot.rect.midY)
        if let w = WindowList.at(center) {
            let running = NSRunningApplication(processIdentifier: w.pid)
            meta.app = running?.localizedName ?? w.owner
            meta.bundleID = running?.bundleIdentifier
            meta.window = w.title
        }
        var capture = try write(shot, meta: meta, created: created)
        lib.add(capture)
        // Name what's in the area. The reference is already copied; notes.md fills in a moment later.
        let hints = await elements(in: shot.rect)
        capture.meta.hints = hints
        if capture.meta.url.isEmpty { capture.meta.url = hints.lazy.map(\.url).first { !$0.isEmpty } ?? "" }
        lib.update(capture)
    }

    static func elements(in rect: CGRect) async -> [ElementInfo] {
        await withCheckedContinuation { c in
            AXReader.shared.queue.async { c.resume(returning: AXReader.shared.elements(in: rect)) }
        }
    }

    private static func write(_ shot: Shot, meta inMeta: CaptureMeta, created: Date) throws -> Capture {
        var meta = inMeta
        meta.rect = shot.rect
        meta.scale = shot.scale
        meta.imageSize = CGSize(width: shot.image.width, height: shot.image.height)
        let dir = try Library.shared.newFolder(created)
        let capture = Capture(folder: dir, meta: meta)
        try Shooter.writePNG(shot.image, to: capture.screenshot)
        try Library.shared.save(capture)
        return capture
    }
}
