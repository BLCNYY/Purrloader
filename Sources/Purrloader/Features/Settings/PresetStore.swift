import Foundation
import Observation
import PurrloaderCore

/// A saved setup: a named snapshot of every download setting.
struct Preset: Codable, Identifiable, Hashable {
    var id = UUID()
    var name: String
    var options: DownloadOptions

    static func == (a: Preset, b: Preset) -> Bool { a.id == b.id && a.name == b.name && a.options == b.options }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}

@MainActor @Observable
final class PresetStore {
    static let shared = PresetStore()
    static let schemaVersion = 1

    var presets: [Preset] = [] {
        didSet { JSONFile.save(presets, to: AppPaths.presets, schemaVersion: Self.schemaVersion) }
    }

    private init() {
        presets =
            JSONFile.load([Preset].self, from: AppPaths.presets, schemaVersion: Self.schemaVersion) ?? Self.builtIn
    }

    static var builtIn: [Preset] {
        var mp4 = DownloadOptions()
        mp4.format.maxHeight = 1080
        mp4.format.codec = .h264

        var podcast = DownloadOptions()
        podcast.format.mode = .audio
        podcast.format.audioQuality = "128K"
        podcast.extras.sponsorBlock = .remove
        podcast.output.template = "%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s"

        var archive = DownloadOptions()
        archive.format.maxHeight = 2160
        archive.format.container = .mkv
        archive.subtitles.enabled = true
        archive.subtitles.languages = "all,-live_chat"
        archive.extras.writeInfoJSON = true
        archive.extras.writeDescription = true
        archive.playlist.useArchive = true
        archive.output.template = "%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s"

        var flac = DownloadOptions()
        flac.format.mode = .audio
        flac.format.audioFormat = .flac

        return [
            Preset(name: "Default", options: DownloadOptions()),
            Preset(name: "1080p MP4 (H.264)", options: mp4),
            Preset(name: "Podcast MP3", options: podcast),
            Preset(name: "Archive 4K MKV", options: archive),
            Preset(name: "Music FLAC", options: flac),
        ]
    }

    /// Saves under a name, replacing a setup that already has it.
    func add(name: String, options: DownloadOptions) -> Preset {
        var options = options
        options.format.exactFormat = ""  // stream IDs only make sense for one video
        if let index = presets.firstIndex(where: { $0.name == name }) {
            presets[index].options = options
            return presets[index]
        }
        let preset = Preset(name: name, options: options)
        presets.append(preset)
        return preset
    }

    func delete(_ preset: Preset) { presets.removeAll { $0.id == preset.id } }
}
