import AppKit
import Carbon

/// The one global shortcut, ⌃⇧Space unless changed in Settings. It works from any app:
/// it opens the bar with the keyboard on it (then 1, 2 or 3 picks a tool), cancels a
/// capture that's open, and stops a recording.
@MainActor
final class Hotkey: ObservableObject {
    static let shared = Hotkey()

    struct Combo: Equatable {
        var keyCode: UInt32
        var carbonMods: UInt32
        var label: String       // the key itself, e.g. "Space" or "K"

        static let standard = Combo(keyCode: UInt32(kVK_Space), carbonMods: UInt32(controlKey | shiftKey), label: "Space")

        /// Modifier symbols in Apple's order, then the key.
        var parts: [String] {
            var p: [String] = []
            if carbonMods & UInt32(controlKey) != 0 { p.append("⌃") }
            if carbonMods & UInt32(optionKey) != 0 { p.append("⌥") }
            if carbonMods & UInt32(shiftKey) != 0 { p.append("⇧") }
            if carbonMods & UInt32(cmdKey) != 0 { p.append("⌘") }
            return p + [label]
        }
        var text: String { parts.joined() }
    }

    @Published private(set) var combo: Combo
    /// Set when macOS refused the shortcut, e.g. because another app owns it.
    @Published private(set) var failed = false

    private var ref: EventHotKeyRef?
    private var handlerInstalled = false
    private static let signature = OSType(0x434C_4652) // "CLFR"
    private enum Key { static let code = "hotkey.keyCode", mods = "hotkey.mods", label = "hotkey.label" }

    private init() {
        let d = UserDefaults.standard
        if d.object(forKey: Key.code) != nil, let label = d.string(forKey: Key.label) {
            combo = Combo(keyCode: UInt32(d.integer(forKey: Key.code)), carbonMods: UInt32(d.integer(forKey: Key.mods)), label: label)
        } else {
            combo = .standard
        }
    }

    /// The current shortcut as text, for menus, tips and help tags.
    static var text: String { shared.combo.text }

    func register() {
        installHandler()
        failed = !bind(combo)
    }

    /// Try a new shortcut. Keeps the old one and returns false if macOS refuses it.
    @discardableResult
    func change(to new: Combo) -> Bool {
        guard new != combo else { return true }
        let old = combo
        unbind()
        if bind(new) {
            combo = new
            failed = false
            let d = UserDefaults.standard
            d.set(Int(new.keyCode), forKey: Key.code)
            d.set(Int(new.carbonMods), forKey: Key.mods)
            d.set(new.label, forKey: Key.label)
            return true
        }
        failed = !bind(old)
        return false
    }

    /// While the Settings recorder listens, the old shortcut must not fire.
    func suspend() { unbind() }
    func resume() { if ref == nil { failed = !bind(combo) } }

    func resetToStandard() { change(to: .standard) }

    private func bind(_ c: Combo) -> Bool {
        var r: EventHotKeyRef?
        let status = RegisterEventHotKey(c.keyCode, c.carbonMods, EventHotKeyID(signature: Self.signature, id: 1),
                                         GetApplicationEventTarget(), 0, &r)
        Self.log("register \(c.text) status=\(status)")
        guard status == noErr else { return false }
        ref = r
        return true
    }

    private func unbind() {
        if let ref { UnregisterEventHotKey(ref) }
        ref = nil
    }

    private func installHandler() {
        guard !handlerInstalled else { return }
        handlerInstalled = true
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let status = InstallEventHandler(GetApplicationEventTarget(), { _, _, _ in
            Task { @MainActor in Hotkey.pressed() }
            return noErr
        }, 1, &spec, nil, nil)
        Self.log("handler installed status=\(status)")
    }

    private static func pressed() {
        if RecordingSession.isRecording { RecordingSession.stop(); return }
        if let s = OverlaySession.active { s.cancel(); return }
        Bar.summon()
    }

    nonisolated static func log(_ line: String) {
        let url = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Clipframes.log")
        let stamp = ISO8601DateFormatter().string(from: Date())
        let data = Data("\(stamp) \(line)\n".utf8)
        if let h = try? FileHandle(forWritingTo: url) { h.seekToEndOfFile(); h.write(data); try? h.close() }
        else { try? data.write(to: url) }
    }

    /// Turns a key press from the Settings recorder into a combo. Needs ⌘, ⌃ or ⌥.
    static func combo(from event: NSEvent) -> Combo? {
        let f = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        guard !f.intersection([.command, .control, .option]).isEmpty else { return nil }
        var mods: UInt32 = 0
        if f.contains(.command) { mods |= UInt32(cmdKey) }
        if f.contains(.control) { mods |= UInt32(controlKey) }
        if f.contains(.option) { mods |= UInt32(optionKey) }
        if f.contains(.shift) { mods |= UInt32(shiftKey) }
        return Combo(keyCode: UInt32(event.keyCode), carbonMods: mods, label: label(for: event))
    }

    private static func label(for event: NSEvent) -> String {
        let named: [Int: String] = [
            kVK_Space: "Space", kVK_Return: "↩", kVK_Tab: "⇥", kVK_Delete: "⌫", kVK_ForwardDelete: "⌦",
            kVK_LeftArrow: "←", kVK_RightArrow: "→", kVK_UpArrow: "↑", kVK_DownArrow: "↓",
            kVK_Home: "↖", kVK_End: "↘", kVK_PageUp: "⇞", kVK_PageDown: "⇟",
            kVK_F1: "F1", kVK_F2: "F2", kVK_F3: "F3", kVK_F4: "F4", kVK_F5: "F5", kVK_F6: "F6",
            kVK_F7: "F7", kVK_F8: "F8", kVK_F9: "F9", kVK_F10: "F10", kVK_F11: "F11", kVK_F12: "F12",
        ]
        if let n = named[Int(event.keyCode)] { return n }
        return (event.charactersIgnoringModifiers ?? "?").uppercased()
    }
}
