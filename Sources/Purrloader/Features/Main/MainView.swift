import PurrloaderCore
import SwiftUI
import UniformTypeIdentifiers

struct MainView: View {
    @Environment(MainModel.self) private var model
    @State private var isDropTargeted = false
    @State private var showMore = false
    @State private var showWelcome = !UserDefaults.standard.bool(forKey: WelcomeView.seenKey)
    @FocusState private var urlFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            // Room for the traffic lights in the hidden title bar.
            Color.clear.frame(height: 38)
            NoticeBanner()
            VStack(alignment: .leading, spacing: 14) {
                urlField
                linkStatus
                controls
            }
            .padding(.horizontal, 20)
            .padding(.bottom, 18)
            .animation(.snappy(duration: 0.25), value: model.info?.url)

            Rectangle().fill(Theme.stroke).frame(height: 1)
            QueueView()
        }
        .frame(minWidth: 720, minHeight: 580)
        .defaultFocus($urlFocused, true)
        .ignoresSafeArea(edges: .top)
        .overlay {
            if isDropTargeted {
                RoundedRectangle(cornerRadius: 16).strokeBorder(
                    .white.opacity(0.6), style: StrokeStyle(lineWidth: 2, dash: [8, 6])
                )
                .padding(8)
                .allowsHitTesting(false)
            }
        }
        .onDrop(of: [.url, .fileURL, .plainText], isTargeted: $isDropTargeted) { providers in
            Task { await accept(providers) }
            return !providers.isEmpty
        }
        .sheet(isPresented: $showWelcome) { WelcomeView().purrloaderSheetStyle() }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            model.pasteFromClipboardIfUseful()
        }
        .onAppear { model.pasteFromClipboardIfUseful() }
        .task {
            // Right after launch the window isn't key yet and focus would go to the first button.
            try? await Task.sleep(for: .milliseconds(150))
            urlFocused = true
        }
    }

    private var urlField: some View {
        @Bindable var model = model
        return HStack(spacing: 10) {
            Image(systemName: "link").font(.system(size: 14, weight: .medium)).foregroundStyle(Theme.tertiary)
                .accessibilityHidden(true)
            TextField(
                "Link", text: $model.urlText,
                prompt: Text("Paste a link, or drop links and .txt files").foregroundStyle(Theme.tertiary)
            )
            .textFieldStyle(.plain)
            .font(.system(size: 15, weight: .regular))
            .labelsHidden()
            .focused($urlFocused)
            .onSubmit { model.info == nil ? model.refetch() : model.download() }
            .onChange(of: model.urlText) { model.linkChanged() }
            if !model.urlText.isEmpty {
                Button {
                    model.urlText = ""
                } label: {
                    Image(systemName: "xmark")
                }
                .buttonStyle(CircleButtonStyle(size: 22, filled: true))
                .help("Clear")
                .accessibilityLabel("Clear link")
            }
        }
        .padding(.horizontal, 16)
        .frame(height: 48)
        .background(RoundedRectangle(cornerRadius: 14, style: .continuous).fill(Theme.surface))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(urlFocused ? .white.opacity(0.3) : Theme.stroke, lineWidth: 1)
        )
        .animation(.easeOut(duration: 0.15), value: urlFocused)
    }

    @ViewBuilder private var linkStatus: some View {
        if model.isFetching {
            HStack(spacing: 8) {
                ProgressView().controlSize(.small)
                Text("Getting video info…").foregroundStyle(Theme.secondary)
            }
            .font(.system(size: 12))
            .padding(.leading, 4)
        } else if let error = model.fetchError {
            FetchErrorRow(error: error)
        } else if let info = model.info {
            InfoCard(info: info)
        } else if model.links.count > 1 {
            Label("\(model.links.count) links will be added to the queue", systemImage: "list.bullet")
                .font(.system(size: 12)).foregroundStyle(Theme.secondary).padding(.leading, 4)
        }
    }

    private var controls: some View {
        @Bindable var model = model
        return HStack(spacing: 8) {
            PillSegment(
                selection: $model.options.format.mode, options: [(.video, "Video"), (.audio, "Audio")],
                label: "Download as")

            if model.options.format.mode == .video {
                ChipPicker(
                    name: "Quality", label: model.options.format.maxHeight.map(Format.heightLabel) ?? "Best",
                    selection: $model.options.format.maxHeight
                ) {
                    Text("Best").tag(Int?.none)
                    ForEach(qualityChoices, id: \.self) { Text(Format.heightLabel($0)).tag(Int?.some($0)) }
                }
                .help("Highest quality to download")
            } else {
                ChipPicker(
                    name: "Audio format", label: model.options.format.audioFormat.title,
                    selection: $model.options.format.audioFormat
                ) {
                    ForEach(audioChoices) { Text($0.title).tag($0) }
                }
                .help("MP3 plays everywhere. Best quality keeps the original sound without converting it.")
            }

            ChipButton(text: model.folderName, icon: "folder") { model.chooseFolder() }
                .help("Save to \((model.folder.path as NSString).abbreviatingWithTildeInPath)")
                .accessibilityLabel("Save to \(model.folderName)")

            Spacer(minLength: 0)

            ChipButton(text: moreTitle, icon: "slider.horizontal.3") { showMore.toggle() }
                .help("Subtitles, sponsor skipping, part of a video, browser sign-in")
                .accessibilityLabel("More options")
                .accessibilityValue(model.activeMoreOptions == 0 ? "" : "\(model.activeMoreOptions) on")
                .popover(isPresented: $showMore, arrowEdge: .bottom) {
                    MoreOptionsPopover().environment(model)
                }

            Button("Download") { model.download() }
                .buttonStyle(PrimaryCapsuleStyle())
                .help("Download (⌘↩)")
                .disabled(!model.canDownload)
        }
    }

    private var moreTitle: String {
        model.activeMoreOptions == 0 ? "More" : "More · \(model.activeMoreOptions)"
    }

    /// The video's tiers, plus the saved choice so the menu can show it as selected.
    private var qualityChoices: [Int] {
        var heights = model.qualityHeights
        if let current = model.options.format.maxHeight, !heights.contains(current) { heights.append(current) }
        return heights.sorted(by: >)
    }

    private var audioChoices: [AudioFormat] {
        var formats: [AudioFormat] = [.mp3, .m4a, .original]
        if !formats.contains(model.options.format.audioFormat) { formats.append(model.options.format.audioFormat) }
        return formats
    }

    private func accept(_ providers: [NSItemProvider]) async {
        var urls: [URL] = []
        var strings: [String] = []
        for provider in providers {
            if provider.canLoadObject(ofClass: URL.self), let url = await provider.load(URL.self) {
                urls.append(url)
            } else if provider.canLoadObject(ofClass: String.self), let text = await provider.load(String.self) {
                strings.append(text)
            }
        }
        model.accept(dropped: urls, strings: strings)
    }
}

extension NSItemProvider {
    @MainActor fileprivate func load<T: _ObjectiveCBridgeable & Sendable>(_ type: T.Type) async -> T?
    where T._ObjectiveCType: NSItemProviderReading {
        await withCheckedContinuation { continuation in
            _ = loadObject(ofClass: type) { value, _ in continuation.resume(returning: value) }
        }
    }
}

/// A link that couldn't be read, with the button most likely to fix it.
struct FetchErrorRow: View {
    @Environment(MainModel.self) private var model
    let error: FriendlyError
    @State private var choosingBrowser = false
    @State private var updating = false

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.warning).accessibilityHidden(true)
            Text(error.message)
                .font(.system(size: 12, weight: .medium)).foregroundStyle(.white.opacity(0.85))
                .lineLimit(3).textSelection(.enabled)
            Spacer(minLength: 8)
            if updating {
                ProgressView().controlSize(.small)
            } else if let fix = error.fix {
                Button(fix.title) { perform(fix) }
                    .buttonStyle(SecondaryCapsuleStyle())
                    .popover(isPresented: $choosingBrowser, arrowEdge: .bottom) {
                        BrowserPicker(current: model.options.network.cookieBrowser) { browser in
                            choosingBrowser = false
                            model.options.network.cookieBrowser = browser
                            model.refetch()
                        }
                    }
            }
        }
        .padding(.leading, 4)
    }

    private func perform(_ fix: FriendlyError.Fix) {
        switch fix {
        case .browserSignIn: choosingBrowser = true
        case .updateAndRetry:
            updating = true
            Task {
                await ToolManager.shared.updateYtdlp()
                updating = false
                model.refetch()
            }
        case .waitForStart: model.downloadWhenStreamStarts()
        case .retry, .chooseFolder, .useBestQuality: model.refetch()
        }
    }
}

struct InfoCard: View {
    @Environment(MainModel.self) private var model
    let info: MediaInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .center, spacing: 14) {
                Thumbnail(url: info.thumbnail, width: 120)
                VStack(alignment: .leading, spacing: 5) {
                    Text(info.title).font(.system(size: 15, weight: .semibold)).lineLimit(2).textSelection(.enabled)
                    Text(subtitle).font(.system(size: 12)).foregroundStyle(Theme.secondary).lineLimit(1)
                    HStack(spacing: 6) {
                        if info.isLive { Tag(text: "LIVE", color: .red) }
                        if !model.options.format.exactFormat.isEmpty { Tag(text: "Custom format", color: .white) }
                    }
                }
                Spacer(minLength: 0)
            }
            if info.isPlaylist && !info.entries.isEmpty {
                PlaylistPicker(info: info)
            }
        }
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Color.white.opacity(0.04)))
        .overlay(RoundedRectangle(cornerRadius: 16, style: .continuous).strokeBorder(Theme.stroke))
        .transition(.opacity.combined(with: .move(edge: .top)))
    }

    private var subtitle: String {
        var parts: [String] = []
        if let uploader = info.uploader { parts.append(uploader) }
        if info.isPlaylist { parts.append(info.entries.count == 1 ? "1 item" : "\(info.entries.count) items") }
        if let d = Format.duration(info.duration) { parts.append(d) }
        return parts.joined(separator: "  ·  ")
    }
}

struct PlaylistPicker: View {
    @Environment(MainModel.self) private var model
    let info: MediaInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("\(model.selectedEntries.count) of \(info.entries.count) selected")
                    .font(.system(size: 12)).foregroundStyle(Theme.secondary)
                Spacer()
                Button("Select All") { model.selectedEntries = Set(info.entries.map(\.index)) }
                Button("None") { model.selectedEntries = [] }
            }
            .buttonStyle(.plain)
            .font(.system(size: 12, weight: .medium))
            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(info.entries) { entry in
                        row(entry, selected: model.selectedEntries.contains(entry.index))
                    }
                }
            }
            .frame(height: min(CGFloat(info.entries.count) * 30, 170))
        }
    }

    private func row(_ entry: MediaInfo.Entry, selected: Bool) -> some View {
        Button {
            if selected { model.selectedEntries.remove(entry.index) } else { model.selectedEntries.insert(entry.index) }
        } label: {
            HStack(spacing: 10) {
                Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(selected ? .white : Theme.tertiary)
                Text("\(entry.index)").monospacedDigit().foregroundStyle(Theme.tertiary).frame(
                    width: 32, alignment: .trailing)
                Text(entry.title).lineLimit(1).foregroundStyle(selected ? .white : Theme.secondary)
                Spacer()
                if let d = Format.duration(entry.duration) { Text(d).monospacedDigit().foregroundStyle(Theme.tertiary) }
            }
            .font(.system(size: 12))
            .padding(.horizontal, 10).frame(height: 28)
            .background(RoundedRectangle(cornerRadius: 8).fill(selected ? Color.white.opacity(0.05) : .clear))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(entry.title)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
