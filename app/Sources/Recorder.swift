import AppKit
import AVFoundation
import ScreenCaptureKit
import SwiftUI

/// Writes a ScreenCaptureKit stream straight to a .mov.
final class StreamRecorder: NSObject, SCStreamDelegate, SCStreamOutput, SCRecordingOutputDelegate {
    var onFinished: ((URL) -> Void)?
    var onError: ((Error) -> Void)?

    private var stream: SCStream?
    private var recording: SCRecordingOutput?
    private var url: URL?
    private var done = false
    private let frameQueue = DispatchQueue(label: "clipframes.frames")

    func start(filter: SCContentFilter, to url: URL, showClicks: Bool, sourceRect: CGRect? = nil) async throws {
        let scale = CGFloat(filter.pointPixelScale)
        let config = SCStreamConfiguration()
        let size = sourceRect?.size ?? filter.contentRect.size
        if let sourceRect { config.sourceRect = sourceRect }
        config.width = Self.even(size.width * scale)
        config.height = Self.even(size.height * scale)
        config.minimumFrameInterval = CMTime(value: 1, timescale: 30)
        config.showsCursor = true
        config.showMouseClicks = showClicks
        config.queueDepth = 6

        let stream = SCStream(filter: filter, configuration: config, delegate: self)
        let rc = SCRecordingOutputConfiguration()
        rc.outputURL = url
        rc.outputFileType = .mov
        rc.videoCodecType = .h264
        let recording = SCRecordingOutput(configuration: rc, delegate: self)
        try stream.addStreamOutput(self, type: .screen, sampleHandlerQueue: frameQueue)
        try stream.addRecordingOutput(recording)
        self.stream = stream
        self.recording = recording
        self.url = url
        done = false
        try await stream.startCapture()
    }

    func stop() {
        guard let stream else { return }
        Task { do { try await stream.stopCapture() } catch { self.fail(error) } }
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {}

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
            guard let self, !self.done else { return }
            if let url = self.url, FileManager.default.fileExists(atPath: url.path) { self.finish(url) } else { self.fail(error) }
        }
    }

    func recordingOutputDidFinishRecording(_ recordingOutput: SCRecordingOutput) {
        DispatchQueue.main.async { [weak self] in
            guard let self, let url = self.url, !self.done else { return }
            self.finish(url)
        }
    }

    func recordingOutput(_ recordingOutput: SCRecordingOutput, didFailWithError error: Error) {
        DispatchQueue.main.async { [weak self] in self?.fail(error) }
    }

    private func finish(_ url: URL) {
        done = true
        stream = nil
        recording = nil
        onFinished?(url)
    }

    private func fail(_ error: Error) {
        guard !done else { return }
        done = true
        if let url { try? FileManager.default.removeItem(at: url) }
        stream = nil
        recording = nil
        onError?(error)
    }

    private static func even(_ v: CGFloat) -> Int {
        let i = max(2, Int(v.rounded()))
        return i - i % 2
    }
}

/// One recording: the stream, the clicks, the HUD, and turning it into a capture.
@MainActor
final class RecordingSession: ObservableObject {
    static private(set) var active: RecordingSession?

    @Published var elapsed: TimeInterval = 0
    @Published var clickCount = 0
    @Published var stopping = false

    private let recorder = StreamRecorder()
    private let rect: CGRect
    private let app: String
    private let window: String
    /// When frames started arriving. Click times and frame numbers are measured from here.
    private var started = Date()
    @Published var live = false
    private var tempURL: URL
    private var clicks: [ClickRecord] = []
    private var clickMonitor: Any?
    private var timer: Timer?
    private var hud: NSPanel?
    private var outline: NSPanel?
    var bundleID: String?
    var hintsTask: Task<[ElementInfo], Never>?

    private init(rect: CGRect, app: String, window: String, tempURL: URL) {
        self.rect = rect
        self.app = app
        self.window = window
        self.tempURL = tempURL
    }

    static var isRecording: Bool { active != nil }

    /// Record a window, or any area of the screen.
    static func start(area: CGRect, window ref: ScreenWindow?) async {
        guard active == nil else { return }
        do {
            let filter: SCContentFilter
            let rect: CGRect
            var source: CGRect?
            if let ref {
                (filter, rect) = try await Shooter.windowFilter(ref)
            } else {
                let r = try await Shooter.regionFilter(area)
                filter = r.0; rect = r.1; source = r.2
            }
            let owner = ref ?? WindowList.at(CGPoint(x: rect.midX, y: rect.midY))
            let running = owner.flatMap { NSRunningApplication(processIdentifier: $0.pid) }
            let tmp = Library.shared.root.appendingPathComponent(".recording-\(UUID().uuidString).mov")
            let session = RecordingSession(rect: rect, app: running?.localizedName ?? owner?.owner ?? "", window: owner?.title ?? "", tempURL: tmp)
            session.bundleID = running?.bundleIdentifier
            active = session
            session.preparing() // outline, pill and click watching start now, not when capture does
            session.recorder.onFinished = { url in Task { @MainActor in await session.finish(url) } }
            session.recorder.onError = { error in Task { @MainActor in session.failed(error) } }
            try await session.recorder.start(filter: filter, to: tmp, showClicks: UserDefaults.standard.bool(forKey: Pref.showClicks), sourceRect: source)
            session.began()
            session.hintsTask = Task { await Captures.elements(in: rect) }
        } catch {
            active?.tearDown()
            active = nil
            Bar.showAfterCapturing()
            Toast.show("Couldn't start recording: \(error.localizedDescription)", icon: "exclamationmark.triangle.fill", tint: .orange)
        }
    }

    static func stop() { active?.stop() }

    private func preparing() {
        clickMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { _ in
            let p = Geo.mouse()
            let at = Date()
            Task { @MainActor in RecordingSession.active?.clicked(at: p, when: at) }
        }
        showHUD()
        MenuState.shared.refresh()
    }

    private func began() {
        started = Date()
        live = true
        timer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { _ in
            Task { @MainActor in
                guard let s = RecordingSession.active else { return }
                s.elapsed = Date().timeIntervalSince(s.started)
            }
        }
    }

    nonisolated private static let controls: Set<String> = ["Button", "Link", "CheckBox", "RadioButton", "PopUpButton", "MenuButton",
                                                 "TextField", "TextArea", "ComboBox", "Slider", "Tab", "MenuItem", "Switch", "SearchField"]

    private func clicked(at p: CGPoint, when: Date) {
        guard rect.contains(p) else { return }
        clickCount += 1
        let index = clicks.count
        clicks.append(ClickRecord(time: 0, point: p, element: nil))
        let session = self
        AXReader.shared.queue.async {
            let first = AXReader.shared.element(at: p)?.0
            // Apps like Chrome can report the old page for a moment after something
            // slides in. Look again once it has settled; if the first answer was a
            // container and the second is a control, the control is what was clicked.
            AXReader.shared.queue.asyncAfter(deadline: .now() + 0.35) {
                let second = AXReader.shared.element(at: p)?.0
                var chosen = first
                if let s = second, Self.controls.contains(s.role), !(first.map { Self.controls.contains($0.role) } ?? false),
                   s.frame.contains(p) {
                    chosen = s
                }
                Task { @MainActor in
                    guard index < session.clicks.count else { return }
                    session.clicks[index].element = chosen
                    // Measured from the first frame; a click just before it counts as the start.
                    session.clicks[index].time = max(0, when.timeIntervalSince(session.started))
                }
            }
        }
    }

    func stop() {
        guard !stopping else { return }
        stopping = true
        recorder.stop()
    }

    fileprivate func tearDown() {
        timer?.invalidate()
        if let m = clickMonitor { NSEvent.removeMonitor(m) }
        hud?.orderOut(nil)
        hud = nil
        outline?.orderOut(nil)
        outline = nil
        RecordingSession.active = nil
        Stage.restoreBehindFrontApp()
        Bar.showAfterCapturing()
        MenuState.shared.refresh()
    }

    private func failed(_ error: Error) {
        tearDown()
        Toast.show("Recording failed: \(error.localizedDescription)", icon: "exclamationmark.triangle.fill", tint: .orange)
    }

    private func finish(_ url: URL) async {
        tearDown()
        let lib = Library.shared
        lib.working = "Making frames…"
        defer { lib.working = nil }
        // Clicks resolve on the AX queue, the second look 0.35 s later; let the last ones land.
        try? await Task.sleep(for: .milliseconds(450))
        await withCheckedContinuation { c in AXReader.shared.queue.async { c.resume() } }
        await Task.yield()
        do {
            let dir = try lib.newFolder(started)
            let video = dir.appendingPathComponent("video.mov")
            try FileManager.default.moveItem(at: url, to: video)
            let d = UserDefaults.standard
            let frames = try await FrameExtractor.extract(
                video: video, into: dir.appendingPathComponent("frames", isDirectory: true),
                fps: d.double(forKey: Pref.fps), maxFrames: d.integer(forKey: Pref.maxFrames), width: d.integer(forKey: Pref.width))

            var clicks = self.clicks.sorted { $0.time < $1.time }
            for i in clicks.indices {
                clicks[i].frame = FrameExtractor.frame(after: clicks[i].time, interval: frames.interval, count: frames.count)
            }
            var meta = CaptureMeta(kind: .recording, created: started)
            meta.app = app
            meta.bundleID = bundleID ?? clicks.first?.element?.bundleID
            meta.window = window
            meta.url = clicks.lazy.compactMap { $0.element?.url }.first { !$0.isEmpty } ?? ""
            if meta.app.isEmpty { meta.app = clicks.first?.element?.app ?? "" }
            meta.rect = rect
            meta.imageSize = frames.size
            meta.scale = frames.size.width / max(1, rect.width)
            let hints = await hintsTask?.value
            // Chrome sometimes answers a hit-test with the whole page. Use the smallest
            // named element under the click instead.
            if let hints {
                let big = rect.width * rect.height * 0.25
                func area(_ r: CGRect) -> CGFloat { r.width * r.height }
                for i in clicks.indices {
                    guard let el = clicks[i].element, area(el.frame) > big || el.isWeak else { continue }
                    if let better = hints.filter({ $0.frame.contains(clicks[i].point) }).min(by: { area($0.frame) < area($1.frame) }) {
                        clicks[i].element = better
                    }
                }
            }
            meta.clicks = clicks
            meta.hints = hints
            meta.duration = frames.duration
            meta.frameCount = frames.count
            meta.interval = frames.interval
            let capture = Capture(folder: dir, meta: meta)
            markClicks(capture)
            try lib.save(capture)
            lib.add(capture)
        } catch {
            Toast.show("Couldn't make frames: \(error.localizedDescription)", icon: "exclamationmark.triangle.fill", tint: .orange)
        }
    }

    /// Ring each click on the first frame after it.
    private func markClicks(_ c: Capture) {
        let byFrame = Dictionary(grouping: c.meta.clicks.enumerated(), by: { $0.element.frame ?? 1 })
        for (frame, items) in byFrame {
            let url = c.frame(frame)
            guard let image = Shooter.readImage(url) else { continue }
            let marks = items.map { (number: $0.offset + 1, point: $0.element.point, box: CGRect?.none, ring: true) }
            let out = Shooter.annotate(image, rect: c.meta.rect, scale: c.meta.scale, marks: marks)
            try? Shooter.writePNG(out, to: url)
        }
    }

    // MARK: Outline and controls at the edge of the recorded area

    private func showHUD() {
        Stage.park()
        let screen = Geo.screen(containing: CGPoint(x: rect.midX, y: rect.midY)) ?? NSScreen.main!

        // A thin outline just outside the recorded area. Clipframes' windows are never recorded.
        let o = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        o.isOpaque = false
        o.backgroundColor = .clear
        o.hasShadow = false
        o.ignoresMouseEvents = true
        o.level = .statusBar
        o.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
        o.contentView = NSHostingView(rootView: RecordingOutline())
        o.setFrame(Geo.cocoaRect(rect.insetBy(dx: -3, dy: -3)), display: true)
        o.orderFrontRegardless()
        outline = o

        let p = ControlPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        p.onStop = { [weak self] in self?.stop() }
        p.isOpaque = false
        p.backgroundColor = .clear
        p.hasShadow = false
        p.level = .statusBar
        p.becomesKeyOnlyIfNeeded = true
        p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        let host = FirstClickHostingView(rootView: RecordingHUD().environmentObject(self))
        p.contentView = host
        let size = host.fittingSize
        // Just below the bottom-right corner; inside it when the area touches the screen's bottom.
        let area = Geo.cocoaRect(rect)
        let vf = screen.visibleFrame
        let m = PanelMargin.value
        var origin = NSPoint(x: area.maxX - size.width + m, y: area.minY - size.height + m - 8)
        if origin.y < vf.minY - m { origin.y = area.minY - m + 8 }
        origin.x = min(max(vf.minX + 4, origin.x), vf.maxX - size.width - 4)
        p.setFrame(NSRect(origin: origin, size: size), display: true)
        p.orderFrontRegardless()
        hud = p
    }
}

/// The recording controls. A click on the Stop end is handled here, so it works
/// on the first try while the timer keeps redrawing the view.
final class ControlPanel: NSPanel {
    var onStop: (@MainActor () -> Void)?
    private var downOnStop = false

    private func onStopButton(_ event: NSEvent) -> Bool {
        let x = event.locationInWindow.x
        // Stop is the trailing capsule, just inside the transparent margin.
        let m = PanelMargin.value
        return x > frame.width - m - 80 && x < frame.width - m
    }

    override func sendEvent(_ event: NSEvent) {
        switch event.type {
        case .leftMouseDown:
            downOnStop = onStopButton(event)
        case .leftMouseUp:
            if downOnStop, onStopButton(event) {
                let stop = onStop
                Task { @MainActor in stop?() }
            }
            downOnStop = false
            return
        default:
            break
        }
        super.sendEvent(event)
    }
}

struct RecordingOutline: View {
    @State private var pulse = false
    var body: some View {
        RoundedRectangle(cornerRadius: 4, style: .continuous)
            .strokeBorder(Brand.record.opacity(pulse ? 0.55 : 0.95), style: StrokeStyle(lineWidth: 2, dash: [8, 5]))
            .onAppear {
                withAnimation(.easeInOut(duration: 1.1).repeatForever(autoreverses: true)) { pulse = true }
            }
    }
}

/// Transparent room around a floating pill, so its shadow is never cut off.
enum PanelMargin { static let value: CGFloat = 32 }

struct RecordingHUD: View {
    @EnvironmentObject var session: RecordingSession

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "circle.fill")
                .font(.system(size: 9))
                .foregroundStyle(session.live ? Brand.record : Brand.text3)
                .symbolEffect(.pulse, options: .repeating)
            Text(session.live ? Fmt.clock(session.elapsed) : "Starting")
                .font(.system(size: 13, weight: .semibold))
                .monospacedDigit()
                .foregroundStyle(session.live ? .white : Brand.text2)
                .frame(width: 58, alignment: .leading)
            // Always shown, so the pill never changes width while recording.
            Label("\(session.clickCount)", systemImage: "cursorarrow.click")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)
                .monospacedDigit()
                .frame(width: 38, alignment: .leading)
                .help("Clicks noted so far")
            Button {
                session.stop()
            } label: {
                HStack(spacing: 6) {
                    RoundedRectangle(cornerRadius: 2).fill(.white).frame(width: 9, height: 9)
                    Text(session.stopping ? "Saving" : "Stop").font(.system(size: 12, weight: .semibold))
                }
                .frame(width: 70, height: 26)
                .background(Capsule().fill(Brand.record))
                .foregroundStyle(.white)
                .contentShape(Capsule())
            }
            .buttonStyle(PressScale())
            .disabled(session.stopping)
            .help("Stop recording (\(Hotkey.clip))")
        }
        .padding(.leading, 14)
        .padding(.trailing, 5)
        .padding(.vertical, 5)
        .surface(radius: 18)
        .padding(PanelMargin.value)
        .fixedSize()
    }
}

struct PressScale: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.96 : 1)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}
