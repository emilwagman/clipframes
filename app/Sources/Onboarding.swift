import AppKit
import ServiceManagement
import SwiftUI

/// First run. Four steps, each one screen:
/// welcome (perception) → permissions → try it on your own screen → paste it (realization).
@MainActor
final class Onboarding: ObservableObject {
    static let shared = Onboarding()

    enum Step: Int, CaseIterable { case welcome, permissions, tryIt, paste }

    @Published var step: Step = .welcome
    @Published var firstCapture: Capture?
    @Published var openAtLogin = true
    private(set) var waitingForCapture = false
    private var window: NSWindow?

    static var done: Bool { UserDefaults.standard.bool(forKey: Pref.onboarded) }

    func show(at step: Step? = nil) {
        if let step { self.step = step }
        if window == nil {
            let host = NSHostingController(rootView: OnboardingView().environmentObject(self).environmentObject(Library.shared))
            let w = NSWindow(contentViewController: host)
            w.styleMask = [.titled, .closable, .fullSizeContentView]
            w.titlebarAppearsTransparent = true
            w.titleVisibility = .hidden
            w.isMovableByWindowBackground = true
            w.backgroundColor = NSColor(red: 0.086, green: 0.075, blue: 0.067, alpha: 1)
            w.appearance = NSAppearance(named: .darkAqua)
            w.setContentSize(NSSize(width: 760, height: 580))
            w.isReleasedWhenClosed = false
            w.center()
            window = w
        }
        // Background apps can't just ask to come forward any more; bring the window up regardless.
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }

    func next() {
        guard let n = Step(rawValue: step.rawValue + 1) else { return finish() }
        withAnimation(.smooth(duration: 0.3)) { step = n }
    }

    /// Step 3: get out of the way and start Element on the user's own screen.
    func tryElement() {
        waitingForCapture = true
        window?.orderOut(nil)
        OverlaySession.toggle(.element, from: .library)
    }

    /// Called for every new capture; the first one during onboarding brings the window back.
    func captured(_ capture: Capture) {
        guard waitingForCapture else { return }
        waitingForCapture = false
        firstCapture = capture
        step = .paste
        show()
    }

    /// The overlay was cancelled during step 3.
    func cancelledTry() {
        guard waitingForCapture else { return }
        waitingForCapture = false
        show()
    }

    func finish() {
        UserDefaults.standard.set(true, forKey: Pref.onboarded)
        if openAtLogin { try? SMAppService.mainApp.register() } else { try? SMAppService.mainApp.unregister() }
        window?.close()
        Bar.show()
    }
}

// MARK: Views

struct OnboardingView: View {
    @EnvironmentObject var flow: Onboarding
    @EnvironmentObject var lib: Library

    var body: some View {
        ZStack {
            Brand.ink
            // Warm light from above, strongest on the first page.
            RadialGradient(colors: [Brand.accent.opacity(flow.step == .welcome ? 0.30 : 0.14), .clear],
                           center: UnitPoint(x: 0.5, y: 0.0), startRadius: 0, endRadius: 460)
                .animation(.smooth(duration: 0.5), value: flow.step)
            VStack(spacing: 0) {
                StepDots(current: flow.step.rawValue - 1, count: Onboarding.Step.allCases.count - 1)
                    .opacity(flow.step == .welcome ? 0 : 1)
                    .padding(.top, 4)
                Group {
                    switch flow.step {
                    case .welcome: WelcomeStep()
                    case .permissions: PermissionsStep()
                    case .tryIt: TryStep()
                    case .paste: PasteStep()
                    }
                }
                .transition(.asymmetric(insertion: .opacity.combined(with: .offset(y: 10)), removal: .opacity))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .padding(.horizontal, 52)
            .padding(.top, 22)
            .padding(.bottom, 44)
        }
        .frame(width: 760, height: 580)
        .ignoresSafeArea()
        .foregroundStyle(Color(red: 0.99, green: 0.96, blue: 0.93))
        .environment(\.colorScheme, .dark)
    }
}

struct StepDots: View {
    let current: Int
    let count: Int
    var body: some View {
        HStack(spacing: 7) {
            ForEach(0..<count, id: \.self) { i in
                Capsule()
                    .fill(i == current ? Brand.accent : Color.white.opacity(i < current ? 0.45 : 0.15))
                    .frame(width: i == current ? 20 : 6, height: 6)
            }
        }
        .animation(.smooth(duration: 0.25), value: current)
    }
}

private struct Headline: View {
    let eyebrow: String
    let title: String
    let text: String
    var body: some View {
        VStack(spacing: 12) {
            Text(eyebrow)
                .font(Brand.display(13, .semibold))
                .foregroundStyle(Brand.accent)
            Text(title)
                .font(Brand.display(34, .bold)).tracking(-0.5)
                .multilineTextAlignment(.center)
            Text(text)
                .font(.system(size: 15))
                .foregroundStyle(Brand.text2)
                .multilineTextAlignment(.center)
                .lineSpacing(2)
                .frame(maxWidth: 500)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

struct BigButton: View {
    let title: String
    var icon: String? = nil
    var enabled = true
    let action: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Text(title).font(Brand.display(16, .semibold))
                if let icon { Image(systemName: icon).font(.system(size: 13, weight: .bold)) }
            }
            .padding(.horizontal, 30)
            .frame(height: 50)
            .background(
                Capsule().fill(enabled
                    ? AnyShapeStyle(LinearGradient(colors: [Color(red: 1.0, green: 0.55, blue: 0.27), Brand.accentDeep], startPoint: .top, endPoint: .bottom))
                    : AnyShapeStyle(Color.white.opacity(0.08)))
            )
            .overlay(Capsule().strokeBorder(LinearGradient(colors: [.white.opacity(enabled ? 0.35 : 0.08), .clear], startPoint: .top, endPoint: .bottom), lineWidth: 1))
            .shadow(color: Brand.accent.opacity(enabled ? (hover ? 0.5 : 0.35) : 0), radius: hover ? 18 : 12, y: 6)
            .foregroundStyle(enabled ? .white : Brand.text3)
            .contentShape(Capsule())
        }
        .buttonStyle(PressScale())
        .disabled(!enabled)
        .onHover { h in withAnimation(.easeOut(duration: 0.15)) { hover = h } }
        .keyboardShortcut(.defaultAction)
    }
}

struct KeyCaps: View {
    let keys: String
    var size: CGFloat = 15
    var body: some View {
        HStack(spacing: size * 0.3) {
            ForEach(Array(keys.enumerated()), id: \.offset) { _, k in
                Text(String(k))
                    .font(.system(size: size, weight: .semibold, design: .rounded))
                    .frame(minWidth: size * 1.9, minHeight: size * 1.9)
                    .background(
                        RoundedRectangle(cornerRadius: size * 0.45, style: .continuous)
                            .fill(LinearGradient(colors: [Color.white.opacity(0.13), Color.white.opacity(0.06)], startPoint: .top, endPoint: .bottom))
                    )
                    .overlay(RoundedRectangle(cornerRadius: size * 0.45, style: .continuous).strokeBorder(Color.white.opacity(0.12)))
                    .shadow(color: .black.opacity(0.35), radius: 0, y: size * 0.08)
            }
        }
    }
}

/// Page one: the logo and one button.
struct WelcomeStep: View {
    @EnvironmentObject var flow: Onboarding
    @State private var appeared = false

    var body: some View {
        VStack(spacing: 0) {
            Spacer(minLength: 0)
            LogoMark(size: 116, glow: true)
                .scaleEffect(appeared ? 1 : 0.86)
                .opacity(appeared ? 1 : 0)
            Wordmark(size: 44)
                .padding(.top, 30)
            Text("Show your agent what you mean.")
                .font(Brand.display(19, .medium))
                .foregroundStyle(Brand.text2)
                .padding(.top, 10)
            Spacer(minLength: 0)
            BigButton(title: "Get started", icon: "arrow.right") { flow.next() }
            Text("Takes a minute. Everything stays on your Mac.")
                .font(.system(size: 12))
                .foregroundStyle(Brand.text3)
                .padding(.top, 14)
        }
        .onAppear { withAnimation(.spring(duration: 0.7, bounce: 0.35)) { appeared = true } }
    }
}

struct PermissionsStep: View {
    @EnvironmentObject var flow: Onboarding
    @EnvironmentObject var lib: Library

    var body: some View {
        VStack(spacing: 28) {
            Spacer(minLength: 0)
            Headline(eyebrow: "Step 1 of 3",
                     title: "Let Clipframes see your screen",
                     text: "Two switches in System Settings. Nothing is uploaded; your captures stay in a folder on this Mac.")
            VStack(spacing: 10) {
                PermissionRow(icon: "rectangle.dashed.badge.record", title: "Screen Recording",
                              text: "To take screenshots and record clips.",
                              granted: lib.permissions.screen) { lib.requestScreen() }
                PermissionRow(icon: "hand.point.up.left", title: "Accessibility",
                              text: "To know you pointed at the \"Save\" button, not just pixels.",
                              granted: lib.permissions.accessibility) { lib.requestAccessibility() }
            }
            .frame(maxWidth: 560)
            if lib.permissions.accessibility && !lib.permissions.screen {
                HStack(spacing: 8) {
                    Text("Switched on Screen Recording already? macOS needs a restart of the app.")
                        .font(.system(size: 12.5)).foregroundStyle(Brand.text3)
                    Button("Quit and reopen") { lib.relaunch() }
                        .buttonStyle(.plain).font(.system(size: 12.5, weight: .semibold)).foregroundStyle(Brand.accent)
                }
            }
            Spacer(minLength: 0)
            BigButton(title: lib.permissions.ready ? "Continue" : "Waiting for both switches",
                      icon: lib.permissions.ready ? "arrow.right" : nil,
                      enabled: lib.permissions.ready) { flow.next() }
        }
    }
}

struct PermissionRow: View {
    let icon: String
    let title: String
    let text: String
    let granted: Bool
    let action: () -> Void

    var body: some View {
        HStack(spacing: 16) {
            Image(systemName: icon)
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(granted ? Brand.accent : Color.white.opacity(0.85))
                .frame(width: 44, height: 44)
                .background(RoundedRectangle(cornerRadius: 13, style: .continuous).fill(granted ? Brand.accent.opacity(0.14) : Color.white.opacity(0.06)))
            VStack(alignment: .leading, spacing: 3) {
                Text(title).font(Brand.display(16, .semibold))
                Text(text).font(.system(size: 13)).foregroundStyle(Brand.text2)
            }
            Spacer()
            if granted {
                HStack(spacing: 6) {
                    Image(systemName: "checkmark").font(.system(size: 12, weight: .bold))
                    Text("Allowed").font(Brand.display(14, .semibold))
                }
                .foregroundStyle(Brand.accent)
                .transition(.scale(scale: 0.8).combined(with: .opacity))
            } else {
                Button(action: action) {
                    Text("Allow")
                        .font(Brand.display(14, .semibold))
                        .padding(.horizontal, 20).frame(height: 34)
                        .background(Capsule().fill(Brand.accent))
                        .foregroundStyle(.white)
                        .contentShape(Capsule())
                }
                .buttonStyle(PressScale())
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 20, style: .continuous).fill(Brand.raised))
        .overlay(RoundedRectangle(cornerRadius: 20, style: .continuous).strokeBorder(granted ? Brand.accent.opacity(0.35) : Brand.line))
        .animation(.smooth(duration: 0.25), value: granted)
    }
}

struct TryStep: View {
    @EnvironmentObject var flow: Onboarding
    var body: some View {
        VStack(spacing: 26) {
            Spacer(minLength: 0)
            Headline(eyebrow: "Step 2 of 3",
                     title: "Point at something",
                     text: "Pick something in your own app that's hard to put into words: a button, a badge, a card. Hover it, then click.")
            VStack(spacing: 8) {
                ToolLine(icon: "cursorarrow.rays", title: "Element", text: "Click one thing", keys: Hotkey.element, highlighted: true)
                ToolLine(icon: "viewfinder", title: "Screenshot", text: "Drag an area", keys: Hotkey.screenshot)
                ToolLine(icon: "record.circle", title: "Clip", text: "Record an area", keys: Hotkey.clip)
            }
            .frame(maxWidth: 480)
            Spacer(minLength: 0)
            HStack(spacing: 18) {
                Button("Skip") { flow.finish() }
                    .buttonStyle(.plain).foregroundStyle(Brand.text3)
                    .font(Brand.display(14, .medium))
                BigButton(title: "Try Element now", icon: "cursorarrow.rays") { flow.tryElement() }
            }
        }
    }
}

struct ToolLine: View {
    let icon: String
    let title: String
    let text: String
    let keys: String
    var highlighted = false
    var body: some View {
        HStack(spacing: 14) {
            Image(systemName: icon)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(highlighted ? .white : Brand.accent)
                .frame(width: 34, height: 34)
                .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(highlighted ? Brand.accent : Brand.accent.opacity(0.14)))
            Text(title).font(Brand.display(15, .semibold))
            Text(text).font(.system(size: 13)).foregroundStyle(Brand.text2)
            Spacer()
            KeyCaps(keys: keys, size: 11)
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .background(RoundedRectangle(cornerRadius: 16, style: .continuous).fill(highlighted ? Brand.accent.opacity(0.10) : Brand.raised))
        .overlay(RoundedRectangle(cornerRadius: 16, style: .continuous).strokeBorder(highlighted ? Brand.accent.opacity(0.4) : Brand.line))
    }
}

struct PasteStep: View {
    @EnvironmentObject var flow: Onboarding
    @State private var copied = false

    var body: some View {
        VStack(spacing: 24) {
            Spacer(minLength: 0)
            Headline(eyebrow: "Step 3 of 3",
                     title: "Now paste it into your agent",
                     text: "It's on your clipboard. Paste it into Claude Code or Codex with ⌘V, then say what you want changed.")
            if let c = flow.firstCapture {
                VStack(spacing: 0) {
                    if let img = NSImage(contentsOf: c.thumbnail) {
                        Image(nsImage: img).resizable().aspectRatio(contentMode: .fit)
                            .frame(maxHeight: 90)
                            .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                            .padding(20)
                            .frame(maxWidth: .infinity)
                            .background(Color.white.opacity(0.04))
                    }
                    HStack(spacing: 12) {
                        Text(c.reference)
                            .font(.system(size: 11.5, design: .monospaced))
                            .foregroundStyle(Brand.text2)
                            .lineLimit(2)
                            .truncationMode(.middle)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        Button {
                            Library.shared.copy([c])
                            copied = true
                        } label: {
                            Label(copied ? "Copied" : "Copy again", systemImage: copied ? "checkmark" : "doc.on.doc")
                                .font(Brand.display(12.5, .semibold))
                                .padding(.horizontal, 12).frame(height: 30)
                                .background(Capsule().fill(Color.white.opacity(0.08)))
                                .contentTransition(.symbolEffect(.replace))
                        }
                        .buttonStyle(PressScale())
                    }
                    .padding(14)
                }
                .background(RoundedRectangle(cornerRadius: 20, style: .continuous).fill(Brand.raised))
                .overlay(RoundedRectangle(cornerRadius: 20, style: .continuous).strokeBorder(Brand.line))
                .clipShape(RoundedRectangle(cornerRadius: 20, style: .continuous))
                .frame(maxWidth: 560)
            }
            Toggle(isOn: $flow.openAtLogin) {
                Text("Open Clipframes when I log in").font(.system(size: 13.5)).foregroundStyle(Brand.text2)
            }
            .toggleStyle(.switch)
            .tint(Brand.accent)
            Spacer(minLength: 0)
            BigButton(title: "Start using Clipframes", icon: "checkmark") { flow.finish() }
        }
    }
}
