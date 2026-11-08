import AppKit
import PurrloaderCore
import SwiftUI

struct MenuBarLabel: View {
    private var manager: DownloadManager { .shared }

    var body: some View {
        Image(nsImage: MenuBarIcon.image)
        if manager.activeCount > 0 {
            Text(manager.overallFraction.map { "\(Int($0 * 100))%" } ?? "\(manager.activeCount)")
        }
    }
}

struct MenuBarView: View {
    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings
    @Environment(\.dismiss) private var dismiss
    private var manager: DownloadManager { .shared }

    var body: some View {
        let visible = Array(manager.jobs.filter { !$0.status.isDone }.prefix(5))
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                Text("Purrloader").font(.system(size: 13, weight: .semibold))
                Spacer()
                Text(summary).font(.system(size: 12)).foregroundStyle(Theme.secondary)
            }
            .padding(.horizontal, 14)
            .padding(.top, 12)
            .padding(.bottom, visible.isEmpty ? 10 : 12)

            if !visible.isEmpty {
                VStack(spacing: 10) {
                    ForEach(visible) { job in MenuBarJobRow(job: job) }
                }
                .padding(.horizontal, 14)
                .padding(.bottom, 12)
            }

            MenuDivider()
            VStack(spacing: 0) {
                MenuRow("Open Purrloader") {
                    openWindow(id: "main")
                    NSApp.activate()
                    dismiss()
                }
                if manager.activeCount > 0 || manager.queuedCount > 0 {
                    MenuRow("Pause All Downloads", shortcut: "⌥⌘P") { manager.pauseAll() }
                } else if manager.jobs.contains(where: { $0.status == .paused }) {
                    MenuRow("Resume All Downloads", shortcut: "⌥⌘R") { manager.resumeAll() }
                }
            }
            .padding(5)
            MenuDivider()
            VStack(spacing: 0) {
                MenuRow("Settings…", shortcut: "⌘,") {
                    SettingsTab.select(.general)
                    openSettings()
                    NSApp.activate()
                    dismiss()
                }
                MenuRow("Quit Purrloader", shortcut: "⌘Q") { NSApp.terminate(nil) }
            }
            .padding(5)
        }
        .frame(width: 300)
        .preferredColorScheme(.dark)
    }

    private var summary: String {
        let active = manager.activeCount, queued = manager.queuedCount
        if active == 0 && queued == 0 {
            return manager.jobs.contains(where: { $0.status == .paused }) ? "Paused" : "Nothing downloading"
        }
        return queued > 0 ? "\(active) active · \(queued) waiting" : "\(active) active"
    }
}

private struct MenuBarJobRow: View {
    let job: DownloadJob

    var body: some View {
        HStack(spacing: 10) {
            Thumbnail(url: job.thumbnail, width: 48)
            VStack(alignment: .leading, spacing: 4) {
                Text(job.title).font(.system(size: 12, weight: .medium)).lineLimit(1).truncationMode(.middle)
                if job.status.isActive { SlimProgress(fraction: job.fraction) }
                HStack(spacing: 4) {
                    Text(job.statusText).lineLimit(1)
                    Spacer(minLength: 4)
                    if job.status.isActive, let fraction = job.fraction {
                        Text("\(Int(fraction * 100))%").monospacedDigit()
                    }
                }
                .font(.system(size: 11))
                .foregroundStyle(Theme.secondary)
            }
        }
    }
}

/// A row like a menu item: the title, an optional shortcut hint, and the highlight on hover.
private struct MenuRow: View {
    let title: String
    var shortcut: String?
    let action: () -> Void
    @State private var hovering = false

    init(_ title: String, shortcut: String? = nil, action: @escaping () -> Void) {
        self.title = title
        self.shortcut = shortcut
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack {
                Text(title)
                Spacer()
                if let shortcut {
                    Text(shortcut).foregroundStyle(hovering ? .white.opacity(0.75) : Theme.tertiary)
                }
            }
            .font(.system(size: 13))
            .padding(.horizontal, 9)
            .frame(height: 24)
            .contentShape(Rectangle())
            .background(
                RoundedRectangle(cornerRadius: 6, style: .continuous).fill(.white.opacity(hovering ? 0.1 : 0))
            )
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}

private struct MenuDivider: View {
    var body: some View {
        Rectangle().fill(.white.opacity(0.1)).frame(height: 1).padding(.horizontal, 14)
    }
}

/// The app icon's cat as an outline, for the menu bar. A template image, so macOS tints it
/// for light and dark menu bars. Geometry matches the SVG layers in Resources/Purrloader.icon
/// (1024-unit canvas, here with y pointing up);
/// the whiskers are left out because they shrink to specks at menu bar size.
@MainActor
enum MenuBarIcon {
    static let image: NSImage = {
        let image = NSImage(size: size, flipped: false) { rect in
            guard let ctx = NSGraphicsContext.current?.cgContext else { return false }
            draw(in: ctx, rect: rect)
            return true
        }
        image.isTemplate = true
        image.accessibilityDescription = "Purrloader"
        return image
    }()

    /// The cat's bounds in icon units, with room for the outline stroke.
    private static let canvas = CGRect(x: 230, y: 234, width: 564, height: 580)
    private static let height: CGFloat = 16
    static let size = NSSize(width: (height * canvas.width / canvas.height).rounded(), height: height)

    static func draw(in ctx: CGContext, rect: CGRect) {
        let scale = rect.height / canvas.height
        ctx.translateBy(x: (rect.width - canvas.width * scale) / 2, y: 0)
        ctx.scaleBy(x: scale, y: scale)
        ctx.translateBy(x: -canvas.minX, y: -canvas.minY)

        let line = 1.4 / scale  // 1.4pt, like a regular-weight SF Symbol
        ctx.setStrokeColor(.black)
        ctx.setFillColor(.black)
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        ctx.setLineWidth(line)

        let head = silhouette
        ctx.addPath(head)
        ctx.strokePath()

        // Eyes stay solid dots: hollow rings blur into blobs at this size.
        for x in [424.0, 600.0] {
            ctx.fillEllipse(in: CGRect(x: x - 30, y: 512, width: 60, height: 60))
        }

        // Download arrow.
        ctx.move(to: CGPoint(x: 512, y: 520)); ctx.addLine(to: CGPoint(x: 512, y: 382))
        ctx.move(to: CGPoint(x: 456, y: 438)); ctx.addLine(to: CGPoint(x: 512, y: 382));
        ctx.addLine(to: CGPoint(x: 568, y: 438))
        ctx.strokePath()
    }

    /// Head and ears merged into one shape, with the same rounding as the app icon.
    private static let silhouette: CGPath = {
        let face = CGPath(
            roundedRect: CGRect(x: 282, y: 286, width: 460, height: 360), cornerWidth: 180, cornerHeight: 170,
            transform: nil)
        let ears = [
            [CGPoint(x: 300, y: 520), CGPoint(x: 336, y: 762), CGPoint(x: 486, y: 628)],
            [CGPoint(x: 724, y: 520), CGPoint(x: 688, y: 762), CGPoint(x: 538, y: 628)],
        ].map { points -> CGPath in
            let ear = CGMutablePath()
            ear.addLines(between: points)
            ear.closeSubpath()
            return ear
        }
        var shape: CGPath = face
        for part in [face] + ears {
            shape = shape.union(part)
                .union(part.copy(strokingWithWidth: 44, lineCap: .round, lineJoin: .round, miterLimit: 10))
        }
        return shape
    }()
}
