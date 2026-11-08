import Foundation

/// The parts of yt-dlp's `-J --flat-playlist` output the UI needs, plus the raw dictionary for previews.
public struct MediaInfo: @unchecked Sendable {
    public struct Entry: Identifiable, Hashable, Sendable {
        /// 1-based position in the playlist, what -I expects.
        public let index: Int
        public let id: String
        public let title: String
        public let duration: Double?
    }

    public struct StreamFormat: Identifiable, Hashable, Sendable {
        public let id: String
        public let ext: String
        public let height: Int?
        public let fps: Double?
        public let vcodec: String?
        public let acodec: String?
        public let bitrate: Double?
        public let size: Int64?
        public let note: String
        public let dynamicRange: String?

        public var hasVideo: Bool { vcodec != nil && vcodec != "none" }
        public var hasAudio: Bool { acodec != nil && acodec != "none" }
        public var resolution: String {
            guard hasVideo else { return "audio" }
            return height.map { "\($0)p" + (fps.map { $0 > 30 ? "\(Int($0))" : "" } ?? "") } ?? "video"
        }
    }

    public struct SubtitleTrack: Identifiable, Hashable, Sendable {
        /// What --sub-langs expects.
        public let code: String
        public let isAutomatic: Bool
        public var id: String { code }

        public init(code: String, isAutomatic: Bool) {
            self.code = code
            self.isAutomatic = isAutomatic
        }

        public var title: String {
            let language = code.hasSuffix("-orig") ? String(code.dropLast(5)) : code
            let name = Locale.current.localizedString(forIdentifier: language) ?? language
            return isAutomatic ? "\(name) (auto-generated)" : name
        }
    }

    /// The whole JSON, for the file name preview.
    public let raw: [String: Any]
    public let url: String
    public let title: String
    public let uploader: String?
    public let duration: Double?
    public let thumbnail: URL?
    public let isPlaylist: Bool
    public let entries: [Entry]
    public let formats: [StreamFormat]
    public let isLive: Bool
    public let subtitleTracks: [SubtitleTrack]

    public init(json: [String: Any], url: String) {
        raw = json
        self.url = url
        title = json["title"] as? String ?? url
        uploader = (json["uploader"] ?? json["channel"] ?? json["playlist_uploader"]) as? String
        duration = json["duration"] as? Double
        isLive = (json["is_live"] as? Bool) == true || (json["live_status"] as? String) == "is_upcoming"
        isPlaylist = (json["_type"] as? String) == "playlist"

        if let thumb = json["thumbnail"] as? String {
            thumbnail = URL(string: thumb)
        } else if let thumbs = json["thumbnails"] as? [[String: Any]], let last = thumbs.last?["url"] as? String {
            thumbnail = URL(string: last)
        } else if let first = (json["entries"] as? [[String: Any]])?.first,
            let thumbs = first["thumbnails"] as? [[String: Any]], let last = thumbs.last?["url"] as? String
        {
            thumbnail = URL(string: last)
        } else {
            thumbnail = nil
        }

        entries = (json["entries"] as? [[String: Any]] ?? []).enumerated().map { offset, e in
            Entry(
                index: offset + 1,
                id: e["id"] as? String ?? "\(offset)",
                title: e["title"] as? String ?? e["url"] as? String ?? "Item \(offset + 1)",
                duration: e["duration"] as? Double)
        }

        formats = (json["formats"] as? [[String: Any]] ?? []).compactMap { f in
            guard let id = f["format_id"] as? String else { return nil }
            let size = (f["filesize"] as? NSNumber ?? f["filesize_approx"] as? NSNumber)?.int64Value
            return StreamFormat(
                id: id,
                ext: f["ext"] as? String ?? "",
                height: f["height"] as? Int,
                fps: f["fps"] as? Double,
                vcodec: f["vcodec"] as? String,
                acodec: f["acodec"] as? String,
                bitrate: f["tbr"] as? Double,
                size: size,
                note: f["format_note"] as? String ?? "",
                dynamicRange: f["dynamic_range"] as? String)
        }
        .filter { $0.hasVideo || $0.hasAudio }
        .reversed()  // yt-dlp lists worst first

        subtitleTracks = Self.subtitleTracks(manual: json["subtitles"], automatic: json["automatic_captions"])
    }

    /// Real subtitles, plus the auto-generated ones worth offering: the original spoken language and
    /// the reader's own language or English. Sites can list a hundred machine translations otherwise.
    private static func subtitleTracks(manual: Any?, automatic: Any?) -> [SubtitleTrack] {
        let manual = Set((manual as? [String: Any] ?? [:]).keys).subtracting(["live_chat"])
        let automatic = Set((automatic as? [String: Any] ?? [:]).keys)
        let wanted = Set([Locale.current.language.languageCode?.identifier, "en"].compactMap { $0 })
        var tracks = manual.map { SubtitleTrack(code: $0, isAutomatic: false) }
        for key in automatic {
            var code: String?
            if key.hasSuffix("-orig") {
                let base = String(key.dropLast(5))
                code = automatic.contains(base) ? base : key
            } else if wanted.contains(key) {
                code = key
            }
            if let code, !tracks.contains(where: { $0.code == code }) {
                tracks.append(SubtitleTrack(code: code, isAutomatic: true))
            }
        }
        return tracks.sorted { ($0.isAutomatic ? 1 : 0, $0.title) < ($1.isAutomatic ? 1 : 0, $1.title) }
    }

    /// The saved choice if this video has it, else the system language, then English, then anything.
    public func defaultSubtitleCode(preferring saved: String?) -> String? {
        let codes = subtitleTracks.map(\.code)
        if let saved, codes.contains(saved) { return saved }
        for language in [Locale.current.language.languageCode?.identifier, "en"].compactMap({ $0 }) {
            if let match = codes.first(where: { $0 == language || $0.hasPrefix(language + "-") }) { return match }
        }
        return codes.first
    }

    /// Distinct video heights actually on offer, best first.
    public var availableHeights: [Int] {
        Array(Set(formats.filter(\.hasVideo).compactMap(\.height))).sorted(by: >)
    }
}

/// Approximates yt-dlp's output template rendering for the live filename preview.
public enum TemplatePreview {
    public static let presets: [(String, String)] = [
        ("Title", "%(title)s.%(ext)s"),
        ("Title [ID]", "%(title)s [%(id)s].%(ext)s"),
        ("Uploader / Title", "%(uploader)s/%(title)s.%(ext)s"),
        ("Date - Title", "%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s"),
        ("Playlist / 001 - Title", "%(playlist_title,playlist|NA)s/%(playlist_index)03d - %(title)s.%(ext)s"),
        ("Channel / Date - Title [ID]", "%(channel,uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s"),
    ]

    nonisolated(unsafe) static let sample: [String: Any] = [
        "id": "a1b2c3d4e5f", "title": "Example Video Title", "uploader": "Example Channel",
        "channel": "Example Channel", "upload_date": "20240131", "playlist_title": "Example Playlist",
        "playlist_index": 1, "duration": 212, "height": 1080, "resolution": "1920x1080",
    ]

    public static func render(_ template: String, info: [String: Any]?, ext: String, restrict: Bool) -> String {
        var fields = sample
        if let info {
            for (k, v) in info where !(v is [Any]) && !(v is [String: Any]) { fields[k] = v }
            if (info["_type"] as? String) == "playlist" {
                fields["playlist_title"] = info["title"]
                if let first = (info["entries"] as? [[String: Any]])?.first {
                    for (k, v) in first where !(v is [Any]) && !(v is [String: Any]) { fields[k] = v }
                }
            }
        }
        fields["ext"] = ext

        let pattern = #"%\(([^)]+)\)([-#0+ ]*\d*(?:\.\d+)?)([diouxXeEfFgGcrsaBjlqDSU])"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return template }
        let ns = template as NSString
        var result = ""
        var last = 0
        for match in regex.matches(in: template, range: NSRange(location: 0, length: ns.length)) {
            result += ns.substring(with: NSRange(location: last, length: match.range.location - last))
            let expr = ns.substring(with: match.range(at: 1))
            let flags = ns.substring(with: match.range(at: 2))
            let conversion = ns.substring(with: match.range(at: 3))
            result += sanitize(
                value(for: expr, fields: fields, flags: flags, conversion: conversion), restrict: restrict)
            last = match.range.location + match.range.length
        }
        result += ns.substring(from: last)
        return result
    }

    private static func value(for expr: String, fields: [String: Any], flags: String, conversion: String) -> String {
        var expr = expr
        var fallback = "NA"
        if let bar = expr.firstIndex(of: "|") {
            fallback = String(expr[expr.index(after: bar)...])
            expr = String(expr[..<bar])
        }
        var dateFormat: String?
        if let gt = expr.firstIndex(of: ">") {
            dateFormat = String(expr[expr.index(after: gt)...])
            expr = String(expr[..<gt])
        }
        // "a,b" means first available of a or b
        let raw = expr.split(separator: ",").lazy.compactMap { fields[String($0).trimmed] }.first
        guard let raw, !(raw is NSNull) else { return fallback }

        if let dateFormat, let text = raw as? String, text.count == 8 {
            let parser = DateFormatter(); parser.dateFormat = "yyyyMMdd"
            if let date = parser.date(from: text) { return strftime(dateFormat, date) }
        }
        if conversion == "d", let n = (raw as? NSNumber)?.intValue {
            let width = Int(flags.filter(\.isNumber)) ?? 0
            return flags.hasPrefix("0") ? String(format: "%0\(width)d", n) : String(n)
        }
        return "\(raw)"
    }

    private static func strftime(_ format: String, _ date: Date) -> String {
        let map = [
            "%Y": "yyyy", "%m": "MM", "%d": "dd", "%y": "yy", "%H": "HH", "%M": "mm", "%S": "ss", "%b": "MMM",
            "%B": "MMMM",
        ]
        var f = format
        for (k, v) in map { f = f.replacingOccurrences(of: k, with: v) }
        let formatter = DateFormatter(); formatter.dateFormat = f
        return formatter.string(from: date)
    }

    private static func sanitize(_ s: String, restrict: Bool) -> String {
        // Mirrors outputArguments: ": " becomes " - ", a bare ":" becomes "-".
        var out = s.replacingOccurrences(of: "/", with: "⧸")
            .replacingOccurrences(of: #"\s*:\s+"#, with: " - ", options: .regularExpression)
            .replacingOccurrences(of: ":", with: "-")
        if restrict {
            out =
                out.applyingTransform(.toLatin, reverse: false)?.applyingTransform(.stripDiacritics, reverse: false)
                ?? out
            out = String(out.map { $0.isASCII && ($0.isLetter || $0.isNumber || "-_.[]()".contains($0)) ? $0 : "_" })
        }
        return out
    }
}
