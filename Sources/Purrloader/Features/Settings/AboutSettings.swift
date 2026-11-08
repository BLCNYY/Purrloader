import PurrloaderCore
import SwiftUI

struct AboutSettings: View {
    private var tools: ToolManager { .shared }

    var body: some View {
        @Bindable var tools = ToolManager.shared
        Form {
            Section {
                HStack(spacing: 14) {
                    Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: 64, height: 64)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Purrloader").font(.system(size: 17, weight: .semibold))
                        Text("The quiet way to save videos.").foregroundStyle(.secondary)
                        Text("Version \(AppInfo.version)").font(.system(size: 11)).foregroundStyle(.tertiary)
                    }
                }
                .padding(.vertical, 4)
            }
            Section {
                ForEach(Tool.allCases) { tool in
                    LabeledContent(tool.rawValue) {
                        if tools.transfers[tool] != nil {
                            ProgressView().controlSize(.small)
                        } else {
                            Text(tools.versions[tool] ?? "Not installed")
                                .foregroundStyle(tools.versions[tool] == nil ? .red : .secondary)
                                .textSelection(.enabled)
                        }
                    }
                }
                Toggle("Keep yt-dlp up to date automatically", isOn: $tools.autoUpdate)
                Picker("yt-dlp channel", selection: $tools.channel) {
                    ForEach(UpdateChannel.allCases) { Text($0.title).tag($0) }
                }
                HStack {
                    Button(tools.isUpdating ? "Updating…" : "Update Now") { Task { await tools.updateYtdlp() } }
                        .disabled(tools.isUpdating || tools.isInstalling)
                    Button("Reinstall Tools") { Task { await tools.reinstallAll() } }.disabled(tools.isInstalling)
                    Spacer()
                    Button("Show Tools Folder") { NSWorkspace.shared.open(AppPaths.bin) }
                }
                if !tools.updateLog.isEmpty {
                    Text(tools.updateLog).font(.caption).foregroundStyle(.secondary).textSelection(.enabled).lineLimit(
                        4)
                }
            } header: {
                Text("Helper tools")
            } footer: {
                Text(
                    "Purrloader keeps yt-dlp up to date by itself. Nightly gets fixes for broken sites a few days before stable."
                )
            }
            Section {
                Link("Purrloader on GitHub", destination: AppInfo.repositoryURL)
                Link("License (MIT)", destination: AppInfo.fileURL("LICENSE"))
                Link("Third-party notices", destination: AppInfo.fileURL("THIRD_PARTY_NOTICES.md"))
            } footer: {
                Text("Built on yt-dlp, FFmpeg and Deno. Sponsor segments come from the SponsorBlock community.")
            }
        }
        .formStyle(.grouped)
        .scrollBounceBehavior(.basedOnSize)
        .frame(width: 520, height: 680)
    }
}
