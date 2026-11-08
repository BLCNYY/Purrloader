import Foundation

/// A yt-dlp error turned into a short sentence plus the one action most likely to fix it.
/// The original text stays in the download's log.
public struct FriendlyError: Codable, Sendable, Equatable, Hashable {
    public enum Kind: String, Codable, Sendable {
        case browserCookies, signInRequired, unsupportedLink, invalidLink, unavailable, formatUnavailable
        case blocked, network, diskFull, folderNotWritable, notStarted, unexpectedExit, other
    }

    public enum Fix: Equatable, Sendable {
        case browserSignIn, updateAndRetry, chooseFolder, retry, useBestQuality, waitForStart

        public var title: String {
            switch self {
            case .browserSignIn: "Use Browser Sign-In"
            case .updateAndRetry: "Update and Retry"
            case .chooseFolder: "Choose Another Folder"
            case .retry: "Retry"
            case .useBestQuality: "Use Best Quality"
            case .waitForStart: "Wait for Start"
            }
        }
    }

    public var kind: Kind
    public var message: String

    public var fix: Fix? {
        switch kind {
        case .browserCookies, .signInRequired, .unavailable: .browserSignIn
        case .unsupportedLink, .blocked: .updateAndRetry
        case .folderNotWritable: .chooseFolder
        case .formatUnavailable: .useBestQuality
        case .notStarted: .waitForStart
        case .network, .diskFull, .unexpectedExit, .other: .retry
        case .invalidLink: nil
        }
    }

    public init(kind: Kind, message: String) {
        self.kind = kind
        self.message = message
    }

    public init(kind: Kind) {
        self.init(kind: kind, message: Self.messages[kind] ?? "Something went wrong.")
    }

    /// Maps a raw yt-dlp error line (without the "ERROR:" prefix).
    public init(raw: String) {
        let text = raw.lowercased()
        func has(_ needles: String...) -> Bool { needles.contains { text.contains($0) } }

        if has("cookies database", "decrypt", "could not copy", "failed to load cookies", "keyring") {
            self.init(kind: .browserCookies)
        } else if has(
            "not a bot", "confirm your age", "age-restricted", "members-only", "join this channel",
            "private video", "login required", "log in", "sign in", "use --cookies")
        {
            self.init(kind: .signInRequired)
        } else if has("unsupported url") {
            self.init(kind: .unsupportedLink)
        } else if has("is not a valid url") {
            self.init(kind: .invalidLink)
        } else if has(
            "video unavailable", "this video has been removed", "not available in your country", "geo restrict")
        {
            self.init(kind: .unavailable)
        } else if has("requested format is not available") {
            self.init(kind: .formatUnavailable)
        } else if has(
            "http error 403", "http error 429", "precondition check failed", "nsig extraction failed",
            "signature extraction", "unable to extract")
        {
            self.init(kind: .blocked)
        } else if has(
            "unable to download webpage", "nodename nor servname", "timed out", "network is unreachable",
            "connection reset", "temporary failure in name resolution")
        {
            self.init(kind: .network)
        } else if has("no space left") {
            self.init(kind: .diskFull)
        } else if has("permission denied", "operation not permitted", "read-only file system") {
            self.init(kind: .folderNotWritable)
        } else if has("this live event will begin", "premieres in", "is_upcoming") {
            self.init(kind: .notStarted)
        } else if let match = raw.firstMatch(of: #/^\[[^\]]+\] [^:]+: (.+)$/#) {
            // Drop yt-dlp's "[extractor] id: " prefix.
            self.init(kind: .other, message: String(match.1))
        } else {
            self.init(kind: .other, message: raw)
        }
    }

    private static let messages: [Kind: String] = [
        .browserCookies: "Couldn't read your sign-in from that browser. Try a different browser.",
        .signInRequired:
            "This needs you to be signed in. Sign in on the site in your browser, then let Purrloader use it.",
        .unsupportedLink: "Purrloader can't download from this link yet.",
        .invalidLink: "That doesn't look like a link.",
        .unavailable: "This video isn't available. It may have been removed, be private, or be blocked where you are.",
        .formatUnavailable: "That quality isn't available for this video.",
        .blocked: "The site blocked the download. Updating Purrloader's helper tools usually fixes this.",
        .network: "Couldn't reach the site. Check your internet connection.",
        .diskFull: "Your disk is full. Free up some space, then try again.",
        .folderNotWritable: "Purrloader can't save to that folder.",
        .notStarted: "This stream hasn't started yet.",
        .unexpectedExit: "The download stopped unexpectedly.",
    ]
}
