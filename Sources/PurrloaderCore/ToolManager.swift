import CryptoKit
import Foundation
import OSLog
import Observation
import Synchronization

public enum Tool: String, CaseIterable, Identifiable, Sendable {
    case ytdlp = "yt-dlp"
    case ffmpeg
    case ffprobe
    case deno

    public var id: String { rawValue }

    /// yt-dlp is the folder ("onedir") build: it starts in a fraction of a second, where the
    /// single-file build unpacks itself for ~4 s on every run. Each version gets its own folder and
    /// `current` points at the newest, so an update never pulls files from under a running download.
    public var path: URL {
        self == .ytdlp
            ? Self.ytdlpCurrent.appendingPathComponent("yt-dlp_macos") : AppPaths.bin.appendingPathComponent(rawValue)
    }

    /// The real file behind `path`, for launching: a running yt-dlp keeps its own version's folder.
    public var executable: URL { path.resolvingSymlinksInPath() }

    public static let ytdlpBuilds = AppPaths.bin.appendingPathComponent("yt-dlp", isDirectory: true)
    public static let ytdlpCurrent = ytdlpBuilds.appendingPathComponent("current", isDirectory: true)

    public var isInstalled: Bool { FileManager.default.isExecutableFile(atPath: path.path) }

    public var versionArguments: [String] { self == .ffmpeg || self == .ffprobe ? ["-version"] : ["--version"] }
}

public enum UpdateChannel: String, CaseIterable, Identifiable, Codable, Sendable {
    case stable, nightly
    public var id: String { rawValue }
    public var title: String { rawValue.capitalized }
    public var ytdlpRepository: String { self == .nightly ? "yt-dlp/yt-dlp-nightly-builds" : "yt-dlp/yt-dlp" }
}

/// Where a tool's archive and its publisher's checksum file live.
public struct ToolSource: Sendable {
    var archive: URL
    var checksums: URL
    public var archiveName: String { archive.lastPathComponent }

    #if arch(arm64)
        static let ffmpegArch = "arm64", denoTriple = "aarch64-apple-darwin"
    #else
        static let ffmpegArch = "amd64", denoTriple = "x86_64-apple-darwin"
    #endif

    /// Pins the latest release first, so the archive and its checksums come from the same version.
    public static func resolve(_ tool: Tool, channel: UpdateChannel) async throws -> ToolSource {
        switch tool {
        case .ytdlp:
            let repository = channel.ytdlpRepository
            let tag = try await latestTag(of: repository)
            let base = URL(string: "https://github.com/\(repository)/releases/download/\(tag)/")!
            // yt-dlp publishes one SHA2-256SUMS file per release.
            return ToolSource(
                archive: base.appending(path: "yt-dlp_macos.zip"), checksums: base.appending(path: "SHA2-256SUMS"))
        case .deno:
            let tag = try await latestTag(of: "denoland/deno")
            let archive = URL(
                string: "https://github.com/denoland/deno/releases/download/\(tag)/deno-\(denoTriple).zip")!
            return ToolSource(archive: archive, checksums: archive.appendingPathExtension("sha256sum"))
        case .ffmpeg, .ffprobe:
            // martin-riedl.de redirects "latest" to a versioned path that has a .sha256 next to the zip.
            let redirect = URL(
                string:
                    "https://ffmpeg.martin-riedl.de/redirect/latest/macos/\(ffmpegArch)/release/\(tool.rawValue).zip")!
            let archive = try await finalURL(of: redirect)
            return ToolSource(archive: archive, checksums: archive.appendingPathExtension("sha256"))
        }
    }

    /// The newest release tag, read from where github.com/…/releases/latest redirects. Unlike the API,
    /// this has no rate limit.
    public static func latestTag(of repository: String) async throws -> String {
        let url = try await finalURL(of: URL(string: "https://github.com/\(repository)/releases/latest")!)
        guard url.pathComponents.dropLast().last == "tag" else { throw ToolError.unexpectedResponse }
        return url.lastPathComponent
    }

    private static func finalURL(of url: URL) async throws -> URL {
        var request = URLRequest(url: url)
        request.httpMethod = "HEAD"
        let (_, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200, let final = http.url else {
            throw ToolError.unexpectedResponse
        }
        return final
    }
}

public enum Checksums {
    /// Finds the SHA-256 for `fileName` in a checksum file: `<hash>  <name>` lines (sha256sum style,
    /// with an optional `*` before binary names), or a file holding just the hash.
    public static func sha256(for fileName: String, in text: String) -> String? {
        let lines = text.split(whereSeparator: \.isNewline).map {
            $0.split(whereSeparator: \.isWhitespace).map(String.init)
        }
        func isHash(_ s: String) -> Bool { s.count == 64 && s.allSatisfy(\.isHexDigit) }
        for fields in lines where fields.count >= 2 && isHash(fields[0]) {
            let name = fields[1].hasPrefix("*") ? String(fields[1].dropFirst()) : fields[1]
            if name == fileName { return fields[0].lowercased() }
        }
        if lines.count == 1, let only = lines.first, only.count == 1, isHash(only[0]) { return only[0].lowercased() }
        return nil
    }

    public static func sha256(ofFile url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let chunk = try handle.read(upToCount: 1 << 20), !chunk.isEmpty { hasher.update(data: chunk) }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }
}

/// A file download that reports progress, which the async URLSession API doesn't.
public final class ProgressDownload: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
    private let destination: URL
    private let progress: @Sendable (Int64, Int64) -> Void
    private let state = Mutex<(continuation: CheckedContinuation<Void, Error>?, moveError: Error?)>((nil, nil))

    private init(destination: URL, progress: @escaping @Sendable (Int64, Int64) -> Void) {
        self.destination = destination
        self.progress = progress
    }

    public static func run(
        _ url: URL, to destination: URL, progress: @escaping @Sendable (_ written: Int64, _ expected: Int64) -> Void
    ) async throws {
        let delegate = ProgressDownload(destination: destination, progress: progress)
        let session = URLSession(configuration: .ephemeral, delegate: delegate, delegateQueue: nil)
        defer { session.finishTasksAndInvalidate() }
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                delegate.state.withLock { $0.continuation = continuation }
                session.downloadTask(with: url).resume()
            }
        } onCancel: {
            session.invalidateAndCancel()
        }
    }

    public func urlSession(
        _ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64,
        totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64
    ) {
        progress(totalBytesWritten, totalBytesExpectedToWrite)
    }

    public func urlSession(
        _ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL
    ) {
        // The temporary file is deleted when this returns, so move it now.
        do {
            if let http = downloadTask.response as? HTTPURLResponse, http.statusCode != 200 {
                throw ToolError.http(http.statusCode)
            }
            try? FileManager.default.removeItem(at: destination)
            try FileManager.default.moveItem(at: location, to: destination)
        } catch {
            state.withLock { $0.moveError = error }
        }
    }

    public func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        let (continuation, moveError) = state.withLock { state in
            defer { state.continuation = nil }
            return (state.continuation, state.moveError)
        }
        if let failure = error ?? moveError { continuation?.resume(throwing: failure) } else { continuation?.resume() }
    }
}

public enum ToolError: LocalizedError, Equatable {
    case http(Int), unexpectedResponse, missingChecksum(Tool), checksumMismatch(Tool), unpack(Tool), missingExecutable(
        Tool)

    public var errorDescription: String? {
        switch self {
        case .http(let code): "The server answered with HTTP \(code)."
        case .unexpectedResponse: "The server sent something unexpected."
        case .missingChecksum(let tool): "No published checksum was found for \(tool.rawValue)."
        case .checksumMismatch(let tool):
            "The downloaded \(tool.rawValue) didn't match its published checksum, so it was deleted."
        case .unpack(let tool): "Couldn't unpack \(tool.rawValue)."
        case .missingExecutable(let tool): "\(tool.rawValue) wasn't where expected in its download."
        }
    }
}

/// Installs, verifies and updates the helper tools, and reports their versions.
@MainActor @Observable
public final class ToolManager {
    public static let shared = ToolManager()

    public struct Transfer: Equatable {
        public var written: Int64 = 0
        public var expected: Int64 = 0
    }

    public var versions: [Tool: String] = [:]
    /// Tools being downloaded right now, with their progress.
    public var transfers: [Tool: Transfer] = [:]
    /// Shown when the helper tools are missing and couldn't be installed.
    public var setupError: String?
    /// Shown when a silent yt-dlp update failed; cleared on success or dismissal.
    public var updateError: String?
    public var isUpdating = false
    public var updateLog = ""

    @ObservationIgnored private var updateTask: Task<Void, Never>?
    private static let log = Logger(subsystem: "com.blcnyy.purrloader", category: "tools")

    public var channel: UpdateChannel {
        get {
            access(keyPath: \.channel)
            return UpdateChannel(rawValue: UserDefaults.standard.string(forKey: "ytdlpChannel") ?? "") ?? .stable
        }
        set {
            withMutation(keyPath: \.channel) { UserDefaults.standard.set(newValue.rawValue, forKey: "ytdlpChannel") }
            Task { await updateYtdlp() }
        }
    }

    public var autoUpdate: Bool {
        get {
            access(keyPath: \.autoUpdate)
            return UserDefaults.standard.bool(forKey: "autoUpdateYtdlp")
        }
        set { withMutation(keyPath: \.autoUpdate) { UserDefaults.standard.set(newValue, forKey: "autoUpdateYtdlp") } }
    }

    public var isReady: Bool { Tool.allCases.allSatisfy { versions[$0] != nil } }
    public var isInstalling: Bool { !transfers.isEmpty }

    /// Combined progress of the tools being installed, for the welcome sheet.
    public var installProgress: Transfer? {
        guard !transfers.isEmpty else { return nil }
        return transfers.values.reduce(into: Transfer()) { total, transfer in
            total.written += transfer.written
            total.expected += max(transfer.expected, 0)
        }
    }

    private init() {
        UserDefaults.standard.register(defaults: ["autoUpdateYtdlp": true])
    }

    /// Installs whatever is missing, reads versions, then quietly updates yt-dlp.
    public func bootstrap() async {
        setupError = nil
        cleanUp()
        let missing = Tool.allCases.filter { !$0.isInstalled }
        if !missing.isEmpty {
            do {
                try await install(missing)
            } catch {
                Self.log.error("Installing tools failed: \(String(describing: error), privacy: .public)")
                setupError = error.localizedDescription
            }
        }
        await refreshVersions()
        if missing.isEmpty && autoUpdate { await updateYtdlp(silently: true) }
    }

    /// Downloads tools in parallel. Each archive is checked against its publisher's SHA-256
    /// before anything in it is unpacked or made executable.
    public func install(_ tools: [Tool]) async throws {
        for tool in tools { transfers[tool] = Transfer() }
        defer { for tool in tools { transfers[tool] = nil } }
        let channel = channel
        try await withThrowingTaskGroup(of: Void.self) { group in
            for tool in tools {
                group.addTask {
                    try await Self.retrying {
                        try await Self.installOne(tool, channel: channel) { written, expected in
                            Task { @MainActor in
                                if ToolManager.shared.transfers[tool] != nil {
                                    ToolManager.shared.transfers[tool] = Transfer(written: written, expected: expected)
                                }
                            }
                        }
                    }
                }
            }
            try await group.waitForAll()
        }
    }

    /// Download servers hiccup now and then (martin-riedl.de's "latest" redirect sometimes answers 404),
    /// so try a few times. A checksum mismatch is final.
    private nonisolated static func retrying(_ attempt: @Sendable () async throws -> Void) async throws {
        for delay in [1, 3] {
            do {
                return try await attempt()
            } catch let error as ToolError {
                if case .checksumMismatch = error { throw error }
            } catch is CancellationError {
                throw CancellationError()
            } catch {}
            try await Task.sleep(for: .seconds(delay))
        }
        try await attempt()
    }

    private nonisolated static func installOne(
        _ tool: Tool, channel: UpdateChannel,
        progress: @escaping @Sendable (Int64, Int64) -> Void
    ) async throws {
        let fm = FileManager.default
        let source = try await ToolSource.resolve(tool, channel: channel)
        let work = AppPaths.temp.appendingPathComponent(
            "install-\(tool.rawValue)-\(UUID().uuidString)", isDirectory: true)
        try fm.createDirectory(at: work, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: work) }

        async let checksumText = URLSession.shared.data(from: source.checksums)
        let archive = work.appendingPathComponent(source.archiveName)
        try await ProgressDownload.run(source.archive, to: archive, progress: progress)

        let (sums, response) = try await checksumText
        guard (response as? HTTPURLResponse)?.statusCode == 200,
            let expected = Checksums.sha256(for: source.archiveName, in: String(decoding: sums, as: UTF8.self))
        else { throw ToolError.missingChecksum(tool) }
        guard try Checksums.sha256(ofFile: archive) == expected else {
            try? fm.removeItem(at: archive)
            throw ToolError.checksumMismatch(tool)
        }

        let unpacked = work.appendingPathComponent("unpacked", isDirectory: true)
        let result = try await Shell.run(
            URL(fileURLWithPath: "/usr/bin/ditto"), ["-x", "-k", archive.path, unpacked.path])
        guard result.status == 0 else { throw ToolError.unpack(tool) }

        let executableName = tool.path.lastPathComponent
        guard
            let executable = fm.enumerator(at: unpacked, includingPropertiesForKeys: nil)?
                .compactMap({ $0 as? URL }).first(where: { $0.lastPathComponent == executableName })
        else { throw ToolError.missingExecutable(tool) }
        try fm.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        removexattr(executable.path, "com.apple.quarantine", 0)

        if tool == .ytdlp {
            let build = Tool.ytdlpBuilds.appendingPathComponent(UUID().uuidString, isDirectory: true)
            try fm.createDirectory(at: Tool.ytdlpBuilds, withIntermediateDirectories: true)
            try fm.moveItem(at: executable.deletingLastPathComponent(), to: build)
            // Swap the link in one step: write a new one beside it, then rename over the old.
            let link = Tool.ytdlpBuilds.appendingPathComponent("current-\(UUID().uuidString)")
            try fm.createSymbolicLink(atPath: link.path, withDestinationPath: build.lastPathComponent)
            guard rename(link.path, Tool.ytdlpCurrent.path) == 0 else { throw CocoaError(.fileWriteUnknown) }
        } else {
            if fm.fileExists(atPath: tool.path.path) {
                // Park the old binary: a running download may still be using it.
                try fm.moveItem(
                    at: tool.path, to: AppPaths.temp.appendingPathComponent("old-\(tool.rawValue)-\(UUID().uuidString)")
                )
            }
            try fm.moveItem(at: executable, to: tool.path)
        }
    }

    /// Removes leftovers from earlier runs: unpack folders, parked binaries and old yt-dlp versions.
    /// Runs before any download starts, so nothing is in use.
    private func cleanUp() {
        let fm = FileManager.default
        for name in (try? fm.contentsOfDirectory(atPath: AppPaths.temp.path)) ?? []
        where name.hasPrefix("old-") || name.hasPrefix("install-") {
            try? fm.removeItem(at: AppPaths.temp.appendingPathComponent(name))
        }
        let current = (try? fm.destinationOfSymbolicLink(atPath: Tool.ytdlpCurrent.path)) ?? ""
        for name in (try? fm.contentsOfDirectory(atPath: Tool.ytdlpBuilds.path)) ?? []
        where name != "current" && name != current {
            try? fm.removeItem(at: Tool.ytdlpBuilds.appendingPathComponent(name))
        }
    }

    public func reinstallAll() async {
        setupError = nil
        do {
            try await install(Tool.allCases)
        } catch {
            setupError = error.localizedDescription
        }
        await refreshVersions()
    }

    public func refreshVersions() async {
        // Run all version checks at once; a freshly installed binary's first launch is slow while macOS scans it.
        let outputs = await withTaskGroup(of: (Tool, String?).self) { group in
            for tool in Tool.allCases {
                group.addTask {
                    guard tool.isInstalled, let result = try? await Shell.run(tool.executable, tool.versionArguments),
                        result.status == 0
                    else { return (tool, nil) }
                    return (tool, result.output)
                }
            }
            var outputs: [Tool: String] = [:]
            for await (tool, output) in group { outputs[tool] = output }
            return outputs
        }
        for tool in Tool.allCases {
            versions[tool] = outputs[tool].map { Self.parseVersion($0, of: tool) }
        }
    }

    nonisolated public static func parseVersion(_ output: String, of tool: Tool) -> String {
        let first = output.split(separator: "\n").first.map(String.init)?.trimmed ?? ""
        switch tool {
        case .ytdlp:
            return first
        case .ffmpeg, .ffprobe:
            // "ffmpeg version 8.0-https://… Copyright …" -> "8.0"
            let parts = first.split(separator: " ")
            return parts.count > 2 ? String(parts[2].split(separator: "-").first ?? parts[2]) : first
        case .deno:
            // "deno 2.5.0 (stable, release, aarch64-apple-darwin)" -> "2.5.0"
            return first.split(separator: " ").dropFirst().first.map(String.init) ?? first
        }
    }

    /// Installs the newest yt-dlp for the current channel if it differs from the installed one.
    /// Several callers at once share one update. A silent update only reports failures.
    public func updateYtdlp(silently: Bool = false) async {
        if let updateTask {
            await updateTask.value
            return
        }
        let task = Task { await performUpdate(silently: silently) }
        updateTask = task
        await task.value
        updateTask = nil
    }

    private func performUpdate(silently: Bool) async {
        isUpdating = true
        defer { isUpdating = false }
        let before = versions[.ytdlp]
        do {
            let latest = try await ToolSource.latestTag(of: channel.ytdlpRepository)
            if latest != before {
                try await install([.ytdlp])
                await refreshVersions()
            }
            updateError = nil
            if let now = versions[.ytdlp], !silently {
                updateLog =
                    now == before
                    ? "yt-dlp \(now) is the newest version." : "Updated yt-dlp to \(now) (\(channel.title))."
            }
        } catch {
            Self.log.error("Updating yt-dlp failed: \(String(describing: error), privacy: .public)")
            updateError = error.localizedDescription
            updateLog = "Couldn't update yt-dlp: \(error.localizedDescription)"
        }
    }
}
