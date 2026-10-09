import SwiftUI

@main
struct ClipframesApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var library = Library.shared
    @StateObject private var menu = MenuState.shared

    var body: some Scene {
        MenuBarExtra {
            MenuContent().environmentObject(library).environmentObject(menu)
        } label: {
            MenuLabel().environmentObject(menu)
        }

        Settings {
            SettingsView().environmentObject(library)
        }
        .commands {
            CommandGroup(after: .appInfo) { CheckForUpdatesButton() }
            CommandGroup(replacing: .newItem) {
                Button("Capture an element") { OverlaySession.toggle(.element, from: .menu) }
                Button("Take a screenshot") { OverlaySession.toggle(.screenshot, from: .menu) }
                Button("Record a clip") { OverlaySession.toggle(.clip, from: .menu) }
                Divider()
                Button("Show the bar") { Bar.summon() }
                Button("Open the library") { LibraryWindow.show() }
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        if MainActor.assumeIsolated({ RenderShots.runIfAsked() }) { return }
        MainActor.assumeIsolated {
            Hotkey.shared.register()
            _ = Updates.shared
            Library.shared.start()
            if !UserDefaults.standard.bool(forKey: Pref.showInDock) { NSApp.setActivationPolicy(.accessory) }
            if !Onboarding.done {
                Onboarding.shared.show()
            } else if UserDefaults.standard.bool(forKey: Pref.showBarAtLaunch) {
                Bar.show()
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    // Clicking the Dock icon brings the bar back.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows: Bool) -> Bool {
        MainActor.assumeIsolated {
            if Onboarding.done { Bar.show() } else { Onboarding.shared.show() }
        }
        return false
    }

    func applicationWillTerminate(_ notification: Notification) {
        AXReader.shared.queue.sync { AXReader.shared.sleepIdle(olderThan: -1) }
    }
}

/// Drives the menu bar icon while recording.
@MainActor
final class MenuState: ObservableObject {
    static let shared = MenuState()
    @Published var recording = false
    @Published var elapsed: TimeInterval = 0
    private var timer: Timer?

    func refresh() {
        recording = RecordingSession.isRecording
        timer?.invalidate()
        guard recording else { return }
        timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { _ in
            Task { @MainActor in MenuState.shared.elapsed = RecordingSession.active?.elapsed ?? 0 }
        }
    }
}
