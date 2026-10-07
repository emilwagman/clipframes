import AppKit
import SwiftUI

/// Keeps the library window out of the way of the app you're working in.
///
/// Showing any floating panel (overlay, recording HUD, toast) while Clipframes
/// is hidden unhides the whole app, which would pop the library window in front
/// of your work. So before a panel appears, the library window is parked, and
/// afterwards it goes back *behind* the app you were using.
@MainActor
enum Stage {
    private static var parked: [NSWindow] = []

    static func park() {
        // Unhiding is asynchronous, so windows of a hidden app don't report visible yet.
        let wasHidden = NSApp.isHidden
        if wasHidden { NSApp.unhideWithoutActivation() }
        let visible = NSApp.windows.filter {
            !($0 is NSPanel) && $0.level == .normal && $0.styleMask.contains(.titled) && !$0.isMiniaturized
                && ($0.isVisible || wasHidden)
        }
        visible.forEach { $0.orderOut(nil) }
        parked.append(contentsOf: visible.filter { w in !parked.contains { $0 === w } })
    }

    /// Put parked windows back just below the front window of the app in front.
    static func restoreBehindFrontApp() {
        guard !parked.isEmpty else { return }
        let front = NSWorkspace.shared.frontmostApplication
        if front?.processIdentifier == getpid() { return restoreToFront() }
        let anchor = front.flatMap { app in WindowList.all().first { $0.pid == app.processIdentifier } }
        for w in parked {
            if let anchor { w.order(.below, relativeTo: Int(anchor.id)) } else { w.orderBack(nil) }
        }
        parked.removeAll()
    }

    static func restoreToFront() {
        parked.forEach { $0.makeKeyAndOrderFront(nil) }
        parked.removeAll()
    }

    static var hasParked: Bool { !parked.isEmpty }
}

/// Hosting view for floating panels: a click lands on the first try even when
/// Clipframes isn't the app in front.
final class FirstClickHostingView<Content: View>: NSHostingView<Content> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
}
