import Foundation
import Testing

@testable import PurrloaderCore

struct YtDlpCommandTests {
    let environment = YtDlpCommand.Environment(
        ffmpeg: "/tools/ffmpeg", deno: "/tools/deno", archive: "/data/archive.txt")
    let folder = URL(fileURLWithPath: "/Users/me/Downloads")
    let url = "https://example.com/watch/123"

    func arguments(_ configure: (inout DownloadOptions) -> Void = { _ in }, playlistItems: String? = nil) -> [String] {
        var options = DownloadOptions()
        configure(&options)
        return YtDlpCommand(options: options, environment: environment).download(
            url: url, folder: folder, playlistItems: playlistItems)
    }

    @Test func defaultVideoIsQuickTimeFriendlyMP4() {
        let a = arguments()
        #expect(a.contains(sequence: ["-f", "bv*+ba/b"]))
        #expect(a.contains(sequence: ["-S", "ext:mp4:m4a"]))
        #expect(a.contains(sequence: ["--merge-output-format", "mp4"]))
        #expect(a.contains(sequence: ["--remux-video", "mp4"]))
        #expect(a.contains("--embed-thumbnail"))
        #expect(a.contains("--embed-metadata"))
        #expect(a.contains("--embed-chapters"))
        #expect(a.contains("--ignore-config"))
        #expect(a.contains(sequence: ["--ffmpeg-location", "/tools/ffmpeg"]))
        #expect(a.contains(sequence: ["--js-runtimes", "deno:/tools/deno"]))
        #expect(a.contains(sequence: ["-P", "/Users/me/Downloads", "-o", "%(title)s.%(ext)s"]))
        #expect(a.contains("--no-windows-filenames"))
        #expect(a.suffix(2) == ["--", url])
    }

    @Test func macsWithoutAV1PlaybackGetH264() {
        var options = DownloadOptions()
        options.format.maxHeight = 2160
        var older = environment
        older.playsAV1 = false
        let a = YtDlpCommand(options: options, environment: older).download(url: url, folder: folder)
        #expect(a.contains(sequence: ["-S", "vcodec:h264,ext:mp4:m4a"]))
    }

    @Test func qualityLimitAppliesToBothFallbacks() {
        let a = arguments { $0.format.maxHeight = 1080 }
        #expect(a.contains(sequence: ["-f", "bv*[height<=1080]+ba/b[height<=1080]"]))
    }

    @Test func h264PreferenceRanksCodecFirst() {
        let a = arguments { $0.format.codec = .h264 }
        #expect(a.contains(sequence: ["-S", "vcodec:h264,res,acodec:m4a"]))
    }

    @Test func mp3UsesBestVBR() {
        let a = arguments { $0.format.mode = .audio }
        #expect(a.contains(sequence: ["-f", "ba/b", "-x", "--audio-format", "mp3", "--audio-quality", "0"]))
        #expect(a.contains("--embed-thumbnail"))
        #expect(!a.contains("--merge-output-format"))
    }

    @Test func bestQualityAudioIsNotConverted() {
        let a = arguments {
            $0.format.mode = .audio
            $0.format.audioFormat = .original
        }
        #expect(!a.contains("--audio-format"))
        #expect(a.contains("-x"))
    }

    @Test func coverArtAndChaptersSkippedWhereUnsupported() {
        let wav = arguments {
            $0.format.mode = .audio
            $0.format.audioFormat = .wav
        }
        #expect(!wav.contains("--embed-thumbnail"))
        #expect(!wav.contains("--embed-chapters"))
        let webm = arguments { $0.format.container = .webm }
        #expect(!webm.contains("--embed-thumbnail"))
    }

    @Test func subtitlesEmbedInVideoOnly() {
        let video = arguments {
            $0.subtitles.enabled = true
            $0.subtitles.languages = "de"
        }
        #expect(video.contains(sequence: ["--write-subs", "--write-auto-subs", "--sub-langs", "de"]))
        #expect(video.contains("--embed-subs"))
        let audio = arguments {
            $0.format.mode = .audio
            $0.subtitles.enabled = true
        }
        #expect(!audio.contains("--embed-subs"))
        #expect(!arguments().contains("--write-subs"))
    }

    @Test func skipSponsorsAndPartOfVideo() {
        let a = arguments {
            $0.extras.sponsorBlock = .remove
            $0.extras.sectionStart = "1:30"
        }
        #expect(a.contains(sequence: ["--sponsorblock-remove", "sponsor,selfpromo,interaction"]))
        #expect(a.contains(sequence: ["--download-sections", "*1:30-inf"]))
    }

    @Test func browserSignIn() {
        #expect(
            arguments { $0.network.cookieBrowser = .firefox }.contains(sequence: ["--cookies-from-browser", "firefox"]))
        let chrome = arguments {
            $0.network.cookieBrowser = .chrome
            $0.network.cookieProfile = "Profile 1"
        }
        #expect(chrome.contains(sequence: ["--cookies-from-browser", "chrome:Profile 1"]))
        let arc = arguments { $0.network.cookieBrowser = .arc }
        #expect(arc.contains { $0.hasPrefix("chromium:") && $0.hasSuffix("/Arc/User Data/Default") })
    }

    @Test func tickedPlaylistItemsOverrideRange() {
        let a = arguments({ $0.playlist.range = "5-20" }, playlistItems: "1:3,7")
        #expect(a.contains(sequence: ["--yes-playlist", "-I", "1:3,7"]))
        #expect(!a.contains("5:20"))
    }

    @Test func playlistRangeAndReverse() {
        #expect(arguments { $0.playlist.range = "5-20" }.contains(sequence: ["-I", "5:20"]))
        let reversed = arguments {
            $0.playlist.range = "5-20"
            $0.playlist.reverse = true
        }
        #expect(reversed.contains(sequence: ["-I", "20:5:-1"]))
        #expect(arguments { $0.playlist.latestCount = 3 }.contains(sequence: ["-I", "1:3"]))
        #expect(
            arguments { $0.playlist.useArchive = true }.contains(sequence: ["--download-archive", "/data/archive.txt"]))
    }

    @Test func extraArgumentsGoLastBeforeTheURL() {
        let a = arguments { $0.extraArguments = #"--match-filters "duration < 600""# }
        #expect(Array(a.suffix(4)) == ["--match-filters", "duration < 600", "--", url])
    }

    @Test func infoFetchUsesSameAccessOptions() {
        var options = DownloadOptions()
        options.network.proxy = "socks5://127.0.0.1:1080"
        let a = YtDlpCommand(options: options, environment: environment).info(url: url)
        #expect(a.contains(sequence: ["--proxy", "socks5://127.0.0.1:1080"]))
        #expect(a.suffix(5) == ["-J", "--flat-playlist", "--no-warnings", "--", url])
    }
}

extension Array where Element: Equatable {
    /// True if `sequence` appears here as consecutive elements.
    func contains(sequence: [Element]) -> Bool {
        guard sequence.count <= count else { return false }
        return (0...(count - sequence.count)).contains { Array(self[$0..<($0 + sequence.count)]) == sequence }
    }
}
