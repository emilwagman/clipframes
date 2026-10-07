// axspike: measures how much the macOS accessibility tree says about each app.
//
//   axspike apps                      running apps with windows
//   axspike wake <pid>                turn on Chromium/Electron web content tree
//   axspike tree <pid>                walk the front window, print coverage stats + samples
//   axspike grid <pid> [n]            hit-test an n×n grid over the front window
//   axspike at <x> <y>                element at a screen point
import AppKit
import ApplicationServices

setbuf(stdout, nil)
let system = AXUIElementCreateSystemWide()
AXUIElementSetMessagingTimeout(system, 1.0)

func attr(_ el: AXUIElement, _ name: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(el, name as CFString, &v) == .success ? v : nil
}

func str(_ el: AXUIElement, _ name: String) -> String {
    guard let v = attr(el, name) else { return "" }
    if let s = v as? String { return s.trimmingCharacters(in: .whitespacesAndNewlines).replacingOccurrences(of: "\n", with: " ") }
    if let u = v as? URL { return u.absoluteString }
    if let a = v as? [String] { return a.joined(separator: " ") }
    return ""
}

func children(_ el: AXUIElement) -> [AXUIElement] {
    guard let v = attr(el, kAXChildrenAttribute) as? [AnyObject] else { return [] }
    return v.compactMap { CFGetTypeID($0) == AXUIElementGetTypeID() ? ($0 as! AXUIElement) : nil }
}

func parent(_ el: AXUIElement) -> AXUIElement? {
    guard let v = attr(el, kAXParentAttribute), CFGetTypeID(v) == AXUIElementGetTypeID() else { return nil }
    return (v as! AXUIElement)
}

func frame(_ el: AXUIElement) -> CGRect {
    var o = CGPoint.zero, s = CGSize.zero
    if let v = attr(el, kAXPositionAttribute), CFGetTypeID(v) == AXValueGetTypeID() { AXValueGetValue(v as! AXValue, .cgPoint, &o) }
    if let v = attr(el, kAXSizeAttribute), CFGetTypeID(v) == AXValueGetTypeID() { AXValueGetValue(v as! AXValue, .cgSize, &s) }
    return CGRect(origin: o, size: s)
}

func innerText(_ el: AXUIElement, limit: Int = 3) -> String {
    var out: [String] = []
    var stack = children(el).reversed().map { ($0, 0) }
    var seen = 0
    while let (c, d) = stack.popLast(), out.count < limit, seen < 200 {
        seen += 1
        let role = str(c, kAXRoleAttribute)
        let t = role == "AXStaticText" ? str(c, kAXValueAttribute) : ""
        if !t.isEmpty { out.append(String(t.prefix(30))) }
        if d < 6 { for k in children(c).reversed() { stack.append((k, d + 1)) } }
    }
    return out.joined(separator: " | ")
}

struct Facts {
    var role = "", title = "", desc = "", value = "", ident = "", domID = "", classes = "", help = ""
    var name: String { [title, desc, help].first { !$0.isEmpty } ?? "" }
    init(_ el: AXUIElement) {
        role = str(el, kAXRoleAttribute).replacingOccurrences(of: "AX", with: "")
        title = str(el, kAXTitleAttribute)
        desc = str(el, kAXDescriptionAttribute)
        value = String(str(el, kAXValueAttribute).prefix(60))
        ident = str(el, kAXIdentifierAttribute)
        domID = str(el, "AXDOMIdentifier")
        classes = String(str(el, "AXDOMClassList").prefix(80))
        help = str(el, kAXHelpAttribute)
    }
    var label: String {
        var s = role
        if !name.isEmpty { s += " \"\(name.prefix(40))\"" }
        else if !value.isEmpty, role == "StaticText" { s += " '\(value.prefix(40))'" }
        if !ident.isEmpty { s += " id=\(ident.prefix(40))" }
        if !domID.isEmpty { s += " #\(domID.prefix(30))" }
        if !classes.isEmpty { s += " .\(classes.split(separator: " ").prefix(3).joined(separator: "."))" }
        return s
    }
}

func frontWindow(_ pid: pid_t) -> AXUIElement? {
    let app = AXUIElementCreateApplication(pid)
    AXUIElementSetMessagingTimeout(app, 1.0)
    if let w = attr(app, kAXFocusedWindowAttribute), CFGetTypeID(w) == AXUIElementGetTypeID() { return (w as! AXUIElement) }
    if let ws = attr(app, kAXWindowsAttribute) as? [AnyObject], let w = ws.first { return (w as! AXUIElement) }
    return nil
}

func path(_ el: AXUIElement) -> (String, Int, String) {
    var parts: [String] = []
    var url = ""
    var cur = parent(el), depth = 0
    while let c = cur, depth < 60 {
        let f = Facts(c)
        if f.role == "Application" { break }
        if f.role == "WebArea", url.isEmpty { url = str(c, "AXURL") }
        if !(f.name.isEmpty && f.ident.isEmpty && f.domID.isEmpty) { parts.insert(f.label, at: 0) }
        cur = parent(c); depth += 1
    }
    return (parts.suffix(4).joined(separator: " › "), depth, url)
}

let args = CommandLine.arguments
switch args.count > 1 ? args[1] : "" {
case "apps":
    for app in NSWorkspace.shared.runningApplications where app.activationPolicy == .regular {
        let w = frontWindow(app.processIdentifier)
        print("\(app.processIdentifier)\t\(app.bundleIdentifier ?? "")\t\(app.localizedName ?? "")\t\(w.map { str($0, kAXTitleAttribute) } ?? "(no window)")")
    }

case "wake":
    let pid = pid_t(args[2])!
    let app = AXUIElementCreateApplication(pid)
    let a = AXUIElementSetAttributeValue(app, "AXManualAccessibility" as CFString, kCFBooleanTrue)
    let b = AXUIElementSetAttributeValue(app, "AXEnhancedUserInterface" as CFString, kCFBooleanTrue)
    print("AXManualAccessibility=\(a.rawValue) AXEnhancedUserInterface=\(b.rawValue)")

case "move", "click", "rclick":
    let pt = CGPoint(x: Double(args[2])!, y: Double(args[3])!)
    let src = CGEventSource(stateID: .hidSystemState)
    CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: pt, mouseButton: .left)?.post(tap: .cghidEventTap)
    if args[1] != "move" {
        usleep(80_000)
        let right = args[1] == "rclick"
        let d = CGEvent(mouseEventSource: src, mouseType: right ? .rightMouseDown : .leftMouseDown, mouseCursorPosition: pt, mouseButton: right ? .right : .left)
        d?.flags = []; d?.post(tap: .cghidEventTap)
        usleep(60_000)
        let u = CGEvent(mouseEventSource: src, mouseType: right ? .rightMouseUp : .leftMouseUp, mouseCursorPosition: pt, mouseButton: right ? .right : .left)
        u?.flags = []; u?.post(tap: .cghidEventTap)
    }

case "key":
    // axspike key <keycode> [ctrl,alt,cmd,shift]
    let code = CGKeyCode(args[2])!
    var flags: CGEventFlags = []
    let mods = args.count > 3 ? args[3] : ""
    if mods.contains("ctrl") { flags.insert(.maskControl) }
    if mods.contains("alt") { flags.insert(.maskAlternate) }
    if mods.contains("cmd") { flags.insert(.maskCommand) }
    if mods.contains("shift") { flags.insert(.maskShift) }
    let src = CGEventSource(stateID: .hidSystemState)
    let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); down?.flags = flags; down?.post(tap: .cghidEventTap)
    usleep(30_000)
    let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); up?.flags = flags; up?.post(tap: .cghidEventTap)
    // Release the modifiers, or the next synthetic click becomes a control-click.
    usleep(20_000)
    let clear = CGEvent(source: src); clear?.type = .flagsChanged; clear?.flags = []; clear?.post(tap: .cghidEventTap)

case "type":
    let src = CGEventSource(stateID: .hidSystemState)
    for ch in args[2].utf16 {
        var c = ch
        let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true)
        down?.flags = []; down?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); down?.post(tap: .cghidEventTap)
        let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false)
        up?.flags = []; up?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); up?.post(tap: .cghidEventTap)
        usleep(12_000)
    }

case "find":
    // axspike find <pid> <text>: frames of elements whose name or text contains <text>
    let pid = pid_t(args[2])!
    guard let win = frontWindow(pid) else { print("no window"); exit(1) }
    var stack = [win]; var n = 0
    while let el = stack.popLast(), n < 5000 {
        n += 1
        let f = Facts(el)
        if (f.name + " " + f.value).localizedCaseInsensitiveContains(args[3]) {
            let r = frame(el); print("\(Int(r.midX)) \(Int(r.midY))  \(Int(r.minX)),\(Int(r.minY)) \(Int(r.width))x\(Int(r.height))  \(f.label)")
        }
        stack.append(contentsOf: children(el))
    }

case "windows":
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    for d in list.prefix(40) {
        let b = CGRect(dictionaryRepresentation: d[kCGWindowBounds as String] as! NSDictionary) ?? .zero
        print("L\(d[kCGWindowLayer as String] ?? 0) a\(d[kCGWindowAlpha as String] ?? 1) \(Int(b.minX)),\(Int(b.minY)) \(Int(b.width))x\(Int(b.height)) \(d[kCGWindowOwnerName as String] ?? "") '\(d[kCGWindowName as String] ?? "")'")
    }

case "setframe":
    // axspike setframe <pid> x y w h: move and size the front window
    let pid = pid_t(args[2])!
    guard let win = frontWindow(pid) else { print("no window"); exit(1) }
    var o = CGPoint(x: Double(args[3])!, y: Double(args[4])!)
    var sz = CGSize(width: Double(args[5])!, height: Double(args[6])!)
    AXUIElementSetAttributeValue(win, kAXPositionAttribute as CFString, AXValueCreate(.cgPoint, &o)!)
    AXUIElementSetAttributeValue(win, kAXSizeAttribute as CFString, AXValueCreate(.cgSize, &sz)!)
    print(frame(win))

case "wins":
    // axspike wins <pid> [minimize-index]: list an app's windows, optionally minimize one
    let app = AXUIElementCreateApplication(pid_t(args[2])!)
    let ws = (attr(app, kAXWindowsAttribute) as? [AnyObject] ?? []).map { $0 as! AXUIElement }
    for (i, w) in ws.enumerated() { print(i, str(w, kAXTitleAttribute), frame(w)) }
    if args.count > 3, let i = Int(args[3]), i < ws.count {
        AXUIElementSetAttributeValue(ws[i], kAXMinimizedAttribute as CFString, kCFBooleanTrue)
        print("minimized \(i)")
    }

case "glide":
    // axspike glide x y [ms]: ease the pointer to a point
    let to = CGPoint(x: Double(args[2])!, y: Double(args[3])!)
    let ms = args.count > 4 ? Double(args[4])! : 500
    let from = CGEvent(source: nil)?.location ?? to
    let steps = max(2, Int(ms / 12))
    let src = CGEventSource(stateID: .hidSystemState)
    for i in 1...steps {
        let t = Double(i) / Double(steps)
        let e = t < 0.5 ? 4 * t * t * t : 1 - pow(-2 * t + 2, 3) / 2
        let p = CGPoint(x: from.x + (to.x - from.x) * e, y: from.y + (to.y - from.y) * e)
        let ev = CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: .left)
        ev?.flags = []; ev?.post(tap: .cghidEventTap)
        usleep(12_000)
    }

case "slowtype":
    let src = CGEventSource(stateID: .hidSystemState)
    for ch in args[2].utf16 {
        var c = ch
        let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true)
        down?.flags = []; down?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); down?.post(tap: .cghidEventTap)
        let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false)
        up?.flags = []; up?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); up?.post(tap: .cghidEventTap)
        usleep(UInt32(38_000 + Int.random(in: 0...30_000)))
    }

case "webkids":
    // axspike webkids <pid>: frames of the page area and its first two levels
    guard let win = frontWindow(pid_t(args[2])!) else { exit(1) }
    var stack = [win]; var web: AXUIElement?
    while let el = stack.popLast() { if str(el, kAXRoleAttribute) == "AXWebArea" { web = el; break }; stack.append(contentsOf: children(el)) }
    guard let w = web else { print("no web area"); exit(1) }
    print("WebArea", frame(w))
    for c in children(w) { print("  ", Facts(c).label, frame(c)); for g in children(c).prefix(3) { print("     ", Facts(g).label, frame(g)) } }

case "drag":
    // axspike drag x1 y1 x2 y2 [ms] [holdms]: press, move, release (holdms keeps the button down at the end)
    let a = CGPoint(x: Double(args[2])!, y: Double(args[3])!)
    let b = CGPoint(x: Double(args[4])!, y: Double(args[5])!)
    let ms = args.count > 6 ? Double(args[6])! : 600
    let hold = args.count > 7 ? UInt32(args[7])! : 0
    let src = CGEventSource(stateID: .hidSystemState)
    func post(_ t: CGEventType, _ p: CGPoint) { let e = CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: p, mouseButton: .left); e?.flags = []; e?.post(tap: .cghidEventTap) }
    post(.mouseMoved, a); usleep(60_000); post(.leftMouseDown, a); usleep(80_000)
    let steps = max(2, Int(ms / 12))
    for i in 1...steps {
        let t = Double(i) / Double(steps)
        let e = t < 0.5 ? 4 * t * t * t : 1 - pow(-2 * t + 2, 3) / 2
        post(.leftMouseDragged, CGPoint(x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e))
        usleep(12_000)
    }
    usleep(hold * 1000)
    post(.leftMouseUp, b)

case "press":
    var hit: AXUIElement?
    guard AXUIElementCopyElementAtPosition(system, Float(args[2])!, Float(args[3])!, &hit) == .success, let el = hit else { print("none"); exit(1) }
    print(Facts(el).label, AXUIElementPerformAction(el, kAXPressAction as CFString).rawValue)

case "unwake":
    let app = AXUIElementCreateApplication(pid_t(args[2])!)
    let a = AXUIElementSetAttributeValue(app, "AXManualAccessibility" as CFString, kCFBooleanFalse)
    let b = AXUIElementSetAttributeValue(app, "AXEnhancedUserInterface" as CFString, kCFBooleanFalse)
    print("off: \(a.rawValue) \(b.rawValue)")

case "tree":
    let pid = pid_t(args[2])!
    guard let win = frontWindow(pid) else { print("no window"); exit(1) }
    let start = Date()
    var stack: [(AXUIElement, Int, Bool)] = [(win, 0, false)]
    var n = 0, named = 0, ident = 0, dom = 0, cls = 0, interactive = 0, interNamed = 0, maxDepth = 0
    var samples: [String] = []
    var webSamples: [String] = []
    var webNodes = 0
    let interactiveRoles: Set<String> = ["Button", "CheckBox", "RadioButton", "PopUpButton", "MenuButton", "TextField",
                                         "TextArea", "Link", "Slider", "ComboBox", "Tab", "Switch", "Cell", "Row", "MenuItem", "DisclosureTriangle"]
    while let (el, d, inWeb) = stack.popLast(), n < 6000, Date().timeIntervalSince(start) < 20 {
        n += 1; maxDepth = max(maxDepth, d)
        let f = Facts(el)
        let web = inWeb || f.role == "WebArea"
        if web { webNodes += 1 }
        if !f.name.isEmpty || (f.role == "StaticText" && !f.value.isEmpty) { named += 1 }
        if !f.ident.isEmpty { ident += 1 }
        if !f.domID.isEmpty { dom += 1 }
        if !f.classes.isEmpty { cls += 1 }
        if interactiveRoles.contains(f.role) {
            interactive += 1
            if !f.name.isEmpty || !f.value.isEmpty { interNamed += 1 }
            if web { if webSamples.count < 24 { webSamples.append(f.label) } }
            else if samples.count < 12 { samples.append(f.label) }
        } else if web, !f.classes.isEmpty || !f.domID.isEmpty, f.name.isEmpty, webSamples.count < 24,
                  f.classes.contains("btn") || f.classes.contains("nav") || f.classes.contains("card") {
            webSamples.append("\(f.label)  text: \(innerText(el))")
        }
        for c in children(el).reversed() { stack.append((c, d + 1, web)) }
    }
    func pct(_ a: Int, _ b: Int) -> String { b == 0 ? "–" : "\(a * 100 / b)%" }
    print(String(format: "nodes=%d depth=%d time=%.1fs", n, maxDepth, Date().timeIntervalSince(start)))
    print("named=\(pct(named, n)) identifier=\(pct(ident, n)) domID=\(pct(dom, n)) classes=\(pct(cls, n))")
    print("interactive=\(interactive) namedInteractive=\(pct(interNamed, interactive))")
    for s in samples { print("  · \(s)") }
    if webNodes > 0 { print("web content: \(webNodes) nodes"); for s in webSamples { print("  ◦ \(s)") } }

case "grid":
    let pid = pid_t(args[2])!
    let n = args.count > 3 ? Int(args[3])! : 5
    guard let win = frontWindow(pid) else { print("no window"); exit(1) }
    let r = frame(win)
    print("window \"\(str(win, kAXTitleAttribute))\" \(Int(r.width))×\(Int(r.height))")
    var total = 0.0, hits = 0, useful = 0
    for iy in 0..<n {
        for ix in 0..<n {
            let x = r.minX + r.width * (Double(ix) + 0.5) / Double(n)
            let y = r.minY + r.height * (Double(iy) + 0.5) / Double(n)
            var hit: AXUIElement?
            let t0 = Date()
            let err = AXUIElementCopyElementAtPosition(system, Float(x), Float(y), &hit)
            let ms = Date().timeIntervalSince(t0) * 1000
            total += ms
            guard err == .success, let el = hit else { print(String(format: "  (%4.0f,%4.0f) error %d", x, y, err.rawValue)); continue }
            var hp: pid_t = 0; AXUIElementGetPid(el, &hp)
            hits += 1
            let f = Facts(el)
            let (p, depth, url) = path(el)
            let text = f.name.isEmpty ? innerText(el) : ""
            let good = !f.name.isEmpty || !f.ident.isEmpty || !f.domID.isEmpty || !f.value.isEmpty || !text.isEmpty
            if good { useful += 1 }
            print(String(format: "  %@ %3.0fms d%-2d %@%@%@  ⟵ %@%@", good ? "✓" : "·", ms, depth, f.label, text.isEmpty ? "" : "  text: \(text)",
                         hp == pid ? "" : " [other pid \(hp)]", p.isEmpty ? "(no named ancestors)" : p, url.isEmpty ? "" : "  <\(url.prefix(50))>"))
        }
    }
    print(String(format: "useful=%d/%d  avg=%.0fms", useful, hits, hits > 0 ? total / Double(hits) : 0))

case "at":
    var hit: AXUIElement?
    guard AXUIElementCopyElementAtPosition(system, Float(args[2])!, Float(args[3])!, &hit) == .success, let el = hit else { print("none"); exit(1) }
    let (p, d, url) = path(el)
    print("\(Facts(el).label)\n  path(d\(d)): \(p)\n  url: \(url)")

default:
    print("usage: axspike apps | wake <pid> | tree <pid> | grid <pid> [n] | at <x> <y>")
}
