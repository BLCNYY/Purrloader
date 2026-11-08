import AppKit
import Observation
import PurrloaderCore

/// State of the main window: the link being looked at and the options for the next download.
@MainActor @Observable
final class MainModel {
    var urlText = ""
    var info: MediaInfo?
    var isFetching = false
    var fetchError: FriendlyError?
    var selectedEntries: Set<Int> = []
    /// The subtitle track picked for this link; nil uses the saved language.
    var subtitleLanguage: String?
    var downloadPart = false

    var options: DownloadOptions {
        didSet {
            if let data = try? JSONEncoder().encode(options) { UserDefaults.standard.set(data, forKey: "lastOptions") }
        }
    }

    /// Where downloads go. The Save to chip and Settings both change it.
    var folder: URL {
        didSet {
            UserDefaults.standard.set(folder.path, forKey: "downloadFolder")
            folderName = FileManager.default.displayName(atPath: folder.path)
        }
    }
    private(set) var folderName: String

    @ObservationIgnored private var fetchTask: Task<Void, Never>?
    @ObservationIgnored private var lastClipboardLink: String?

    init() {
        if let data = UserDefaults.standard.data(forKey: "lastOptions"),
            let saved = try? JSONDecoder().decode(DownloadOptions.self, from: data)
        {
            options = saved
        } else {
            options = DownloadOptions()
        }
        let folder =
            UserDefaults.standard.string(forKey: "downloadFolder").map { URL(fileURLWithPath: $0) }
            ?? AppPaths.defaultDownloads
        self.folder = folder
        folderName = FileManager.default.displayName(atPath: folder.path)
    }

    var links: [String] { urlText.webLinks }

    var canDownload: Bool {
        !links.isEmpty && ToolManager.shared.isReady && fetchError == nil
            && !(info?.isPlaylist == true && selectedEntries.isEmpty)
    }

    /// Quality tiers for the picker. With formats known, only tiers the video has: a tier shows when some
    /// stream is taller than the tier below it, so a 1920×800 film offers 1080p.
    var qualityHeights: [Int] {
        let tiers = [2160, 1440, 1080, 720, 480]
        let heights = info?.availableHeights ?? []
        guard !heights.isEmpty else { return tiers }
        return tiers.enumerated().compactMap { index, tier in
            let below = index + 1 < tiers.count ? tiers[index + 1] : 0
            return heights.contains { $0 > below && $0 <= tier } ? tier : nil
        }
    }

    // MARK: Subtitles

    var subtitlesOn: Bool {
        get { options.subtitles.enabled }
        set { options.subtitles.enabled = newValue }
    }

    /// Languages offered in the More popover: this video's tracks, or the system language and English.
    var subtitleChoices: [MediaInfo.SubtitleTrack] {
        if let tracks = info?.subtitleTracks, !tracks.isEmpty { return tracks }
        let languages = [Locale.current.language.languageCode?.identifier, "en"].compactMap { $0 }
        return languages.reduce(into: []) { result, code in
            if !result.contains(where: { $0.code == code + ".*" }) {
                result.append(MediaInfo.SubtitleTrack(code: code + ".*", isAutomatic: false))
            }
        }
    }

    var subtitleSelection: String {
        get { subtitleLanguage ?? subtitleChoices.first?.code ?? "en.*" }
        set {
            subtitleLanguage = newValue
            options.subtitles.languages = newValue
        }
    }

    var skipSponsors: Bool {
        get { options.extras.sponsorBlock == .remove }
        set {
            options.extras.sponsorBlock = newValue ? .remove : .off
            if newValue && options.extras.sponsorCategories.isEmpty {
                options.extras.sponsorCategories = SponsorBlockMode.defaultCategories
            }
        }
    }

    /// How many More popover options are on, shown on its chip.
    var activeMoreOptions: Int {
        [subtitlesOn, skipSponsors, downloadPart, options.network.cookieBrowser != .none].filter { $0 }.count
    }

    // MARK: Fetching

    func linkChanged() {
        fetchTask?.cancel()
        info = nil
        fetchError = nil
        options.format.exactFormat = ""
        subtitleLanguage = nil
        isFetching = false
        let links = links
        // Only fetch details for a single link; several links are treated as a batch.
        guard links.count == 1, let link = links.first else { return }
        fetchTask = Task {
            try? await Task.sleep(for: .milliseconds(350))
            guard !Task.isCancelled else { return }
            await fetch(link)
        }
    }

    func fetch(_ link: String) async {
        // A link can arrive (clipboard, drop) while tools are still installing on launch.
        while !ToolManager.shared.isReady {
            try? await Task.sleep(for: .milliseconds(300))
            if Task.isCancelled { return }
        }
        isFetching = true
        fetchError = nil
        defer { if !Task.isCancelled { isFetching = false } }
        do {
            let command = YtDlpCommand(options: options, environment: .installed(for: options))
            let result = try await Shell.run(Tool.ytdlp.executable, command.info(url: link))
            guard !Task.isCancelled, links.first == link else { return }
            guard result.status == 0,
                let json = try JSONSerialization.jsonObject(with: result.stdout) as? [String: Any]
            else {
                let line = result.errorOutput.split(separator: "\n").last { $0.hasPrefix("ERROR:") }
                let error =
                    line.map { FriendlyError(raw: String($0.dropFirst(6)).trimmed) }
                    ?? FriendlyError(kind: .unsupportedLink)
                // A link picked up from the clipboard that isn't a video (a profile, an article) wasn't
                // asked for, so it goes away quietly instead of showing an error.
                if link == lastClipboardLink, urlText.trimmed == link,
                    [.unsupportedLink, .invalidLink].contains(error.kind)
                {
                    urlText = ""
                    return
                }
                fetchError = error
                return
            }
            let info = MediaInfo(json: json, url: link)
            self.info = info
            selectedEntries = Set(info.entries.map(\.index))
            subtitleLanguage = info.defaultSubtitleCode(preferring: options.subtitles.languages)
        } catch {
            if !Task.isCancelled { fetchError = FriendlyError(kind: .unexpectedExit) }
        }
    }

    func refetch() {
        guard let link = links.first else { return }
        fetchTask?.cancel()
        fetchTask = Task { await fetch(link) }
    }

    // MARK: Clipboard & drops

    func pasteFromClipboardIfUseful() {
        guard UserDefaults.standard.bool(forKey: "clipboardAutofill"),
            let link = Clipboard.link, link != lastClipboardLink
        else { return }
        lastClipboardLink = link
        guard urlText.trimmed.isEmpty else { return }
        urlText = link
        linkChanged()
    }

    func pasteAndFetch() {
        guard let text = NSPasteboard.general.string(forType: .string) else { return }
        urlText = text.trimmed
        linkChanged()
    }

    /// Accepts dropped web links and .txt batch files (one URL per line).
    func accept(dropped urls: [URL], strings: [String]) {
        var found: [String] = []
        for url in urls {
            if url.isFileURL {
                if let text = try? String(contentsOf: url, encoding: .utf8) {
                    found += text.split(whereSeparator: \.isNewline).map { String($0).trimmed }
                        .filter { !$0.hasPrefix("#") && !$0.hasPrefix(";") }.flatMap(\.webLinks)
                }
            } else {
                found.append(url.absoluteString)
            }
        }
        found += strings.flatMap(\.webLinks)
        guard !found.isEmpty else { return }
        urlText = found.joined(separator: "\n")
        linkChanged()
    }

    // MARK: Download

    /// Options for the next download: the saved ones plus this link's picks from the More popover.
    private var optionsForDownload: DownloadOptions {
        var opts = options
        if subtitlesOn { opts.subtitles.languages = subtitleSelection }
        if !downloadPart {
            opts.extras.sectionStart = ""
            opts.extras.sectionEnd = ""
        }
        return opts
    }

    func download() {
        guard canDownload else { return }
        let manager = DownloadManager.shared
        var opts = optionsForDownload
        if let info, links.count == 1 {
            var items: String?
            if info.isPlaylist, selectedEntries.count < info.entries.count {
                items = PlaylistSelection.compress(Array(selectedEntries))
            }
            if info.isPlaylist { opts.playlist.noPlaylist = false }
            manager.enqueue(
                DownloadJob(
                    url: info.url, title: info.title, thumbnail: info.thumbnail,
                    options: opts, folder: folder, playlistItems: items))
        } else {
            for link in links {
                manager.enqueue(DownloadJob(url: link, title: nil, thumbnail: nil, options: opts, folder: folder))
            }
        }
        urlText = ""
        info = nil
        fetchError = nil
        options.format.exactFormat = ""
        options.extras.sectionStart = ""
        options.extras.sectionEnd = ""
        downloadPart = false
    }

    /// For a scheduled stream: queue it now and let yt-dlp wait until it goes live.
    func downloadWhenStreamStarts() {
        guard let link = links.first else { return }
        var opts = optionsForDownload
        opts.extras.waitForVideo = true
        DownloadManager.shared.enqueue(
            DownloadJob(url: link, title: nil, thumbnail: nil, options: opts, folder: folder))
        urlText = ""
        fetchError = nil
    }

    /// Asks for a folder; nil if the person cancels.
    static func pickFolder(startingAt folder: URL) -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.directoryURL = folder
        panel.prompt = "Choose"
        panel.message = "Choose where Purrloader saves downloads."
        return panel.runModal() == .OK ? panel.url : nil
    }

    func chooseFolder() {
        if let url = Self.pickFolder(startingAt: folder) { folder = url }
    }

    /// The command that the Download button would run right now.
    var previewCommand: String {
        let link = links.first ?? "URL"
        var items: String?
        if let info, info.isPlaylist, selectedEntries.count < info.entries.count {
            items = PlaylistSelection.compress(Array(selectedEntries))
        }
        let options = optionsForDownload
        let arguments = YtDlpCommand(options: options, environment: .installed(for: options))
            .download(url: link, folder: folder, playlistItems: items)
        return Shell.commandLine([Tool.ytdlp.path.path] + arguments)
    }
}
