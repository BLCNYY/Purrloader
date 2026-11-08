# Architecture

Purrloader is a SwiftUI app for macOS 15+, built with Swift Package Manager in Swift 6 language mode with strict
concurrency. It isn't sandboxed and isn't meant for the App Store: it downloads and runs helper tools.

The Windows and Linux app lives in `desktop/` and is a port of the same app; see
[Windows and Linux](#windows-and-linux) at the end.

## Targets

| Target | What's in it |
| --- | --- |
| `PurrloaderCore` (library) | Everything that isn't UI. Testable without the app. |
| `Purrloader` (app) | SwiftUI windows, the queue, settings, menu bar. |
| `PurrloaderCoreTests` | Swift Testing suites for the library. |

## Layout

```
Sources/
  PurrloaderCore/
    DownloadOptions.swift   every setting, grouped (format, playlist, subtitles, extras, network, output)
    YtDlpCommand.swift      DownloadOptions -> yt-dlp arguments
    DownloadProgress.swift  parses yt-dlp's output lines into progress; playlist index compression
    FriendlyError.swift     raw yt-dlp errors -> a sentence and a fix-it action
    MediaInfo.swift         the parsed `-J --flat-playlist` JSON; file name template preview
    ToolManager.swift       installs, verifies, versions and updates yt-dlp, ffmpeg, ffprobe and Deno
    ChildProcess.swift      running processes: AsyncStream of output lines, or collected output
    AppRelease.swift        GitHub Releases lookup and version comparison
    Support.swift           paths, versioned JSON files, formatting helpers
  Purrloader/
    App/                    @main, AppDelegate, menu bar extra, app update notice
    Features/Main/          main window: link field, info card, playlist checklist, More popover
    Features/Queue/         DownloadManager (the queue), queue rows, details sheet, history
    Features/Settings/      General, Advanced (all yt-dlp options), About; saved setups
    Features/Onboarding/    first-launch welcome sheet
    Components/             theme, custom controls, window style and blur, browser picker
Tests/PurrloaderCoreTests/
Resources/                  Info.plist and the Icon Composer icon (Purrloader.icon)
dmg/                        make-dmg.sh and the drawn DMG background
```

## Data

Everything lives in `~/Library/Application Support/Purrloader`:

- `bin/` the helper tools. yt-dlp is the macOS folder ("onedir") build, which starts in about 0.2 s where the
  single-file build unpacks itself for about 4 s on every run. Each yt-dlp version gets its own folder under
  `bin/yt-dlp/`, and the `current` symlink points at the newest. Downloads launch the resolved path, so an
  update never swaps files out from under a running yt-dlp. Old versions are removed at the next launch.
- `queue.json` the queue, restored on launch. Interrupted downloads resume from their `.part` files.
- `history.json`, `presets.json` (Saved Setups), `download-archive.txt`.
- `tmp/` scratch space; also yt-dlp's working directory (a working directory inside ~/Downloads makes its
  startup wait on a privacy check).

JSON files are wrapped as `{"schemaVersion": 1, "content": …}`. `DownloadOptions` decodes group by group over
its defaults, so a missing or unreadable value falls back to the default instead of losing the whole file.
Settings that aren't per-download (folder, parallel downloads, clipboard autofill and so on) are in
UserDefaults under `com.blcnyy.purrloader`.

## Helper tools

On first launch `ToolManager` downloads, in parallel:

| Tool | From | Checked against |
| --- | --- | --- |
| yt-dlp | `github.com/yt-dlp/yt-dlp` (or `yt-dlp-nightly-builds`), `yt-dlp_macos.zip` | the release's `SHA2-256SUMS` |
| ffmpeg, ffprobe | `ffmpeg.martin-riedl.de`, release builds | the `.sha256` next to each zip |
| Deno | `github.com/denoland/deno`, `deno-<arch>-apple-darwin.zip` | the release's `.sha256sum` |

The latest release is pinned first (GitHub's `/releases/latest` redirect, which has no API rate limit;
martin-riedl.de's `redirect/latest` URL), so the archive and its checksum always come from the same version.
An archive is hashed before it's unpacked; on a mismatch it's deleted and an error is shown. Downloads are
retried a couple of times, because martin-riedl.de's redirect occasionally answers 404.

yt-dlp updates itself quietly at launch (Settings > About can turn that off or switch to nightly). When a
download fails with a "blocked" error (HTTP 403/429, signature extraction), Purrloader updates yt-dlp and
retries that download once on its own.

## Downloads

- `YtDlpCommand` builds the arguments. Progress comes from `--newline --progress-template` with
  `[purrloader]` prefixes; final paths come from `--print-to-file after_move:filepath`.
- `ChildProcess` reads stdout and stderr through readability handlers into an `AsyncStream` of lines.
  `DownloadManager` consumes it on the main actor and feeds each line to `DownloadProgress`.
- Pause sends SIGINT, which makes yt-dlp stop ffmpeg and leave resumable `.part` files. Resume starts yt-dlp
  again with the same arguments and it continues them. Cancel also deletes the partial files.
- Quitting pauses running downloads and saves them as queued, so they resume on the next launch.
- Video defaults to MP4 that QuickTime and Quick Look play: streams that are already H.264/AV1 + AAC are
  preferred (`-S ext:mp4:m4a`) and remuxed, never re-encoded. Metadata, chapters and cover art are embedded
  when the format can hold them (not WebM or WAV).
- File names are Mac-friendly: `--no-windows-filenames`, and ": " in titles becomes " - " because Finder shows
  ":" as "/".

## Notes

- The window blur uses the private `CGSSetWindowBackgroundBlurRadius`, as Ghostty does. It's looked up with
  `dlsym` in `Components/WindowBlur.swift`; if macOS ever removes it, the window falls back to
  `NSVisualEffectView` instead of failing to launch.
- yt-dlp has no Arc cookie support; the Arc option reads Arc's profile as Chromium and may fail to decrypt.
  Export a cookies.txt from Arc if it does.
- Safari cookies need Full Disk Access for Purrloader.

## Building and releasing

```bash
./build.sh             # universal (Apple Silicon + Intel) app in build/
./build.sh --install   # also copy to /Applications and launch
swift test
./release.sh           # signed, notarized DMG (see below)
```

`VERSION` holds the version number. `build.sh` writes it into the app's Info.plist, and About and the update
check read it from there.

`release.sh` signs with a Developer ID, hardened runtime and a secure timestamp, notarizes and staples the app,
then builds the DMG with `dmg/make-dmg.sh` and signs, notarizes and staples that too. It reads:

- `PURRLOADER_SIGN_IDENTITY`: `Developer ID Application: Name (TEAMID)`
- `PURRLOADER_NOTARY_PROFILE`: a profile saved with `xcrun notarytool store-credentials`

Without them it still builds an ad-hoc signed DMG and prints a warning. For everyday development,
`build.sh` also honours `PURRLOADER_SIGN_IDENTITY` (any identity, e.g. Apple Development) so macOS keeps
privacy permissions across rebuilds.

The DMG is a black window with the cat, the name and a hand-drawn arrow to Applications; the background is
drawn by `dmg/background.swift`. The icon is an Icon Composer file; `build.sh` compiles it with `actool`.

## Windows and Linux

SwiftUI only runs on Apple platforms, so the Windows and Linux app in `desktop/` is a port in Rust: the same
features, the same words and the same look, point for point, drawn with egui.

```
Cargo.toml                   workspace, locked dependencies in Cargo.lock
desktop/
  src/main.rs                native window, isolated smoke mode, single-instance lock
  src/app.rs                 main window, More popover, queue, sheets, Settings and History windows
  src/settings_ui.rs         Settings > Advanced, section by section
  src/theme.rs               colours, Inter in place of SF Pro, text helpers
  src/widgets.rs             the Mac controls: pill segment, chips, switches, pop-up buttons, menus, forms
  src/icons.rs               look-alikes of the SF Symbols the Mac app uses
  src/platform.rs            native tray, activation and file-manager actions
  src/engine.rs              queue state machine, background tasks, history, retry and shutdown
  src/process.rs             concurrent pipe readers, process groups, cancellation and timeouts
  src/tools.rs               verified helper archives and versioned installations
  src/store.rs               versioned JSON envelopes and atomic persistence
  src/options.rs             typed download settings with group-level recovery
  src/command.rs             options to argument vectors (never shell execution)
  src/progress.rs            yt-dlp stream and playlist progress parser
  src/error.rs               recoverable errors and suggested actions
  src/media.rs               metadata, stream formats and subtitle tracks
  src/support.rs             links, selections, checksums and formatting
  tests/                     compatibility, persistence, extraction and subprocess tests
  test-support/helper.rs     deterministic executable used only in integration tests
  assets/                    app and tray artwork, Inter
  packaging/                 native NSIS, AppImage, Debian and portable-archive packaging
```

The UI owns `Engine`; background workers send typed messages through a channel and wake egui when work arrives.
No network request or download process runs on the UI thread. The queue runs oldest-first with a configurable
1–10 job limit. Pause/cancel keep a slot occupied until the subprocess is reaped, preventing duplicate writers
when a user quickly pauses and resumes. Metadata requests use generation tokens and cancellation so an older
link cannot replace a newer preview. Logs retain at most 4,000 lines per job.

On Unix each helper gets a process group. Pause sends SIGINT to the group; a three-second grace period is
followed by SIGKILL. On Windows helpers use hidden consoles and a kill-on-close Job Object owns their process trees.
Both pipes are drained concurrently and pending lines are delivered before the exit event. Active jobs are
saved queued before shutdown, while explicitly paused jobs remain paused. Cancel removes only known partial
files inside that job's selected output folder; completed media stays on disk.

Data stays in `%APPDATA%\Purrloader` or `$XDG_CONFIG_HOME/Purrloader` (normally `~/.config/Purrloader`).
The files are `settings.json`, `queue.json`, `history.json` and `presets.json`, in the same schema-one envelope
and camelCase keys as the Mac app's. Writes use a temporary file in the same directory, flush it, and atomically replace the destination. Unknown
schemas and damaged JSON produce an error instead of overwriting the original. macOS development and smoke
tests use separate directories. `PURRLOADER_DATA_DIR` can select an isolated directory for manual testing.

Each tool version gets its own `bin/<package>/<build>/` folder and `current` names the newest, because Windows
can't replace a running `.exe`.
The publisher's checksum is verified before unpacking; ZIP traversal, symbolic links and special archive
entries are rejected. FFmpeg's `.tar.xz` is decoded in Rust. Running jobs use resolved version paths so an update
cannot replace an executable or DLL they are using. yt-dlp supports stable/nightly updates and one automatic
update-and-retry for a blocked download.

`VERSION` supplies the displayed app version through `build.rs`, as well as installer versions. The native
tray uses Windows shell integration or Linux StatusNotifier over D-Bus, with show, pause, resume, history,
settings and quit actions. Closing the window retains the app while its tray is enabled; explicit quit saves
and stops downloads. Startup holds an exclusive file lock, so a second launch only brings the running window forward.

[desktop/README.md](../desktop/README.md) covers building, packaging and platform prerequisites.
`.github/workflows/ci.yml` builds and tests all four Windows/Linux architecture combinations and renders a
window smoke screenshot. `.github/workflows/release.yml` packages those binaries on their native runners,
checks binary architecture, includes dependency licenses and publishes the existing release asset names.
The Swift build and local DMG release scripts are unchanged.
