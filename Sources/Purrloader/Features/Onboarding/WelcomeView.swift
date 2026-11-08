import PurrloaderCore
import SwiftUI

/// Shown once, on first launch, while the helper tools download.
struct WelcomeView: View {
    static let seenKey = "didShowWelcome"

    @Environment(\.dismiss) private var dismiss
    private var tools: ToolManager { .shared }

    var body: some View {
        VStack(spacing: 18) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 96, height: 96)
                .accessibilityHidden(true)
            VStack(spacing: 6) {
                Text("Welcome to Purrloader").font(.system(size: 20, weight: .semibold))
                Text("Paste a link to a video and Purrloader saves it, or just its audio, to your Mac.")
                    .font(.system(size: 13)).foregroundStyle(Theme.secondary)
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }

            toolStatus
                .frame(minHeight: 44)

            Text("Only download content you own or have permission to download.")
                .font(.system(size: 11)).foregroundStyle(Theme.tertiary)
                .multilineTextAlignment(.center)

            Button("Get Started") {
                UserDefaults.standard.set(true, forKey: Self.seenKey)
                dismiss()
            }
            .buttonStyle(PrimaryCapsuleStyle())
            .keyboardShortcut(.defaultAction)
        }
        .padding(.horizontal, 36)
        .padding(.vertical, 30)
        .frame(width: 440)
    }

    @ViewBuilder private var toolStatus: some View {
        if let error = tools.setupError {
            VStack(spacing: 8) {
                Label(
                    "Couldn't download the helper tools. Check your internet connection.",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.warning)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .help(error)
                Button("Try Again") {
                    Task {
                        await tools.bootstrap(); DownloadManager.shared.schedule()
                    }
                }
                .buttonStyle(SecondaryCapsuleStyle())
            }
        } else if tools.isReady {
            Label("Ready to go.", systemImage: "checkmark.circle.fill")
                .font(.system(size: 12, weight: .medium)).foregroundStyle(.green)
        } else {
            let progress = tools.installProgress
            VStack(alignment: .leading, spacing: 6) {
                ProgressView(value: Double(progress?.written ?? 0), total: Double(max(progress?.expected ?? 0, 1)))
                    .progressViewStyle(.linear)
                    .tint(.white)
                Text(caption(progress)).font(.system(size: 11)).foregroundStyle(Theme.secondary).monospacedDigit()
            }
        }
    }

    private func caption(_ progress: ToolManager.Transfer?) -> String {
        guard let progress, progress.expected > 0 else { return "Downloading free helper tools…" }
        return "Downloading free helper tools: \(Format.bytes(progress.written)) of \(Format.bytes(progress.expected))"
    }
}
