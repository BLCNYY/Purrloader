import PurrloaderCore
import SwiftUI

@main
struct PurrloaderApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var model = MainModel()
    @AppStorage("showMenuBarIcon") private var showMenuBarIcon = true

    var body: some Scene {
        Window("Purrloader", id: "main") {
            MainView().environment(model).purrloaderWindowStyle()
        }
        .defaultSize(width: 780, height: 660)
        .windowResizability(.contentMinSize)
        .windowStyle(.hiddenTitleBar)
        .windowBackgroundDragBehavior(.enabled)
        .commands { PurrloaderCommands(model: model) }

        Window("History", id: "history") {
            HistoryView().purrloaderWindowStyle()
        }
        .defaultSize(width: 760, height: 520)
        .keyboardShortcut("y", modifiers: .command)

        Settings {
            SettingsView().environment(model).purrloaderWindowStyle()
        }

        MenuBarExtra(isInserted: $showMenuBarIcon) {
            MenuBarView()
        } label: {
            MenuBarLabel()
        }
        .menuBarExtraStyle(.window)
    }
}

struct PurrloaderCommands: Commands {
    let model: MainModel
    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings

    var body: some Commands {
        CommandGroup(replacing: .appInfo) {
            Button("About Purrloader") {
                SettingsTab.select(.about)
                openSettings()
            }
        }
        CommandGroup(replacing: .appSettings) {
            // Always General, rather than whichever tab Advanced Options… or About last left it on.
            Button("Settings…") {
                SettingsTab.select(.general)
                openSettings()
            }
            .keyboardShortcut(",", modifiers: .command)
        }
        CommandGroup(after: .newItem) {
            Button("Paste Link and Get Info") {
                openWindow(id: "main")
                model.pasteAndFetch()
            }
            .keyboardShortcut("v", modifiers: [.command, .shift])
            Button("Download") { model.download() }
                .keyboardShortcut(.return, modifiers: .command)
        }
        CommandMenu("Downloads") {
            Button("Advanced Options…") {
                SettingsTab.select(.advanced)
                openSettings()
            }
            .keyboardShortcut("o", modifiers: [.command, .option])
            Divider()
            Button("Pause All") { DownloadManager.shared.pauseAll() }
                .keyboardShortcut("p", modifiers: [.command, .option])
            Button("Resume All") { DownloadManager.shared.resumeAll() }
                .keyboardShortcut("r", modifiers: [.command, .option])
            Button("Clear Recent") { DownloadManager.shared.clearRecent() }
                .keyboardShortcut("k", modifiers: .command)
            Divider()
            Button("Update Helper Tools") { Task { await ToolManager.shared.updateYtdlp() } }
                .keyboardShortcut("u", modifiers: [.command, .shift])
        }
    }
}
