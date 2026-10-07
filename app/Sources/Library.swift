import AppKit
import ServiceManagement
import SwiftUI

enum Pref {
    static let fps = "framesPerSecond"
    static let maxFrames = "maxFrames"
    static let width = "frameWidth"
    static let showClicks = "showClicks"
    static let readWebApps = "readWebApps"
    static let onboarded = "onboarded2"
    static let showBarAtLaunch = "showBarAtLaunch"
    static let showInDock = "showInDock"

    static func register() {
        UserDefaults.standard.register(defaults: [
            fps: 2.0, maxFrames: 20, width: 1280, showClicks: true, readWebApps: true, onboarded: false,
            showBarAtLaunch: true, showInDock: true,
        ])
    }
}

struct Permissions: Equatable {
    var screen = CGPreflightScreenCaptureAccess()
    var accessibility = AXIsProcessTrusted()
    var ready: Bool { screen && accessibility }
}

@MainActor
final class Library: ObservableObject {
    static let shared = Library()

    enum Filter: String, CaseIterable { case all = "All", elements = "Elements", screenshots = "Screenshots", clips = "Clips" }

    @Published var captures: [Capture] = []
    @Published var selection: Set<String> = []
    @Published var search = ""
    @Published var filter: Filter = .all
    @Published var working: String?
    @Published var permissions = Permissions()
    @Published var copiedID: String?

    let root = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Clipframes", isDirectory: true)
    private var permissionTimer: Timer?
    private var idleTimer: Timer?
    private var copiedTask: Task<Void, Never>?

    private init() { Pref.register() }

    func start() {
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        reload()
        permissionTimer = Timer.scheduledTimer(withTimeInterval: 1.5, repeats: true) { _ in
            Task { @MainActor in
                let p = Permissions()
                if p != Library.shared.permissions { Library.shared.permissions = p }
            }
        }
        // Chrome and Electron build their page tree a few seconds after being asked,
        // so ask as soon as an app comes to the front, before anyone points at it.
        NSWorkspace.shared.notificationCenter.addObserver(forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main) { note in
            guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  app.processIdentifier != getpid() else { return }
            let pid = app.processIdentifier
            AXReader.shared.queue.async { AXReader.shared.prepare(pid) }
        }
        for app in NSWorkspace.shared.runningApplications where app.activationPolicy == .regular && app.processIdentifier != getpid() {
            let pid = app.processIdentifier
            AXReader.shared.queue.async { AXReader.shared.prepare(pid) }
        }
        idleTimer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { _ in
            AXReader.shared.queue.async { AXReader.shared.sleepIdle(olderThan: 300) }
        }
    }

    var filtered: [Capture] {
        let q = search.lowercased().trimmingCharacters(in: .whitespaces)
        return captures.filter { c in
            switch filter {
            case .all: break
            case .elements: if c.meta.kind != .element { return false }
            case .screenshots: if c.meta.kind != .screenshot { return false }
            case .clips: if c.meta.kind != .recording { return false }
            }
            return q.isEmpty || c.searchText.contains(q)
        }
    }

    var selected: [Capture] { captures.filter { selection.contains($0.id) } }

    // MARK: Disk

    func reload() {
        let dirs = (try? FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil,
                                                                 options: [.skipsHiddenFiles])) ?? []
        captures = dirs.compactMap(Self.load).sorted { $0.meta.created > $1.meta.created }
    }

    static func load(_ dir: URL) -> Capture? {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("capture.json")),
              let meta = try? d.decode(CaptureMeta.self, from: data) else { return nil }
        return Capture(folder: dir, meta: meta)
    }

    func newFolder(_ date: Date) throws -> URL {
        let base = Fmt.folderName(date)
        var dir = root.appendingPathComponent(base, isDirectory: true)
        var n = 2
        while FileManager.default.fileExists(atPath: dir.path) {
            dir = root.appendingPathComponent("\(base)_\(n)", isDirectory: true)
            n += 1
        }
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    func save(_ capture: Capture) throws {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.prettyPrinted, .sortedKeys]
        try e.encode(capture.meta).write(to: capture.folder.appendingPathComponent("capture.json"))
        try capture.notes.write(to: capture.notesFile, atomically: true, encoding: .utf8)
    }

    /// A new capture is finished: keep it, select it, put its reference on the clipboard.
    func add(_ capture: Capture) {
        captures.insert(capture, at: 0)
        selection = [capture.id]
        Habit.captured()
        copy([capture], toast: true)
        Onboarding.shared.captured(capture)
        if let tip = Habit.takeTip() {
            Task { @MainActor in
                try? await Task.sleep(for: .seconds(2.6))
                Toast.show(tip, icon: "keyboard", tint: Brand.accent)
            }
        }
    }

    func update(_ capture: Capture) {
        try? save(capture)
        if let i = captures.firstIndex(where: { $0.id == capture.id }) { captures[i] = capture }
    }

    // MARK: Actions

    func copy(_ list: [Capture], toast: Bool = false) {
        guard !list.isEmpty else { return }
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(list.map(\.reference).joined(separator: "\n"), forType: .string)
        copiedID = list.count == 1 ? list[0].id : "many"
        copiedTask?.cancel()
        copiedTask = Task { @MainActor in
            try? await Task.sleep(for: .seconds(1.6))
            if !Task.isCancelled { copiedID = nil }
        }
        if toast {
            Toast.show(list.count == 1 ? "Copied. Paste it into your agent." : "Copied \(list.count) references")
        }
    }

    func copyLast() {
        if let c = captures.first { copy([c], toast: true) }
    }

    func reveal(_ c: Capture) {
        NSWorkspace.shared.activateFileViewerSelecting([c.meta.kind == .recording ? c.framesFolder : c.thumbnail])
    }

    func trash(_ list: [Capture]) {
        for c in list {
            try? FileManager.default.trashItem(at: c.folder, resultingItemURL: nil)
        }
        let ids = Set(list.map(\.id))
        captures.removeAll { ids.contains($0.id) }
        selection.subtract(ids)
    }

    // MARK: Permissions

    func requestScreen() {
        if !CGRequestScreenCaptureAccess() {
            NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")!)
        }
    }

    func requestAccessibility() {
        let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        if !AXIsProcessTrustedWithOptions(opts) {
            NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!)
        }
    }

    func relaunch() {
        let url = Bundle.main.bundleURL
        let config = NSWorkspace.OpenConfiguration()
        config.createsNewApplicationInstance = true
        NSWorkspace.shared.openApplication(at: url, configuration: config) { _, _ in
            Task { @MainActor in NSApp.terminate(nil) }
        }
    }
}

// MARK: A short confirmation that floats near the bottom of the screen

@MainActor
enum Toast {
    private static var panel: NSPanel?
    private static var hideTask: Task<Void, Never>?

    static func show(_ text: String, icon: String = "checkmark.circle.fill", tint: Color = .green) {
        let appWasInFront = NSApp.isActive
        if !appWasInFront { Stage.park() }
        let view = HStack(spacing: 8) {
            Image(systemName: icon).foregroundStyle(tint)
            Text(text).font(.system(size: 13, weight: .medium))
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .surface(radius: 20)
        .padding(PanelMargin.value)
        .fixedSize()

        let host = NSHostingView(rootView: view)
        let size = host.fittingSize
        let p = panel ?? {
            let p = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
            p.isOpaque = false
            p.backgroundColor = .clear
            p.hasShadow = false
            p.level = .statusBar
            p.ignoresMouseEvents = true
            p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .transient]
            panel = p
            return p
        }()
        p.contentView = host
        let screen = Geo.screen(containing: Geo.mouse()) ?? NSScreen.main!
        let vf = screen.visibleFrame
        p.setFrame(NSRect(x: vf.midX - size.width / 2, y: vf.minY + 110 - PanelMargin.value, width: size.width, height: size.height), display: true)
        p.alphaValue = 0
        p.orderFrontRegardless()
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.18
            p.animator().alphaValue = 1
        }
        hideTask?.cancel()
        hideTask = Task { @MainActor in
            try? await Task.sleep(for: .seconds(2.0))
            guard !Task.isCancelled else { return }
            NSAnimationContext.runAnimationGroup({ ctx in
                ctx.duration = 0.25
                p.animator().alphaValue = 0
            }, completionHandler: {
                MainActor.assumeIsolated {
                    p.orderOut(nil)
                    if !appWasInFront, RecordingSession.active == nil, OverlaySession.active == nil { Stage.restoreBehindFrontApp() }
                }
            })
        }
    }
}
