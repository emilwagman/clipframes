import Carbon

/// Global shortcuts that work from any app:
/// ⌃⇧1 element, ⌃⇧2 screenshot, ⌃⇧3 clip (again to stop). Three neighbouring
/// keys that leave macOS's own screenshot keys alone: Clipframes makes notes for
/// an agent, it doesn't replace the screenshots you give to people.
enum Hotkey {
    static let element = "⌃⇧1"
    static let screenshot = "⌃⇧2"
    static let clip = "⌃⇧3"
    private static var refs: [EventHotKeyRef?] = []
    /// Shortcuts macOS refused, e.g. because another app already owns them.
    nonisolated(unsafe) static var failed: [String] = []

    static func log(_ line: String) {
        let url = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Clipframes.log")
        let stamp = ISO8601DateFormatter().string(from: Date())
        let data = Data("\(stamp) \(line)\n".utf8)
        if let h = try? FileHandle(forWritingTo: url) { h.seekToEndOfFile(); h.write(data); try? h.close() }
        else { try? data.write(to: url) }
    }
    private static let signature = OSType(0x434C_4652) // "CLFR"

    static func register() {
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let handlerStatus = InstallEventHandler(GetApplicationEventTarget(), { _, event, _ in
            var id = EventHotKeyID()
            GetEventParameter(event, EventParamName(kEventParamDirectObject), EventParamType(typeEventHotKeyID),
                              nil, MemoryLayout<EventHotKeyID>.size, nil, &id)
            Task { @MainActor in
                switch id.id {
                case 1: OverlaySession.toggle(.element, from: .shortcut)
                case 2: OverlaySession.toggle(.screenshot, from: .shortcut)
                case 3: OverlaySession.toggle(.clip, from: .shortcut)
                default: break
                }
            }
            return noErr
        }, 1, &spec, nil, nil)
        log("handler installed status=\(handlerStatus)")

        let mods = UInt32(controlKey | shiftKey)
        for (key, id) in [(kVK_ANSI_1, 1), (kVK_ANSI_2, 2), (kVK_ANSI_3, 3)] {
            var ref: EventHotKeyRef?
            let status = RegisterEventHotKey(UInt32(key), mods, EventHotKeyID(signature: signature, id: UInt32(id)),
                                             GetApplicationEventTarget(), 0, &ref)
            let name = [element, screenshot, clip][id - 1]
            log("register \(name) status=\(status)")
            if status != noErr { failed.append(name) }
            refs.append(ref)
        }
    }
}
