import Testing

@testable import PurrloaderCore

struct FriendlyErrorTests {
    @Test(arguments: [
        (
            "[youtube] abc: Sign in to confirm you're not a bot. Use --cookies-from-browser",
            FriendlyError.Kind.signInRequired
        ),
        ("[youtube] abc: Private video. Sign in if you've been granted access", .signInRequired),
        ("Failed to decrypt with DPAPI", .browserCookies),
        ("could not find chrome cookies database", .browserCookies),
        ("Unsupported URL: https://example.com/", .unsupportedLink),
        ("'nope' is not a valid URL", .invalidLink),
        ("[vimeo] 1: Video unavailable", .unavailable),
        ("Requested format is not available. Use --list-formats", .formatUnavailable),
        ("unable to download video data: HTTP Error 403: Forbidden", .blocked),
        ("[youtube] abc: nsig extraction failed: Some formats may be missing", .blocked),
        ("Unable to download webpage: <urlopen error [Errno 8] nodename nor servname provided>", .network),
        ("[Errno 28] No space left on device", .diskFull),
        ("[Errno 13] Permission denied: '/Volumes/Backup/x.mp4'", .folderNotWritable),
        ("[youtube] abc: This live event will begin in 3 hours", .notStarted),
    ])
    func mapsRawErrors(raw: String, kind: FriendlyError.Kind) {
        #expect(FriendlyError(raw: raw).kind == kind)
    }

    @Test func everyKnownProblemOffersAFix() {
        #expect(FriendlyError(kind: .signInRequired).fix == .browserSignIn)
        #expect(FriendlyError(kind: .blocked).fix == .updateAndRetry)
        #expect(FriendlyError(kind: .folderNotWritable).fix == .chooseFolder)
        #expect(FriendlyError(kind: .network).fix == .retry)
        #expect(FriendlyError(kind: .formatUnavailable).fix == .useBestQuality)
        #expect(FriendlyError(kind: .notStarted).fix == .waitForStart)
    }

    @Test func messagesDontPointAtMenusThatNoLongerExist() {
        for kind in [FriendlyError.Kind.browserCookies, .signInRequired, .blocked, .folderNotWritable, .notStarted] {
            let message = FriendlyError(kind: kind).message
            #expect(!message.contains("→"))
            #expect(!message.contains("yt-dlp"))
        }
    }

    @Test func unknownErrorsLoseTheExtractorPrefix() {
        let error = FriendlyError(raw: "[generic] xyz: Something odd happened")
        #expect(error.kind == .other)
        #expect(error.message == "Something odd happened")
    }
}
