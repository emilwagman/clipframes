import AppKit

/// Local-only usage counts that drive the adoption nudges. Nothing leaves the Mac.
///
/// Adoption (MVA) for Clipframes: captures on 4 different days within 14 days,
/// most of them started from the keyboard (the shortcut, then 1/2/3) rather than by clicking.
@MainActor
enum Habit {
    enum Source: String { case bar, shortcut, menu, library }

    private static let d = UserDefaults.standard
    private static let daysKey = "habit.days"           // [yyyy-MM-dd: count]
    private static let shortcutKey = "habit.shortcutUses"
    private static let barKey = "habit.barUses"
    private static let tipKey = "habit.tipShown."       // + tool
    private static let barToolKey = "habit.barUses."    // + tool
    private static let loginOfferKey = "habit.loginOffered"

    // MARK: Recording what happened

    static func started(_ mode: OverlaySession.Mode, from source: Source) {
        let tool = name(mode)
        switch source {
        case .shortcut:
            d.set(d.integer(forKey: shortcutKey) + 1, forKey: shortcutKey)
        case .bar, .menu, .library:
            d.set(d.integer(forKey: barKey) + 1, forKey: barKey)
            let n = d.integer(forKey: barToolKey + tool) + 1
            d.set(n, forKey: barToolKey + tool)
            // Second click on a tool: teach the shortcut, once.
            if n >= 2, !d.bool(forKey: tipKey + "shortcut") {
                d.set(true, forKey: tipKey + "shortcut")
                pendingTip = "Tip: \(Hotkey.text) opens the bar from any app. Then press \(number(mode)) for \(tool)."
            }
        }
    }

    /// Called after every saved capture.
    static func captured() {
        var days = d.dictionary(forKey: daysKey) as? [String: Int] ?? [:]
        let today = dayKey(Date())
        days[today, default: 0] += 1
        // Keep 60 days.
        let cutoff = dayKey(Date().addingTimeInterval(-60 * 86400))
        days = days.filter { $0.key >= cutoff }
        d.set(days, forKey: daysKey)
    }

    /// A tip waiting to be shown after the current capture's confirmation.
    static var pendingTip: String?

    static func takeTip() -> String? {
        defer { pendingTip = nil }
        return pendingTip
    }

    // MARK: Reading it back

    static var totalCaptures: Int {
        (d.dictionary(forKey: daysKey) as? [String: Int] ?? [:]).values.reduce(0, +)
    }

    /// Captures and distinct days in the last `days` days.
    static func recent(days: Int) -> (captures: Int, days: Int) {
        let all = d.dictionary(forKey: daysKey) as? [String: Int] ?? [:]
        let cutoff = dayKey(Date().addingTimeInterval(-Double(days - 1) * 86400))
        let recent = all.filter { $0.key >= cutoff }
        return (recent.values.reduce(0, +), recent.count)
    }

    static var shortcutShare: Double {
        let s = Double(d.integer(forKey: shortcutKey)), b = Double(d.integer(forKey: barKey))
        return s + b == 0 ? 0 : s / (s + b)
    }

    /// Adopted: captures on 4+ days within 14.
    static var adopted: Bool { recent(days: 14).days >= 4 }

    /// Offer Open at login once, after the third capture.
    static var shouldOfferLogin: Bool {
        !d.bool(forKey: loginOfferKey) && totalCaptures >= 3
    }

    static func loginOffered() { d.set(true, forKey: loginOfferKey) }

    // MARK: Helpers

    static func name(_ mode: OverlaySession.Mode) -> String {
        switch mode {
        case .element: "Element"
        case .screenshot: "Screenshot"
        case .clip: "Clip"
        }
    }

    /// The key that picks a tool once the bar has the keyboard.
    static func number(_ mode: OverlaySession.Mode) -> String {
        switch mode {
        case .element: "1"
        case .screenshot: "2"
        case .clip: "3"
        }
    }

    private static func dayKey(_ date: Date) -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: date)
    }
}
