import AppKit
import PurrloaderCore

enum Clipboard {
    static var link: String? {
        NSPasteboard.general.string(forType: .string)?.trimmed.webLinks.first
    }

    static func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
