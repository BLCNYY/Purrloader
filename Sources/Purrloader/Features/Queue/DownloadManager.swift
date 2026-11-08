import AppKit
import Observation
import PurrloaderCore
import UserNotifications

enum JobStatus: Equatable {
    case queued, starting, downloading, processing, paused, finished, cancelled
    case failed(FriendlyError)

    var isActive: Bool { self == .starting || self == .downloading || self == .processing }

    var isDone: Bool {
        switch self {
        case .finished, .cancelled, .failed: true
        default: false
        }
    }

    /// Finished and cancelled jobs sit under "Recent"; failed ones stay up top until dealt with.
    var isRecent: Bool { self == .finished || self == .cancelled }

    var error: FriendlyError? {
        if case .failed(let error) = self { error } else { nil }
    }
}

@MainActor @Observable
final class DownloadJob: Identifiable {
    let id = UUID()
    let url: String
    var title: String
    let thumbnail: URL?
    var options: DownloadOptions
    var folder: URL
    /// Ticked playlist items in yt-dlp -I syntax; nil downloads what the options say.
    let playlistItems: String?

    var status: JobStatus = .queued
    var progress = DownloadProgress()
    var log: [String] = []
    var files: [URL] = []
    var command: [String] = []

    @ObservationIgnored var process: ChildProcess?
    @ObservationIgnored var stopReason: JobStatus?
    /// A blocked download gets one automatic yt-dlp update and retry.
    @ObservationIgnored var didAutoUpdate = false

    init(
        url: String, title: String?, thumbnail: URL?, options: DownloadOptions, folder: URL,
        playlistItems: String? = nil
    ) {
        self.url = url
        self.title = title ?? url
        self.thumbnail = thumbnail
        self.options = options
        self.folder = folder
        self.playlistItems = playlistItems
        command =
            [Tool.ytdlp.path.path] + makeCommand().download(url: url, folder: folder, playlistItems: playlistItems)
    }

    func makeCommand() -> YtDlpCommand {
        YtDlpCommand(options: options, environment: .installed(for: options))
    }

    var commandLine: String { Shell.commandLine(command) }

    var currentItemTitle: String? { progress.isPlaylist ? progress.reportedTitle : nil }
    var fraction: Double? { status == .finished ? 1 : progress.fraction }

    func appendLog(_ line: String) {
        log.append(line)
        if log.count > 4000 { log.removeFirst(log.count - 4000) }
    }
}

@MainActor @Observable
final class DownloadManager {
    static let shared = DownloadManager()
    static let queueSchemaVersion = 1

    var jobs: [DownloadJob] = []

    var maxConcurrent: Int {
        get {
            access(keyPath: \.maxConcurrent)
            return max(1, UserDefaults.standard.integer(forKey: "maxConcurrent"))
        }
        set {
            withMutation(keyPath: \.maxConcurrent) { UserDefaults.standard.set(newValue, forKey: "maxConcurrent") }
            schedule()
        }
    }

    var activeCount: Int { jobs.filter { $0.status.isActive }.count }
    var queuedCount: Int { jobs.filter { $0.status == .queued }.count }

    /// Average progress of active jobs, for the menu bar.
    var overallFraction: Double? {
        let active = jobs.filter { $0.status.isActive }
        guard !active.isEmpty else { return nil }
        return active.map { $0.fraction ?? 0 }.reduce(0, +) / Double(active.count)
    }

    @ObservationIgnored private var askedForNotifications = false

    private init() {
        UserDefaults.standard.register(defaults: ["maxConcurrent": 3, "notifications": true, "clipboardAutofill": true])
        restoreQueue()
    }

    func enqueue(_ job: DownloadJob) {
        jobs.insert(job, at: 0)
        requestNotificationPermissionOnce()
        schedule()
    }

    func schedule() {
        guard ToolManager.shared.isReady else { return }
        // Oldest queued first; jobs are stored newest first.
        while activeCount < maxConcurrent, let next = jobs.last(where: { $0.status == .queued }) {
            start(next)
        }
        stateChanged()
    }

    private func start(_ job: DownloadJob) {
        job.status = .starting
        job.progress = DownloadProgress()
        job.stopReason = nil
        job.files = []

        let filepathLog = AppPaths.temp.appendingPathComponent("\(job.id.uuidString).paths")
        try? FileManager.default.removeItem(at: filepathLog)
        let args = job.makeCommand().download(
            url: job.url, folder: job.folder, playlistItems: job.playlistItems, filepathLog: filepathLog)
        job.command = [Tool.ytdlp.path.path] + args
        job.appendLog("$ " + job.commandLine)

        do {
            // Not the download folder: a working directory inside ~/Downloads makes yt-dlp's startup wait on a privacy check.
            let (process, lines) = try ChildProcess.lines(Tool.ytdlp.executable, args, currentDirectory: AppPaths.temp)
            job.process = process
            Task {
                for await line in lines { handle(line.text, job: job) }
                finish(job, code: await process.exitCode(), filepathLog: filepathLog)
            }
        } catch {
            job.appendLog("Couldn't start yt-dlp: \(error.localizedDescription)")
            job.status = .failed(FriendlyError(kind: .unexpectedExit))
            schedule()
        }
    }

    private func handle(_ line: String, job: DownloadJob) {
        if job.progress.consume(line) { job.appendLog(line) }
        guard job.status.isActive else { return }
        switch job.progress.phase {
        case .starting: job.status = .starting
        case .downloading: job.status = .downloading
        case .processing: job.status = .processing
        }
        if !job.progress.isPlaylist, job.title == job.url, let title = job.progress.reportedTitle { job.title = title }
    }

    private func finish(_ job: DownloadJob, code: Int32, filepathLog: URL) {
        job.process = nil
        if let paths = try? String(contentsOf: filepathLog, encoding: .utf8) {
            job.files = paths.split(separator: "\n").map { URL(fileURLWithPath: String($0)) }
            try? FileManager.default.removeItem(at: filepathLog)
        }

        if let reason = job.stopReason {
            job.status = reason
            if reason == .cancelled { removePartials(of: job) }
        } else if code == 0 {
            job.status = .finished
            let sizes = job.files.compactMap { try? $0.resourceValues(forKeys: [.fileSizeKey]).fileSize }
            if !sizes.isEmpty { job.progress.totalBytes = Int64(sizes.reduce(0, +)) }
            HistoryStore.shared.record(job)
            notify(title: "Downloaded", body: job.title, file: job.files.first)
        } else {
            let error = job.progress.lastError.map(FriendlyError.init(raw:)) ?? FriendlyError(kind: .unexpectedExit)
            if error.kind == .blocked && !job.didAutoUpdate {
                updateAndRetry(job)
                return
            }
            job.status = .failed(error)
            notify(title: "Couldn't download", body: "\(job.title)\n\(error.message)")
        }
        schedule()
    }

    /// Sites often block old yt-dlp versions; a fresh one usually gets through.
    private func updateAndRetry(_ job: DownloadJob) {
        job.didAutoUpdate = true
        job.status = .processing
        job.progress.stage = "Updating helper tools, then trying again…"
        job.appendLog("[purrloader] Blocked; updating yt-dlp and retrying once.")
        Task {
            await ToolManager.shared.updateYtdlp(silently: true)
            // Unless it was cancelled or paused meanwhile.
            guard job.status == .processing, job.process == nil else { return }
            job.status = .queued
            schedule()
        }
        schedule()
    }

    // MARK: Controls

    /// Pausing stops yt-dlp; resuming restarts it and yt-dlp continues the .part files.
    func pause(_ job: DownloadJob) {
        if job.status == .queued {
            job.status = .paused
            stateChanged()
            return
        }
        stop(job, as: .paused)
    }

    func resume(_ job: DownloadJob) {
        guard job.status == .paused else { return }
        job.status = .queued
        schedule()
    }

    func cancel(_ job: DownloadJob) {
        if job.status.isActive {
            stop(job, as: .cancelled)
        } else {
            job.status = .cancelled
            removePartials(of: job)
        }
        stateChanged()
    }

    /// Retries, optionally with changed settings: a browser to sign in with, a different folder.
    func retry(_ job: DownloadJob, adjusting adjust: (inout DownloadOptions) -> Void = { _ in }, folder: URL? = nil) {
        guard !job.status.isActive else { return }
        adjust(&job.options)
        if let folder { job.folder = folder }
        job.status = .queued
        job.progress = DownloadProgress()
        schedule()
    }

    func remove(_ job: DownloadJob) {
        guard !job.status.isActive else { return }
        jobs.removeAll { $0.id == job.id }
        stateChanged()
    }

    func clearRecent() {
        jobs.removeAll { $0.status.isRecent }
        stateChanged()
    }

    func pauseAll() { jobs.filter { $0.status.isActive || $0.status == .queued }.forEach(pause) }
    func resumeAll() { jobs.filter { $0.status == .paused }.forEach(resume) }

    /// Stops running downloads (yt-dlp keeps the .part files) and saves them as queued,
    /// so they pick up where they left off next launch.
    func stopAllForQuit() {
        saveQueue()
        for job in jobs where job.status.isActive {
            job.stopReason = .paused
            job.process?.interrupt()
        }
    }

    func reveal(_ job: DownloadJob) {
        let existing = job.files.filter { FileManager.default.fileExists(atPath: $0.path) }
        if existing.isEmpty {
            NSWorkspace.shared.open(job.folder)
        } else {
            NSWorkspace.shared.activateFileViewerSelecting(existing)
        }
    }

    private func stop(_ job: DownloadJob, as reason: JobStatus) {
        guard let process = job.process, process.isRunning else {
            job.status = reason
            return
        }
        job.stopReason = reason
        job.progress.stage = reason == .paused ? "Pausing…" : "Cancelling…"
        process.interrupt()
        Task {
            try? await Task.sleep(for: .seconds(5))
            if process.isRunning { process.terminate() }
        }
    }

    private func removePartials(of job: DownloadJob) {
        let fm = FileManager.default
        for destination in job.progress.destinations {
            let path = destination.hasPrefix("/") ? destination : job.folder.appendingPathComponent(destination).path
            for suffix in [".part", ".ytdl", ".part.ytdl"] { try? fm.removeItem(atPath: path + suffix) }
            let dir = (path as NSString).deletingLastPathComponent
            let name = (path as NSString).lastPathComponent
            for file in (try? fm.contentsOfDirectory(atPath: dir)) ?? [] where file.hasPrefix(name + ".part-Frag") {
                try? fm.removeItem(atPath: (dir as NSString).appendingPathComponent(file))
            }
        }
    }

    // MARK: Persistence

    private struct SavedJob: Codable {
        enum Status: String, Codable { case queued, paused, finished, cancelled, failed }

        var url: String
        var title: String
        var thumbnail: URL?
        var options: DownloadOptions
        var folder: URL
        var playlistItems: String?
        var status: Status
        var error: FriendlyError?
        var files: [URL]
        var totalBytes: Int64
    }

    private func saveQueue() {
        let saved = jobs.prefix(200).map { job -> SavedJob in
            let status: SavedJob.Status
            switch job.status {
            // A running job is saved as queued: if Purrloader quits now it resumes next launch.
            case .queued, .starting, .downloading, .processing: status = .queued
            case .paused: status = .paused
            case .finished: status = .finished
            case .cancelled: status = .cancelled
            case .failed: status = .failed
            }
            return SavedJob(
                url: job.url, title: job.title, thumbnail: job.thumbnail, options: job.options, folder: job.folder,
                playlistItems: job.playlistItems, status: status, error: job.status.error, files: job.files,
                totalBytes: job.progress.totalBytes)
        }
        JSONFile.save(saved, to: AppPaths.queue, schemaVersion: Self.queueSchemaVersion)
    }

    private func restoreQueue() {
        guard let saved = JSONFile.load([SavedJob].self, from: AppPaths.queue, schemaVersion: Self.queueSchemaVersion)
        else { return }
        jobs = saved.map { item in
            let job = DownloadJob(
                url: item.url, title: item.title, thumbnail: item.thumbnail, options: item.options,
                folder: item.folder, playlistItems: item.playlistItems)
            job.files = item.files
            job.progress.totalBytes = item.totalBytes
            switch item.status {
            case .queued: job.status = .queued  // interrupted by quit: resumes once tools are ready
            case .paused: job.status = .paused
            case .finished: job.status = .finished
            case .cancelled: job.status = .cancelled
            case .failed: job.status = .failed(item.error ?? FriendlyError(kind: .unexpectedExit))
            }
            return job
        }
    }

    /// Saves the queue and refreshes the Dock badge.
    private func stateChanged() {
        saveQueue()
        let count = activeCount + queuedCount
        NSApp?.dockTile.badgeLabel = count > 0 ? "\(count)" : nil
    }

    // MARK: Notifications

    /// Asked on the first download rather than at launch, when it's clear what notifications are for.
    private func requestNotificationPermissionOnce() {
        guard !askedForNotifications, UserDefaults.standard.bool(forKey: "notifications") else { return }
        askedForNotifications = true
        Task { _ = try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) }
    }

    /// Clicking a notification with a file shows it in Finder (see AppDelegate).
    private func notify(title: String, body: String, file: URL? = nil) {
        guard UserDefaults.standard.bool(forKey: "notifications") else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        if let file { content.userInfo = ["file": file.path] }
        UNUserNotificationCenter.current().add(
            UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil))
    }
}

extension DownloadJob {
    var statusText: String {
        let p = progress
        switch status {
        case .queued: return "Waiting…"
        case .paused:
            return p.totalBytes > 0
                ? "Paused · \(Format.bytes(p.downloadedBytes)) of \(Format.bytes(p.totalBytes))" : "Paused"
        case .cancelled: return "Cancelled"
        case .failed(let error): return error.message
        case .finished:
            if files.count > 1 { return "Done · \(files.count) files" }
            return p.totalBytes > 0 ? "Done · \(Format.bytes(p.totalBytes))" : "Done"
        case .starting, .processing:
            return p.stage.isEmpty ? "Working…" : p.stage
        case .downloading:
            var parts: [String] = []
            if let i = p.itemIndex, let n = p.itemCount { parts.append("Item \(i) of \(n)") }
            if p.totalBytes > 0 {
                parts.append("\(Format.bytes(p.downloadedBytes)) of \(Format.bytes(p.totalBytes))")
            } else if p.downloadedBytes > 0 {
                parts.append(Format.bytes(p.downloadedBytes))
            }
            if p.speed > 0 { parts.append("\(Format.bytes(Int64(p.speed)))/s") }
            if let eta = p.eta, eta > 0, let left = Format.duration(Double(eta)) { parts.append("\(left) left") }
            if !p.stage.isEmpty { parts.append(p.stage) }
            return parts.isEmpty ? "Downloading…" : parts.joined(separator: " · ")
        }
    }
}
