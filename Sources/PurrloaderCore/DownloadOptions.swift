import Foundation

public enum MediaMode: String, Codable, CaseIterable, Identifiable, Sendable {
    case video, audio
    public var id: String { rawValue }
    public var title: String { self == .video ? "Video" : "Audio Only" }
}

public enum AudioFormat: String, Codable, CaseIterable, Identifiable, Sendable {
    case original = "best", mp3, m4a, opus, flac, wav
    public var id: String { rawValue }
    public var title: String { self == .original ? "Best quality" : rawValue.uppercased() }
    public var isLossy: Bool { [.mp3, .m4a, .opus].contains(self) }
}

public enum Container: String, Codable, CaseIterable, Identifiable, Sendable {
    case auto, mp4, mkv, webm, mov
    public var id: String { rawValue }
    public var title: String { self == .auto ? "Automatic" : rawValue.uppercased() }
}

public enum ContainerMethod: String, Codable, CaseIterable, Identifiable, Sendable {
    case remux, recode
    public var id: String { rawValue }
    public var title: String { self == .remux ? "Remux (fast, lossless)" : "Re-encode (slow)" }
}

public enum CodecPreference: String, Codable, CaseIterable, Identifiable, Sendable {
    case any, h264, hevc, vp9, av1
    public var id: String { rawValue }
    public var title: String {
        switch self {
        case .any: "No preference"
        case .h264: "H.264 (most compatible)"
        case .hevc: "H.265 / HEVC"
        case .vp9: "VP9"
        case .av1: "AV1"
        }
    }
}

public enum SponsorBlockMode: String, Codable, CaseIterable, Identifiable, Sendable {
    case off, mark, remove
    public var id: String { rawValue }
    public var title: String {
        switch self {
        case .off: "Off"
        case .mark: "Mark as chapters"
        case .remove: "Remove segments"
        }
    }

    public static let allCategories = [
        "sponsor", "intro", "outro", "selfpromo", "preview", "filler", "interaction", "music_offtopic", "hook",
    ]
    /// What "Skip sponsor segments" removes: paid spots, self-promotion and like/subscribe reminders.
    public static let defaultCategories = ["sponsor", "selfpromo", "interaction"]
}

public enum CookieBrowser: String, Codable, CaseIterable, Identifiable, Sendable {
    case none, safari, chrome, firefox, arc, brave, edge, chromium, vivaldi, opera
    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .none: "None"
        case .edge: "Microsoft Edge"
        default: rawValue.capitalized
        }
    }

    public var bundleIdentifier: String? {
        switch self {
        case .none: nil
        case .safari: "com.apple.Safari"
        case .chrome: "com.google.Chrome"
        case .firefox: "org.mozilla.firefox"
        case .arc: "company.thebrowser.Browser"
        case .brave: "com.brave.Browser"
        case .edge: "com.microsoft.edgemac"
        case .chromium: "org.chromium.Chromium"
        case .vivaldi: "com.vivaldi.Vivaldi"
        case .opera: "com.operasoftware.Opera"
        }
    }
}

public enum OverwriteRule: String, Codable, CaseIterable, Identifiable, Sendable {
    case skipExisting, never, force
    public var id: String { rawValue }
    public var title: String {
        switch self {
        case .skipExisting: "Skip existing files"
        case .never: "Never overwrite anything"
        case .force: "Always overwrite"
        }
    }
}

/// Every download setting. Saved setups (presets), the queue and history store snapshots of it.
public struct DownloadOptions: Codable, Equatable, Sendable {
    public var format = FormatOptions()
    public var playlist = PlaylistOptions()
    public var subtitles = SubtitleOptions()
    public var extras = ExtraOptions()
    public var network = NetworkOptions()
    public var output = OutputOptions()
    /// Raw yt-dlp arguments, added after everything else.
    public var extraArguments = ""
    /// Keeps the user's own yt-dlp config files from changing Purrloader's output and progress handling.
    public var ignoreConfig = true

    public init() {}

    /// Missing or unreadable values fall back to their defaults, one group at a time,
    /// so settings saved by other versions still load.
    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        format = container.group(.format)
        playlist = container.group(.playlist)
        subtitles = container.group(.subtitles)
        extras = container.group(.extras)
        network = container.group(.network)
        output = container.group(.output)
        extraArguments = (try? container.decode(String.self, forKey: .extraArguments)) ?? ""
        ignoreConfig = (try? container.decode(Bool.self, forKey: .ignoreConfig)) ?? true
    }
}

public struct FormatOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public var mode: MediaMode = .video
    public var maxHeight: Int?
    public var audioFormat: AudioFormat = .mp3
    /// yt-dlp's --audio-quality: "0" is the best VBR, or a bitrate like "192K".
    public var audioQuality = "0"
    public var container: Container = .mp4
    public var containerMethod: ContainerMethod = .remux
    public var codec: CodecPreference = .any
    public var allowHDR = true
    /// Exact stream IDs for one link (yt-dlp -f), picked in the format table.
    public var exactFormat = ""

    public init() {}

    /// Cover art embedding fails the whole download for formats that can't hold it (WebM, WAV).
    public var supportsCoverArt: Bool {
        mode == .audio ? audioFormat != .wav : [.mp4, .mkv, .mov].contains(container)
    }

    public var supportsChapters: Bool { mode == .audio ? audioFormat != .wav : container != .webm }

    /// Extension the final file will most likely have; used for the file name preview.
    public var expectedExtension: String {
        if mode == .audio { return audioFormat == .original ? "m4a" : audioFormat.rawValue }
        return container == .auto ? "mp4" : container.rawValue
    }
}

public struct PlaylistOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public var noPlaylist = false
    /// yt-dlp -I syntax, or a simple "5-20".
    public var range = ""
    public var reverse = false
    public var latestCount = 0
    public var useArchive = false

    public init() {}
}

public struct SubtitleOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public var enabled = false
    public var includeAutoGenerated = true
    /// yt-dlp --sub-langs: codes or regexes, comma separated.
    public var languages = "en.*"
    public var embed = true
    public var convertToSRT = false

    public init() {}
}

public struct ExtraOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public var writeThumbnail = false
    public var embedThumbnail = true
    public var embedMetadata = true
    public var embedChapters = true
    public var splitChapters = false

    public var sponsorBlock: SponsorBlockMode = .off
    public var sponsorCategories = SponsorBlockMode.defaultCategories

    public var sectionStart = ""
    public var sectionEnd = ""
    public var chapterRegex = ""
    public var forceKeyframes = false

    public var liveFromStart = false
    public var waitForVideo = false
    public var waitInterval = 60

    public var writeDescription = false
    public var writeComments = false
    public var writeInfoJSON = false

    public init() {}
}

public struct NetworkOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public var cookieBrowser: CookieBrowser = .none
    public var cookieProfile = ""
    public var cookiesFile = ""
    public var proxy = ""
    public var rateLimit = ""
    public var retries = 10
    public var concurrentFragments = 1
    public var useAria2c = false
    public var impersonate = ""
    public var xff = ""
    public var userAgent = ""

    public init() {}
}

public struct OutputOptions: Codable, Equatable, Sendable, DefaultInitializable {
    public static let defaultTemplate = "%(title)s.%(ext)s"

    public var template = OutputOptions.defaultTemplate
    public var restrictFilenames = false
    public var overwrite: OverwriteRule = .skipExisting

    public init() {}
}

public protocol DefaultInitializable {
    init()
}

extension KeyedDecodingContainer {
    /// Decodes a settings group over its defaults: saved keys win, missing ones keep their default,
    /// and an unreadable group is replaced by defaults.
    fileprivate func group<T: Codable & DefaultInitializable>(_ key: Key) -> T {
        guard let saved = try? decodeIfPresent(JSONValue.self, forKey: key),
            case .object(let savedFields) = saved,
            let defaultsData = try? JSONEncoder().encode(T()),
            case .object(var fields) = try? JSONDecoder().decode(JSONValue.self, from: defaultsData)
        else { return T() }
        fields.merge(savedFields) { _, new in new }
        guard let merged = try? JSONEncoder().encode(JSONValue.object(fields)),
            let value = try? JSONDecoder().decode(T.self, from: merged)
        else { return T() }
        return value
    }
}

/// Any JSON value, for merging saved settings over defaults.
enum JSONValue: Codable, Equatable {
    case null, bool(Bool), number(Double), string(String), array([JSONValue]), object([String: JSONValue])

    init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: JSONValue].self))
        }
    }

    func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null: try container.encodeNil()
        case .bool(let value): try container.encode(value)
        case .number(let value):
            if value == value.rounded(), abs(value) < 1e15 {
                try container.encode(Int64(value))
            } else {
                try container.encode(value)
            }
        case .string(let value): try container.encode(value)
        case .array(let value): try container.encode(value)
        case .object(let value): try container.encode(value)
        }
    }
}
