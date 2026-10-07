// Drives the mouse and keyboard for demo recordings, reading one command per line from stdin.
// Coordinates are global screen points, top-left origin.
//
//   move X Y [MS]          glide to X,Y over MS milliseconds (default 600), eased, slightly curved
//   click [X Y [MS]]       glide there first if X Y given, then click
//   down / up              press or release the left button where the cursor is
//   drag X Y [MS]          glide with the button held down
//   key COMBO              e.g. ctrl+shift+1, cmd+v, return, escape
//   type TEXT              types TEXT at a steady human pace
//   wait MS
//   warp X Y               jump instantly (setup only, never on camera)
//   where                  prints the cursor position
//
// Build: swiftc -O drive.swift -o drive
import AppKit
import CoreGraphics

let src = CGEventSource(stateID: .hidSystemState)
var buttonDown = false

func cursor() -> CGPoint { CGEvent(source: nil)?.location ?? .zero }

func post(_ type: CGEventType, _ p: CGPoint) {
    CGEvent(mouseEventSource: src, mouseType: type, mouseCursorPosition: p, mouseButton: .left)?.post(tap: .cghidEventTap)
}

func ms(_ v: Double) { usleep(useconds_t(max(0, v) * 1000)) }

func glide(to end: CGPoint, ms duration: Double) {
    let start = cursor()
    let dx = end.x - start.x, dy = end.y - start.y
    let dist = hypot(dx, dy)
    if dist < 1 { return }
    // A gentle arc: control point pushed sideways by a fraction of the distance.
    let bend = min(60, dist * 0.12)
    let ctrl = CGPoint(x: start.x + dx / 2 - dy / dist * bend, y: start.y + dy / 2 + dx / dist * bend)
    let steps = max(2, Int(duration / 1000 * 120))
    for i in 1...steps {
        let t0 = Double(i) / Double(steps)
        let t = t0 < 0.5 ? 4 * t0 * t0 * t0 : 1 - pow(-2 * t0 + 2, 3) / 2   // ease in-out cubic
        let u = 1 - t
        let p = CGPoint(x: u * u * start.x + 2 * u * t * ctrl.x + t * t * end.x,
                        y: u * u * start.y + 2 * u * t * ctrl.y + t * t * end.y)
        post(buttonDown ? .leftMouseDragged : .mouseMoved, p)
        ms(duration / Double(steps))
    }
}

let keyCodes: [String: CGKeyCode] = [
    "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12, "w": 13, "e": 14, "r": 15,
    "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23, "=": 24, "9": 25, "7": 26, "-": 27, "8": 28, "0": 29,
    "]": 30, "o": 31, "u": 32, "[": 33, "i": 34, "p": 35, "return": 36, "l": 37, "j": 38, "'": 39, "k": 40, ";": 41, "\\": 42,
    ",": 43, "/": 44, "n": 45, "m": 46, ".": 47, "tab": 48, "space": 49, "`": 50, "delete": 51, "escape": 53,
    "left": 123, "right": 124, "down": 125, "up": 126,
]

func key(_ combo: String) {
    var flags: CGEventFlags = []
    var code: CGKeyCode?
    for part in combo.lowercased().split(separator: "+").map(String.init) {
        switch part {
        case "cmd": flags.insert(.maskCommand)
        case "ctrl": flags.insert(.maskControl)
        case "shift": flags.insert(.maskShift)
        case "alt", "opt": flags.insert(.maskAlternate)
        default: code = keyCodes[part]
        }
    }
    guard let code else { FileHandle.standardError.write("unknown key: \(combo)\n".data(using: .utf8)!); return }
    let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true)
    let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false)
    down?.flags = flags; up?.flags = flags
    down?.post(tap: .cghidEventTap); ms(40); up?.post(tap: .cghidEventTap)
}

func type(_ text: String) {
    for ch in text {
        var units = Array(String(ch).utf16)
        let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true)
        let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false)
        down?.keyboardSetUnicodeString(stringLength: units.count, unicodeString: &units)
        up?.keyboardSetUnicodeString(stringLength: units.count, unicodeString: &units)
        down?.post(tap: .cghidEventTap); ms(12); up?.post(tap: .cghidEventTap)
        ms(Double.random(in: 45...95))
    }
}

while let line = readLine() {
    let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
    guard let cmd = parts.first, !cmd.hasPrefix("#") else { continue }
    let args = parts.count > 1 ? parts[1].split(separator: " ").compactMap { Double($0) } : []
    switch cmd {
    case "move" where args.count >= 2: glide(to: CGPoint(x: args[0], y: args[1]), ms: args.count > 2 ? args[2] : 600)
    case "drag" where args.count >= 2: glide(to: CGPoint(x: args[0], y: args[1]), ms: args.count > 2 ? args[2] : 600)
    case "click":
        if args.count >= 2 { glide(to: CGPoint(x: args[0], y: args[1]), ms: args.count > 2 ? args[2] : 600); ms(90) }
        post(.leftMouseDown, cursor()); ms(70); post(.leftMouseUp, cursor())
    case "down": buttonDown = true; post(.leftMouseDown, cursor())
    case "up": post(.leftMouseUp, cursor()); buttonDown = false
    case "key": key(parts.count > 1 ? parts[1] : "")
    case "type": type(parts.count > 1 ? parts[1] : "")
    case "wait": ms(args.first ?? 0)
    case "warp" where args.count >= 2:
        let p = CGPoint(x: args[0], y: args[1]); CGWarpMouseCursorPosition(p); post(.mouseMoved, p)
    case "where": let p = cursor(); print("\(Int(p.x)) \(Int(p.y))")
    default: FileHandle.standardError.write("bad line: \(line)\n".data(using: .utf8)!)
    }
}
