import AppKit
import Foundation

// MARK: What we know about a UI element

struct ElementInfo: Codable, Equatable {
    var app = ""
    var bundleID = ""
    var pid: Int32 = 0
    var window = ""
    var url = ""
    var role = ""
    var name = ""
    var value = ""
    var identifier = ""
    var domID = ""
    var domClasses = ""
    var innerText = ""
    var ocrText = ""
    var path: [String] = []
    var frame = CGRect.zero // global, top-left origin

    var roleName: String { role.isEmpty ? "Element" : role }

    var headline: String {
        if !name.isEmpty { return "\(roleName) \"\(name)\"" }
        if !innerText.isEmpty { return "\(roleName) containing \"\(innerText)\"" }
        if !value.isEmpty { return "\(roleName) \"\(value)\"" }
        if !ocrText.isEmpty { return "\(roleName) near the text \"\(ocrText)\"" }
        return roleName
    }

    /// Short label for the hover chip.
    var chip: String {
        let text = [name, innerText, value].first { !$0.isEmpty } ?? ""
        return text.isEmpty ? roleName : "\(roleName) · \(text.prefix(48))"
    }

    var selector: String {
        var parts: [String] = []
        if !domID.isEmpty { parts.append("#\(domID)") }
        let classes = domClasses.split(separator: " ").prefix(6)
        if !classes.isEmpty { parts.append("." + classes.joined(separator: ".")) }
        if !identifier.isEmpty { parts.append("id=\(identifier)") }
        return parts.joined(separator: " ")
    }

    var isWeak: Bool {
        name.isEmpty && innerText.isEmpty && value.isEmpty && domID.isEmpty && identifier.isEmpty
    }

    var searchText: String {
        [app, window, url, name, innerText, value, domID, domClasses, identifier, ocrText].joined(separator: " ")
    }
}

struct PinRecord: Codable, Identifiable, Equatable {
    var id = UUID()
    var number: Int
    var point: CGPoint // global, top-left origin
    var note: String
    var element: ElementInfo?
}

extension PinRecord {
    /// Where the number badge sits: the corner of the element's box, so it
    /// doesn't cover the label. Large areas keep the badge at the click.
    var badge: CGPoint {
        if let f = element?.frame, f.width > 0, f.width < 700, f.height < 320 {
            return CGPoint(x: f.minX, y: f.minY)
        }
        return point
    }
}

struct ClickRecord: Codable, Equatable {
    var time: Double
    var point: CGPoint
    var frame: Int?
    var element: ElementInfo?
}

enum CaptureKind: String, Codable {
    case element, screenshot, recording
}

struct CaptureMeta: Codable {
    var kind: CaptureKind
    var created: Date
    var app = ""
    var bundleID: String?
    var window = ""
    var url = ""
    var rect = CGRect.zero // what was captured, global top-left points
    var scale = 2.0
    var imageSize = CGSize.zero // pixels
    var note = ""
    var pins: [PinRecord] = []
    var clicks: [ClickRecord] = []
    var element: ElementInfo?
    var hints: [ElementInfo]?
    var duration: Double?
    var frameCount: Int?
    var interval: Double?
}

// MARK: A saved capture on disk

struct Capture: Identifiable, Hashable {
    let folder: URL
    var meta: CaptureMeta

    var id: String { folder.lastPathComponent }
    static func == (a: Capture, b: Capture) -> Bool { a.id == b.id && a.meta.note == b.meta.note }
    func hash(into h: inout Hasher) { h.combine(id) }

    var screenshot: URL { folder.appendingPathComponent("screenshot.png") }
    var annotated: URL { folder.appendingPathComponent("annotated.png") }
    var notesFile: URL { folder.appendingPathComponent("notes.md") }
    var framesFolder: URL { folder.appendingPathComponent("frames", isDirectory: true) }
    var video: URL { folder.appendingPathComponent("video.mov") }
    func frame(_ i: Int) -> URL { framesFolder.appendingPathComponent(String(format: "%03d.png", i)) }

    var thumbnail: URL {
        switch meta.kind {
        case .element: return screenshot
        case .screenshot: return meta.pins.isEmpty ? screenshot : annotated
        case .recording: return frame(1)
        }
    }

    /// Window or page title without the " - Google Chrome" tail; falls back to the app.
    var title: String {
        if meta.kind == .element, let el = meta.element { return el.headline }
        return windowTitle
    }

    var windowTitle: String {
        var w = meta.window
        for sep in [" - ", " — ", " – "] where w.hasSuffix(sep + meta.app) { w = String(w.dropLast(sep.count + meta.app.count)) }
        if !w.isEmpty { return w }
        if !meta.app.isEmpty { return meta.app }
        return meta.kind == .recording ? "Clip" : "Screenshot"
    }

    var subtitle: String {
        switch meta.kind {
        case .element:
            return windowTitle == meta.app || meta.app.isEmpty ? "Element" : "Element · \(windowTitle)"
        case .screenshot:
            let n = meta.pins.count
            if n > 0 { return "\(n) pin\(n == 1 ? "" : "s")" }
            let h = meta.hints?.count ?? 0
            return h == 0 ? "Screenshot" : "Screenshot · \(h) elements"
        case .recording:
            var s = "\(Fmt.seconds(meta.duration ?? 0)) · \(meta.frameCount ?? 0) frames"
            if !meta.clicks.isEmpty { s += " · \(meta.clicks.count) click\(meta.clicks.count == 1 ? "" : "s")" }
            return s
        }
    }

    /// The one line you paste into Claude Code or Codex.
    var reference: String {
        let what = meta.app.isEmpty ? "" : " of \(meta.app)" + (meta.window.isEmpty || meta.window == meta.app ? "" : " \"\(meta.window)\"")
        switch meta.kind {
        case .element:
            let el = meta.element
            let sel = (el?.selector ?? "").isEmpty ? "" : " (\(el!.selector))"
            return "[Element: \(el?.headline ?? "Element")\(sel)\(meta.app.isEmpty ? "" : " in")\(what.replacingOccurrences(of: " of ", with: " ")). Read \(notesFile.path)]"
        case .screenshot:
            let pins = meta.pins.isEmpty ? "" : ", \(meta.pins.count) pinned element\(meta.pins.count == 1 ? "" : "s")"
            return "[Screenshot\(what)\(pins). Read \(notesFile.path)]"
        case .recording:
            let clicks = meta.clicks.isEmpty ? "" : ", \(meta.clicks.count) click\(meta.clicks.count == 1 ? "" : "s")"
            return "[Screen recording\(what), \(Fmt.seconds(meta.duration ?? 0)), \(meta.frameCount ?? 0) frames\(clicks). Read \(notesFile.path)]"
        }
    }

    var searchText: String {
        var parts: [String] = [meta.app, meta.window, meta.url, meta.note, meta.element?.searchText ?? ""]
        for p in meta.pins { parts.append(p.note); parts.append(p.element?.searchText ?? "") }
        for c in meta.clicks { parts.append(c.element?.searchText ?? "") }
        for h in meta.hints ?? [] { parts.append(h.searchText) }
        return parts.joined(separator: " ").lowercased()
    }

    // MARK: notes.md, the file the agent reads

    var notes: String {
        var o: [String] = []
        let kindTitle = ["element": "Element", "screenshot": "Screenshot", "recording": "Screen clip"][meta.kind.rawValue]!
        let wt = windowTitle
        let heading = meta.kind == .element ? title
            : (meta.app.isEmpty ? wt : (wt == meta.app ? meta.app : "\(meta.app), \"\(wt)\""))
        o.append("# \(kindTitle): \(heading)")
        o.append("")
        if !meta.note.isEmpty { o.append("> \(meta.note.replacingOccurrences(of: "\n", with: "\n> "))"); o.append("") }
        if !meta.url.isEmpty { o.append("- Page: \(meta.url)") }
        o.append("- Taken: \(Fmt.stamp(meta.created))")
        let w = Int(meta.imageSize.width), h = Int(meta.imageSize.height)

        switch meta.kind {
        case .element:
            if !meta.app.isEmpty { o.append("- App: \(wt == meta.app ? meta.app : "\(meta.app), \"\(wt)\"")") }
            o.append("- Image: \(screenshot.path) (\(w)×\(h) px): the element with a little space around it")
            if let el = meta.element {
                if !el.selector.isEmpty { o.append("- Selector: \(el.selector)") }
                if !el.path.isEmpty { o.append("- Inside: \(el.path.joined(separator: " › "))") }
                if !el.name.isEmpty, !el.innerText.isEmpty { o.append("- Text inside: \(el.innerText)") }
                if !el.value.isEmpty, el.value != el.name { o.append("- Value: \(el.value)") }
                if !el.ocrText.isEmpty { o.append("- Text nearby: \(el.ocrText)") }
                o.append("- Size on screen: \(Int(el.frame.width))×\(Int(el.frame.height)) pt")
            }
        case .screenshot:
            if meta.pins.isEmpty {
                o.append("- Image: \(screenshot.path) (\(w)×\(h) px)")
                o.append(contentsOf: hintLines)
            } else {
                o.append("- Image with numbered pins: \(annotated.path)")
                o.append("- Same image without pins: \(screenshot.path)")
                o.append("- Both are \(w)×\(h) px. Positions below are in those pixels.")
                for pin in meta.pins {
                    o.append("")
                    o.append("## \(pin.number). \(pin.note.isEmpty ? "(no note)" : pin.note)")
                    o.append(contentsOf: describe(pin.element, at: pin.point))
                }
            }
        case .recording:
            o.append("- Length: \(Fmt.seconds(meta.duration ?? 0))")
            o.append("- Frames: \(meta.frameCount ?? 0), \(Fmt.seconds(meta.interval ?? 0)) apart, in order: \(framesFolder.path)/ (001.png, 002.png, …), \(w)×\(h) px")
            o.append("- Video: \(video.path)")
            o.append(contentsOf: hintLines)
            if !meta.clicks.isEmpty {
                o.append("- Each click is marked on the frame right after it with a numbered ring.")
                o.append("")
                o.append("## Clicks")
                for (i, c) in meta.clicks.enumerated() {
                    let frameText = c.frame.map { ", frame \(String(format: "%03d", $0))" } ?? ""
                    o.append("")
                    o.append("\(i + 1). \(Fmt.clock(c.time, precise: true))\(frameText): \(c.element?.headline ?? "somewhere on screen")")
                    o.append(contentsOf: describe(c.element, at: c.point, compact: true))
                }
            }
        }
        o.append("")
        return o.joined(separator: "\n")
    }

    /// Elements found inside the captured area, with their boxes in image pixels.
    private var hintLines: [String] {
        guard let hints = meta.hints, !hints.isEmpty else { return [] }
        let s = meta.scale
        var o = ["", "## What's in this area", ""]
        for el in hints {
            let f = el.frame
            var line = "- \(el.headline)"
            if !el.selector.isEmpty { line += " · \(el.selector)" }
            line += " · x \(Int((f.minX - meta.rect.minX) * s)), y \(Int((f.minY - meta.rect.minY) * s)), \(Int(f.width * s))×\(Int(f.height * s))"
            o.append(line)
        }
        return o
    }

    private func describe(_ el: ElementInfo?, at point: CGPoint, compact: Bool = false) -> [String] {
        var o: [String] = []
        let s = meta.scale
        let px = Int((point.x - meta.rect.minX) * s), py = Int((point.y - meta.rect.minY) * s)
        if let el {
            if !compact { o.append("- Element: \(el.headline)") }
            if !el.selector.isEmpty { o.append("- Selector: \(el.selector)") }
            if !el.path.isEmpty { o.append("- Inside: \(el.path.joined(separator: " › "))") }
            if !el.ocrText.isEmpty, compact || !el.headline.contains(el.ocrText) { o.append("- Text nearby: \(el.ocrText)") }
            if el.app != meta.app, !el.app.isEmpty { o.append("- App: \(el.app)") }
            if !el.url.isEmpty, el.url != meta.url { o.append("- Page: \(el.url)") }
            if el.frame.width > 0 {
                let f = el.frame
                o.append("- Box: x \(Int((f.minX - meta.rect.minX) * s)), y \(Int((f.minY - meta.rect.minY) * s)), \(Int(f.width * s))×\(Int(f.height * s))")
            }
        }
        o.append("- Point: x \(px), y \(py)")
        return o.map { compact ? "   \($0)" : $0 }
    }
}

// MARK: Formatting

enum Fmt {
    static func seconds(_ s: Double) -> String { String(format: "%.1f s", s) }
    static func clock(_ s: TimeInterval, precise: Bool = false) -> String {
        let t = max(0, s)
        return precise ? String(format: "%d:%04.1f", Int(t) / 60, t.truncatingRemainder(dividingBy: 60))
            : String(format: "%d:%02d", Int(t) / 60, Int(t) % 60)
    }
    static func stamp(_ d: Date) -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd HH:mm"
        return f.string(from: d)
    }
    static func relative(_ d: Date) -> String {
        let f = DateFormatter()
        f.doesRelativeDateFormatting = true
        f.dateStyle = .medium
        f.timeStyle = .short
        return f.string(from: d)
    }
    static func folderName(_ d: Date) -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        return f.string(from: d)
    }
}

// MARK: Coordinates. Accessibility and CoreGraphics use a top-left origin on
// the primary screen; AppKit uses bottom-left.

enum Geo {
    static var primaryTop: CGFloat { NSScreen.screens.first?.frame.maxY ?? 0 }
    static func toGlobal(_ p: NSPoint) -> CGPoint { CGPoint(x: p.x, y: primaryTop - p.y) }
    static func mouse() -> CGPoint { toGlobal(NSEvent.mouseLocation) }
    static func rect(of screen: NSScreen) -> CGRect {
        let f = screen.frame
        return CGRect(x: f.minX, y: primaryTop - f.maxY, width: f.width, height: f.height)
    }
    static func cocoaRect(_ r: CGRect) -> NSRect {
        NSRect(x: r.minX, y: primaryTop - r.maxY, width: r.width, height: r.height)
    }
    static func screen(containing p: CGPoint) -> NSScreen? {
        NSScreen.screens.first { rect(of: $0).contains(p) } ?? NSScreen.main
    }
}
