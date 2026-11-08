import Foundation
import Testing

@testable import PurrloaderCore

struct PlaylistSelectionTests {
    @Test(arguments: [
        ([1, 2, 3, 5, 7, 8], "1:3,5,7:8"),
        ([4], "4"),
        ([], ""),
        ([9, 1, 2, 2, 10], "1:2,9:10"),
        ([1, 3, 5], "1,3,5"),
    ])
    func compressesRuns(indices: [Int], expected: String) {
        #expect(PlaylistSelection.compress(indices) == expected)
    }
}

struct ShellSplitTests {
    @Test(arguments: [
        ("--a b", ["--a", "b"]),
        ("  spaced   out  ", ["spaced", "out"]),
        (#"--match-filters "duration < 600""#, ["--match-filters", "duration < 600"]),
        ("'single quoted' \"double\"", ["single quoted", "double"]),
        (#"it\'s escaped\ space"#, ["it's", "escaped space"]),
        (#"'no \escape in single'"#, [#"no \escape in single"#]),
        (#"a"b"c"#, ["abc"]),
        ("''", [""]),
        ("", []),
    ])
    func splitsLikeAShell(text: String, expected: [String]) {
        #expect(Shell.split(text) == expected)
    }

    @Test func quoteRoundTrips() {
        let args = ["plain", "with space", "it's", ""]
        #expect(Shell.split(Shell.commandLine(args)) == args)
    }
}

struct ChecksumTests {
    @Test func findsTheRightLineInASumsFile() {
        let sums = """
            0f922c15ccc0b83eefa1c8bc9104e434318c4bf16e408e1ee93b7745a6efe0aa  yt-dlp_macos
            20d98fa608998eff87d1d35a6e9fb65e4b874639a6cc89addc19da3becde3ef1  yt-dlp_macos.zip
            """
        #expect(
            Checksums.sha256(for: "yt-dlp_macos.zip", in: sums)
                == "20d98fa608998eff87d1d35a6e9fb65e4b874639a6cc89addc19da3becde3ef1")
        #expect(Checksums.sha256(for: "missing.zip", in: sums) == nil)
    }

    @Test func acceptsBinaryMarkerAndBareHash() {
        let hash = String(repeating: "ab", count: 32)
        #expect(Checksums.sha256(for: "deno.zip", in: "\(hash) *deno.zip\n") == hash)
        #expect(Checksums.sha256(for: "anything.zip", in: hash.uppercased() + "\n") == hash)
    }

    @Test func hashesFiles() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("purrloader-test-\(UUID().uuidString)")
        try Data("abc".utf8).write(to: url)
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(try Checksums.sha256(ofFile: url) == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    }
}

struct DownloadOptionsTests {
    @Test func missingKeysKeepTheirDefaults() throws {
        let json = #"{"format": {"mode": "audio"}, "network": {"retries": 3}}"#
        let options = try JSONDecoder().decode(DownloadOptions.self, from: Data(json.utf8))
        #expect(options.format.mode == .audio)
        #expect(options.format.audioFormat == .mp3)
        #expect(options.network.retries == 3)
        #expect(options.extras.embedMetadata)
        #expect(options.ignoreConfig)
    }

    @Test func unreadableGroupFallsBackToDefaults() throws {
        let json = #"{"format": {"mode": "hologram"}, "output": {"template": "%(id)s.%(ext)s"}}"#
        let options = try JSONDecoder().decode(DownloadOptions.self, from: Data(json.utf8))
        #expect(options.format == FormatOptions())
        #expect(options.output.template == "%(id)s.%(ext)s")
    }

    @Test func roundTrips() throws {
        var options = DownloadOptions()
        options.format.maxHeight = 720
        options.subtitles.enabled = true
        options.extras.sponsorCategories = ["intro"]
        let decoded = try JSONDecoder().decode(DownloadOptions.self, from: JSONEncoder().encode(options))
        #expect(decoded == options)
    }
}

struct AppReleaseTests {
    @Test(arguments: [
        ("1.1", "1.0", true), ("v1.10", "1.9", true), ("1.0.1", "1.0", true),
        ("1.0", "1.0", false), ("1.0", "1.0.0", false), ("0.9", "1.0", false),
    ])
    func comparesVersions(candidate: String, current: String, newer: Bool) {
        #expect(AppRelease.isNewer(candidate, than: current) == newer)
    }
}

struct WebLinkTests {
    @Test func dropsTrailingPunctuation() {
        #expect("Watch https://example.com/v/1. Thanks!".webLinks == ["https://example.com/v/1"])
        #expect("ftp://nope and https://ok.example".webLinks == ["https://ok.example"])
    }
}
