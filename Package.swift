// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "Purrloader",
    platforms: [.macOS(.v15)],
    targets: [
        // Everything that isn't UI: options, yt-dlp commands, progress parsing, errors, tools, processes.
        .target(name: "PurrloaderCore"),
        .executableTarget(name: "Purrloader", dependencies: ["PurrloaderCore"]),
        .testTarget(name: "PurrloaderCoreTests", dependencies: ["PurrloaderCore"]),
    ]
)
