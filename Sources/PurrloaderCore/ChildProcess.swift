import Foundation
import Synchronization

public struct ProcessResult: Sendable {
    public var status: Int32
    public var stdout: Data
    public var stderr: Data
    public var output: String { String(decoding: stdout, as: UTF8.self) }
    public var errorOutput: String { String(decoding: stderr, as: UTF8.self) }
}

public struct OutputLine: Sendable, Equatable {
    public var text: String
    public var isError: Bool
}

/// A running helper process. Output arrives through its pipes' readability handlers and the
/// process counts as finished once it has exited and both pipes reached end of file.
public final class ChildProcess: Sendable {
    private struct State {
        var openPipes = 2
        var exitCode: Int32?
        var finished = false
        var waiters: [CheckedContinuation<Int32, Never>] = []
    }

    private let process: Process
    private let state = Mutex(State())
    private let onData: @Sendable (Data, _ isError: Bool) -> Void
    private let onFinish: @Sendable (Int32) -> Void

    private init(
        process: Process, onData: @escaping @Sendable (Data, Bool) -> Void,
        onFinish: @escaping @Sendable (Int32) -> Void
    ) {
        self.process = process
        self.onData = onData
        self.onFinish = onFinish
    }

    private static func launch(
        _ executable: URL, _ arguments: [String], currentDirectory: URL?,
        onData: @escaping @Sendable (Data, Bool) -> Void,
        onFinish: @escaping @Sendable (Int32) -> Void
    ) throws -> ChildProcess {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        process.environment = Shell.environment
        if let currentDirectory { process.currentDirectoryURL = currentDirectory }
        process.standardInput = FileHandle.nullDevice
        let out = Pipe(), err = Pipe()
        process.standardOutput = out
        process.standardError = err

        let child = ChildProcess(process: process, onData: onData, onFinish: onFinish)
        for (pipe, isError) in [(out, false), (err, true)] {
            pipe.fileHandleForReading.readabilityHandler = { handle in
                let data = handle.availableData
                if data.isEmpty {
                    handle.readabilityHandler = nil
                    child.update { $0.openPipes -= 1 }
                } else {
                    child.onData(data, isError)
                }
            }
        }
        process.terminationHandler = { process in
            child.update { $0.exitCode = process.terminationStatus }
            // A grandchild (ffmpeg) can hold the pipes open after yt-dlp is killed; don't wait on it forever.
            Task {
                try? await Task.sleep(for: .seconds(3))
                child.update { $0.openPipes = 0 }
            }
        }
        do {
            try process.run()
        } catch {
            out.fileHandleForReading.readabilityHandler = nil
            err.fileHandleForReading.readabilityHandler = nil
            throw error
        }
        return child
    }

    private func update(_ change: (inout State) -> Void) {
        let (code, waiters) = state.withLock { state -> (Int32?, [CheckedContinuation<Int32, Never>]) in
            change(&state)
            guard !state.finished, state.openPipes <= 0, let code = state.exitCode else { return (nil, []) }
            state.finished = true
            defer { state.waiters = [] }
            return (code, state.waiters)
        }
        guard let code else { return }
        onFinish(code)
        for waiter in waiters { waiter.resume(returning: code) }
    }

    public var isRunning: Bool { process.isRunning }

    /// Like Ctrl-C: yt-dlp stops ffmpeg and leaves resumable .part files behind.
    public func interrupt() { process.interrupt() }

    public func terminate() { process.terminate() }

    /// Waits until the process has exited and all its output was delivered.
    public func exitCode() async -> Int32 {
        await withCheckedContinuation { continuation in
            let code = state.withLock { state -> Int32? in
                if state.finished { return state.exitCode }
                state.waiters.append(continuation)
                return nil
            }
            if let code { continuation.resume(returning: code) }
        }
    }

    /// Starts a process and streams its output as lines (stdout and stderr, \r counting as a line break).
    public static func lines(
        _ executable: URL, _ arguments: [String], currentDirectory: URL? = nil
    ) throws
        -> (process: ChildProcess, lines: AsyncStream<OutputLine>)
    {
        let (stream, continuation) = AsyncStream.makeStream(of: OutputLine.self)
        let outBuffer = LineBuffer(), errBuffer = LineBuffer()
        let process = try launch(executable, arguments, currentDirectory: currentDirectory) { data, isError in
            for line in (isError ? errBuffer : outBuffer).append(data) {
                continuation.yield(OutputLine(text: line, isError: isError))
            }
        } onFinish: { _ in
            for line in outBuffer.flush() { continuation.yield(OutputLine(text: line, isError: false)) }
            for line in errBuffer.flush() { continuation.yield(OutputLine(text: line, isError: true)) }
            continuation.finish()
        }
        return (process, stream)
    }

    /// Runs a process to the end and collects its output.
    public static func run(_ executable: URL, _ arguments: [String]) async throws -> ProcessResult {
        let collected = Mutex((stdout: Data(), stderr: Data()))
        let process = try launch(executable, arguments, currentDirectory: nil) { data, isError in
            collected.withLock { isError ? $0.stderr.append(data) : $0.stdout.append(data) }
        } onFinish: { _ in
        }
        let status = await process.exitCode()
        return collected.withLock { ProcessResult(status: status, stdout: $0.stdout, stderr: $0.stderr) }
    }
}

/// Accumulates pipe output and hands back complete lines, treating \r as a line break too.
public final class LineBuffer: Sendable {
    private let buffer = Mutex((data: Data(), scanned: 0))

    public init() {}

    public func append(_ chunk: Data) -> [String] {
        buffer.withLock { state in
            state.data.append(chunk)
            var lines: [String] = []
            var start = state.data.startIndex
            // Only scan bytes not looked at before; a long JSON line can arrive in hundreds of chunks.
            var index = state.data.startIndex + state.scanned
            while index < state.data.endIndex {
                let byte = state.data[index]
                if byte == 0x0A || byte == 0x0D {
                    let line = String(decoding: state.data[start..<index], as: UTF8.self)
                    if !line.isEmpty { lines.append(line) }
                    start = index + 1
                }
                index += 1
            }
            state.data.removeSubrange(state.data.startIndex..<start)
            state.scanned = state.data.count
            return lines
        }
    }

    public func flush() -> [String] {
        buffer.withLock { state in
            defer { state = (Data(), 0) }
            let rest = String(decoding: state.data, as: UTF8.self)
            return rest.isEmpty ? [] : [rest]
        }
    }
}

public enum Shell {
    /// PATH with our managed tools first so yt-dlp finds ffprobe, Deno and aria2c.
    public static var environment: [String: String] {
        var env = ProcessInfo.processInfo.environment
        env["PATH"] = [
            AppPaths.bin.path, "/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin",
        ]
        .joined(separator: ":")
        env["PYTHONIOENCODING"] = "utf-8"
        return env
    }

    public static func run(_ executable: URL, _ arguments: [String]) async throws -> ProcessResult {
        try await ChildProcess.run(executable, arguments)
    }

    public static func which(_ name: String) -> URL? {
        for dir in (environment["PATH"] ?? "").split(separator: ":") {
            let url = URL(fileURLWithPath: String(dir)).appendingPathComponent(name)
            if FileManager.default.isExecutableFile(atPath: url.path) { return url }
        }
        return nil
    }

    public static func quote(_ arg: String) -> String {
        if !arg.isEmpty, arg.allSatisfy({ $0.isLetter || $0.isNumber || "-_./:=+,@%".contains($0) }) { return arg }
        return "'" + arg.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }

    public static func commandLine(_ args: [String]) -> String {
        args.map(quote).joined(separator: " ")
    }

    /// Splits a string into arguments the way a POSIX shell would (quotes and backslashes, no expansion).
    public static func split(_ text: String) -> [String] {
        var args: [String] = []
        var current = ""
        var hasToken = false
        var quote: Character?
        var escaped = false
        for ch in text {
            if escaped {
                current.append(ch)
                escaped = false
                hasToken = true
            } else if ch == "\\" && quote != "'" {
                escaped = true
            } else if let q = quote {
                if ch == q { quote = nil } else { current.append(ch) }
            } else if ch == "'" || ch == "\"" {
                quote = ch
                hasToken = true
            } else if ch.isWhitespace {
                if hasToken {
                    args.append(current)
                    current = ""
                    hasToken = false
                }
            } else {
                current.append(ch)
                hasToken = true
            }
        }
        if hasToken { args.append(current) }
        return args
    }
}
