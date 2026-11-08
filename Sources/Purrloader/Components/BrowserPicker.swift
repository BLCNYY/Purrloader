import PurrloaderCore
import SwiftUI

/// Browsers whose sign-in yt-dlp can read, limited to the ones installed on this Mac.
@MainActor
enum InstalledBrowsers {
    private static let installed: [CookieBrowser] = {
        let found = CookieBrowser.allCases.filter { browser in
            browser.bundleIdentifier.map { NSWorkspace.shared.urlForApplication(withBundleIdentifier: $0) != nil }
                ?? false
        }
        return found.isEmpty ? CookieBrowser.allCases.filter { $0 != .none } : found
    }()

    static func list(including current: CookieBrowser) -> [CookieBrowser] {
        current == .none || installed.contains(current) ? installed : installed + [current]
    }
}

/// The list behind "Use Browser Sign-In": pick a browser and the download retries with its sign-in.
struct BrowserPicker: View {
    let current: CookieBrowser
    let choose: (CookieBrowser) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Sign in with my browser").font(.system(size: 13, weight: .semibold))
            Text("Sign in to the site in one of these browsers first. Purrloader then uses that sign-in.")
                .font(.system(size: 11)).foregroundStyle(Theme.secondary).fixedSize(horizontal: false, vertical: true)
            VStack(spacing: 2) {
                ForEach(InstalledBrowsers.list(including: current)) { browser in
                    Button {
                        choose(browser)
                    } label: {
                        HStack {
                            Text(browser.title)
                            Spacer()
                            if browser == current { Image(systemName: "checkmark").foregroundStyle(Theme.secondary) }
                        }
                        .font(.system(size: 13))
                        .padding(.horizontal, 10).frame(height: 28)
                        .background(RoundedRectangle(cornerRadius: 7).fill(Color.white.opacity(0.05)))
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
            Text("Safari needs Full Disk Access for Purrloader in System Settings → Privacy & Security.")
                .font(.system(size: 11)).foregroundStyle(Theme.tertiary).fixedSize(horizontal: false, vertical: true)
        }
        .padding(14)
        .frame(width: 280)
        .preferredColorScheme(.dark)
    }
}
