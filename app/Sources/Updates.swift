import Sparkle
import SwiftUI

/// Sparkle, checking the appcast in the repo (SUFeedURL in Info.plist).
/// Sparkle asks once, on the second launch, whether to check automatically.
@MainActor
final class Updates: ObservableObject {
    static let shared = Updates()
    private let controller = SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
    @Published private(set) var canCheck = false

    private init() {
        controller.updater.publisher(for: \.canCheckForUpdates).assign(to: &$canCheck)
    }

    var updater: SPUUpdater { controller.updater }

    func check() { controller.checkForUpdates(nil) }

    static var version: String {
        let info = Bundle.main.infoDictionary ?? [:]
        let short = info["CFBundleShortVersionString"] as? String ?? "?"
        let build = info["CFBundleVersion"] as? String ?? "?"
        return "\(short) (\(build))"
    }
}

struct CheckForUpdatesButton: View {
    @ObservedObject var updates = Updates.shared
    var body: some View {
        Button("Check for updates…") { updates.check() }.disabled(!updates.canCheck)
    }
}

struct UpdatesSection: View {
    private let updater = Updates.shared.updater
    @State private var automatic = Updates.shared.updater.automaticallyChecksForUpdates
    @State private var download = Updates.shared.updater.automaticallyDownloadsUpdates

    var body: some View {
        Section("Updates") {
            Toggle("Check for updates automatically", isOn: $automatic)
                .onChange(of: automatic) { _, on in updater.automaticallyChecksForUpdates = on }
            Toggle("Download and install them automatically", isOn: $download)
                .disabled(!automatic)
                .onChange(of: download) { _, on in updater.automaticallyDownloadsUpdates = on }
            LabeledContent("Version", value: Updates.version)
            HStack { Spacer(); CheckForUpdatesButton() }
        }
    }
}
