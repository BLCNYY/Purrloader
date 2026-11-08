# Third-party notices

Purrloader itself is MIT licensed (see [LICENSE](LICENSE)). It works by running a few free, open-source
helper tools. **None of them are bundled with Purrloader.** On first launch the app downloads each one
directly from its publisher, checks it against the SHA-256 checksum that publisher provides, and keeps it in
Purrloader's tools folder: `~/Library/Application Support/Purrloader/bin` on a Mac, `%APPDATA%\Purrloader\bin` on
Windows and `~/.config/Purrloader/bin` on Linux. Their licenses apply to those copies.

The Windows and Linux app is built with a few open-source libraries that *are* bundled with it; they're listed at
the end.

## yt-dlp

- What it does in Purrloader: reads links and downloads the media.
- Source: <https://github.com/yt-dlp/yt-dlp> (stable) and <https://github.com/yt-dlp/yt-dlp-nightly-builds> (nightly)
- Downloaded file: `yt-dlp_macos.zip` on a Mac, `yt-dlp_win.zip` or `yt-dlp_win_arm64.zip` on Windows,
  `yt-dlp_linux.zip` or `yt-dlp_linux_aarch64.zip` on Linux, verified against the release's `SHA2-256SUMS`
- License: [The Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE) (public domain). These builds
  bundle Python and other components under their own licenses, listed in the yt-dlp repository.

## FFmpeg and ffprobe

- What they do in Purrloader: combine video and audio, convert formats, cut sections, embed cover art,
  chapters and subtitles.
- Source: <https://ffmpeg.org>; macOS builds by Martin Riedl, <https://ffmpeg.martin-riedl.de>
- Downloaded files: `ffmpeg.zip` and `ffprobe.zip` (release builds), each verified against its `.sha256` file
- License: these builds are configured with `--enable-gpl --enable-version3`, so they are distributed under the
  [GNU General Public License, version 3 or later](https://www.gnu.org/licenses/gpl-3.0.html). FFmpeg's
  source code is available at <https://ffmpeg.org/download.html>; the build scripts are at
  <https://git.martin-riedl.de/ffmpeg/build-script>.

On Windows and Linux, FFmpeg and ffprobe come from yt-dlp's own builds, which carry patches for problems yt-dlp
runs into:

- Source: <https://github.com/yt-dlp/FFmpeg-Builds>
- Downloaded file: `ffmpeg-master-latest-win64-gpl-shared.zip` (or `winarm64`) on Windows,
  `ffmpeg-master-latest-linux64-gpl.tar.xz` (or `linuxarm64`) on Linux, verified against the release's
  `checksums.sha256`
- License: GPL builds, distributed under the [GNU General Public License, version 3 or
  later](https://www.gnu.org/licenses/gpl-3.0.html). The build scripts and the source of every component are
  linked from the repository above.

Purrloader runs FFmpeg as a separate program and does not link to it.

## Deno

- What it does in Purrloader: yt-dlp uses it as a JavaScript runtime for some sites.
- Source: <https://github.com/denoland/deno>
- Downloaded file: `deno-<architecture>-apple-darwin.zip`, `deno-<architecture>-pc-windows-msvc.zip` or
  `deno-<architecture>-unknown-linux-gnu.zip`, verified against the release's `.sha256sum` file
- License: [MIT](https://github.com/denoland/deno/blob/main/LICENSE.md)

## SponsorBlock

- What it does in Purrloader: "Skip sponsor segments" asks the SponsorBlock API (through yt-dlp) which parts
  of a video are sponsor spots, self-promotion or reminders to subscribe.
- Source: <https://sponsor.ajay.app>
- License: SponsorBlock's database is licensed under
  [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/). Segment data is fetched on demand
  for the video being downloaded; Purrloader doesn't store or redistribute it.

## Bundled with the Windows and Linux app

The Windows and Linux app (`electron/` in this repository) ships with these, each under its own license:

- [Electron](https://www.electronjs.org), which includes Chromium and Node.js: [MIT](https://github.com/electron/electron/blob/main/LICENSE).
  The licenses of Chromium's components are in the `LICENSES.chromium.html` file next to the app.
- [Preact](https://preactjs.com): [MIT](https://github.com/preactjs/preact/blob/main/LICENSE)
- [htm](https://github.com/developit/htm): [Apache 2.0](https://github.com/developit/htm/blob/master/LICENSE)
- [yauzl](https://github.com/thejoshwolfe/yauzl), which unpacks the helper tools: [MIT](https://github.com/thejoshwolfe/yauzl/blob/master/LICENSE)
- [Inter](https://rsms.me/inter/), the typeface, by Rasmus Andersson: [SIL Open Font License 1.1](https://github.com/rsms/inter/blob/master/LICENSE.txt).
  Apple's San Francisco fonts and SF Symbols may only be used on Apple platforms, so the Windows and Linux app
  uses Inter and its own drawings of the icons.
