import AppKit
import SwiftUI

extension View {
    /// The matte (black, or white in light mode) at 80% over a 40px blur in a window; solid in full screen.
    func purrloaderWindowStyle() -> some View {
        scrollContentBackground(.hidden)
            .background {
                if !WindowBlur.isAvailable {
                    VisualEffectBackground().overlay(Theme.matte.opacity(0.55)).ignoresSafeArea()
                }
            }
            .background(WindowConfigurator())
    }

    /// For sheets, which sit over the window: the solid matte.
    func purrloaderSheetStyle() -> some View {
        scrollContentBackground(.hidden)
            .background(Theme.matte)
    }
}

/// Makes the hosting window translucent with a background blur, and opaque in full screen.
private struct WindowConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView { WindowObserver() }
    func updateNSView(_ nsView: NSView, context: Context) {}

    private final class WindowObserver: NSView {
        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            NotificationCenter.default.removeObserver(self)
            guard let window else { return }
            window.isOpaque = false
            window.titlebarAppearsTransparent = true
            apply(fullScreen: window.styleMask.contains(.fullScreen))
            NotificationCenter.default.addObserver(
                self, selector: #selector(willEnterFullScreen),
                name: NSWindow.willEnterFullScreenNotification, object: window)
            NotificationCenter.default.addObserver(
                self, selector: #selector(didExitFullScreen),
                name: NSWindow.didExitFullScreenNotification, object: window)
        }

        @objc private func willEnterFullScreen() { apply(fullScreen: true) }
        @objc private func didExitFullScreen() { apply(fullScreen: false) }

        private func apply(fullScreen: Bool) {
            guard let window else { return }
            if fullScreen {
                window.backgroundColor = Theme.matteNS
            } else if WindowBlur.isAvailable {
                window.backgroundColor = Theme.translucentMatteNS
            } else {
                window.backgroundColor = .clear
            }
            WindowBlur.apply(radius: fullScreen ? 0 : Theme.blurRadius, to: window)
        }
    }
}
