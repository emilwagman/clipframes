import AppKit
import ApplicationServices

/// A window on screen, from the window server list.
struct ScreenWindow: Equatable {
    let id: CGWindowID
    let pid: pid_t
    let frame: CGRect // global, top-left origin
    let owner: String
    let title: String
}

enum WindowList {
    /// Front-to-back normal windows of other apps.
    static func all() -> [ScreenWindow] {
        guard let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
            as? [[String: Any]] else { return [] }
        let me = getpid()
        return list.compactMap { d in
            guard (d[kCGWindowLayer as String] as? Int) == 0,
                  let pid = d[kCGWindowOwnerPID as String] as? pid_t, pid != me,
                  let num = d[kCGWindowNumber as String] as? CGWindowID,
                  let b = d[kCGWindowBounds as String] as? NSDictionary,
                  let frame = CGRect(dictionaryRepresentation: b),
                  frame.width > 40, frame.height > 40,
                  (d[kCGWindowAlpha as String] as? Double ?? 1) > 0.05 else { return nil }
            return ScreenWindow(id: num, pid: pid, frame: frame,
                             owner: d[kCGWindowOwnerName as String] as? String ?? "",
                             title: d[kCGWindowName as String] as? String ?? "")
        }
    }

    static func at(_ p: CGPoint) -> ScreenWindow? { all().first { $0.frame.contains(p) } }
}

/// Reads the accessibility tree. All calls run on `queue`.
final class AXReader {
    static let shared = AXReader()
    let queue = DispatchQueue(label: "clipframes.ax", qos: .userInteractive)
    private var woken: [pid_t: Date] = [:]
    private let interactive: Set<String> = ["Button", "Link", "CheckBox", "RadioButton", "PopUpButton", "MenuButton",
                                            "TextField", "TextArea", "ComboBox", "Slider", "Tab", "MenuItem", "Cell",
                                            "Row", "Switch", "DisclosureTriangle", "Image", "Heading", "SearchField"]

    static var trusted: Bool { AXIsProcessTrusted() }

    /// Element under a point, skipping our own windows.
    func element(at p: CGPoint) -> (ElementInfo, ScreenWindow)? {
        guard let win = WindowList.at(p) else { return nil }
        let running = NSRunningApplication(processIdentifier: win.pid)
        wake(win.pid, running)

        var base = ElementInfo()
        base.app = running?.localizedName ?? win.owner
        base.bundleID = running?.bundleIdentifier ?? ""
        base.pid = win.pid
        base.window = win.title
        base.frame = win.frame
        base.role = "Window"

        guard Self.trusted else { return (base, win) }
        let app = AXUIElementCreateApplication(win.pid)
        AXUIElementSetMessagingTimeout(app, 0.25)
        var hit: AXUIElement?
        guard AXUIElementCopyElementAtPosition(app, Float(p.x), Float(p.y), &hit) == .success, let deepest = hit else {
            return (base, win)
        }
        let target = bestTarget(deepest)
        var info = describe(target, base: base)
        if info.frame.width < 1 { info.frame = win.frame }
        return (info, win)
    }

    /// Everything worth naming inside an area, found by hit-testing a grid of points.
    func elements(in rect: CGRect, limit: Int = 40) -> [ElementInfo] {
        let area = rect.width * rect.height
        guard area > 0 else { return [] }
        let step = max(18, (area / 350).squareRoot())
        var seen = Set<String>()
        var out: [ElementInfo] = []
        var y = rect.minY + step / 2
        while y < rect.maxY {
            var x = rect.minX + step / 2
            while x < rect.maxX {
                if let (el, _) = element(at: CGPoint(x: x, y: y)), Self.worthNaming(el, in: rect) {
                    let key = "\(el.pid)|\(el.headline)|\(el.selector)|\(Int(el.frame.minX)),\(Int(el.frame.minY))"
                    if seen.insert(key).inserted { out.append(el) }
                }
                x += step
            }
            y += step
        }
        return Array(out.sorted { ($0.frame.minY, $0.frame.minX) < ($1.frame.minY, $1.frame.minX) }.prefix(limit))
    }

    private static func worthNaming(_ el: ElementInfo, in rect: CGRect) -> Bool {
        if ["Window", "WebArea", "ScrollArea", "Application", "SplitGroup", "Unknown"].contains(el.role) { return false }
        if el.isWeak { return false }
        // Containers as big as the area say nothing the image doesn't.
        return !(el.frame.width >= rect.width * 0.9 && el.frame.height >= rect.height * 0.9)
    }

    /// Prefer the control a person means: a StaticText inside a Button means the Button.
    private func bestTarget(_ el: AXUIElement) -> AXUIElement {
        // Text inside a styled element (a badge, a clickable div) means that element,
        // unless the text sits directly in a control, which the loop below finds anyway.
        if str(el, kAXRoleAttribute) == "AXStaticText", let p = parent(el) {
            let pRole = str(p, kAXRoleAttribute).replacingOccurrences(of: "AX", with: "")
            let styled = !str(p, "AXDOMClassList").isEmpty || !str(p, "AXDOMIdentifier").isEmpty
            if styled, !["Cell", "Row", "WebArea"].contains(pRole) { return p }
        }
        var cur: AXUIElement? = el
        for _ in 0..<4 {
            guard let c = cur else { break }
            let role = str(c, kAXRoleAttribute).replacingOccurrences(of: "AX", with: "")
            if ["WebArea", "Window", "ScrollArea", "Application"].contains(role) { break }
            if interactive.contains(role) { return c }
            if role != "StaticText", pressable(c) { return c }
            cur = parent(c)
        }
        return el
    }

    private func pressable(_ el: AXUIElement) -> Bool {
        var names: CFArray?
        guard AXUIElementCopyActionNames(el, &names) == .success, let list = names as? [String] else { return false }
        return list.contains(kAXPressAction as String)
    }

    private func describe(_ el: AXUIElement, base: ElementInfo) -> ElementInfo {
        var i = base
        i.role = str(el, kAXRoleAttribute).replacingOccurrences(of: "AX", with: "")
        i.name = [kAXTitleAttribute, kAXDescriptionAttribute, kAXPlaceholderValueAttribute, kAXHelpAttribute]
            .lazy.map { self.str(el, $0) }.first { !$0.isEmpty } ?? ""
        let value = str(el, kAXValueAttribute)
        if value.count < 200 { i.value = value }
        let ident = str(el, kAXIdentifierAttribute)
        if !ident.hasPrefix("_NS:") { i.identifier = ident }
        i.domID = str(el, "AXDOMIdentifier")
        i.domClasses = str(el, "AXDOMClassList")
        if i.name.isEmpty { i.innerText = innerText(el) }
        i.frame = frame(el)

        var path: [String] = []
        var cur = parent(el)
        var depth = 0
        var inPage = true // labels above the page (browser chrome) are noise
        while let c = cur, depth < 50 {
            let role = str(c, kAXRoleAttribute).replacingOccurrences(of: "AX", with: "")
            if role == "Application" { break }
            if role == "Window", i.window.isEmpty { i.window = str(c, kAXTitleAttribute) }
            if role == "WebArea" {
                if i.url.isEmpty { i.url = str(c, "AXURL") }
                inPage = false
            } else if inPage {
                let label = shortLabel(c, role: role)
                if !label.isEmpty, role != "Window" { path.insert(label, at: 0) }
            }
            cur = parent(c)
            depth += 1
        }
        if i.url.isEmpty { path = path.filter { !$0.hasPrefix("Group \"\(i.window.prefix(20))") } }
        i.path = Array(path.suffix(4))
        return i
    }

    private func shortLabel(_ el: AXUIElement, role: String) -> String {
        let name = [kAXTitleAttribute, kAXDescriptionAttribute].lazy.map { self.str(el, $0) }.first { !$0.isEmpty } ?? ""
        let dom = str(el, "AXDOMIdentifier")
        let classes = str(el, "AXDOMClassList").split(separator: " ").prefix(3)
        var s = ""
        if !name.isEmpty { s = "\(role) \"\(name.prefix(40))\"" }
        if !dom.isEmpty { s = (s.isEmpty ? role : s) + " #\(dom)" }
        if name.isEmpty, dom.isEmpty, !classes.isEmpty { s = "\(role) ." + classes.joined(separator: ".") }
        return s
    }

    private func innerText(_ el: AXUIElement) -> String {
        var out: [String] = []
        var stack = children(el).reversed().map { ($0, 0) }
        var seen = 0
        while let (c, d) = stack.popLast(), out.count < 3, seen < 150 {
            seen += 1
            if str(c, kAXRoleAttribute) == "AXStaticText" {
                let t = str(c, kAXValueAttribute)
                if !t.isEmpty { out.append(String(t.prefix(40))) }
            }
            if d < 5 { for k in children(c).reversed() { stack.append((k, d + 1)) } }
        }
        return out.joined(separator: " ")
    }

    // MARK: Chromium and Electron only build their web tree when asked.

    /// Ask an app for its tree ahead of time; Chromium takes a few seconds to build it.
    func prepare(_ pid: pid_t) {
        wake(pid, NSRunningApplication(processIdentifier: pid))
    }

    private func wake(_ pid: pid_t, _ app: NSRunningApplication?) {
        guard UserDefaults.standard.bool(forKey: Pref.readWebApps) else { return }
        if woken[pid] == nil {
            guard let app, Self.isChromium(app) else { return }
            let el = AXUIElementCreateApplication(pid)
            AXUIElementSetAttributeValue(el, "AXManualAccessibility" as CFString, kCFBooleanTrue)
            AXUIElementSetAttributeValue(el, "AXEnhancedUserInterface" as CFString, kCFBooleanTrue)
        }
        woken[pid] = Date()
    }

    /// Turn it back off for apps we haven't looked at in a while; keeping it on costs them CPU.
    func sleepIdle(olderThan age: TimeInterval) {
        let now = Date()
        for (pid, last) in woken where now.timeIntervalSince(last) > age {
            let el = AXUIElementCreateApplication(pid)
            AXUIElementSetAttributeValue(el, "AXManualAccessibility" as CFString, kCFBooleanFalse)
            AXUIElementSetAttributeValue(el, "AXEnhancedUserInterface" as CFString, kCFBooleanFalse)
            woken[pid] = nil
        }
    }

    static func isChromium(_ app: NSRunningApplication) -> Bool {
        let id = (app.bundleIdentifier ?? "").lowercased()
        if ["chrome", "chromium", "edgemac", "brave", "company.thebrowser", "vivaldi", "opera"].contains(where: id.contains) { return true }
        guard let url = app.bundleURL else { return false }
        return FileManager.default.fileExists(
            atPath: url.appendingPathComponent("Contents/Frameworks/Electron Framework.framework").path)
    }

    // MARK: Attribute helpers

    private func attr(_ el: AXUIElement, _ name: String) -> AnyObject? {
        var v: AnyObject?
        return AXUIElementCopyAttributeValue(el, name as CFString, &v) == .success ? v : nil
    }

    private func str(_ el: AXUIElement, _ name: String) -> String {
        guard let v = attr(el, name) else { return "" }
        if let s = v as? String { return s.trimmingCharacters(in: .whitespacesAndNewlines).replacingOccurrences(of: "\n", with: " ") }
        if let u = v as? URL { return u.absoluteString }
        if let a = v as? [String] { return a.joined(separator: " ") }
        return ""
    }

    private func children(_ el: AXUIElement) -> [AXUIElement] {
        guard let v = attr(el, kAXChildrenAttribute) as? [AnyObject] else { return [] }
        return v.compactMap { CFGetTypeID($0) == AXUIElementGetTypeID() ? ($0 as! AXUIElement) : nil }
    }

    private func parent(_ el: AXUIElement) -> AXUIElement? {
        guard let v = attr(el, kAXParentAttribute), CFGetTypeID(v) == AXUIElementGetTypeID() else { return nil }
        return (v as! AXUIElement)
    }

    private func frame(_ el: AXUIElement) -> CGRect {
        var o = CGPoint.zero, s = CGSize.zero
        if let v = attr(el, kAXPositionAttribute), CFGetTypeID(v) == AXValueGetTypeID() { AXValueGetValue(v as! AXValue, .cgPoint, &o) }
        if let v = attr(el, kAXSizeAttribute), CFGetTypeID(v) == AXValueGetTypeID() { AXValueGetValue(v as! AXValue, .cgSize, &s) }
        return CGRect(origin: o, size: s)
    }
}
