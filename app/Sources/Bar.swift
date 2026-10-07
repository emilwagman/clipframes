import AppKit
import SwiftUI

/// The floating bar: the app's front door. Three tools, the library, and
/// first-run permissions. Sits above the Dock; drag it anywhere.
@MainActor
enum Bar {
    private static var panel: NSPanel?
    private static var hiddenForCapture = false
    private static var wanted = true
    /// The shortcut showed the bar, so Esc should hide it again.
    private static var summonedFromHidden = false

    static func show() {
        wanted = true
        if NSApp.isHidden { Stage.park() }
        let p = panel ?? make()
        p.alphaValue = 0
        p.orderFrontRegardless()
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.2
            ctx.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0.9, 0.3, 1.0)
            p.animator().alphaValue = 1
        }
    }

    static func close() {
        wanted = false
        fadeOut()
    }

    static func toggle() {
        if let p = panel, p.isVisible { close() } else { show() }
    }

    /// What the global shortcut does: bring the bar up with the keyboard on it, so 1, 2 or 3
    /// picks a tool. Pressed again while the bar has the keyboard, it puts things back.
    static func summon() {
        if let p = panel, p.isVisible, p.isKeyWindow { dismissKeyboard(); return }
        guard Library.shared.permissions.ready else { Onboarding.shared.show(at: .permissions); return }
        summonedFromHidden = !(panel?.isVisible ?? false)
        if summonedFromHidden { show() }
        // A non-activating panel can take the keyboard without pulling the app you're in to the back.
        panel?.makeKey()
    }

    /// Esc, or the shortcut again: give the keyboard back, and hide the bar if the shortcut brought it.
    static func dismissKeyboard() {
        guard let p = panel else { return }
        p.resignKey()
        if summonedFromHidden { summonedFromHidden = false; close() }
    }

    /// A tool was picked from the keyboard.
    static func pick(_ mode: OverlaySession.Mode) {
        summonedFromHidden = false
        panel?.resignKey()
        OverlaySession.toggle(mode, from: .shortcut)
    }

    static func hideWhileCapturing() {
        guard let p = panel, p.isVisible else { return }
        hiddenForCapture = true
        p.orderOut(nil)
    }

    static func showAfterCapturing() {
        guard hiddenForCapture else { return }
        hiddenForCapture = false
        if wanted { panel?.orderFrontRegardless() }
    }

    private static func fadeOut() {
        guard let p = panel else { return }
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.14
            p.animator().alphaValue = 0
        }, completionHandler: { p.orderOut(nil) })
    }

    private static func make() -> NSPanel {
        let p = BarPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        p.isOpaque = false
        p.backgroundColor = .clear
        p.hasShadow = false
        p.level = .floating
        p.isMovableByWindowBackground = true
        p.hidesOnDeactivate = false
        p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        p.becomesKeyOnlyIfNeeded = true
        let host = FirstClickHostingView(rootView: BarView().environmentObject(Library.shared))
        host.sizingOptions = [.intrinsicContentSize]
        p.contentView = host
        let size = host.fittingSize
        let vf = (NSScreen.main ?? NSScreen.screens[0]).visibleFrame
        p.setFrame(NSRect(x: vf.midX - size.width / 2, y: vf.minY + 16 - PanelMargin.value + 18, width: size.width, height: size.height), display: true)
        p.setFrameAutosaveName("ClipframesBar3")
        panel = p
        return p
    }
}

/// The bar's window. It takes the keyboard only when the shortcut asks it to.
final class BarPanel: NSPanel {
    override var canBecomeKey: Bool { true }

    override func becomeKey() { super.becomeKey(); BarFocus.shared.keyboard = true }
    override func resignKey() { super.resignKey(); BarFocus.shared.keyboard = false }

    override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 18, 83: Bar.pick(.element)      // 1, keypad 1
        case 19, 84: Bar.pick(.screenshot)   // 2
        case 20, 85: Bar.pick(.clip)         // 3
        case 53: Bar.dismissKeyboard()       // esc
        default: super.keyDown(with: event)
        }
    }

    override func cancelOperation(_ sender: Any?) { Bar.dismissKeyboard() }
}

/// Whether the bar has the keyboard, so the tools can show their number keys.
@MainActor
final class BarFocus: ObservableObject {
    static let shared = BarFocus()
    @Published var keyboard = false
}

struct BarView: View {
    @EnvironmentObject var lib: Library

    var body: some View {
        Group {
            if lib.permissions.ready {
                tools
            } else {
                setup
            }
        }
        .padding(6)
        .surface(radius: 22)
        .padding(PanelMargin.value) // room for the shadow
        .fixedSize()
        .animation(.smooth(duration: 0.2), value: lib.permissions.ready)
    }

    private var tools: some View {
        HStack(spacing: 2) {
            LogoMark(size: 30)
                .padding(.leading, 8).padding(.trailing, 8)
                .help("Clipframes")
            ToolButton(icon: "cursorarrow.rays", title: "Element", key: "1", tint: Brand.accent) {
                OverlaySession.toggle(.element)
            }
            ToolButton(icon: "viewfinder", title: "Screenshot", key: "2", tint: Brand.accent) {
                OverlaySession.toggle(.screenshot)
            }
            ToolButton(icon: "record.circle", title: "Clip", key: "3", tint: Brand.record) {
                OverlaySession.toggle(.clip)
            }
            Rectangle().fill(.white.opacity(0.12)).frame(width: 1, height: 34).padding(.horizontal, 6)
            IconButton(icon: "rectangle.stack", help: "Open the library") { LibraryWindow.show() }
            IconButton(icon: "xmark", help: "Hide the bar. Open Clipframes from the Dock or menu bar to bring it back.") { Bar.close() }
        }
    }

    private var setup: some View {
        HStack(spacing: 14) {
            LogoMark(size: 40)
            Text("Clipframes isn't set up yet").font(.system(size: 14, weight: .semibold))
            PillButton(title: "Finish setup") { Onboarding.shared.show(at: .permissions) }
            IconButton(icon: "xmark", help: "Hide the bar") { Bar.close() }
        }
        .padding(.leading, 6)
        .frame(height: 50)
    }
}

struct ToolButton: View {
    let icon: String
    let title: String
    let key: String
    let tint: Color
    let action: () -> Void
    @State private var hover = false
    @ObservedObject private var focus = BarFocus.shared

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Image(systemName: icon)
                    .font(.system(size: 17, weight: .medium))
                    .foregroundStyle(hover ? Brand.accent : .white)
                    .frame(height: 20)
                Text(title).font(Brand.display(11, .medium)).foregroundStyle(hover ? .white : Brand.text2)
            }
            .frame(width: 76, height: 50)
            .overlay(alignment: .topTrailing) {
                // With the keyboard on the bar, each tool shows the number that picks it.
                if focus.keyboard {
                    Text(key).font(Brand.display(10, .bold)).foregroundStyle(Brand.text2)
                        .frame(width: 16, height: 16)
                        .background(RoundedRectangle(cornerRadius: 5, style: .continuous).fill(.white.opacity(0.1)))
                        .padding(4)
                        .transition(.opacity)
                }
            }
            .animation(.easeOut(duration: 0.12), value: focus.keyboard)
            .background(RoundedRectangle(cornerRadius: 13, style: .continuous).fill(hover ? Brand.accent.opacity(0.14) : .clear))
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(PressScale())
        .onHover { h in withAnimation(.easeOut(duration: 0.12)) { hover = h } }
        .help(title)
    }
}

struct IconButton: View {
    let icon: String
    let help: String
    let action: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(hover ? .white : .secondary)
                .frame(width: 34, height: 34)
                .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(.white.opacity(hover ? 0.10 : 0)))
                .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(PressScale())
        .onHover { h in withAnimation(.easeOut(duration: 0.12)) { hover = h } }
        .help(help)
    }
}

struct PillButton: View {
    let title: String
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 12, weight: .semibold))
                .padding(.horizontal, 16).frame(height: 32)
                .background(Capsule().fill(Brand.accent))
                .foregroundStyle(.white)
                .contentShape(Capsule())
        }
        .buttonStyle(PressScale())
    }
}

/// The library window, made by hand so the app can start with only the bar.
@MainActor
enum LibraryWindow {
    private static var window: NSWindow?

    static func show() {
        if window == nil {
            let host = NSHostingController(rootView: LibraryView().environmentObject(Library.shared))
            host.sceneBridgingOptions = [.toolbars, .title]
            let w = NSWindow(contentViewController: host)
            w.title = "Clipframes"
            w.styleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
            w.titlebarAppearsTransparent = true
            w.titleVisibility = .hidden
            w.appearance = NSAppearance(named: .darkAqua)
            w.backgroundColor = NSColor(white: 0.075, alpha: 1)
            w.toolbarStyle = .unifiedCompact
            w.setContentSize(NSSize(width: 1120, height: 760))
            w.minSize = NSSize(width: 820, height: 520)
            w.isReleasedWhenClosed = false
            w.center()
            w.setFrameAutosaveName("ClipframesLibrary2")
            window = w
        }
        let lib = Library.shared
        if lib.selection.isEmpty, let first = lib.captures.first { lib.selection = [first.id] }
        Stage.restoreToFront()
        // Background apps can't just ask to come forward any more; bring the window up regardless.
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }
}
