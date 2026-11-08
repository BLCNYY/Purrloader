import PurrloaderCore
import SwiftUI

struct QueueView: View {
    @State private var detailsJob: DownloadJob?
    private var manager: DownloadManager { .shared }

    var body: some View {
        let current = manager.jobs.filter { !$0.status.isRecent }
        let recent = manager.jobs.filter { $0.status.isRecent }
        Group {
            if manager.jobs.isEmpty {
                empty
            } else {
                ScrollView {
                    LazyVStack(spacing: 8) {
                        ForEach(current) { row($0) }
                        if !recent.isEmpty {
                            recentHeader.padding(.top, current.isEmpty ? 0 : 10)
                            ForEach(recent) { row($0) }
                        }
                    }
                    .padding(16)
                    .animation(.snappy(duration: 0.3), value: manager.jobs.map(\.id))
                    .animation(.snappy(duration: 0.3), value: recent.map(\.id))
                }
            }
        }
        .sheet(item: $detailsJob) { JobDetailsView(job: $0).purrloaderSheetStyle() }
    }

    private func row(_ job: DownloadJob) -> some View {
        JobRow(job: job, showDetails: { detailsJob = job })
            .transition(.asymmetric(insertion: .move(edge: .top).combined(with: .opacity), removal: .opacity))
    }

    private var empty: some View {
        VStack(spacing: 10) {
            Image(systemName: "arrow.down")
                .font(.system(size: 20, weight: .medium))
                .foregroundStyle(Theme.tertiary)
                .frame(width: 52, height: 52)
                .overlay(Circle().strokeBorder(Theme.stroke, lineWidth: 1.5))
                .accessibilityHidden(true)
            Text("Paste. Purr. Done.").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.secondary)
            Text("Paste a link above, or drop one anywhere in this window.")
                .font(.system(size: 12)).foregroundStyle(Theme.tertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var recentHeader: some View {
        HStack {
            Text("Recent").font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.tertiary)
                .accessibilityAddTraits(.isHeader)
            Spacer()
            Button("Clear") { withAnimation(.snappy(duration: 0.3)) { manager.clearRecent() } }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(Theme.secondary)
                .help("Remove finished downloads from this list. The files stay where they are. (⌘K)")
        }
        .padding(.horizontal, 6)
    }
}

struct JobRow: View {
    @Environment(MainModel.self) private var model
    let job: DownloadJob
    let showDetails: () -> Void
    @State private var hovering = false
    @State private var choosingBrowser = false
    @State private var updating = false
    private var manager: DownloadManager { .shared }

    var body: some View {
        HStack(spacing: 14) {
            Thumbnail(url: job.thumbnail, width: job.status.isRecent ? 64 : 80)
                .opacity(job.status == .cancelled ? 0.5 : 1)
            VStack(alignment: .leading, spacing: 7) {
                Text(job.title).font(.system(size: 13, weight: .semibold)).lineLimit(1).truncationMode(.middle)
                if job.status.isActive {
                    SlimProgress(fraction: job.fraction)
                }
                HStack(spacing: 6) {
                    statusIcon
                    Text(job.statusText).lineLimit(job.status.error == nil ? 1 : 2).truncationMode(.tail)
                    if job.status.isActive, let f = job.fraction {
                        Spacer()
                        Text("\(Int(f * 100))%").foregroundStyle(Theme.ink.opacity(0.8))
                    }
                }
                .font(.system(size: 11)).foregroundStyle(Theme.secondary).monospacedDigit()
                .help(job.statusText)
                if let item = job.currentItemTitle, job.status.isActive {
                    Text(item).font(.system(size: 11)).foregroundStyle(Theme.tertiary).lineLimit(1)
                }
            }
            Spacer(minLength: 8)
            buttons
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Theme.ink.opacity(hovering ? 0.07 : 0.04))
        )
        .overlay(RoundedRectangle(cornerRadius: 16, style: .continuous).strokeBorder(Theme.stroke))
        .onHover { hovering = $0 }
        .animation(.easeOut(duration: 0.12), value: hovering)
        .contextMenu {
            if job.status == .finished, let file = job.files.first { Button("Open") { NSWorkspace.shared.open(file) } }
            Button("Show in Finder") { manager.reveal(job) }
            Button("Copy Link") { Clipboard.copy(job.url) }
            Button("Show Details") { showDetails() }
            Divider()
            Button("Remove from List") { manager.remove(job) }.disabled(job.status.isActive)
        }
        .onTapGesture(count: 2) {
            if job.status == .finished { manager.reveal(job) } else { showDetails() }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(job.title)
        .accessibilityValue(job.statusText)
    }

    @ViewBuilder private var statusIcon: some View {
        switch job.status {
        case .finished: Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .failed: Image(systemName: "exclamationmark.circle.fill").foregroundStyle(.red)
        case .paused: Image(systemName: "pause.circle.fill")
        case .cancelled: Image(systemName: "xmark.circle.fill")
        default: EmptyView()
        }
    }

    private var buttons: some View {
        HStack(spacing: 4) {
            switch job.status {
            case .queued, .starting, .downloading, .processing:
                icon("pause.fill", "Pause") { manager.pause(job) }
                icon("xmark", "Cancel") { manager.cancel(job) }
            case .paused:
                icon("play.fill", "Resume") { manager.resume(job) }
                icon("xmark", "Cancel") { manager.cancel(job) }
            case .failed(let error):
                if updating {
                    ProgressView().controlSize(.small).padding(.horizontal, 8)
                } else if let fix = error.fix {
                    fixButton(fix)
                }
                if error.fix != .retry { icon("arrow.clockwise", "Retry") { manager.retry(job) } }
                icon("trash", "Remove") { manager.remove(job) }
            case .cancelled:
                icon("arrow.clockwise", "Download Again") { manager.retry(job) }
                icon("trash", "Remove") { manager.remove(job) }
            case .finished:
                icon("magnifyingglass", "Show in Finder") { manager.reveal(job) }
                icon("trash", "Remove from list") { manager.remove(job) }
            }
            icon("text.alignleft", "Details") { showDetails() }
        }
        .opacity(hovering || job.status.isActive || job.status.error != nil ? 1 : 0.6)
    }

    private func fixButton(_ fix: FriendlyError.Fix) -> some View {
        Button(fix.title) { perform(fix) }
            .buttonStyle(SecondaryCapsuleStyle())
            .fixedSize()
            .popover(isPresented: $choosingBrowser, arrowEdge: .bottom) {
                BrowserPicker(current: job.options.network.cookieBrowser) { browser in
                    choosingBrowser = false
                    // Remember it, so the next private video works straight away.
                    model.options.network.cookieBrowser = browser
                    manager.retry(job) { $0.network.cookieBrowser = browser }
                }
            }
    }

    private func perform(_ fix: FriendlyError.Fix) {
        switch fix {
        case .browserSignIn: choosingBrowser = true
        case .updateAndRetry:
            updating = true
            Task {
                await ToolManager.shared.updateYtdlp()
                updating = false
                manager.retry(job)
            }
        case .chooseFolder:
            guard let folder = MainModel.pickFolder(startingAt: job.folder) else { return }
            model.folder = folder
            manager.retry(job, folder: folder)
        case .retry: manager.retry(job)
        case .useBestQuality:
            manager.retry(job) {
                $0.format.maxHeight = nil
                $0.format.exactFormat = ""
            }
        case .waitForStart: manager.retry(job) { $0.extras.waitForVideo = true }
        }
    }

    private func icon(_ symbol: String, _ label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) { Image(systemName: symbol) }
            .buttonStyle(CircleButtonStyle(size: 28))
            .help(label)
            .accessibilityLabel(label)
    }
}

/// What happened during a download: its full log, and the command behind it for the curious.
struct JobDetailsView: View {
    let job: DownloadJob
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text(job.title).font(.system(size: 15, weight: .semibold)).lineLimit(2)
                Text(job.statusText).font(.system(size: 12)).foregroundStyle(Theme.secondary).lineLimit(2)
            }

            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 1) {
                        ForEach(Array(job.log.enumerated()), id: \.offset) { index, line in
                            Text(line)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(
                                    line.hasPrefix("ERROR") ? .red : line.hasPrefix("WARNING") ? .orange : .primary
                                )
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .id(index)
                        }
                    }
                    .textSelection(.enabled)
                    .padding(8)
                }
                .background(Theme.surface, in: RoundedRectangle(cornerRadius: 8))
                .onChange(of: job.log.count) { proxy.scrollTo(job.log.count - 1, anchor: .bottom) }
                .onAppear { proxy.scrollTo(job.log.count - 1, anchor: .bottom) }
            }

            DisclosureGroup("Command") {
                HStack(alignment: .top) {
                    Text(job.commandLine)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    Button {
                        Clipboard.copy(job.commandLine)
                    } label: {
                        Image(systemName: "doc.on.doc")
                    }
                    .buttonStyle(.borderless).help("Copy command").accessibilityLabel("Copy command")
                }
                .padding(.top, 6)
            }
            .font(.system(size: 12))

            HStack {
                Button("Copy Log") { Clipboard.copy(job.log.joined(separator: "\n")) }
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(minWidth: 680, idealWidth: 760, minHeight: 440, idealHeight: 540)
    }
}
