import AppKit
import SwiftUI

/// Settings' shortcut field: click it, press the new shortcut. Esc leaves it as it was.
struct ShortcutRecorder: View {
    @ObservedObject private var hotkey = Hotkey.shared
    @State private var listening = false
    @State private var monitor: Any?
    @State private var problem: String?

    var body: some View {
        VStack(alignment: .trailing, spacing: 6) {
            HStack(spacing: 8) {
                if hotkey.combo != .standard && !listening {
                    Button("Reset") { problem = nil; hotkey.resetToStandard() }
                        .buttonStyle(.link).font(.callout)
                }
                Button { listening ? stop() : start() } label: {
                    Group {
                        if listening {
                            Text("Press a shortcut…").foregroundStyle(.secondary)
                        } else {
                            KeyCaps(keys: hotkey.combo.parts, size: 11)
                        }
                    }
                    .frame(minWidth: 130, minHeight: 26)
                }
                .buttonStyle(.bordered)
            }
            if let problem {
                Text(problem).font(.callout).foregroundStyle(.orange)
            } else if hotkey.failed {
                Text("Another app is using \(hotkey.combo.text). Pick a different shortcut.").font(.callout).foregroundStyle(.orange)
            }
        }
        .onDisappear { stop() }
    }

    private func start() {
        problem = nil
        listening = true
        hotkey.suspend()
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            if event.keyCode == 53 { stop(); return nil }   // esc: keep the old one
            guard let combo = Hotkey.combo(from: event) else {
                problem = "Use ⌘, ⌃ or ⌥ together with a key."
                return nil
            }
            if hotkey.change(to: combo) {
                problem = nil
            } else {
                problem = "Another app is using \(combo.text). Pick a different shortcut."
            }
            stop()
            return nil
        }
    }

    private func stop() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
        listening = false
        hotkey.resume()
    }
}
