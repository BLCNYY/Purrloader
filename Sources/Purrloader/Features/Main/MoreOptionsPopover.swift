import PurrloaderCore
import SwiftUI

/// The few extras a normal download might need. Everything else lives in Settings > Advanced.
struct MoreOptionsPopover: View {
    @Environment(MainModel.self) private var model
    @Environment(\.openSettings) private var openSettings
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        @Bindable var model = model
        VStack(alignment: .leading, spacing: 0) {
            section {
                switchRow("Subtitles", isOn: $model.subtitlesOn)
                if model.subtitlesOn {
                    Picker("Language", selection: $model.subtitleSelection) {
                        ForEach(model.subtitleChoices) { Text($0.title).tag($0.code) }
                    }
                    note(model.options.format.mode == .video ? "Added to the video." : "Saved next to the audio file.")
                }
            }
            section {
                switchRow("Skip sponsor segments", isOn: $model.skipSponsors)
                note(
                    "Cuts out sponsor spots, self-promotion and reminders to subscribe. Works on supported sites only.")
            }
            section {
                switchRow("Download only a part", isOn: $model.downloadPart)
                if model.downloadPart {
                    HStack(spacing: 8) {
                        TextField("Start", text: $model.options.extras.sectionStart, prompt: Text("0:00"))
                        Text("to").foregroundStyle(Theme.secondary)
                        TextField("End", text: $model.options.extras.sectionEnd, prompt: Text("End"))
                    }
                    .textFieldStyle(.roundedBorder)
                    .labelsHidden()
                    note("Times like 1:30 or 1:02:45.")
                }
            }
            section {
                Picker("Sign in with my browser", selection: $model.options.network.cookieBrowser) {
                    Text("Don't sign in").tag(CookieBrowser.none)
                    Divider()
                    ForEach(InstalledBrowsers.list(including: model.options.network.cookieBrowser)) {
                        Text($0.title).tag($0)
                    }
                }
                .onChange(of: model.options.network.cookieBrowser) {
                    if model.info != nil || model.fetchError != nil { model.refetch() }
                }
                note("For private, age-restricted or members-only content you can watch in that browser.")
            }
            Button("Advanced Options…") {
                dismiss()
                SettingsTab.select(.advanced)
                openSettings()
            }
            .buttonStyle(.link)
            .padding(14)
        }
        .toggleStyle(.switch)
        .controlSize(.small)
        .font(.system(size: 13))
        .frame(width: 330)
    }

    private func section<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8, content: content)
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .bottom) { Rectangle().fill(Theme.stroke).frame(height: 1) }
    }

    private func switchRow(_ title: String, isOn: Binding<Bool>) -> some View {
        Toggle(isOn: isOn) {
            Text(title).frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func note(_ text: String) -> some View {
        Text(text).font(.system(size: 11)).foregroundStyle(Theme.secondary).fixedSize(horizontal: false, vertical: true)
    }
}
