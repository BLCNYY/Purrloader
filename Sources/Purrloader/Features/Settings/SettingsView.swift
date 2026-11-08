import PurrloaderCore
import SwiftUI

enum SettingsTab: String {
    case general, advanced, about

    static let storageKey = "settingsTab"

    /// Switches the Settings window to a tab, e.g. right before opening it.
    static func select(_ tab: SettingsTab) {
        UserDefaults.standard.set(tab.rawValue, forKey: storageKey)
    }
}

struct SettingsView: View {
    @AppStorage(SettingsTab.storageKey) private var tab: SettingsTab = .general

    var body: some View {
        TabView(selection: $tab) {
            GeneralSettings().clipsBelowToolbar()
                .tabItem { Label("General", systemImage: "gearshape") }
                .tag(SettingsTab.general)
            AdvancedSettings().clipsBelowToolbar()
                .tabItem { Label("Advanced", systemImage: "slider.horizontal.3") }
                .tag(SettingsTab.advanced)
            AboutSettings().clipsBelowToolbar()
                .tabItem { Label("About", systemImage: "info.circle") }
                .tag(SettingsTab.about)
        }
        .background(KeepsWindowCentered())
    }
}

extension View {
    /// The window's toolbar floats over its content and is see-through here, so a form scrolled up showed
    /// its rows behind the tab buttons. This cuts the content off at the bottom edge of the toolbar instead.
    fileprivate func clipsBelowToolbar() -> some View {
        mask {
            GeometryReader { geo in
                Rectangle().padding(.top, geo.safeAreaInsets.top)
            }
            .ignoresSafeArea()
        }
    }
}

/// Opens the Settings window in the middle of the screen, and keeps it there: tabs have different sizes, and
/// AppKit would otherwise grow the window from its top-left corner, pushing the bigger Advanced tab off centre.
private struct KeepsWindowCentered: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView { Tracker() }
    func updateNSView(_ nsView: NSView, context: Context) {}

    final class Tracker: NSView {
        private var lastFrame: NSRect?
        private var observers: [NSObjectProtocol] = []
        private var needsCentering = true

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            observers.forEach(NotificationCenter.default.removeObserver)
            observers = []
            guard let window else { return }
            watch(NSWindow.didResizeNotification, of: window) { $0.keepCenter() }
            // SwiftUI keeps the Settings window around when it's closed; centre it again next time it opens.
            watch(NSWindow.willCloseNotification, of: window) { $0.needsCentering = true }
            watch(NSWindow.didBecomeKeyNotification, of: window) { $0.centerIfNeeded() }
            DispatchQueue.main.async { [weak self] in self?.centerIfNeeded() }
        }

        private func watch(
            _ name: Notification.Name, of window: NSWindow, _ handler: @escaping @MainActor (Tracker) -> Void
        ) {
            observers.append(
                NotificationCenter.default.addObserver(forName: name, object: window, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { if let self { handler(self) } }
                })
        }

        private func centerIfNeeded() {
            guard needsCentering, let window, let screen = window.screen ?? NSScreen.main else { return }
            needsCentering = false
            let area = screen.visibleFrame
            window.setFrameOrigin(
                NSPoint(
                    x: (area.midX - window.frame.width / 2).rounded(),
                    y: (area.midY - window.frame.height / 2).rounded()))
            lastFrame = window.frame
        }

        private func keepCenter() {
            guard let window else { return }
            let frame = window.frame
            if let last = lastFrame, abs(frame.width - last.width) > 0.5 || abs(frame.height - last.height) > 0.5 {
                var origin = NSPoint(x: last.midX - frame.width / 2, y: last.midY - frame.height / 2)
                if let area = window.screen?.visibleFrame {
                    origin.x = min(max(origin.x, area.minX), area.maxX - frame.width)
                    origin.y = min(max(origin.y, area.minY), area.maxY - frame.height)
                }
                window.setFrameOrigin(NSPoint(x: origin.x.rounded(), y: origin.y.rounded()))
            }
            lastFrame = window.frame
        }
    }
}

private struct GeneralSettings: View {
    @Environment(MainModel.self) private var model
    @AppStorage("clipboardAutofill") private var clipboardAutofill = true
    @AppStorage("notifications") private var notifications = true
    @AppStorage("showMenuBarIcon") private var showMenuBarIcon = true

    var body: some View {
        @Bindable var manager = DownloadManager.shared
        Form {
            Section {
                LabeledContent("Save downloads to") {
                    HStack {
                        Text((model.folder.path as NSString).abbreviatingWithTildeInPath)
                            .lineLimit(1).truncationMode(.middle).foregroundStyle(.secondary)
                        Button("Change…") { model.chooseFolder() }
                    }
                }
                Stepper("Download \(manager.maxConcurrent) at a time", value: $manager.maxConcurrent, in: 1...10)
            }
            Section {
                Toggle("Fill in links from the clipboard", isOn: $clipboardAutofill)
                Toggle("Notify me when downloads finish or fail", isOn: $notifications)
                Toggle("Show Purrloader in the menu bar", isOn: $showMenuBarIcon)
            }
        }
        .formStyle(.grouped)
        .frame(width: 520, height: 300)
    }
}
