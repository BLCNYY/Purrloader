import AppKit
import SwiftUI

enum Theme {
    /// Matte black in dark mode, matte white in light mode: full screen, behind sheets, and (at 80%) over the
    /// blurred desktop.
    static let matteNS = NSColor(name: "PurrloaderMatte") { appearance in
        appearance.isDark
            ? NSColor(srgbRed: 0.035, green: 0.035, blue: 0.04, alpha: 1)
            : NSColor(srgbRed: 0.975, green: 0.975, blue: 0.98, alpha: 1)
    }
    static let matte = Color(nsColor: matteNS)
    /// The window background: the matte at Ghostty's background-opacity, over the blur.
    static let translucentMatteNS = NSColor(name: "PurrloaderTranslucentMatte") { appearance in
        matteNS.resolved(for: appearance).withAlphaComponent(windowOpacity)
    }
    /// Same as Ghostty's background-opacity / background-blur-radius.
    static let windowOpacity: CGFloat = 0.8
    static let blurRadius: UInt32 = 40

    /// Text and the marks drawn with it: white in dark mode, near-black in light mode. Surfaces, strokes and
    /// dimmed text are this at lower opacity, so every control reads the same in both.
    static let ink = Color(
        nsColor: NSColor(name: "PurrloaderInk") { appearance in
            appearance.isDark ? .white : NSColor(srgbRed: 0.1, green: 0.1, blue: 0.11, alpha: 1)
        })
    /// What sits on an `ink` fill, like the Download button's title.
    static let onInk = Color(
        nsColor: NSColor(name: "PurrloaderOnInk") { appearance in
            appearance.isDark ? .black : .white
        })

    static let surface = ink.opacity(0.06)
    static let stroke = ink.opacity(0.08)
    static let secondary = ink.opacity(0.55)
    static let tertiary = ink.opacity(0.4)
    static let warning = Color.orange.opacity(0.9)
}

extension NSAppearance {
    var isDark: Bool { bestMatch(from: [.darkAqua, .aqua]) == .darkAqua }
}

extension NSColor {
    fileprivate func resolved(for appearance: NSAppearance) -> NSColor {
        var color = self
        appearance.performAsCurrentDrawingAppearance { color = self.usingColorSpace(.sRGB) ?? self }
        return color
    }
}

/// Settings > General > Appearance: follow the system, or always light or dark.
enum AppAppearance: String, CaseIterable, Identifiable {
    case system, light, dark

    static let storageKey = "appearance"
    var id: String { rawValue }
    var title: String {
        switch self {
        case .system: "System"
        case .light: "Light"
        case .dark: "Dark"
        }
    }

    /// Applies the stored choice to every window, menu and popover.
    @MainActor static func apply() {
        let stored = UserDefaults.standard.string(forKey: storageKey).flatMap(AppAppearance.init) ?? .system
        NSApp.appearance =
            switch stored {
            case .system: nil
            case .light: NSAppearance(named: .aqua)
            case .dark: NSAppearance(named: .darkAqua)
            }
    }
}
