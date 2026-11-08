import Foundation

public enum AppPaths {
    public static let support: URL = make(
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Purrloader", isDirectory: true))
    public static let bin = make(support.appendingPathComponent("bin", isDirectory: true))
    public static let temp = make(support.appendingPathComponent("tmp", isDirectory: true))
    public static let archive = support.appendingPathComponent("download-archive.txt")
    public static let presets = support.appendingPathComponent("presets.json")
    public static let history = support.appendingPathComponent("history.json")
    public static let queue = support.appendingPathComponent("queue.json")

    public static var defaultDownloads: URL {
        FileManager.default.urls(for: .downloadsDirectory, in: .userDomainMask)[0]
    }

    private static func make(_ url: URL) -> URL {
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }
}

public enum AppInfo {
    public static let repository = "BLCNYY/Purrloader"
    public static let repositoryURL = URL(string: "https://github.com/\(repository)")!

    /// From the bundle's Info.plist, which build.sh fills in from the VERSION file.
    public static var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.0"
    }

    /// A file in the repository, e.g. LICENSE.
    public static func fileURL(_ path: String) -> URL {
        repositoryURL.appending(path: "blob/main/\(path)")
    }
}

/// JSON files with a schemaVersion, so later versions of Purrloader know what they're reading.
public enum JSONFile {
    private struct Envelope<Content: Codable>: Codable {
        var schemaVersion: Int
        var content: Content
    }

    /// nil if the file is missing, unreadable, or written by a newer schema.
    public static func load<Content: Codable>(_ type: Content.Type, from url: URL, schemaVersion: Int) -> Content? {
        guard let data = try? Data(contentsOf: url),
            let envelope = try? decoder.decode(Envelope<Content>.self, from: data),
            envelope.schemaVersion <= schemaVersion
        else { return nil }
        return envelope.content
    }

    public static func save<Content: Codable>(_ content: Content, to url: URL, schemaVersion: Int) {
        guard let data = try? encoder.encode(Envelope(schemaVersion: schemaVersion, content: content)) else { return }
        try? data.write(to: url, options: .atomic)
    }

    private static var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }

    private static var decoder: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }
}

public enum Format {
    public static func bytes(_ value: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: value, countStyle: .file)
    }

    public static func duration(_ seconds: Double?) -> String? {
        guard let seconds, seconds > 0 else { return nil }
        let s = Int(seconds.rounded())
        return s >= 3600
            ? String(format: "%d:%02d:%02d", s / 3600, (s % 3600) / 60, s % 60)
            : String(format: "%d:%02d", s / 60, s % 60)
    }

    public static func heightLabel(_ height: Int) -> String {
        switch height {
        case 4320...: "8K"
        case 2160...: "4K"
        default: "\(height)p"
        }
    }
}

extension String {
    public var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }

    /// http(s) links found in the string, in order. Links pasted from prose often end with
    /// punctuation that isn't part of them, so that is dropped.
    public var webLinks: [String] {
        split(whereSeparator: { $0.isWhitespace })
            .map { String($0).trimmingCharacters(in: CharacterSet(charactersIn: ".,;:!?\"'”’>")) }
            .filter {
                guard let url = URL(string: $0), let scheme = url.scheme?.lowercased() else { return false }
                return (scheme == "http" || scheme == "https") && url.host != nil
            }
    }
}
