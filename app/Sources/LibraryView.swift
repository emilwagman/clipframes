import ServiceManagement
import SwiftUI

// MARK: Look

enum Lib {
    static let bg = Brand.ink
    static let sidebar = Color(red: 0.11, green: 0.098, blue: 0.087)
    static let card = Color(red: 1.0, green: 0.93, blue: 0.86).opacity(0.05)
    static let cardStroke = Brand.line
    static let dim = Brand.text2
    static let faint = Brand.text3

    static func tint(_ kind: CaptureKind) -> Color { kind == .recording ? Brand.record : Brand.accent }
    static func icon(_ kind: CaptureKind) -> String {
        switch kind {
        case .element: "cursorarrow.rays"
        case .screenshot: "viewfinder"
        case .recording: "record.circle"
        }
    }
    static func name(_ kind: CaptureKind) -> String {
        switch kind {
        case .element: "Element"
        case .screenshot: "Screenshot"
        case .recording: "Clip"
        }
    }
}

// MARK: Main window

struct LibraryView: View {
    @EnvironmentObject var lib: Library

    var body: some View {
        HStack(spacing: 0) {
            Sidebar()
                .frame(width: 330)
                .background(Lib.sidebar)
            Rectangle().fill(Color.white.opacity(0.06)).frame(width: 1)
            Detail()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Lib.bg)
        }
        .ignoresSafeArea(edges: .top)
        .foregroundStyle(.white)
        .environment(\.colorScheme, .dark)
    }
}

// MARK: Sidebar

struct Sidebar: View {
    @EnvironmentObject var lib: Library
    @FocusState private var searchFocused: Bool

    var body: some View {
        VStack(spacing: 14) {
            HStack(spacing: 2) {
                Spacer()
                LogoMark(size: 20).padding(.trailing, 6)
                NewButton(icon: "cursorarrow.rays", help: "Capture an element (\(Hotkey.element))") { OverlaySession.toggle(.element, from: .library) }
                NewButton(icon: "viewfinder", help: "Take a screenshot (\(Hotkey.screenshot))") { OverlaySession.toggle(.screenshot, from: .library) }
                NewButton(icon: "record.circle", help: "Record a clip (\(Hotkey.clip))") { OverlaySession.toggle(.clip, from: .library) }
            }
            .padding(.horizontal, 10)
            .frame(height: 40) // shares the titlebar with the window buttons
            filters
            search
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 4, pinnedViews: []) {
                    if let working = lib.working {
                        HStack(spacing: 10) {
                            ProgressView().controlSize(.small)
                            Text(working).font(.system(size: 13)).foregroundStyle(Lib.dim)
                        }
                        .padding(12)
                    }
                    ForEach(groups, id: \.0) { day, items in
                        Text(day.uppercased())
                            .font(.system(size: 11, weight: .semibold)).tracking(0.8)
                            .foregroundStyle(Lib.faint)
                            .padding(.horizontal, 10).padding(.top, 12).padding(.bottom, 2)
                        ForEach(items) { c in
                            CaptureRow(capture: c, selected: lib.selection.contains(c.id))
                                .contentShape(Rectangle())
                                .onTapGesture { select(c) }
                                .contextMenu { RowMenu(capture: c) }
                        }
                    }
                    if lib.filtered.isEmpty && lib.working == nil {
                        VStack(spacing: 6) {
                            Image(systemName: lib.search.isEmpty ? "tray" : "magnifyingglass")
                                .font(.system(size: 22)).foregroundStyle(Lib.faint)
                            Text(lib.search.isEmpty ? "Nothing here yet" : "No matches").foregroundStyle(Lib.dim)
                        }
                        .frame(maxWidth: .infinity).padding(.top, 60)
                    }
                }
                .padding(.horizontal, 10)
                .padding(.bottom, 12)
            }
            .onDeleteCommand { lib.trash(lib.selected) }
        }
    }

    private func select(_ c: Capture) {
        if NSEvent.modifierFlags.contains(.command) {
            if lib.selection.contains(c.id) { lib.selection.remove(c.id) } else { lib.selection.insert(c.id) }
        } else {
            lib.selection = [c.id]
        }
    }

    private var groups: [(String, [Capture])] {
        let cal = Calendar.current
        var out: [(String, [Capture])] = []
        for c in lib.filtered {
            let d = c.meta.created
            let label = cal.isDateInToday(d) ? "Today" : cal.isDateInYesterday(d) ? "Yesterday"
                : d.formatted(.dateTime.weekday(.wide).month(.abbreviated).day())
            if out.last?.0 == label { out[out.count - 1].1.append(c) } else { out.append((label, [c])) }
        }
        return out
    }

    private var filters: some View {
        HStack(spacing: 4) {
            ForEach(Library.Filter.allCases, id: \.self) { f in
                let on = lib.filter == f
                Button { withAnimation(.smooth(duration: 0.18)) { lib.filter = f } } label: {
                    Text(f.rawValue)
                        .font(.system(size: 12, weight: on ? .semibold : .medium))
                        .foregroundStyle(on ? .white : Lib.dim)
                        .padding(.horizontal, 10).frame(height: 28)
                        .background(Capsule().fill(on ? Color.white.opacity(0.14) : .clear))
                        .contentShape(Capsule())
                }
                .buttonStyle(.plain)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
    }

    private var search: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass").foregroundStyle(Lib.faint)
            TextField("Search apps, elements, selectors", text: $lib.search)
                .textFieldStyle(.plain)
                .focused($searchFocused)
            if !lib.search.isEmpty {
                Button { lib.search = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(Lib.faint) }
                    .buttonStyle(.plain)
            }
        }
        .font(.system(size: 13))
        .padding(.horizontal, 11).frame(height: 34)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Color.white.opacity(searchFocused ? 0.1 : 0.06)))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(searchFocused ? Brand.accent.opacity(0.7) : Lib.cardStroke))
        .padding(.horizontal, 12)
    }
}

struct NewButton: View {
    let icon: String
    let help: String
    let action: () -> Void
    @State private var hover = false
    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(hover ? .white : Lib.dim)
                .frame(width: 32, height: 28)
                .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Color.white.opacity(hover ? 0.1 : 0)))
                .contentShape(Rectangle())
        }
        .buttonStyle(PressScale())
        .onHover { hover = $0 }
        .help(help)
    }
}

struct RowMenu: View {
    @EnvironmentObject var lib: Library
    let capture: Capture
    var body: some View {
        let targets = lib.selection.contains(capture.id) ? lib.selected : [capture]
        Button(targets.count > 1 ? "Copy \(targets.count) references" : "Copy reference") { lib.copy(targets, toast: true) }
        Button("Show in Finder") { lib.reveal(capture) }
        Divider()
        Button(targets.count > 1 ? "Move \(targets.count) to Trash" : "Move to Trash", role: .destructive) { lib.trash(targets) }
    }
}

struct CaptureRow: View {
    let capture: Capture
    let selected: Bool
    @State private var hover = false

    var body: some View {
        HStack(spacing: 12) {
            ZStack(alignment: .bottomLeading) {
                Thumb(url: capture.thumbnail, fit: capture.meta.kind == .element)
                    .frame(width: 84, height: 54)
                    .background(Color.white.opacity(0.05))
                    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(Color.white.opacity(0.08)))
                Image(systemName: Lib.icon(capture.meta.kind))
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(.white)
                    .frame(width: 18, height: 18)
                    .background(Circle().fill(Lib.tint(capture.meta.kind)))
                    .overlay(Circle().strokeBorder(Color.black.opacity(0.35), lineWidth: 1))
                    .offset(x: -4, y: 4)
            }
            VStack(alignment: .leading, spacing: 3) {
                Text(capture.title).font(.system(size: 13, weight: .semibold)).lineLimit(1)
                Text(detailLine).font(.system(size: 12)).foregroundStyle(Lib.dim).lineLimit(1)
            }
            Spacer(minLength: 4)
            Text(capture.meta.created.formatted(date: .omitted, time: .shortened))
                .font(.system(size: 11)).monospacedDigit().foregroundStyle(Lib.faint)
        }
        .padding(8)
        .background(RoundedRectangle(cornerRadius: 12, style: .continuous)
            .fill(selected ? Brand.accent.opacity(0.28) : Color.white.opacity(hover ? 0.05 : 0)))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
            .strokeBorder(selected ? Brand.accent.opacity(0.55) : .clear))
        .onHover { hover = $0 }
        .animation(.easeOut(duration: 0.12), value: hover)
    }

    private var detailLine: String {
        switch capture.meta.kind {
        case .element: return [capture.meta.app, capture.windowTitle].filter { !$0.isEmpty }.uniqued().joined(separator: " · ")
        case .screenshot:
            let n = capture.meta.hints?.count ?? 0
            return "\(capture.meta.app.isEmpty ? "Screenshot" : capture.meta.app)\(n > 0 ? " · \(n) elements" : "")"
        case .recording:
            return "\(Fmt.seconds(capture.meta.duration ?? 0)) · \(capture.meta.clicks.count) click\(capture.meta.clicks.count == 1 ? "" : "s")"
        }
    }
}

extension Array where Element == String {
    func uniqued() -> [String] { var seen = Set<String>(); return filter { seen.insert($0).inserted } }
}

struct AppIcon: View {
    let bundleID: String?
    var body: some View {
        if let id = bundleID, let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: id) {
            Image(nsImage: NSWorkspace.shared.icon(forFile: url.path)).resizable()
        } else {
            Image(systemName: "app.dashed").resizable().foregroundStyle(.secondary)
        }
    }
}

struct Thumb: View {
    let url: URL
    var fit = false
    var body: some View {
        if let image = NSImage(contentsOf: url) {
            Image(nsImage: image).resizable().aspectRatio(contentMode: fit ? .fit : .fill)
                .padding(fit ? 6 : 0)
        } else {
            Rectangle().fill(Color.white.opacity(0.05))
        }
    }
}

// MARK: Detail

struct Detail: View {
    @EnvironmentObject var lib: Library

    var body: some View {
        let selected = lib.selected
        Group {
            if !lib.permissions.ready {
                SetupPrompt()
            } else if selected.count > 1 {
                MultiSelection(captures: selected)
            } else if let c = selected.first {
                CaptureDetail(capture: c).id(c.id)
            } else {
                Welcome()
            }
        }
    }
}

struct CaptureDetail: View {
    @EnvironmentObject var lib: Library
    let capture: Capture
    @State private var note = ""
    @State private var frame = 1
    @State private var tab: Tab = .notes
    @State private var saveTask: Task<Void, Never>?

    enum Tab: Hashable { case clicks, elements, notes }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 26) {
                header
                AgentCard(capture: capture)
                preview
                tabs
                noteField
            }
            .padding(.horizontal, 36)
            .padding(.top, 58)
            .padding(.bottom, 36)
            .frame(maxWidth: 1000, alignment: .leading)
        }
        .onAppear {
            note = capture.meta.note
            tab = capture.meta.clicks.isEmpty ? ((capture.meta.hints ?? []).isEmpty ? .notes : .elements) : .clicks
        }
    }

    // MARK: Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Label(Lib.name(capture.meta.kind), systemImage: Lib.icon(capture.meta.kind))
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 10).frame(height: 24)
                    .background(Capsule().fill(Lib.tint(capture.meta.kind).opacity(0.85)))
                Text(Fmt.relative(capture.meta.created)).font(.system(size: 12)).foregroundStyle(Lib.faint)
            }
            Text(capture.title)
                .font(Brand.display(28, .bold)).tracking(-0.4)
                .lineLimit(2)
                .textSelection(.enabled)
            HStack(spacing: 8) {
                if !capture.meta.app.isEmpty {
                    AppIcon(bundleID: capture.meta.bundleID).frame(width: 16, height: 16)
                    Text(capture.meta.kind == .element && capture.windowTitle != capture.meta.app
                         ? "\(capture.meta.app) · \(capture.windowTitle)" : capture.meta.app)
                        .lineLimit(1)
                }
                if !capture.meta.url.isEmpty {
                    Text(capture.meta.url).font(.system(size: 12, design: .monospaced)).foregroundStyle(Lib.faint)
                        .lineLimit(1).truncationMode(.middle)
                }
            }
            .font(.system(size: 13))
            .foregroundStyle(Lib.dim)
        }
    }

    // MARK: Preview

    @ViewBuilder
    private var preview: some View {
        switch capture.meta.kind {
        case .element:
            if let img = NSImage(contentsOf: capture.screenshot) {
                Image(nsImage: img).resizable().aspectRatio(contentMode: .fit)
                    .frame(maxWidth: min(img.size.width, 760), maxHeight: 300)
                    .padding(44)
                    .frame(maxWidth: .infinity)
                    .background(Stage2())
                    .onTapGesture(count: 2) { NSWorkspace.shared.open(capture.screenshot) }
                    .help("Double-click to open full size")
            }
        case .screenshot:
            if let img = NSImage(contentsOf: capture.thumbnail) {
                Image(nsImage: img).resizable().aspectRatio(contentMode: .fit)
                    .frame(maxHeight: 520)
                    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .shadow(color: .black.opacity(0.4), radius: 18, y: 8)
                    .padding(24)
                    .frame(maxWidth: .infinity)
                    .background(Stage2())
                    .onTapGesture(count: 2) { NSWorkspace.shared.open(capture.thumbnail) }
                    .help("Double-click to open full size")
            }
        case .recording:
            ClipPlayer(capture: capture, frame: $frame)
        }
    }

    // MARK: Tabs

    private var tabs: some View {
        let clicks = capture.meta.clicks.count
        let hints = capture.meta.hints?.count ?? 0
        return VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 4) {
                if clicks > 0 { tabButton("Clicks", count: clicks, .clicks) }
                if hints > 0 { tabButton("In this area", count: hints, .elements) }
                if capture.meta.kind == .element, let el = capture.meta.element, !el.selector.isEmpty || !el.path.isEmpty {
                    tabButton("Element", count: nil, .elements)
                }
                tabButton("What your agent reads", count: nil, .notes)
            }
            Group {
                switch tab {
                case .clicks: ClickList(capture: capture, frame: $frame)
                case .elements:
                    if capture.meta.kind == .element, let el = capture.meta.element { ElementFacts(el: el) }
                    else { HintList(hints: capture.meta.hints ?? []) }
                case .notes:
                    Text((try? String(contentsOf: capture.notesFile, encoding: .utf8)) ?? capture.notes)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(Color.white.opacity(0.82))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 14, style: .continuous).fill(Lib.card))
            .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(Lib.cardStroke))
        }
    }

    private func tabButton(_ title: String, count: Int?, _ t: Tab) -> some View {
        let on = tab == t
        return Button { withAnimation(.smooth(duration: 0.18)) { tab = t } } label: {
            HStack(spacing: 6) {
                Text(title)
                if let count { Text("\(count)").monospacedDigit().foregroundStyle(on ? .white.opacity(0.8) : Lib.faint) }
            }
            .font(.system(size: 13, weight: on ? .semibold : .medium))
            .foregroundStyle(on ? .white : Lib.dim)
            .padding(.horizontal, 12).frame(height: 30)
            .background(Capsule().fill(on ? Color.white.opacity(0.12) : .clear))
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
    }

    // MARK: Note

    private var noteField: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Note for your agent").font(.system(size: 13, weight: .semibold)).foregroundStyle(Lib.dim)
            TextField("Optional. Added to the notes file, e.g. what you expected to happen.", text: $note, axis: .vertical)
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .lineLimit(2...6)
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Lib.card))
                .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Lib.cardStroke))
                .onChange(of: note) { _, _ in saveNoteSoon() }
        }
    }

    private func saveNoteSoon() {
        saveTask?.cancel()
        saveTask = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(500))
            guard !Task.isCancelled, note != capture.meta.note else { return }
            var c = capture
            c.meta.note = note
            lib.update(c)
        }
    }
}

/// The soft backdrop behind previews.
struct Stage2: View {
    var body: some View {
        RoundedRectangle(cornerRadius: 18, style: .continuous)
            .fill(LinearGradient(colors: [Color.white.opacity(0.07), Color.white.opacity(0.03)], startPoint: .top, endPoint: .bottom))
            .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).strokeBorder(Lib.cardStroke))
    }
}

/// The main action: the line you paste, and a big Copy.
struct AgentCard: View {
    @EnvironmentObject var lib: Library
    let capture: Capture

    var body: some View {
        let copied = lib.copiedID == capture.id
        HStack(alignment: .center, spacing: 14) {
            VStack(alignment: .leading, spacing: 6) {
                Text("PASTE INTO YOUR AGENT").font(.system(size: 10.5, weight: .semibold)).tracking(1.1).foregroundStyle(Lib.faint)
                Text(capture.reference)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(Color.white.opacity(0.85))
                    .lineLimit(2)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }
            Spacer(minLength: 8)
            Button { lib.reveal(capture) } label: {
                Image(systemName: "folder").font(.system(size: 14, weight: .medium))
                    .frame(width: 38, height: 38)
                    .background(Circle().fill(Color.white.opacity(0.08)))
                    .contentShape(Circle())
            }
            .buttonStyle(PressScale())
            .help("Show in Finder")
            Button { lib.copy([capture]) } label: {
                HStack(spacing: 7) {
                    Image(systemName: copied ? "checkmark" : "doc.on.doc").contentTransition(.symbolEffect(.replace))
                    Text(copied ? "Copied" : "Copy")
                }
                .font(.system(size: 14, weight: .semibold))
                .frame(width: 104, height: 38)
                .background(Capsule().fill(copied ? Color(red: 0.2, green: 0.62, blue: 0.4) : Brand.accent))
                .foregroundStyle(.white)
                .contentShape(Capsule())
            }
            .buttonStyle(PressScale())
            .keyboardShortcut("c", modifiers: [.command, .shift])
            .help("Copy the line for Claude Code or Codex (⇧⌘C)")
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Color.black.opacity(0.35)))
        .overlay(RoundedRectangle(cornerRadius: 16, style: .continuous).strokeBorder(Lib.cardStroke))
        .animation(.smooth(duration: 0.2), value: copied)
    }
}

/// Frames with a scrubber; click markers sit on the timeline.
struct ClipPlayer: View {
    let capture: Capture
    @Binding var frame: Int

    var body: some View {
        let count = max(1, capture.meta.frameCount ?? 1)
        let interval = capture.meta.interval ?? 0
        VStack(spacing: 14) {
            ZStack(alignment: .bottomTrailing) {
                if let img = NSImage(contentsOf: capture.frame(frame)) {
                    Image(nsImage: img).resizable().aspectRatio(contentMode: .fit)
                        .frame(maxHeight: 480)
                        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                        .shadow(color: .black.opacity(0.4), radius: 18, y: 8)
                }
                Button { NSWorkspace.shared.open(capture.video) } label: {
                    Label("Play video", systemImage: "play.fill")
                        .font(.system(size: 12, weight: .semibold))
                        .padding(.horizontal, 12).frame(height: 30)
                        .surface(radius: 15)
                }
                .buttonStyle(PressScale())
                .padding(12)
            }
            .padding(24)
            .frame(maxWidth: .infinity)
            .background(Stage2())

            // Timeline
            VStack(spacing: 8) {
                GeometryReader { geo in
                    let w = geo.size.width
                    ZStack(alignment: .leading) {
                        Capsule().fill(Color.white.opacity(0.1)).frame(height: 6)
                        Capsule().fill(Brand.accent).frame(width: max(6, w * CGFloat(frame) / CGFloat(count)), height: 6)
                        ForEach(Array(capture.meta.clicks.enumerated()), id: \.offset) { i, c in
                            if let f = c.frame {
                                Circle().fill(Brand.record)
                                    .overlay(Text("\(i + 1)").font(.system(size: 8, weight: .bold)).foregroundStyle(.white))
                                    .frame(width: 14, height: 14)
                                    .offset(x: w * (CGFloat(f) - 0.5) / CGFloat(count) - 7)
                                    .onTapGesture { frame = f }
                            }
                        }
                    }
                    .frame(height: 16)
                    .contentShape(Rectangle())
                    .gesture(DragGesture(minimumDistance: 0).onChanged { v in
                        frame = min(count, max(1, Int(v.location.x / max(1, w) * CGFloat(count)) + 1))
                    })
                }
                .frame(height: 16)
                HStack {
                    Text("Frame \(frame) of \(count)").monospacedDigit()
                    Spacer()
                    Text(Fmt.clock((Double(frame) - 0.5) * interval, precise: true)).monospacedDigit()
                    Text("/ \(Fmt.clock(capture.meta.duration ?? 0, precise: true))").monospacedDigit().foregroundStyle(Lib.faint)
                }
                .font(.system(size: 12))
                .foregroundStyle(Lib.dim)
            }
            .padding(.horizontal, 4)

            ScrollViewReader { proxy in
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 6) {
                        ForEach(1...count, id: \.self) { i in
                            Thumb(url: capture.frame(i))
                                .frame(width: 92, height: 58)
                                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous)
                                    .strokeBorder(i == frame ? Brand.accent : Color.white.opacity(0.08), lineWidth: i == frame ? 2 : 1))
                                .opacity(i == frame ? 1 : 0.75)
                                .onTapGesture { frame = i }
                                .id(i)
                        }
                    }
                    .padding(.vertical, 2)
                }
                .onChange(of: frame) { _, f in withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo(f, anchor: .center) } }
            }
        }
        .onKeyPress(.leftArrow) { frame = max(1, frame - 1); return .handled }
        .onKeyPress(.rightArrow) { frame = min(count, frame + 1); return .handled }
        .focusable()
        .focusEffectDisabled()
    }
}

struct ClickList: View {
    let capture: Capture
    @Binding var frame: Int
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(capture.meta.clicks.enumerated()), id: \.offset) { i, c in
                Button { if let f = c.frame { frame = f } } label: {
                    HStack(alignment: .firstTextBaseline, spacing: 12) {
                        Text("\(i + 1)")
                            .font(.system(size: 11, weight: .bold)).foregroundStyle(.white)
                            .frame(width: 20, height: 20)
                            .background(Circle().fill(Brand.record))
                        Text(Fmt.clock(c.time, precise: true)).monospacedDigit().foregroundStyle(Lib.faint).frame(width: 52, alignment: .leading)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(c.element?.headline ?? "Somewhere on screen").font(.system(size: 13, weight: .medium))
                            if let sel = c.element?.selector, !sel.isEmpty {
                                Text(sel).font(.system(size: 11.5, design: .monospaced)).foregroundStyle(Lib.dim)
                            }
                        }
                        Spacer()
                        if let f = c.frame { Text("frame \(f)").font(.system(size: 11)).monospacedDigit().foregroundStyle(Lib.faint) }
                    }
                    .padding(8)
                    .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(c.frame == frame ? Color.white.opacity(0.07) : .clear))
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .font(.system(size: 13))
    }
}

struct ElementFacts: View {
    let el: ElementInfo
    var body: some View {
        Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 18, verticalSpacing: 10) {
            row("Element", el.headline)
            row("Selector", el.selector, mono: true)
            row("Inside", el.path.joined(separator: " › "), mono: true)
            row("Page", el.url, mono: true)
            row("Size", el.frame.width > 0 ? "\(Int(el.frame.width)) × \(Int(el.frame.height)) pt" : "")
        }
        .font(.system(size: 13))
    }

    @ViewBuilder
    private func row(_ label: String, _ value: String, mono: Bool = false) -> some View {
        if !value.isEmpty {
            GridRow {
                Text(label).foregroundStyle(Lib.faint).gridColumnAlignment(.trailing)
                Text(value).font(mono ? .system(size: 12.5, design: .monospaced) : .system(size: 13)).textSelection(.enabled)
            }
        }
    }
}

struct HintList: View {
    let hints: [ElementInfo]
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(hints.enumerated()), id: \.offset) { _, el in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Image(systemName: Brand.icon(forRole: el.role)).foregroundStyle(Lib.faint).frame(width: 16)
                    Text(el.headline).lineLimit(1)
                    if !el.selector.isEmpty {
                        Text(el.selector).font(.system(size: 11.5, design: .monospaced)).foregroundStyle(Lib.dim).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                }
                .font(.system(size: 13))
            }
        }
    }
}

struct CopyButton: View {
    @EnvironmentObject var lib: Library
    let captures: [Capture]
    var body: some View {
        let copied = lib.copiedID == (captures.count == 1 ? captures[0].id : "many")
        Button { lib.copy(captures) } label: {
            Label(copied ? "Copied" : "Copy \(captures.count) references", systemImage: copied ? "checkmark" : "doc.on.doc")
                .font(.system(size: 14, weight: .semibold))
                .padding(.horizontal, 20).frame(height: 40)
                .background(Capsule().fill(Brand.accent))
                .foregroundStyle(.white)
                .contentShape(Capsule())
        }
        .buttonStyle(PressScale())
    }
}

struct MultiSelection: View {
    let captures: [Capture]
    var body: some View {
        VStack(spacing: 20) {
            HStack(spacing: -28) {
                ForEach(captures.prefix(4)) { c in
                    Thumb(url: c.thumbnail)
                        .frame(width: 150, height: 96)
                        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                        .shadow(color: .black.opacity(0.4), radius: 10, y: 4)
                }
            }
            Text("\(captures.count) captures selected").font(.system(size: 22, weight: .bold))
            Text("Their references go into one paste, oldest first.").foregroundStyle(Lib.dim)
            CopyButton(captures: captures.sorted { $0.meta.created < $1.meta.created })
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: Empty states

struct SetupPrompt: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "lock.shield").font(.system(size: 34, weight: .light)).foregroundStyle(Lib.dim)
            Text("Clipframes needs two permissions").font(.system(size: 22, weight: .bold))
            Text("Screen Recording and Accessibility. Setup takes a minute.").foregroundStyle(Lib.dim)
            BigButton(title: "Finish setup", icon: "arrow.right") { Onboarding.shared.show(at: .permissions) }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct Welcome: View {
    @EnvironmentObject var lib: Library

    var body: some View {
        VStack(spacing: 30) {
            VStack(spacing: 10) {
                LogoMark(size: 56).padding(.bottom, 8)
                Text(lib.captures.isEmpty ? "Your first capture shows up here" : "Pick a capture")
                    .font(Brand.display(26, .bold))
                Text("Or start one. Each capture copies a line for Claude Code or Codex.")
                    .font(.system(size: 15)).foregroundStyle(Lib.dim)
            }
            VStack(spacing: 8) {
                Button { OverlaySession.toggle(.element, from: .library) } label: {
                    ToolLine(icon: "cursorarrow.rays", title: "Element", text: "Click one thing", keys: Hotkey.element, highlighted: true)
                }.buttonStyle(PressScale())
                Button { OverlaySession.toggle(.screenshot, from: .library) } label: {
                    ToolLine(icon: "viewfinder", title: "Screenshot", text: "Drag an area", keys: Hotkey.screenshot)
                }.buttonStyle(PressScale())
                Button { OverlaySession.toggle(.clip, from: .library) } label: {
                    ToolLine(icon: "record.circle", title: "Clip", text: "Record an area", keys: Hotkey.clip)
                }.buttonStyle(PressScale())
            }
            .frame(maxWidth: 460)
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: Menu bar

struct MenuLabel: View {
    @EnvironmentObject var menu: MenuState
    var body: some View {
        if menu.recording {
            Text("\(Image(systemName: "record.circle.fill")) \(Fmt.clock(menu.elapsed))")
        } else {
            Image(systemName: "viewfinder")
        }
    }
}

struct MenuContent: View {
    @EnvironmentObject var lib: Library
    @EnvironmentObject var menu: MenuState

    var body: some View {
        Button("Capture an element  \(Hotkey.element)") { OverlaySession.toggle(.element, from: .library) }
        Button("Take a screenshot  \(Hotkey.screenshot)") { OverlaySession.toggle(.screenshot, from: .library) }
        Button(menu.recording ? "Stop recording  \(Hotkey.clip)" : "Record a clip  \(Hotkey.clip)") { OverlaySession.toggle(.clip, from: .library) }

        if let last = lib.captures.first {
            Divider()
            Button("Copy last reference") { lib.copy([last], toast: true) }
            Section("Recent") {
                ForEach(lib.captures.prefix(5)) { c in
                    Button("\(c.title) · \(c.subtitle)") { lib.copy([c], toast: true) }
                }
            }
        }
        Divider()
        Button("Show the bar") { Bar.show() }
        Button("Open the library") { LibraryWindow.show() }
        SettingsLink { Text("Settings…") }
        CheckForUpdatesButton()
        Divider()
        Button("Quit Clipframes") { NSApp.terminate(nil) }
    }
}

// MARK: Settings

struct SettingsView: View {
    @EnvironmentObject var lib: Library
    @AppStorage(Pref.fps) private var fps = 2.0
    @AppStorage(Pref.maxFrames) private var maxFrames = 20
    @AppStorage(Pref.width) private var width = 1280
    @AppStorage(Pref.showClicks) private var showClicks = true
    @AppStorage(Pref.readWebApps) private var readWebApps = true
    @AppStorage(Pref.showBarAtLaunch) private var showBarAtLaunch = true
    @AppStorage(Pref.showInDock) private var showInDock = true
    @State private var openAtLogin = SMAppService.mainApp.status == .enabled

    var body: some View {
        Form {
            Section("Recordings") {
                Picker("Frames per second", selection: $fps) {
                    Text("1").tag(1.0); Text("2").tag(2.0); Text("4").tag(4.0); Text("8").tag(8.0)
                }
                Picker("At most", selection: $maxFrames) {
                    Text("10 frames").tag(10); Text("20 frames").tag(20); Text("30 frames").tag(30); Text("50 frames").tag(50)
                }
                Picker("Frame width", selection: $width) {
                    Text("960 px").tag(960); Text("1280 px").tag(1280); Text("1600 px").tag(1600); Text("Full size").tag(0)
                }
                Toggle("Show clicks in the video", isOn: $showClicks)
                Text("Longer recordings spread the frames out instead of going over the limit.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            Section("Naming elements") {
                Toggle("Read Chrome and Electron apps", isOn: $readWebApps)
                Text("Asks Chrome, Slack, Cursor and similar apps for their page structure, the way a screen reader does. Cursor and VS Code may ask once whether to turn on screen reader mode; choose No.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            Section("Shortcuts") {
                LabeledContent("Capture an element", value: Hotkey.element)
                LabeledContent("Take a screenshot", value: Hotkey.screenshot)
                LabeledContent("Record a clip, stop", value: Hotkey.clip)
            }
            Section("Staying out of the way") {
                Toggle("Show the bar when Clipframes opens", isOn: $showBarAtLaunch)
                Toggle("Show Clipframes in the Dock", isOn: $showInDock)
                    .onChange(of: showInDock) { _, on in
                        NSApp.setActivationPolicy(on ? .regular : .accessory)
                        if !on { Bar.show() }
                    }
                Text("The shortcuts always work. Without the bar and the Dock icon, Clipframes lives in the menu bar.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            Section("This week") {
                let week = Habit.recent(days: 7)
                LabeledContent("Captures", value: "\(week.captures)")
                LabeledContent("Days you used it", value: "\(week.days) of 7")
                LabeledContent("Started with a shortcut", value: "\(Int((Habit.shortcutShare * 100).rounded()))%")
            }
            Section {
                Toggle("Open at login", isOn: $openAtLogin)
                    .onChange(of: openAtLogin) { _, on in
                        do { if on { try SMAppService.mainApp.register() } else { try SMAppService.mainApp.unregister() } }
                        catch { openAtLogin = SMAppService.mainApp.status == .enabled }
                    }
                LabeledContent("Captures are saved in") {
                    Button("~/Clipframes") { NSWorkspace.shared.open(lib.root) }.buttonStyle(.link)
                }
            }
            UpdatesSection()
        }
        .formStyle(.grouped)
        .frame(width: 460)
        .fixedSize(horizontal: false, vertical: true)
    }
}
