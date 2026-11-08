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
    private var manager: DownloadManager { .shared }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Purrloader").font(.system(size: 14, weight: .semibold))
                Spacer()
                Text(summary).font(.system(size: 11)).foregroundStyle(Theme.secondary)
            }
            let visible = Array(manager.jobs.filter { !$0.status.isDone }.prefix(6))
            if visible.isEmpty {
                Text("Nothing downloading")
                    .font(.system(size: 12)).foregroundStyle(Theme.tertiary)
                    .frame(maxWidth: .infinity, minHeight: 44)
            } else {
                ForEach(visible) { job in
                    HStack(spacing: 10) {
                        Thumbnail(url: job.thumbnail, width: 44)
                        VStack(alignment: .leading, spacing: 5) {
                            Text(job.title).font(.system(size: 12, weight: .semibold)).lineLimit(1)
                            if job.status.isActive { SlimProgress(fraction: job.fraction) }
                            Text(job.statusText).font(.system(size: 10)).foregroundStyle(Theme.secondary).lineLimit(1)
                        }
                    }
                }
            }
            HStack(spacing: 8) {
                Button("Open Purrloader") {
                    openWindow(id: "main")
                    NSApp.activate()
                }
                .buttonStyle(PrimaryCapsuleStyle())
                Spacer()
                if manager.activeCount > 0 || manager.queuedCount > 0 {
                    Button {
                        manager.pauseAll()
                    } label: {
                        Image(systemName: "pause.fill")
                    }
                    .buttonStyle(CircleButtonStyle(size: 30)).help("Pause all").accessibilityLabel("Pause all")
                } else if manager.jobs.contains(where: { $0.status == .paused }) {
                    Button {
                        manager.resumeAll()
                    } label: {
                        Image(systemName: "play.fill")
                    }
                    .buttonStyle(CircleButtonStyle(size: 30)).help("Resume all").accessibilityLabel("Resume all")
                }
                Button {
                    NSApp.terminate(nil)
                } label: {
                    Image(systemName: "power")
                }
                .buttonStyle(CircleButtonStyle(size: 30)).help("Quit Purrloader").accessibilityLabel("Quit Purrloader")
            }
            .padding(.top, 2)
        }
        .padding(14)
        .frame(width: 320)
        .preferredColorScheme(.dark)
    }

    private var summary: String {
        let active = manager.activeCount, queued = manager.queuedCount
        if active == 0 && queued == 0 { return "Idle" }
        return queued > 0 ? "\(active) active · \(queued) waiting" : "\(active) active"
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
