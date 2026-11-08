# Architecture

Purrloader is a SwiftUI app for macOS 15+, built with Swift Package Manager in Swift 6 language mode with strict
concurrency. It isn't sandboxed and isn't meant for the App Store: it downloads and runs helper tools.

The Windows and Linux app lives in `electron/` and is a port of the same app; see
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

SwiftUI only runs on Apple platforms, so the Windows and Linux app in `electron/` is a port: the same features,
the same words and the same look, point for point, in Electron.

```
electron/
  src/core/        PurrloaderCore in JavaScript: options, yt-dlp arguments, progress, errors, media info.
                   Pure functions, shared by the main process and the windows; test/ checks them against the
                   same cases as Tests/PurrloaderCoreTests.
  src/main/        the main process: helper tools (tools.js), the queue and history (downloads.js), the main
                   window's model, saved setups and update check (model.js), windows and tray (windows.js),
                   settings and the state hub (store.js), and the actions windows may call (ipc.js)
  src/renderer/    the windows, in Preact with htm templates: main window, Settings, History, tray panel.
                   controls.js has the Mac controls (pill segment, chips, switches, pop-up buttons, grouped
                   forms, menus, popovers, sheets); styles.css has the Mac app's sizes and colours.
  assets/          app icon and tray icons, rendered from the Mac app's icon and MenuBarIcon
  electron-builder.yml   installers: NSIS for Windows, AppImage, .deb and .tar.gz for Linux, x64 and arm64
```

- **State.** The main process owns everything, like the Mac app's singletons. Windows get state through
  `Hub` (only the parts that changed, at most about fifteen times a second) and send named actions through
  `ipc.js`. Text fields keep what's being typed and take the stored value when the main process changes it.
- **Look.** The window background is `#2d2d2f`, what the Mac's matte black at 80% over the blurred desktop comes
  out as, and every colour is drawn relative to it as in `Theme.swift`. The 38-point strip at the top holds the
  menu button on the left and the system's own window buttons on the right. Inter stands in for SF Pro and
  `icons.js` draws look-alikes of the SF Symbols, since Apple's fonts and symbols may only be used on Apple
  platforms. Menus, popovers and sheets are drawn in the window so they look the same everywhere.
- **Helper tools.** `yt-dlp_win.zip` / `yt-dlp_linux.zip` (and the arm64 builds), FFmpeg from
  `yt-dlp/FFmpeg-Builds` (`shared` on Windows: half the download), and Deno, each checked against its
  publisher's SHA-256 before it's unpacked. Every version of a tool gets its own folder under
  `bin/<tool>/` and `bin/<tool>/current` names the newest, because Windows can't replace a running `.exe`.
- **Pause.** Windows has no Ctrl-C for windowless processes, so pausing ends yt-dlp and its ffmpeg with
  `taskkill /T`; the `.part` files resume all the same. Linux pauses with SIGINT like the Mac.
- **Data** is in `%APPDATA%\Purrloader` or `~/.config/Purrloader`, with the same files as on the Mac plus
  `settings.json` for what the Mac keeps in UserDefaults.

```bash
cd electron
npm install
npm start            # run it (on a Mac too, with its data in "Purrloader (Electron)")
npm test
npm run dist         # Windows and Linux installers in electron/dist
```

`.github/workflows/release.yml` builds the installers on Windows and Linux for each `v*` tag and attaches them
to that tag's release.
