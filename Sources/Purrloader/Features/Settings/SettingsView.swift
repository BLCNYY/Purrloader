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
            GeneralSettings()
                .tabItem { Label("General", systemImage: "gearshape") }
                .tag(SettingsTab.general)
            AdvancedSettings()
                .tabItem { Label("Advanced", systemImage: "slider.horizontal.3") }
                .tag(SettingsTab.advanced)
            AboutSettings()
                .tabItem { Label("About", systemImage: "info.circle") }
                .tag(SettingsTab.about)
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
