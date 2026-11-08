import AppKit
import PurrloaderCore
import UserNotifications

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        UNUserNotificationCenter.current().delegate = self
        Task {
            await ToolManager.shared.bootstrap()
            DownloadManager.shared.schedule()
        }
        Task {
            AppUpdater.shared.refresh()
            // Long sessions still hear about new versions; checkIfDue asks GitHub at most once a day.
            while !Task.isCancelled {
                await AppUpdater.shared.checkIfDue()
                try? await Task.sleep(for: .seconds(6 * 3600))
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        let manager = DownloadManager.shared
        let pending = manager.activeCount + manager.queuedCount
        if pending > 0 {
            let alert = NSAlert()
            alert.messageText =
                pending == 1 ? "A download is still in progress" : "\(pending) downloads are still in progress"
            alert.informativeText =
                "If you quit, Purrloader pauses \(pending == 1 ? "it" : "them") and picks up where it left off next time you open it."
            alert.addButton(withTitle: "Quit")
            alert.addButton(withTitle: "Keep Downloading")
            guard alert.runModal() == .alertFirstButtonReturn else { return .terminateCancel }
        }
        manager.stopAllForQuit()
        return .terminateNow
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .sound]
    }

    /// Clicking a "Downloaded" notification shows the file in Finder.
    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse
    ) async {
        let path = response.notification.request.content.userInfo["file"] as? String
        await MainActor.run {
            if let path, FileManager.default.fileExists(atPath: path) {
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
            } else {
                NSApp.activate()
            }
        }
    }
}
