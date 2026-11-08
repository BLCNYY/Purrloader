import Foundation
import VideoToolbox

/// Turns DownloadOptions into yt-dlp arguments (without the executable itself).
public struct YtDlpCommand: Sendable {
    /// Paths the arguments refer to, passed in so building a command never touches the disk.
    public struct Environment: Sendable {
        public var ffmpeg: String
        public var deno: String
        public var archive: String
        /// aria2c, if the person asked for it and it's installed.
        public var aria2c: String?
        /// QuickTime plays AV1 only where the Mac decodes it in hardware (M3 and later).
        public var playsAV1: Bool

        public init(ffmpeg: String, deno: String, archive: String, aria2c: String? = nil, playsAV1: Bool = true) {
            self.ffmpeg = ffmpeg
            self.deno = deno
            self.archive = archive
            self.aria2c = aria2c
            self.playsAV1 = playsAV1
        }

        /// The tools Purrloader manages. Looks for aria2c on disk only when the options want it.
        public static func installed(for options: DownloadOptions) -> Environment {
            Environment(
                ffmpeg: Tool.ffmpeg.path.path, deno: Tool.deno.path.path, archive: AppPaths.archive.path,
                aria2c: options.network.useAria2c ? Shell.which("aria2c")?.path : nil,
                playsAV1: VTIsHardwareDecodeSupported(kCMVideoCodecType_AV1))
        }
    }

    /// Prefixes of the progress lines Purrloader asks yt-dlp to print.
    public static let progressPrefix = "[purrloader]"
    public static let postprocessPrefix = "[purrloaderpp]"

    public var options: DownloadOptions
    public var environment: Environment

    public init(options: DownloadOptions, environment: Environment) {
        self.options = options
        self.environment = environment
    }

    /// A download. `playlistItems` (yt-dlp -I syntax) overrides the playlist settings; `filepathLog`
    /// receives the final path of every file.
    public func download(url: String, folder: URL, playlistItems: String? = nil, filepathLog: URL? = nil) -> [String] {
        var a = access + format
        if let items = playlistItems, !items.isEmpty {
            a += ["--yes-playlist", "-I", items]
            if options.playlist.useArchive { a += ["--download-archive", environment.archive] }
        } else {
            a += playlist
        }
        a += subtitles + extras + transfer + output(folder: folder)
        a += [
            "--newline", "--progress-delta", "0.5",
            "--progress-template",
            "download:\(Self.progressPrefix)%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|"
                + "%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.title)s",
            "--progress-template",
            "postprocess:\(Self.postprocessPrefix)%(progress.status)s|%(progress.postprocessor)s",
        ]
        if let filepathLog { a += ["--print-to-file", "after_move:filepath", filepathLog.path] }
        a += Shell.split(options.extraArguments)
        a += ["--", url]
        return a
    }

    /// Reads a link's details as JSON, without the entries of a playlist.
    public func info(url: String) -> [String] {
        access + ["-J", "--flat-playlist", "--no-warnings", "--", url]
    }

    /// Shared by info and download: config, tools, sign-in and network.
    private var access: [String] {
        let network = options.network
        var a: [String] = []
        if options.ignoreConfig { a.append("--ignore-config") }
        a += ["--ffmpeg-location", environment.ffmpeg, "--js-runtimes", "deno:\(environment.deno)"]

        if network.cookieBrowser != .none {
            var spec: String
            switch network.cookieBrowser {
            case .arc:
                // yt-dlp has no Arc entry; Arc is Chromium-based, so point it at Arc's profile.
                let profile = network.cookieProfile.trimmed.isEmpty ? "Default" : network.cookieProfile.trimmed
                spec = "chromium:" + NSHomeDirectory() + "/Library/Application Support/Arc/User Data/" + profile
            default:
                spec = network.cookieBrowser.rawValue
                if !network.cookieProfile.trimmed.isEmpty { spec += ":" + network.cookieProfile.trimmed }
            }
            a += ["--cookies-from-browser", spec]
        } else if !network.cookiesFile.trimmed.isEmpty {
            a += ["--cookies", (network.cookiesFile.trimmed as NSString).expandingTildeInPath]
        }
        if !network.proxy.trimmed.isEmpty { a += ["--proxy", network.proxy.trimmed] }
        if !network.impersonate.trimmed.isEmpty { a += ["--impersonate", network.impersonate.trimmed] }
        if !network.xff.trimmed.isEmpty { a += ["--xff", network.xff.trimmed] }
        if !network.userAgent.trimmed.isEmpty { a += ["--add-headers", "User-Agent:\(network.userAgent.trimmed)"] }
        return a
    }

    private var format: [String] {
        let format = options.format
        let exact = format.exactFormat.trimmed
        var a: [String] = []
        if format.mode == .audio {
            a += ["-f", exact.isEmpty ? "ba/b" : exact, "-x"]
            if format.audioFormat != .original {
                a += ["--audio-format", format.audioFormat.rawValue]
                if format.audioFormat.isLossy { a += ["--audio-quality", format.audioQuality] }
            }
            return a
        }

        let limit = format.maxHeight.map { "[height<=\($0)]" } ?? ""
        a += ["-f", exact.isEmpty ? "bv*\(limit)+ba/b\(limit)" : exact]
        var sort: [String] = []
        switch format.codec {
        case .any: break
        // Compatibility first: H.264 wins even over resolution.
        case .h264: sort += ["vcodec:h264", "res", "acodec:m4a"]
        case .hevc: sort += ["res", "vcodec:h265"]
        case .vp9: sort += ["res", "vcodec:vp9"]
        case .av1: sort += ["res", "vcodec:av01"]
        }
        if format.container == .mp4 {
            // Prefer streams that are already MP4 (H.264/AV1 + AAC) so the result merges into an MP4 that
            // QuickTime and Quick Look play, without re-encoding. MKV/Automatic keep the very best streams.
            // Macs without an AV1 decoder get H.264 instead, even if that means a lower resolution.
            if format.codec == .any {
                sort.insert(contentsOf: environment.playsAV1 ? ["ext:mp4:m4a"] : ["vcodec:h264", "ext:mp4:m4a"], at: 0)
            } else if !sort.contains("acodec:m4a") {
                sort.append("acodec:m4a")
            }
        }
        if !format.allowHDR { sort.insert("hdr:sdr", at: 0) }
        if !sort.isEmpty { a += ["-S", sort.joined(separator: ",")] }
        if format.container != .auto {
            a += ["--merge-output-format", format.container.rawValue]
            a += [format.containerMethod == .remux ? "--remux-video" : "--recode-video", format.container.rawValue]
        }
        return a
    }

    private var playlist: [String] {
        let playlist = options.playlist
        var a: [String] = [playlist.noPlaylist ? "--no-playlist" : "--yes-playlist"]
        let range = playlist.range.trimmed
        if playlist.latestCount > 0 {
            a += ["-I", playlist.reverse ? "\(playlist.latestCount):1:-1" : "1:\(playlist.latestCount)"]
        } else if !range.isEmpty {
            let bounds = range.split(separator: "-", maxSplits: 1).map { String($0).trimmed }
            if bounds.count == 2, let lo = Int(bounds[0]), let hi = Int(bounds[1]) {
                a += ["-I", playlist.reverse ? "\(hi):\(lo):-1" : "\(lo):\(hi)"]
            } else {
                a += ["-I", range]
            }
        } else if playlist.reverse {
            a += ["-I", "::-1"]
        }
        if playlist.useArchive { a += ["--download-archive", environment.archive] }
        return a
    }

    private var subtitles: [String] {
        let subtitles = options.subtitles
        guard subtitles.enabled else { return [] }
        var a = ["--write-subs"]
        if subtitles.includeAutoGenerated { a.append("--write-auto-subs") }
        a += ["--sub-langs", subtitles.languages.trimmed.isEmpty ? "en.*" : subtitles.languages.trimmed]
        if subtitles.convertToSRT { a += ["--convert-subs", "srt"] }
        if subtitles.embed && options.format.mode == .video { a.append("--embed-subs") }
        return a
    }

    private var extras: [String] {
        let extras = options.extras
        var a: [String] = []
        if extras.writeThumbnail { a.append("--write-thumbnail") }
        if extras.embedThumbnail && options.format.supportsCoverArt { a.append("--embed-thumbnail") }
        if extras.embedMetadata { a.append("--embed-metadata") }
        if extras.embedChapters && options.format.supportsChapters { a.append("--embed-chapters") }
        if extras.splitChapters { a.append("--split-chapters") }

        if extras.sponsorBlock != .off, !extras.sponsorCategories.isEmpty {
            a += [
                extras.sponsorBlock == .mark ? "--sponsorblock-mark" : "--sponsorblock-remove",
                extras.sponsorCategories.joined(separator: ","),
            ]
        }

        let start = extras.sectionStart.trimmed, end = extras.sectionEnd.trimmed
        if !start.isEmpty || !end.isEmpty {
            a += ["--download-sections", "*\(start.isEmpty ? "0" : start)-\(end.isEmpty ? "inf" : end)"]
        }
        if !extras.chapterRegex.trimmed.isEmpty { a += ["--download-sections", extras.chapterRegex.trimmed] }
        if extras.forceKeyframes { a.append("--force-keyframes-at-cuts") }

        if extras.liveFromStart { a.append("--live-from-start") }
        if extras.waitForVideo { a += ["--wait-for-video", "\(max(1, extras.waitInterval))"] }

        if extras.writeDescription { a.append("--write-description") }
        if extras.writeComments { a.append("--write-comments") }
        if extras.writeInfoJSON { a.append("--write-info-json") }
        return a
    }

    private var transfer: [String] {
        let network = options.network
        var a: [String] = []
        if !network.rateLimit.trimmed.isEmpty { a += ["-r", network.rateLimit.trimmed] }
        if network.retries != 10 { a += ["-R", "\(network.retries)"] }
        if network.concurrentFragments > 1 { a += ["-N", "\(network.concurrentFragments)"] }
        if network.useAria2c, let aria2c = environment.aria2c { a += ["--downloader", aria2c] }
        return a
    }

    private func output(folder: URL) -> [String] {
        let output = options.output
        let template = output.template.trimmed.isEmpty ? OutputOptions.defaultTemplate : output.template.trimmed
        var a = ["-P", folder.path, "-o", template]
        if output.restrictFilenames {
            a.append("--restrict-filenames")
        } else {
            // Mac-friendly names: keep characters macOS allows ("?", "|", "*") instead of full-width look-alikes,
            // and turn ":" (shown as "/" in Finder) into " - ".
            a.append("--no-windows-filenames")
            for (pattern, replacement) in [(#"\s*:\s+"#, " - "), (":", "-")] {
                a += ["--replace-in-metadata", "title,playlist_title,uploader,channel", pattern, replacement]
            }
        }
        switch output.overwrite {
        case .skipExisting: break
        case .never: a.append("--no-overwrites")
        case .force: a.append("--force-overwrites")
        }
        return a
    }
}
