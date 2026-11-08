import PurrloaderCore
import SwiftUI

/// One quiet line above the link field, only when something needs attention.
struct NoticeBanner: View {
    private var tools: ToolManager { .shared }
    private var updater: AppUpdater { .shared }

    var body: some View {
        Group {
            if !tools.isReady && tools.setupError == nil {
                banner {
                    ProgressView().controlSize(.mini)
                    Text("Getting Purrloader ready…")
                    Spacer()
                    if let progress = tools.installProgress, progress.expected > 0 {
                        Text("\(Format.bytes(progress.written)) of \(Format.bytes(progress.expected))")
                            .monospacedDigit()
                    }
                }
            } else if let error = tools.setupError {
                banner {
                    Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.warning)
                    Text("Couldn't download Purrloader's helper tools. Check your internet connection.").lineLimit(2)
                        .help(error)
                    Spacer()
                    Button("Retry") {
                        Task {
                            await tools.bootstrap(); DownloadManager.shared.schedule()
                        }
                    }
                    .buttonStyle(SecondaryCapsuleStyle())
                }
            } else if let error = tools.updateError {
                banner {
                    Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.warning)
                    Text("Couldn't update Purrloader's helper tools. Downloads still work; some sites may not.")
                        .lineLimit(2)
                        .help(error)
                    Spacer()
                    if tools.isUpdating {
                        ProgressView().controlSize(.small)
                    } else {
                        Button("Try Again") { Task { await tools.updateYtdlp() } }.buttonStyle(SecondaryCapsuleStyle())
                    }
                    dismissButton { tools.updateError = nil }
                }
            } else if let release = updater.available {
                banner {
                    Image(systemName: "sparkles").foregroundStyle(Theme.secondary)
                    Text("Purrloader \(release.version) is available.").foregroundStyle(Theme.ink.opacity(0.9))
                    Spacer()
                    Link("View", destination: release.page).buttonStyle(SecondaryCapsuleStyle())
                    dismissButton { updater.dismiss() }
                }
            }
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 12)
    }

    private func banner<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        HStack(spacing: 8, content: content)
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(Theme.secondary)
            .padding(.horizontal, 14)
            .frame(minHeight: 40)
            .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.ink.opacity(0.05)))
    }

    private func dismissButton(_ action: @escaping () -> Void) -> some View {
        Button(action: action) { Image(systemName: "xmark") }
            .buttonStyle(CircleButtonStyle(size: 22, filled: false))
            .help("Dismiss")
            .accessibilityLabel("Dismiss")
    }
}
