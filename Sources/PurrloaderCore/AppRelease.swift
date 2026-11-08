import Foundation

/// A published Purrloader release on GitHub.
public struct AppRelease: Equatable, Sendable {
    public var version: String
    public var page: URL

    public init(version: String, page: URL) {
        self.version = version
        self.page = page
    }

    /// Compares dotted version numbers, ignoring a leading "v": "v1.10" is newer than "1.9".
    public static func isNewer(_ candidate: String, than current: String) -> Bool {
        func parts(_ version: String) -> [Int] {
            let trimmed = version.hasPrefix("v") || version.hasPrefix("V") ? String(version.dropFirst()) : version
            return trimmed.split(separator: ".").map { Int($0.prefix(while: \.isNumber)) ?? 0 }
        }
        let a = parts(candidate), b = parts(current)
        for index in 0..<max(a.count, b.count) {
            let x = index < a.count ? a[index] : 0, y = index < b.count ? b[index] : 0
            if x != y { return x > y }
        }
        return false
    }

    /// The latest non-draft, non-prerelease release, or nil if GitHub can't be reached.
    public static func latest(repository: String = AppInfo.repository) async -> AppRelease? {
        var request = URLRequest(url: URL(string: "https://api.github.com/repos/\(repository)/releases/latest")!)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        struct Response: Decodable {
            let tag_name: String
            let html_url: URL
        }
        guard let (data, response) = try? await URLSession.shared.data(for: request),
            (response as? HTTPURLResponse)?.statusCode == 200,
            let release = try? JSONDecoder().decode(Response.self, from: data)
        else { return nil }
        let version = release.tag_name.hasPrefix("v") ? String(release.tag_name.dropFirst()) : release.tag_name
        return AppRelease(version: version, page: release.html_url)
    }
}
