import PurrloaderCore
import SwiftUI

struct HistoryEntry: Codable, Identifiable, Hashable {
    var id = UUID()
    var url: String
    var title: String
    var date: Date
    var folder: String
    var files: [String]
    var thumbnail: String?
    var options: DownloadOptions

    static func == (a: HistoryEntry, b: HistoryEntry) -> Bool { a.id == b.id }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}

@MainActor @Observable
final class HistoryStore {
    static let shared = HistoryStore()
    static let schemaVersion = 1

    var entries: [HistoryEntry] = [] {
        didSet { JSONFile.save(entries, to: AppPaths.history, schemaVersion: Self.schemaVersion) }
    }

    private init() {
        entries = JSONFile.load([HistoryEntry].self, from: AppPaths.history, schemaVersion: Self.schemaVersion) ?? []
    }

    func record(_ job: DownloadJob) {
        entries.insert(
            HistoryEntry(
                url: job.url, title: job.title, date: .now, folder: job.folder.path,
                files: job.files.map(\.path), thumbnail: job.thumbnail?.absoluteString,
                options: job.options), at: 0)
    }

    func delete(_ ids: Set<HistoryEntry.ID>) { entries.removeAll { ids.contains($0.id) } }
    func clear() { entries.removeAll() }
}

struct HistoryView: View {
    @State private var search = ""
    @State private var selection: Set<HistoryEntry.ID> = []
    @State private var confirmClear = false
    private var store: HistoryStore { .shared }

    private var filtered: [HistoryEntry] {
        let query = search.trimmed
        guard !query.isEmpty else { return store.entries }
        return store.entries.filter {
            $0.title.localizedCaseInsensitiveContains(query) || $0.url.localizedCaseInsensitiveContains(query)
        }
    }

    var body: some View {
        Table(filtered, selection: $selection) {
            TableColumn("Title") { entry in
                HStack {
                    Thumbnail(url: entry.thumbnail.flatMap(URL.init(string:)), width: 48)
                    Text(entry.title).lineLimit(1)
                }
            }
            TableColumn("Date") {
                Text($0.date, format: .dateTime.day().month().year().hour().minute()).foregroundStyle(.secondary)
            }
            .width(min: 120, ideal: 140)
            TableColumn("Folder") {
                Text(($0.folder as NSString).abbreviatingWithTildeInPath).foregroundStyle(.secondary).lineLimit(1)
            }
            .width(min: 100, ideal: 160)
        }
        .contextMenu(forSelectionType: HistoryEntry.ID.self) { ids in
            let entries = store.entries.filter { ids.contains($0.id) }
            Button("Download Again") { entries.forEach(redownload) }
            Button("Show in Finder") { entries.first.map(reveal) }
            Button("Copy Link") { Clipboard.copy(entries.map(\.url).joined(separator: "\n")) }
            Divider()
            Button("Remove from History") { store.delete(ids) }
        } primaryAction: { ids in
            store.entries.filter { ids.contains($0.id) }.first.map(reveal)
        }
        .searchable(text: $search, prompt: "Search history")
        .overlay {
            if store.entries.isEmpty {
                ContentUnavailableView(
                    "No History", systemImage: "clock", description: Text("Finished downloads show up here."))
            } else if filtered.isEmpty {
                ContentUnavailableView.search(text: search)
            }
        }
        .toolbar {
            Button {
                store.entries.filter { selection.contains($0.id) }.forEach(redownload)
            } label: {
                Label("Download Again", systemImage: "arrow.down.circle")
            }
            .disabled(selection.isEmpty)
            Button {
                confirmClear = true
            } label: {
                Label("Clear History", systemImage: "trash")
            }
            .disabled(store.entries.isEmpty)
        }
        .confirmationDialog("Clear all download history?", isPresented: $confirmClear) {
            Button("Clear History", role: .destructive) { store.clear() }
        } message: {
            Text("Downloaded files are not affected.")
        }
        .frame(minWidth: 560, minHeight: 320)
    }

    private func redownload(_ entry: HistoryEntry) {
        DownloadManager.shared.enqueue(
            DownloadJob(
                url: entry.url, title: entry.title,
                thumbnail: entry.thumbnail.flatMap(URL.init(string:)),
                options: entry.options, folder: URL(fileURLWithPath: entry.folder)))
    }

    private func reveal(_ entry: HistoryEntry) {
        let files = entry.files.map { URL(fileURLWithPath: $0) }.filter {
            FileManager.default.fileExists(atPath: $0.path)
        }
        if files.isEmpty {
            NSWorkspace.shared.open(URL(fileURLWithPath: entry.folder))
        } else {
            NSWorkspace.shared.activateFileViewerSelecting(files)
        }
    }
}
