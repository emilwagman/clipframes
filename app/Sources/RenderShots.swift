import AppKit
import SwiftUI

/// `Clipframes --render-shots <out> --demo <dir>`: draws the app's real views into PNGs for the
/// website and quits, without showing a window. `<dir>` holds Northwind pictures made by the
/// site's render script (element.png, area.png, frame-1.png…, page.png) for the demo library.
@MainActor
enum RenderShots {
    static func runIfAsked() -> Bool {
        let args = CommandLine.arguments
        guard let i = args.firstIndex(of: "--render-shots"), i + 1 < args.count else { return false }
        let out = URL(fileURLWithPath: args[i + 1], isDirectory: true)
        let demo = args.firstIndex(of: "--demo").flatMap { $0 + 1 < args.count ? URL(fileURLWithPath: args[$0 + 1]) : nil }
        NSApp.setActivationPolicy(.prohibited)
        try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

        let lib = Library.shared
        lib.permissions = Permissions(screen: true, accessibility: true)
        let made = demo.map(makeDemoLibrary) ?? []
        lib.captures = made
        lib.selection = made.first.map { [$0.id] } ?? []

        // The bar, at rest and with the keyboard on it.
        shot(BarView().environmentObject(lib), "bar", to: out)
        BarFocus.shared.keyboard = true
        shot(BarView().environmentObject(lib), "bar-keys", to: out)
        BarFocus.shared.keyboard = false

        // The confirmation after a capture.
        shot(Toast.view("Copied. Paste it into your agent."), "toast-copied", to: out)

        // The capture overlay over a 1440 × 900 screen: Element on "New invoice", then a dragged area.
        let screen = CGRect(x: 0, y: 0, width: 1440, height: 900)
        let element = OverlaySession.forRendering(.element)
        element.hover = sampleButton
        element.mouse = CGPoint(x: sampleButton.frame.midX, y: sampleButton.frame.midY)
        shot(OverlayView(screenRect: screen, bottomInset: 0).environmentObject(element), "overlay-element", to: out, size: screen.size)
        let area = OverlaySession.forRendering(.screenshot)
        area.drag = sampleArea
        area.mouse = CGPoint(x: sampleArea.maxX, y: sampleArea.maxY)
        shot(OverlayView(screenRect: screen, bottomInset: 0).environmentObject(area), "overlay-screenshot", to: out, size: screen.size)
        let clip = OverlaySession.forRendering(.clip)
        clip.drag = CGRect(x: 240, y: 12, width: 1180, height: 330)
        clip.mouse = CGPoint(x: 1420, y: 342)
        shot(OverlayView(screenRect: screen, bottomInset: 0).environmentObject(clip), "overlay-clip", to: out, size: screen.size)

        // Windows.
        shot(LibraryView().environmentObject(lib), "library", to: out, size: CGSize(width: 1120, height: 720))
        shot(SettingsView().environmentObject(lib).frame(width: 460), "settings", to: out)
        let flow = Onboarding.shared
        flow.firstCapture = made.first
        for step in Onboarding.Step.allCases {
            flow.step = step
            shot(OnboardingView().environmentObject(flow).environmentObject(lib), "onboarding-\(step)", to: out, size: CGSize(width: 760, height: 580))
        }

        for c in made { try? FileManager.default.removeItem(at: c.folder) }
        print("rendered to \(out.path)")
        exit(0)
    }

    // MARK: Drawing a view without showing it

    private static func shot<V: View>(_ view: V, _ name: String, to dir: URL, size: CGSize? = nil) {
        let host = NSHostingView(rootView: view)
        let fit = size ?? host.fittingSize
        let window = NSWindow(contentRect: CGRect(x: -20000, y: -20000, width: fit.width, height: fit.height),
                              styleMask: [.borderless], backing: .buffered, defer: false)
        window.isOpaque = false
        window.backgroundColor = .clear
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = host
        host.frame = CGRect(origin: .zero, size: fit)
        host.layoutSubtreeIfNeeded()
        // Let SwiftUI lay out, load thumbnails and settle.
        RunLoop.main.run(until: Date().addingTimeInterval(0.6))
        host.layoutSubtreeIfNeeded()

        let scale: CGFloat = 2
        guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(fit.width * scale), pixelsHigh: Int(fit.height * scale),
                                         bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                         colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0) else { return }
        rep.size = fit
        host.cacheDisplay(in: host.bounds, to: rep)
        try? rep.representation(using: .png, properties: [:])?.write(to: dir.appendingPathComponent("\(name).png"))
        window.contentView = nil
    }

    // MARK: Northwind, the demo app

    /// "New invoice" on the Northwind page at 1440 × 900 (frame measured by the site's render script).
    static var sampleButton: ElementInfo {
        var e = ElementInfo()
        e.app = "Google Chrome"; e.bundleID = "com.google.Chrome"; e.window = "Invoices · Northwind"
        e.url = "http://localhost:3000/invoices"; e.role = "Button"; e.name = "New invoice"
        e.domID = "new-invoice"; e.domClasses = "btn btn-primary"
        e.frame = CGRect(x: 1289.8, y: 46.5, width: 110.2, height: 39)
        return e
    }

    static let sampleArea = CGRect(x: 250, y: 101.5, width: 1160, height: 114)

    /// Demo captures in ~/Clipframes under timestamps of their own, so their reference lines read
    /// like real ones. They are deleted again before the renderer quits.
    private static func makeDemoLibrary(_ pictures: URL) -> [Capture] {
        let fm = FileManager.default
        let base = Date(timeIntervalSince1970: 1_791_540_000) // 9 Oct 2026, mid-morning
        func folder(_ minutesAgo: Double) -> (URL, Date) {
            let d = base.addingTimeInterval(-minutesAgo * 60)
            var url = Library.shared.root.appendingPathComponent(Fmt.folderName(d), isDirectory: true)
            var n = d
            while fm.fileExists(atPath: url.path) { // never touch a real capture
                n = n.addingTimeInterval(-1)
                url = Library.shared.root.appendingPathComponent(Fmt.folderName(n), isDirectory: true)
            }
            try? fm.createDirectory(at: url, withIntermediateDirectories: true)
            return (url, d)
        }
        func copy(_ name: String, to url: URL) { try? fm.copyItem(at: pictures.appendingPathComponent(name), to: url) }
        var out: [Capture] = []

        let (f1, d1) = folder(2)
        copy("element.png", to: f1.appendingPathComponent("screenshot.png"))
        var m1 = CaptureMeta(kind: .element, created: d1)
        m1.app = "Google Chrome"; m1.bundleID = "com.google.Chrome"; m1.window = "Invoices · Northwind"; m1.url = sampleButton.url
        m1.element = sampleButton; m1.rect = sampleButton.frame.insetBy(dx: -40, dy: -24); m1.imageSize = CGSize(width: 381, height: 174)
        out.append(Capture(folder: f1, meta: m1))

        let (f2, d2) = folder(6)
        copy("area.png", to: f2.appendingPathComponent("screenshot.png"))
        var m2 = CaptureMeta(kind: .screenshot, created: d2)
        m2.app = "Google Chrome"; m2.bundleID = "com.google.Chrome"; m2.window = "Invoices · Northwind"; m2.url = sampleButton.url
        m2.rect = sampleArea; m2.imageSize = CGSize(width: sampleArea.width * 2, height: sampleArea.height * 2)
        m2.hints = ["Paid this month", "Outstanding", "Overdue"].enumerated().map { i, n in
            var e = ElementInfo(); e.role = "Group"; e.name = n; e.domID = ["paid-total", "outstanding-total", "overdue-total"][i]; e.domClasses = "stat"
            e.url = sampleButton.url; return e
        }
        out.append(Capture(folder: f2, meta: m2))

        let (f3, d3) = folder(14)
        let frames = f3.appendingPathComponent("frames", isDirectory: true)
        try? fm.createDirectory(at: frames, withIntermediateDirectories: true)
        for i in 1...6 { copy("frame-\(i).png", to: frames.appendingPathComponent(String(format: "%03d.png", i))) }
        var m3 = CaptureMeta(kind: .recording, created: d3)
        m3.app = "Google Chrome"; m3.bundleID = "com.google.Chrome"; m3.window = "Invoices · Northwind"; m3.url = sampleButton.url
        m3.duration = 4.0; m3.frameCount = 6; m3.interval = 0.67; m3.imageSize = CGSize(width: 2880, height: 1800)
        m3.clicks = [ClickRecord(time: 0.8, point: CGPoint(x: 1320, y: 58), frame: 2, element: sampleButton),
                     ClickRecord(time: 2.1, point: CGPoint(x: 700, y: 300), frame: 4, element: nil),
                     ClickRecord(time: 3.4, point: CGPoint(x: 1100, y: 58), frame: 6, element: nil)]
        out.append(Capture(folder: f3, meta: m3))

        for c in out { try? Library.shared.save(c) }
        return out
    }
}
