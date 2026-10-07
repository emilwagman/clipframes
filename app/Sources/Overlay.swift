import AppKit
import SwiftUI

/// Full-screen overlay. Element: hover outlines the element, click captures it.
/// Screenshot and Clip: drag an area (⏎ takes the whole screen).
final class OverlayPanel: NSPanel {
    weak var session: OverlaySession?
    private var pressAt: CGPoint?
    override var canBecomeKey: Bool { true }

    // Pointer handling lives here rather than in SwiftUI gestures: it sees every
    // drag event, including ones from other input sources.
    override func sendEvent(_ event: NSEvent) {
        let p = Geo.toGlobal(convertPoint(toScreen: event.locationInWindow))
        let s = session
        switch event.type {
        case .mouseMoved:
            Task { @MainActor in s?.moved(to: p) }
        case .leftMouseDown:
            if MainActor.assumeIsolated({ s?.hudRect.contains(p) ?? false }) { pressAt = nil; super.sendEvent(event); return }
            pressAt = p
            super.sendEvent(event)
        case .leftMouseDragged:
            if let a = pressAt { Task { @MainActor in s?.dragged(from: a, to: p) } }
        case .leftMouseUp:
            guard let a = pressAt else { super.sendEvent(event); return } // a click on the switcher
            pressAt = nil
            // Let SwiftUI buttons (none today) see it first, then finish.
            super.sendEvent(event)
            Task { @MainActor in s?.released(from: a, to: p) }
        default:
            super.sendEvent(event)
        }
    }

    override func keyDown(with event: NSEvent) {
        let s = session
        switch event.keyCode {
        case 53: Task { @MainActor in s?.cancel() }
        case 36, 76: Task { @MainActor in s?.wholeScreen() }
        case 18: Task { @MainActor in s?.switchTo(.element) }     // 1
        case 19: Task { @MainActor in s?.switchTo(.screenshot) }  // 2
        case 20: Task { @MainActor in s?.switchTo(.clip) }        // 3
        default: super.keyDown(with: event)
        }
    }

    override func cancelOperation(_ sender: Any?) {
        let s = session
        Task { @MainActor in s?.cancel() }
    }
}

@MainActor
final class OverlaySession: ObservableObject {
    enum Mode { case element, screenshot, clip }

    static private(set) var active: OverlaySession?

    @Published private(set) var mode: Mode
    @Published var hover: ElementInfo?
    /// Where the tool switcher sits (global points); clicks there don't capture.
    var hudRect: CGRect = .zero
    @Published var hoverWindow: ScreenWindow?
    @Published var mouse = CGPoint.zero
    @Published var drag: CGRect?

    private var panels: [OverlayPanel] = []
    private var lookupBusy = false
    private var pending: CGPoint?
    private var previousApp: NSRunningApplication?
    private var finished = false

    private init(mode: Mode) { self.mode = mode }

    // MARK: Lifecycle

    static func toggle(_ mode: Mode, from source: Habit.Source = .bar) {
        if mode == .clip, RecordingSession.isRecording { RecordingSession.stop(); return }
        if let a = active {
            // Same tool again closes it; another tool switches in place.
            if a.mode == mode { a.cancel() } else { a.switchTo(mode, from: source) }
            return
        }
        guard Library.shared.permissions.ready else { Onboarding.shared.show(at: .permissions); return }
        Habit.started(mode, from: source)
        let s = OverlaySession(mode: mode)
        active = s
        s.begin()
    }

    /// Change tool without closing the overlay.
    func switchTo(_ new: Mode, from source: Habit.Source = .bar) {
        guard new != mode, drag == nil else { return }
        Habit.started(new, from: source)
        withAnimation(.smooth(duration: 0.18)) {
            mode = new
            hover = nil
            hoverWindow = nil
        }
        if new == .clip { Shooter.prewarm() }
        if new == .element, let pid = previousApp?.processIdentifier, pid != getpid() {
            AXReader.shared.queue.async { AXReader.shared.prepare(pid) }
        }
        pending = mouse
        pump()
    }

    private func begin() {
        previousApp = NSWorkspace.shared.frontmostApplication
        Stage.park()
        Bar.hideWhileCapturing()
        if mode == .clip { Shooter.prewarm() }
        if mode == .element, let pid = previousApp?.processIdentifier, pid != getpid() {
            AXReader.shared.queue.async { AXReader.shared.prepare(pid) }
        }
        for screen in NSScreen.screens {
            let p = OverlayPanel(contentRect: screen.frame, styleMask: [.borderless, .nonactivatingPanel],
                                 backing: .buffered, defer: false)
            p.session = self
            p.isOpaque = false
            p.backgroundColor = .clear
            p.hasShadow = false
            p.level = .screenSaver
            p.ignoresMouseEvents = false
            p.acceptsMouseMovedEvents = true
            p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
            let inset = screen.visibleFrame.minY - screen.frame.minY
            p.contentView = NSHostingView(rootView: OverlayView(screenRect: Geo.rect(of: screen), bottomInset: inset).environmentObject(self))
            p.setFrame(screen.frame, display: true)
            p.orderFrontRegardless()
            panels.append(p)
        }
        let here = Geo.mouse()
        (panels.first { Geo.rect(of: $0.screen ?? NSScreen.main!).contains(here) } ?? panels.first)?.makeKey()
        NSCursor.crosshair.push()
        moved(to: here)
    }

    private func close(showBar: Bool = true) {
        guard !finished else { return }
        finished = true
        NSCursor.pop()
        panels.forEach { $0.orderOut(nil) }
        panels.removeAll()
        OverlaySession.active = nil
        if previousApp?.processIdentifier == getpid() { Stage.restoreToFront() } else { Stage.restoreBehindFrontApp() }
        if showBar, !RecordingSession.isRecording { Bar.showAfterCapturing() }
    }

    func cancel() {
        close()
        Onboarding.shared.cancelledTry()
    }

    // MARK: Pointer

    func moved(to p: CGPoint) {
        mouse = p
        guard drag == nil else { return }
        pending = p
        pump()
    }

    private func pump() {
        guard !lookupBusy, let p = pending else { return }
        pending = nil
        lookupBusy = true
        let mode = mode
        AXReader.shared.queue.async {
            let result: (ElementInfo?, ScreenWindow?)
            if mode == .element {
                let r = AXReader.shared.element(at: p)
                result = (r?.0, r?.1)
            } else {
                let w = WindowList.at(p)
                if let w { AXReader.shared.prepare(w.pid) }
                result = (nil, w)
            }
            Task { @MainActor in
                guard let s = OverlaySession.active else { return }
                s.lookupBusy = false
                if s.hover != result.0 { s.hover = result.0 }
                if s.hoverWindow != result.1 { s.hoverWindow = result.1 }
                s.pump()
            }
        }
    }

    func dragged(from a: CGPoint, to b: CGPoint) {
        mouse = b
        guard mode != .element, hypot(a.x - b.x, a.y - b.y) > 4 else { return }
        drag = CGRect(x: min(a.x, b.x), y: min(a.y, b.y), width: abs(a.x - b.x), height: abs(a.y - b.y))
    }

    func released(from a: CGPoint, to b: CGPoint) {
        switch mode {
        case .element:
            captureElement(at: b)
        case .screenshot, .clip:
            // Only a dragged area counts. A plain click does nothing: outlining whole
            // windows looked too much like Element and blurred the two tools.
            if let r = drag, r.width >= 8, r.height >= 8 {
                finish(area: r, window: nil)
            } else {
                drag = nil
            }
        }
    }

    /// ⏎: the whole screen under the pointer.
    func wholeScreen() {
        guard mode != .element, let screen = Geo.screen(containing: mouse) else { return }
        finish(area: Geo.rect(of: screen), window: nil)
    }

    // MARK: Finish

    private func captureElement(at p: CGPoint) {
        var el = hover
        if el == nil || !(el!.frame.contains(p)) {
            el = AXReader.shared.queue.sync { AXReader.shared.element(at: p)?.0 }
        }
        close()
        guard let el else { return }
        var area = el.frame.insetBy(dx: -10, dy: -10)
        if el.frame.width < 1 { area = CGRect(x: p.x - 120, y: p.y - 60, width: 240, height: 120) }
        Task {
            do {
                let shot = try await Shooter.region(area)
                try await Captures.saveElement(shot, element: el)
            } catch {
                Toast.show("Couldn't capture that: \(error.localizedDescription)", icon: "exclamationmark.triangle.fill", tint: .orange)
            }
        }
    }

    private func finish(area: CGRect, window: ScreenWindow?) {
        let mode = mode
        close(showBar: mode != .clip) // a clip keeps the bar away until it ends
        guard area.width >= 2 else { return }
        Task {
            switch mode {
            case .screenshot:
                do {
                    let shot: Shot = if let window { try await Shooter.window(window) } else { try await Shooter.region(area) }
                    try await Captures.saveScreenshot(shot)
                } catch {
                    Toast.show("Couldn't take the screenshot: \(error.localizedDescription)", icon: "exclamationmark.triangle.fill", tint: .orange)
                }
            case .clip:
                await RecordingSession.start(area: area, window: window)
            case .element:
                break
            }
        }
    }
}

// MARK: Look

enum Brand {
    // Warm tool: one confident colour on warm near-black.
    /// Tangerine. Every tool, every highlight, the primary button.
    static let accent = Color(red: 1.0, green: 0.45, blue: 0.19)
    static let accentDeep = Color(red: 0.95, green: 0.33, blue: 0.12)
    /// Recording, and only recording.
    static let record = Color(red: 1.0, green: 0.29, blue: 0.27)
    /// Warm neutrals.
    static let ink = Color(red: 0.086, green: 0.075, blue: 0.067)       // window background
    static let raised = Color(red: 0.125, green: 0.11, blue: 0.098)     // floating surfaces
    static let line = Color(red: 1.0, green: 0.93, blue: 0.86).opacity(0.10)
    static let text2 = Color(red: 0.98, green: 0.93, blue: 0.88).opacity(0.62)
    static let text3 = Color(red: 0.98, green: 0.93, blue: 0.88).opacity(0.40)

    static func display(_ size: CGFloat, _ weight: Font.Weight = .bold) -> Font {
        .system(size: size, weight: weight, design: .rounded)
    }

    static func color(for mode: OverlaySession.Mode) -> Color { mode == .clip ? record : accent }

    static func icon(for mode: OverlaySession.Mode) -> String {
        switch mode {
        case .element: "cursorarrow.rays"
        case .screenshot: "viewfinder"
        case .clip: "record.circle"
        }
    }

    static func icon(forRole role: String) -> String {
        switch role {
        case "Button", "MenuButton", "PopUpButton": "hand.tap"
        case "Link": "link"
        case "TextField", "TextArea", "SearchField", "ComboBox": "character.cursor.ibeam"
        case "Image": "photo"
        case "CheckBox", "Switch", "RadioButton": "checkmark.square"
        case "Heading", "StaticText": "textformat"
        case "Row", "Cell", "Table", "Outline": "tablecells"
        case "Tab", "TabGroup": "rectangle.topthird.inset.filled"
        default: "square.dashed"
        }
    }
}

// MARK: Views

struct OverlayView: View {
    @EnvironmentObject var session: OverlaySession
    let screenRect: CGRect
    let bottomInset: CGFloat

    private func local(_ r: CGRect) -> CGRect { r.offsetBy(dx: -screenRect.minX, dy: -screenRect.minY) }
    private func global(_ p: CGPoint) -> CGPoint { CGPoint(x: p.x + screenRect.minX, y: p.y + screenRect.minY) }
    private var mouseHere: Bool { screenRect.contains(session.mouse) }
    private var accent: Color { Brand.color(for: session.mode) }

    var body: some View {
        ZStack(alignment: .topLeading) {
            shade

            highlight.allowsHitTesting(false)

            if mouseHere {
                HUD(screenRect: screenRect)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                    .padding(.bottom, bottomInset + 28)
                    .transition(.opacity.combined(with: .offset(y: 6)))
            }
        }
        .frame(width: screenRect.width, height: screenRect.height)
        .environment(\.colorScheme, .dark)
    }

    /// Everything outside the dragged area dims; before a drag the screen barely tints.
    @ViewBuilder
    private var shade: some View {
        if let d = session.drag, screenRect.intersects(d) {
            let r = local(d)
            Path { p in
                p.addRect(CGRect(origin: .zero, size: screenRect.size))
                p.addRect(r)
            }
            .fill(Color.black.opacity(0.38), style: FillStyle(eoFill: true))
        } else if session.drag != nil {
            Color.black.opacity(0.38)
        } else {
            Color.black.opacity(0.06)
        }
    }

    @ViewBuilder
    private var highlight: some View {
        if let d = session.drag {
            if screenRect.intersects(d) { SelectionFrame(rect: local(d), size: d.size, accent: accent, bounds: screenRect.size) }
        } else if session.mode == .element {
            if let h = session.hover, screenRect.intersects(h.frame) {
                ElementFrame(info: h, rect: local(h.frame), accent: accent, bounds: screenRect.size)
            }
        }
    }
}

/// The element under the pointer: a glowing outline that glides between elements, and a label.
struct ElementFrame: View {
    let info: ElementInfo
    let rect: CGRect
    let accent: Color
    let bounds: CGSize

    var body: some View {
        let r = rect.insetBy(dx: -3, dy: -3)
        ZStack(alignment: .topLeading) {
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .fill(accent.opacity(0.12))
                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).strokeBorder(accent, lineWidth: 1.5))
                .shadow(color: accent.opacity(0.45), radius: 8)
                .frame(width: r.width, height: r.height)
                .offset(x: r.minX, y: r.minY)
            ElementChip(info: info, accent: accent)
                .offset(x: max(8, min(r.minX, bounds.width - 420)),
                        y: r.minY > 44 ? r.minY - 38 : min(r.maxY + 8, bounds.height - 44))
        }
        .animation(.smooth(duration: 0.14), value: rect)
    }
}

struct ElementChip: View {
    let info: ElementInfo
    var accent: Color = Brand.accent

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: Brand.icon(forRole: info.role))
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: 20, height: 20)
                .background(Circle().fill(accent))
            Text(info.roleName).font(.system(size: 12, weight: .medium)).foregroundStyle(.secondary)
            let name = [info.name, info.innerText, info.value].first { !$0.isEmpty } ?? ""
            if !name.isEmpty {
                Text(name).font(.system(size: 12, weight: .semibold)).lineLimit(1)
            }
            if !info.selector.isEmpty {
                Text(info.selector).font(.system(size: 11, design: .monospaced)).foregroundStyle(.tertiary).lineLimit(1)
            }
        }
        .padding(.leading, 5).padding(.trailing, 10).padding(.vertical, 4)
        .surface(radius: 15)
        .frame(maxWidth: 420, alignment: .leading)
        .fixedSize()
    }
}

/// The dragged area: a crisp edge, corner handles, and its size.
struct SelectionFrame: View {
    let rect: CGRect
    let size: CGSize
    let accent: Color
    let bounds: CGSize

    var body: some View {
        ZStack(alignment: .topLeading) {
            Rectangle()
                .strokeBorder(.white.opacity(0.9), lineWidth: 1)
                .frame(width: rect.width, height: rect.height)
                .offset(x: rect.minX, y: rect.minY)
            Rectangle()
                .strokeBorder(accent, lineWidth: 1)
                .frame(width: rect.width + 2, height: rect.height + 2)
                .offset(x: rect.minX - 1, y: rect.minY - 1)
            ForEach(0..<4, id: \.self) { i in
                let p = CGPoint(x: i % 2 == 0 ? rect.minX : rect.maxX, y: i < 2 ? rect.minY : rect.maxY)
                Circle()
                    .fill(.white)
                    .overlay(Circle().strokeBorder(accent, lineWidth: 1.5))
                    .frame(width: 9, height: 9)
                    .shadow(color: .black.opacity(0.3), radius: 2)
                    .offset(x: p.x - 4.5, y: p.y - 4.5)
            }
            Text(verbatim: "\(Int(size.width)) × \(Int(size.height))")
                .font(.system(size: 12, weight: .semibold))
                .monospacedDigit()
                .padding(.horizontal, 9).padding(.vertical, 4)
                .surface(radius: 8)
                .fixedSize()
                .offset(x: min(max(8, rect.maxX - 92), bounds.width - 100),
                        y: rect.maxY + 34 < bounds.height ? rect.maxY + 10 : rect.maxY - 34)
        }
    }
}

struct WindowLabel: View {
    let title: String
    let icon: String
    let accent: Color
    var body: some View {
        HStack(spacing: 7) {
            Image(systemName: icon).foregroundStyle(accent)
            Text(title).lineLimit(1)
        }
        .font(.system(size: 13, weight: .semibold))
        .padding(.horizontal, 12).padding(.vertical, 7)
        .surface(radius: 16)
        .frame(width: 180)
    }
}

struct HUD: View {
    @EnvironmentObject var session: OverlaySession
    let screenRect: CGRect

    var body: some View {
        HStack(spacing: 12) {
            // The three tools; the current one is lit. Click or press 1/2/3 to switch.
            HStack(spacing: 2) {
                tool(.element, "1")
                tool(.screenshot, "2")
                tool(.clip, "3")
            }
            .padding(3)
            .background(Capsule().fill(.white.opacity(0.07)))
            Text(instruction).font(.system(size: 13, weight: .semibold))
                .contentTransition(.opacity)
            Rectangle().fill(.white.opacity(0.14)).frame(width: 1, height: 18)
            if session.mode != .element { key("⏎", "Whole screen") }
            key("esc", "Cancel")
        }
        .padding(.leading, 5).padding(.trailing, 14).padding(.vertical, 5)
        .surface(radius: 22)
        .fixedSize()
        .background(GeometryReader { g in
            Color.clear
                .onAppear { report(g) }
                .onChange(of: g.frame(in: .global)) { _, _ in report(g) }
        })
        .animation(.smooth(duration: 0.18), value: session.mode)
    }

    /// Tell the panel where the switcher is, in screen points, so clicks on it don't capture.
    private func report(_ g: GeometryProxy) {
        let f = g.frame(in: .global)
        session.hudRect = f.offsetBy(dx: screenRect.minX, dy: screenRect.minY).insetBy(dx: -4, dy: -4)
    }

    private func tool(_ m: OverlaySession.Mode, _ number: String) -> some View {
        let on = session.mode == m
        return Button { session.switchTo(m) } label: {
            HStack(spacing: 6) {
                Image(systemName: Brand.icon(for: m)).font(.system(size: 12, weight: .semibold))
                if on { Text(Habit.name(m)).font(.system(size: 12, weight: .semibold)) }
            }
            .foregroundStyle(on ? .white : .white.opacity(0.6))
            .padding(.horizontal, on ? 11 : 8)
            .frame(height: 28)
            .background(Capsule().fill(on ? Brand.color(for: m) : .clear))
            .contentShape(Capsule())
        }
        .buttonStyle(PressScale())
        .help("\(Habit.name(m)) (\(number))")
    }

    private var instruction: String {
        switch session.mode {
        case .element: "Click an element to capture it"
        case .screenshot: "Drag an area to capture it"
        case .clip: "Drag an area to record it"
        }
    }

    private func key(_ k: String, _ label: String) -> some View {
        HStack(spacing: 6) {
            Text(k)
                .font(.system(size: 11, weight: .semibold, design: .rounded))
                .padding(.horizontal, 6).frame(minWidth: 22, minHeight: 20)
                .background(RoundedRectangle(cornerRadius: 5, style: .continuous).fill(.white.opacity(0.12)))
                .overlay(RoundedRectangle(cornerRadius: 5, style: .continuous).strokeBorder(.white.opacity(0.08)))
            Text(label).font(.system(size: 12)).foregroundStyle(.secondary)
        }
    }
}

// MARK: One dark surface for everything that floats over other apps

extension View {
    func surface(radius: CGFloat) -> some View {
        self
            .foregroundStyle(.white)
            .background(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .fill(.ultraThinMaterial)
                    .overlay(RoundedRectangle(cornerRadius: radius, style: .continuous).fill(Brand.raised.opacity(0.9)))
            )
            // A warm hairline, a touch brighter on top like a lit edge.
            .overlay(RoundedRectangle(cornerRadius: radius, style: .continuous)
                .strokeBorder(LinearGradient(colors: [Color.white.opacity(0.16), Brand.line], startPoint: .top, endPoint: .bottom), lineWidth: 0.75))
            .shadow(color: .black.opacity(0.22), radius: 1.5, y: 1)
            .shadow(color: .black.opacity(0.28), radius: 12, y: 6)
            .environment(\.colorScheme, .dark)
    }
}
