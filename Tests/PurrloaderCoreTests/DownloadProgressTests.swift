import Foundation
import Testing

@testable import PurrloaderCore

struct DownloadProgressTests {
    @Test func progressLinesUpdateBytesAndStayOutOfTheLog() {
        var progress = DownloadProgress()
        let logged = progress.consume("[purrloader]downloading|1048576|4194304|NA|524288.0|6|Big Buck Bunny")
        #expect(!logged)
        #expect(progress.phase == .downloading)
        #expect(progress.downloadedBytes == 1_048_576)
        #expect(progress.totalBytes == 4_194_304)
        #expect(progress.fraction == 0.25)
        #expect(progress.speed == 524_288)
        #expect(progress.eta == 6)
        #expect(progress.reportedTitle == "Big Buck Bunny")
        #expect(progress.stage == "")
    }

    @Test func estimatedTotalIsUsedWhenExactIsMissing() {
        var progress = DownloadProgress()
        _ = progress.consume("[purrloader]downloading|500|NA|1000|NA|NA|NA")
        #expect(progress.fraction == 0.5)
        #expect(progress.eta == nil)
        #expect(progress.reportedTitle == nil)
    }

    @Test func videoThenAudioCountAsOneItem() {
        var progress = DownloadProgress()
        _ = progress.consume("[purrloader]finished|800|800|NA|NA|NA|Clip")
        _ = progress.consume("[purrloader]downloading|100|200|NA|NA|NA|Clip")
        #expect(progress.downloadedBytes == 900)
        #expect(progress.totalBytes == 1000)
        #expect(progress.fraction == 0.9)
    }

    @Test func playlistProgressSpansAllItems() {
        var progress = DownloadProgress()
        let logged1 = progress.consume("[download] Downloading item 2 of 4")
        #expect(logged1)
        _ = progress.consume("[purrloader]downloading|50|100|NA|NA|NA|Second")
        #expect(progress.isPlaylist)
        #expect(progress.fraction == 0.375)
    }

    @Test func postprocessingIsDescribedPlainly() {
        var progress = DownloadProgress()
        let logged2 = progress.consume("[purrloaderpp]started|Merger")
        #expect(!logged2)
        #expect(progress.phase == .processing)
        #expect(progress.stage == "Combining video and audio…")
        _ = progress.consume("[purrloaderpp]started|SponsorBlock")
        #expect(progress.stage == "Skipping sponsor segments…")
    }

    @Test func errorsAndDestinationsAreRemembered() {
        var progress = DownloadProgress()
        let logged3 = progress.consume("[download] Destination: /tmp/Clip.f137.mp4")
        #expect(logged3)
        let logged4 = progress.consume("ERROR: [site] abc: Video unavailable")
        #expect(logged4)
        #expect(progress.destinations == ["/tmp/Clip.f137.mp4"])
        #expect(progress.lastError == "[site] abc: Video unavailable")
    }

    @Test func stageBeforeTheFirstByte() {
        var progress = DownloadProgress()
        _ = progress.consume("[youtube:tab] Extracting URL: https://example.com/playlist")
        #expect(progress.stage == "Reading playlist…")
    }
}

struct LineBufferTests {
    @Test func splitsOnNewlinesAndCarriageReturnsAcrossChunks() {
        let buffer = LineBuffer()
        #expect(buffer.append(Data("one\ntw".utf8)) == ["one"])
        #expect(buffer.append(Data("o\rthree\r\n".utf8)) == ["two", "three"])
        #expect(buffer.append(Data("tail".utf8)) == [])
        #expect(buffer.flush() == ["tail"])
    }
}
