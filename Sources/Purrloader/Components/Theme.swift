import AppKit
import SwiftUI

enum Theme {
    static let matteNS = NSColor(srgbRed: 0.035, green: 0.035, blue: 0.04, alpha: 1)
    /// Matte black used in full screen and behind sheets.
    static let matte = Color(nsColor: matteNS)
    /// Same as Ghostty's background-opacity / background-blur-radius.
    static let windowOpacity: CGFloat = 0.8
    static let blurRadius: UInt32 = 40

    static let surface = Color.white.opacity(0.06)
    static let stroke = Color.white.opacity(0.08)
    static let secondary = Color.white.opacity(0.55)
    static let tertiary = Color.white.opacity(0.4)
    static let warning = Color.orange.opacity(0.9)
}
