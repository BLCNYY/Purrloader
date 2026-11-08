import PurrloaderCore
import SwiftUI
import UniformTypeIdentifiers

/// Every yt-dlp option Purrloader exposes, grouped by topic. These apply to every download.
struct AdvancedSettings: View {
    @Environment(MainModel.self) private var model
    @State private var section: Section = .format
    @State private var confirmReset = false

    enum Section: String, CaseIterable {
        case format = "Format", playlist = "Playlist", extras = "Extras", network = "Network", output = "Output",
            command = "Command"
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top, spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("These settings apply to every download.").font(.system(size: 13, weight: .semibold))
                    Text("You don't need any of them for everyday downloads.").font(.system(size: 12)).foregroundStyle(
                        .secondary)
                }
                Spacer()
                SavedSetupsMenu()
                Button("Reset to Defaults…") { confirmReset = true }
            }
            .padding(.horizontal, 20)
            .padding(.top, 16)

            PillSegment(
                selection: $section, options: Section.allCases.map { ($0, $0.rawValue) }, height: 30, label: "Section"
            )
            .padding(.top, 14)

            Group {
                switch section {
                case .format: FormatSection()
                case .playlist: PlaylistSection()
                case .extras: ExtrasSection()
                case .network: NetworkSection()
                case .output: OutputSection()
                case .command: CommandSection()
                }
            }
            .frame(maxHeight: .infinity)
        }
        .frame(width: 720, height: 620)
        .confirmationDialog("Reset every advanced setting?", isPresented: $confirmReset) {
            Button("Reset to Defaults", role: .destructive) { model.options = DownloadOptions() }
        } message: {
            Text(
                "Video, audio, subtitle, network and file name settings go back to how Purrloader came. Saved setups are kept."
            )
        }
    }
}

// MARK: - Format

private struct FormatSection: View {
    @Environment(MainModel.self) private var model

    var body: some View {
        @Bindable var model = model
        Form {
            Section {
                Picker("Download", selection: $model.options.format.mode) {
                    ForEach(MediaMode.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
            }
            if model.options.format.mode == .video {
                Section("Video") {
                    Picker("Maximum quality", selection: $model.options.format.maxHeight) {
                        Text("Best available").tag(Int?.none)
                        ForEach([4320, 2160, 1440, 1080, 720, 480, 360], id: \.self) {
                            Text(Format.heightLabel($0)).tag(Int?.some($0))
                        }
                    }
                    Picker("Codec preference", selection: $model.options.format.codec) {
                        ForEach(CodecPreference.allCases) { Text($0.title).tag($0) }
                    }
                    Toggle("Allow HDR", isOn: $model.options.format.allowHDR)
                    Picker("Container", selection: $model.options.format.container) {
                        ForEach(Container.allCases) { Text($0.title).tag($0) }
                    }
                    if model.options.format.container != .auto {
                        Picker("Conversion", selection: $model.options.format.containerMethod) {
                            ForEach(ContainerMethod.allCases) { Text($0.title).tag($0) }
                        }
                    }
                }
            } else {
                Section("Audio") {
                    Picker("Format", selection: $model.options.format.audioFormat) {
                        ForEach(AudioFormat.allCases) { Text($0.title).tag($0) }
                    }
                    if model.options.format.audioFormat.isLossy {
                        Picker("Quality", selection: $model.options.format.audioQuality) {
                            Text("Best (VBR 0)").tag("0")
                            Text("High (VBR 2)").tag("2")
                            Text("Standard (VBR 5)").tag("5")
                            Divider()
                            ForEach(["320K", "256K", "192K", "160K", "128K", "96K", "64K"], id: \.self) {
                                Text($0.replacingOccurrences(of: "K", with: " kbps")).tag($0)
                            }
                        }
                    }
                }
            }
            Section {
                FormatTable()
            } header: {
                Text("Exact streams")
            } footer: {
                Text(
                    "Pick specific streams from the current link's format list (same as yt-dlp -F). Overrides the quality settings above for that link only."
                )
            }
        }
        .formStyle(.grouped)
    }
}

private struct FormatTable: View {
    @Environment(MainModel.self) private var model
    @State private var video: String?
    @State private var audio: String?

    var body: some View {
        @Bindable var model = model
        if let info = model.info, !info.formats.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Table(
                    info.formats,
                    selection: Binding(
                        get: { Set([video, audio].compactMap { $0 }) },
                        set: { select($0, in: info) }
                    )
                ) {
                    TableColumn("ID", value: \.id).width(min: 40, ideal: 60)
                    TableColumn("Ext", value: \.ext).width(40)
                    TableColumn("Resolution") { Text($0.resolution) }.width(min: 60, ideal: 70)
                    TableColumn("Video") {
                        Text(
                            $0.hasVideo
                                ? ($0.vcodec ?? "")
                                    + (($0.dynamicRange ?? "SDR") == "SDR" ? "" : " " + ($0.dynamicRange ?? "")) : "—")
                    }
                    TableColumn("Audio") { Text($0.hasAudio ? $0.acodec ?? "" : "—") }
                    TableColumn("Bitrate") { Text($0.bitrate.map { "\(Int($0))k" } ?? "") }.width(55)
                    TableColumn("Size") { Text($0.size.map(Format.bytes) ?? "") }.width(min: 55, ideal: 70)
                    TableColumn("Note", value: \.note)
                }
                .frame(height: 220)
                HStack {
                    Text(
                        model.options.format.exactFormat.isEmpty
                            ? "Automatic (no streams picked)" : "-f \(model.options.format.exactFormat)"
                    )
                    .font(.callout.monospaced()).foregroundStyle(.secondary)
                    Spacer()
                    Button("Clear") {
                        video = nil; audio = nil; model.options.format.exactFormat = ""
                    }
                    .disabled(model.options.format.exactFormat.isEmpty)
                }
            }
            .onAppear { restoreSelection() }
        } else {
            Text(
                model.info?.isPlaylist == true
                    ? "Stream picking is available for single videos."
                    : "Paste a link in the main window to see its formats."
            )
            .foregroundStyle(.secondary)
            TextField("Format code", text: $model.options.format.exactFormat, prompt: Text("e.g. 137+140 or bv*+ba/b"))
        }
    }

    /// One video (or combined) stream plus one audio-only stream at most.
    private func select(_ ids: Set<String>, in info: MediaInfo) {
        let old = Set([video, audio].compactMap { $0 })
        if let v = video, !ids.contains(v) { video = nil }
        if let a = audio, !ids.contains(a) { audio = nil }
        for id in ids.subtracting(old) {
            guard let format = info.formats.first(where: { $0.id == id }) else { continue }
            if format.hasVideo {
                video = id
                if format.hasAudio { audio = nil }
            } else {
                audio = id
            }
        }
        model.options.format.exactFormat = [video, audio].compactMap { $0 }.joined(separator: "+")
        if video != nil {
            model.options.format.mode = .video
        } else if audio != nil {
            model.options.format.mode = .audio
        }
    }

    private func restoreSelection() {
        let parts = model.options.format.exactFormat.split(separator: "+").map(String.init)
        guard let info = model.info else { return }
        for id in parts {
            if info.formats.first(where: { $0.id == id })?.hasVideo == true { video = id } else { audio = id }
        }
    }
}

// MARK: - Playlist

private struct PlaylistSection: View {
    @Environment(MainModel.self) private var model
    @State private var archiveCount = 0
    @State private var confirmClearArchive = false

    var body: some View {
        @Bindable var model = model
        Form {
            Section {
                Toggle("Only download the video, not the playlist it's in", isOn: $model.options.playlist.noPlaylist)
            } footer: {
                Text("For links that point to one video inside a playlist.")
            }
            Section("Which items") {
                TextField("Range", text: $model.options.playlist.range, prompt: Text("All — or e.g. 5-20, or 1,3,7:10"))
                Stepper(value: $model.options.playlist.latestCount, in: 0...10_000) {
                    LabeledContent("Latest") {
                        Text(
                            model.options.playlist.latestCount == 0
                                ? "All" : "\(model.options.playlist.latestCount) items")
                    }
                }
                Toggle("Reverse order", isOn: $model.options.playlist.reverse)
            }
            Section {
                Toggle("Skip items already downloaded", isOn: $model.options.playlist.useArchive)
                LabeledContent("Archive") {
                    HStack {
                        Text(archiveCount == 1 ? "1 item" : "\(archiveCount) items").foregroundStyle(.secondary)
                        Button("Show") { NSWorkspace.shared.activateFileViewerSelecting([AppPaths.archive]) }
                            .disabled(archiveCount == 0)
                        Button("Clear…") { confirmClearArchive = true }.disabled(archiveCount == 0)
                    }
                }
            } header: {
                Text("Download archive")
            } footer: {
                Text("Records every downloaded video ID so re-running a playlist or channel only fetches new items.")
            }
            if let info = model.info, info.isPlaylist {
                Section {
                    Text(
                        "Items ticked in the main window take priority over the range above (\(model.selectedEntries.count) of \(info.entries.count) ticked)."
                    )
                    .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .task { archiveCount = Self.countArchive() }
        .confirmationDialog("Clear the download archive?", isPresented: $confirmClearArchive) {
            Button("Clear Archive", role: .destructive) {
                try? FileManager.default.removeItem(at: AppPaths.archive)
                archiveCount = 0
            }
        } message: {
            Text("Items you already downloaded will be downloaded again next time.")
        }
    }

    private static func countArchive() -> Int {
        (try? String(contentsOf: AppPaths.archive, encoding: .utf8))?.split(separator: "\n").count ?? 0
    }
}

// MARK: - Extras

private struct ExtrasSection: View {
    @Environment(MainModel.self) private var model

    var body: some View {
        @Bindable var model = model
        Form {
            Section("Subtitles") {
                Toggle("Download subtitles", isOn: $model.options.subtitles.enabled)
                Toggle("Include auto-generated subtitles", isOn: $model.options.subtitles.includeAutoGenerated)
                if model.options.subtitles.enabled {
                    TextField(
                        "Languages", text: $model.options.subtitles.languages,
                        prompt: Text("en.*,de — or all,-live_chat"))
                    Picker("Save as", selection: $model.options.subtitles.embed) {
                        Text("Embed in video").tag(true)
                        Text("Separate files").tag(false)
                    }
                    Toggle("Convert to SRT", isOn: $model.options.subtitles.convertToSRT)
                }
            }
            Section {
                Toggle("Embed thumbnail as cover art", isOn: $model.options.extras.embedThumbnail)
                Toggle("Save thumbnail as a file", isOn: $model.options.extras.writeThumbnail)
                Toggle("Embed metadata", isOn: $model.options.extras.embedMetadata)
                Toggle("Embed chapters", isOn: $model.options.extras.embedChapters)
                Toggle("Split into one file per chapter", isOn: $model.options.extras.splitChapters)
            } header: {
                Text("Thumbnail & metadata")
            } footer: {
                Text("Cover art and chapters are only embedded in formats that can hold them (not WebM or WAV).")
            }
            Section("SponsorBlock") {
                Picker("Segments", selection: $model.options.extras.sponsorBlock) {
                    ForEach(SponsorBlockMode.allCases) { Text($0.title).tag($0) }
                }
                if model.options.extras.sponsorBlock != .off {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 140), alignment: .leading)], alignment: .leading) {
                        ForEach(SponsorBlockMode.allCategories, id: \.self) { category in
                            Toggle(
                                category.replacingOccurrences(of: "_", with: " ").capitalized,
                                isOn: Binding(
                                    get: { model.options.extras.sponsorCategories.contains(category) },
                                    set: { on in
                                        model.options.extras.sponsorCategories.removeAll { $0 == category }
                                        if on { model.options.extras.sponsorCategories.append(category) }
                                    }))
                        }
                    }
                }
            }
            Section {
                TextField(
                    "Chapters", text: $model.options.extras.chapterRegex,
                    prompt: Text("Chapter titles matching (regex), e.g. intro|verse"))
                Toggle("Precise cuts (re-encodes around cut points)", isOn: $model.options.extras.forceKeyframes)
            } header: {
                Text("Only part of the video")
            } footer: {
                Text("Start and end times are set per download in the main window's More menu.")
            }
            Section("Live streams") {
                Toggle("Record from the start", isOn: $model.options.extras.liveFromStart)
                Toggle("Wait for scheduled streams", isOn: $model.options.extras.waitForVideo)
                if model.options.extras.waitForVideo {
                    Stepper(
                        "Check every \(model.options.extras.waitInterval) s", value: $model.options.extras.waitInterval,
                        in: 5...3600, step: 15)
                }
            }
            Section("Also save") {
                Toggle("Description", isOn: $model.options.extras.writeDescription)
                Toggle("Comments (in the info JSON)", isOn: $model.options.extras.writeComments)
                Toggle("Info JSON", isOn: $model.options.extras.writeInfoJSON)
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Network

private struct NetworkSection: View {
    @Environment(MainModel.self) private var model
    @State private var aria2c: URL?

    var body: some View {
        @Bindable var model = model
        Form {
            Section {
                Picker("Cookies from browser", selection: $model.options.network.cookieBrowser) {
                    ForEach(CookieBrowser.allCases) { Text($0.title).tag($0) }
                }
                if model.options.network.cookieBrowser != .none {
                    TextField("Profile", text: $model.options.network.cookieProfile, prompt: Text("Default profile"))
                } else {
                    LabeledContent("Cookies file") {
                        HStack {
                            TextField(
                                "", text: $model.options.network.cookiesFile,
                                prompt: Text("cookies.txt (Netscape format)")
                            ).labelsHidden()
                            Button("Choose…") {
                                let panel = NSOpenPanel()
                                panel.allowedContentTypes = [.plainText, .data]
                                if panel.runModal() == .OK, let url = panel.url {
                                    model.options.network.cookiesFile = url.path
                                }
                            }
                        }
                    }
                }
            } header: {
                Text("Sign-in")
            } footer: {
                Text(
                    model.options.network.cookieBrowser == .arc
                        ? "yt-dlp has no built-in Arc support; Purrloader reads Arc's profile as Chromium, which may fail to decrypt. If it does, export a cookies.txt from Arc instead."
                        : "For private, age-restricted or members-only videos. Safari needs Full Disk Access for Purrloader; Chromium browsers ask for Keychain access."
                )
            }
            Section("Connection") {
                TextField("Proxy", text: $model.options.network.proxy, prompt: Text("socks5://127.0.0.1:1080"))
                TextField("Rate limit", text: $model.options.network.rateLimit, prompt: Text("Unlimited — or e.g. 5M"))
                Stepper("Retries: \(model.options.network.retries)", value: $model.options.network.retries, in: 0...100)
                Stepper(
                    "Concurrent fragments: \(model.options.network.concurrentFragments)",
                    value: $model.options.network.concurrentFragments, in: 1...32)
                Toggle(isOn: $model.options.network.useAria2c) {
                    Text("Use aria2c as external downloader")
                    if aria2c == nil {
                        Text("aria2c not found. Install it with `brew install aria2`.").foregroundStyle(.secondary)
                    }
                }
                .disabled(aria2c == nil && !model.options.network.useAria2c)
            }
            Section("Identity") {
                Picker("Impersonate browser", selection: $model.options.network.impersonate) {
                    Text("Off").tag("")
                    ForEach(["chrome", "edge", "safari", "firefox"], id: \.self) { Text($0.capitalized).tag($0) }
                }
                TextField(
                    "Geo bypass (X-Forwarded-For)", text: $model.options.network.xff,
                    prompt: Text("Off — or a country code like US, or default"))
                TextField("User agent", text: $model.options.network.userAgent, prompt: Text("yt-dlp default"))
            }
        }
        .formStyle(.grouped)
        .task { aria2c = Shell.which("aria2c") }
    }
}

// MARK: - Output

private struct OutputSection: View {
    @Environment(MainModel.self) private var model

    var body: some View {
        @Bindable var model = model
        Form {
            Section("File name") {
                HStack {
                    TextField("Template", text: $model.options.output.template).font(.body.monospaced()).labelsHidden()
                    Menu("Presets") {
                        ForEach(TemplatePreview.presets, id: \.1) { name, template in
                            Button(name) { model.options.output.template = template }
                        }
                    }
                    .fixedSize()
                }
                LabeledContent("Preview") {
                    Text(
                        TemplatePreview.render(
                            model.options.output.template, info: model.info?.raw,
                            ext: model.options.format.expectedExtension,
                            restrict: model.options.output.restrictFilenames)
                    )
                    .font(.callout.monospaced()).textSelection(.enabled).multilineTextAlignment(.trailing)
                }
                Link(
                    "Template fields reference",
                    destination: URL(string: "https://github.com/yt-dlp/yt-dlp#output-template")!
                )
                .font(.callout)
            }
            Section("Files") {
                Toggle("Restrict file names to ASCII, no spaces", isOn: $model.options.output.restrictFilenames)
                Picker("Existing files", selection: $model.options.output.overwrite) {
                    ForEach(OverwriteRule.allCases) { Text($0.title).tag($0) }
                }
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Command

private struct CommandSection: View {
    @Environment(MainModel.self) private var model

    var body: some View {
        @Bindable var model = model
        Form {
            Section {
                TextField(
                    "Extra arguments", text: $model.options.extraArguments,
                    prompt: Text("e.g. --match-filters \"duration < 600\" --sleep-interval 5"), axis: .vertical
                )
                .font(.body.monospaced())
                .lineLimit(2...5)
                .labelsHidden()
            } header: {
                Text("Extra yt-dlp arguments")
            } footer: {
                Text(
                    "Added after everything else, so they override the options in the other sections. Shell-style quoting works."
                )
            }
            Section {
                Toggle("Ignore my yt-dlp config files", isOn: $model.options.ignoreConfig)
            } footer: {
                Text("Keeps ~/.config/yt-dlp/config from changing Purrloader's output and progress handling.")
            }
            Section {
                Text(model.previewCommand)
                    .font(.caption.monospaced())
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                HStack {
                    Spacer()
                    Button("Copy Command") { Clipboard.copy(model.previewCommand) }
                }
            } header: {
                Text("Command preview")
            } footer: {
                Text(
                    "Exactly what the Download button will run for the current link. Each download's full log is under Details in the queue."
                )
            }
        }
        .formStyle(.grouped)
    }
}
