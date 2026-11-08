import Foundation
import Observation
import PurrloaderCore

/// Looks for a newer Purrloader on GitHub Releases at most once a day.
@MainActor @Observable
final class AppUpdater {
    static let shared = AppUpdater()

    private(set) var available: AppRelease?

    private enum Key {
        static let lastCheck = "lastAppUpdateCheck"
        static let latestVersion = "latestAppVersion"
        static let latestPage = "latestAppPage"
        static let dismissed = "dismissedAppVersion"
    }

    func checkIfDue() async {
        let defaults = UserDefaults.standard
        if let last = defaults.object(forKey: Key.lastCheck) as? Date, Date.now.timeIntervalSince(last) < 24 * 3600 {
            return
        }
        guard let release = await AppRelease.latest() else { return }
        defaults.set(Date.now, forKey: Key.lastCheck)
        defaults.set(release.version, forKey: Key.latestVersion)
        defaults.set(release.page.absoluteString, forKey: Key.latestPage)
        refresh()
    }

    /// Shows the last release found, unless it isn't newer or was dismissed.
    func refresh() {
        let defaults = UserDefaults.standard
        guard let version = defaults.string(forKey: Key.latestVersion),
            let page = defaults.string(forKey: Key.latestPage).flatMap(URL.init(string:)),
            AppRelease.isNewer(version, than: AppInfo.version),
            defaults.string(forKey: Key.dismissed) != version
        else {
            available = nil
            return
        }
        available = AppRelease(version: version, page: page)
    }

    func dismiss() {
        UserDefaults.standard.set(available?.version, forKey: Key.dismissed)
        available = nil
    }
}
