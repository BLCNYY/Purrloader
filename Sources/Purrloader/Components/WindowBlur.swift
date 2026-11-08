import AppKit
import Darwin
import SwiftUI

/// The window's background blur, as in Ghostty's background-blur-radius.
///
/// macOS has no public API to blur what's behind a window by a radius, so this calls the private
/// `CGSSetWindowBackgroundBlurRadius`. It's looked up at runtime rather than linked: if a future macOS
/// removes it, Purrloader falls back to the public NSVisualEffectView instead of failing to launch.
@MainActor
enum WindowBlur {
    private typealias MainConnectionID = @convention(c) () -> Int32
    private typealias SetBackgroundBlurRadius = @convention(c) (Int32, UInt32, UInt32) -> Int32

    private static let functions: (connection: MainConnectionID, setRadius: SetBackgroundBlurRadius)? = {
        let defaultHandle = UnsafeMutableRawPointer(bitPattern: -2)  // RTLD_DEFAULT
        guard let connection = dlsym(defaultHandle, "CGSMainConnectionID"),
            let setRadius = dlsym(defaultHandle, "CGSSetWindowBackgroundBlurRadius")
        else { return nil }
        return (
            unsafeBitCast(connection, to: MainConnectionID.self),
            unsafeBitCast(setRadius, to: SetBackgroundBlurRadius.self)
        )
    }()

    static var isAvailable: Bool { functions != nil }

    static func apply(radius: UInt32, to window: NSWindow) {
        guard let functions else { return }
        _ = functions.setRadius(functions.connection(), UInt32(window.windowNumber), radius)
    }
}

/// The public stand-in when the private blur isn't there.
struct VisualEffectBackground: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .hudWindow
        view.blendingMode = .behindWindow
        view.state = .active
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}
