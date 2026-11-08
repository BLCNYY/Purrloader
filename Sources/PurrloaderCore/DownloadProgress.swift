import Foundation

/// What a running yt-dlp is doing, built up from its output one line at a time.
public struct DownloadProgress: Equatable, Sendable {
    public enum Phase: Equatable, Sendable { case starting, downloading, processing }

    public var phase: Phase = .starting
    /// A short plain-language description of the current step, or "" while bytes are flowing.
    public var stage = "Starting…"
    public var fraction: Double?
    public var downloadedBytes: Int64 = 0
    public var totalBytes: Int64 = 0
    public var speed: Double = 0
    public var eta: Int?
    public var itemIndex: Int?
    public var itemCount: Int?
    /// Title yt-dlp reported for the current video; for playlists, the current item.
    public var reportedTitle: String?
    /// Files yt-dlp started writing, for cleaning up .part files on cancel.
    public var destinations: [String] = []
    /// The last ERROR line, without its prefix.
    public var lastError: String?
    /// Bytes of streams already finished in the current item (video before audio), so progress spans the whole item.
    var finishedStreamBytes: Int64 = 0

    public init() {}

    public var isPlaylist: Bool { itemCount != nil }

    /// Feeds one line of output. Returns false for Purrloader's own progress lines, which don't belong in the log.
    public mutating func consume(_ line: String) -> Bool {
        if line.hasPrefix(YtDlpCommand.progressPrefix) {
            parseProgress(String(line.dropFirst(YtDlpCommand.progressPrefix.count)))
            return false
        }
        if line.hasPrefix(YtDlpCommand.postprocessPrefix) {
            let parts = line.dropFirst(YtDlpCommand.postprocessPrefix.count).split(separator: "|")
            if parts.first == "started", parts.count > 1 {
                phase = .processing
                stage = Self.describe(postprocessor: String(parts[1]))
                fraction = nil
            }
            return false
        }

        if line.hasPrefix("ERROR:") {
            lastError = String(line.dropFirst(6)).trimmed
        } else if let match = line.firstMatch(of: #/Downloading item (\d+) of (\d+)/#) {
            itemIndex = Int(match.1)
            itemCount = Int(match.2)
            finishedStreamBytes = 0
        } else if let match = line.firstMatch(of: #/^\[download\] Destination: (.+)$/#) {
            destinations.append(String(match.1))
        } else if line.contains("has already been downloaded")
            || line.contains("has already been recorded in the archive")
        {
            stage = "Already downloaded"
        } else if line.hasPrefix("[Merger]") {
            phase = .processing
            stage = "Combining video and audio…"
        } else if line.contains("Waiting for video") || line.hasPrefix("[wait]") {
            stage = "Waiting for the stream to start…"
        } else if phase == .starting, line.hasPrefix("[") {
            stage = Self.describe(extractorLine: line)
        }
        return true
    }

    /// `status|downloaded|total|estimate|speed|eta|title`, as requested with --progress-template.
    private mutating func parseProgress(_ payload: String) {
        let parts = payload.split(separator: "|", maxSplits: 6, omittingEmptySubsequences: false).map(String.init)
        guard parts.count >= 6 else { return }

        let earlier = Double(finishedStreamBytes)
        let downloaded = earlier + (Double(parts[1]) ?? 0)
        let streamTotal = Double(parts[2]) ?? Double(parts[3]) ?? 0
        let total = streamTotal > 0 ? earlier + streamTotal : 0
        downloadedBytes = Int64(downloaded)
        totalBytes = Int64(total)
        speed = Double(parts[4]) ?? 0
        eta = Double(parts[5]).map { Int($0) }
        let itemFraction = total > 0 ? min(1, downloaded / total) : nil
        if parts[0] == "finished" {
            finishedStreamBytes += Int64(streamTotal > 0 ? streamTotal : downloaded - earlier)
        }
        if let index = itemIndex, let count = itemCount, count > 1 {
            fraction = (Double(index - 1) + (itemFraction ?? 0)) / Double(count)
        } else {
            fraction = itemFraction
        }
        if parts.count > 6, parts[6] != "NA" { reportedTitle = parts[6] }
        phase = .downloading
        stage = parts[0] == "finished" ? "Finishing…" : ""
    }

    /// What yt-dlp is doing before the first byte arrives, in plain words.
    private static func describe(extractorLine line: String) -> String {
        if line.contains("Downloading playlist") || line.contains(":tab]") || line.contains("Downloading page")
            || line.contains("playlist")
        {
            return "Reading playlist…"
        }
        if line.contains("Waiting") { return "Waiting…" }
        if line.contains("Sleeping") { return "Pausing between requests…" }
        return "Getting video info…"
    }

    private static func describe(postprocessor name: String) -> String {
        switch name {
        case "Merger": "Combining video and audio…"
        case "ExtractAudio": "Converting audio…"
        case "VideoRemuxer", "VideoConvertor": "Converting video…"
        case "EmbedThumbnail": "Adding cover art…"
        case "FFmpegMetadata": "Adding details…"
        case "EmbedSubtitle": "Adding subtitles…"
        case "SponsorBlock", "ModifyChapters": "Skipping sponsor segments…"
        case "SplitChapters": "Splitting chapters…"
        case "MoveFiles": "Finishing…"
        default: "Processing…"
        }
    }
}

public enum PlaylistSelection {
    /// Turns ticked playlist positions into yt-dlp's -I syntax: [1,2,3,5,7,8] -> "1:3,5,7:8".
    public static func compress(_ indices: [Int]) -> String {
        let sorted = Array(Set(indices)).sorted()
        var parts: [String] = []
        var i = 0
        while i < sorted.count {
            var j = i
            while j + 1 < sorted.count && sorted[j + 1] == sorted[j] + 1 { j += 1 }
            parts.append(j > i ? "\(sorted[i]):\(sorted[j])" : "\(sorted[i])")
            i = j + 1
        }
        return parts.joined(separator: ",")
    }
}
