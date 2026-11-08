<div align="center">

<img src="docs/images/icon.png" width="128" height="128" alt="Purrloader icon">

# Purrloader

**Save videos and audio from the web. Paste a link, press Download.**

[![macOS 15+](https://img.shields.io/badge/macOS-15%2B-555?logo=apple&logoColor=white)](#macos)
[![Windows 10+](https://img.shields.io/badge/Windows-10%2B-555?logo=windows&logoColor=white)](#windows)
[![Linux](https://img.shields.io/badge/Linux-x64%20%7C%20arm64-555?logo=linux&logoColor=white)](#linux)
[![License: MIT](https://img.shields.io/github/license/BLCNYY/Purrloader?color=555)](LICENSE)
[![Latest release](https://img.shields.io/github/v/release/BLCNYY/Purrloader?color=555)](https://github.com/BLCNYY/Purrloader/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/BLCNYY/Purrloader/total?color=555)](https://github.com/BLCNYY/Purrloader/releases)

### Download for [Mac](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader.dmg) · [Windows](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-Setup-x64.exe) · [Linux](#linux)

Free and open source · Mac, Windows and Linux · No account, no ads, no tracking

<br>

<img src="docs/images/main-window.png" width="780" alt="The Purrloader window with a video's details and two finished downloads">

</div>

## Features

- **Video or audio in one click.** MP4 that plays in QuickTime, or MP3, M4A and lossless audio.
- **Playlists with a checklist.** Tick the videos you want and skip the rest.
- **Pause and resume.** Downloads pick up where they left off, even after you quit.
- **Progress in the menu bar** (or the system tray). Keep an eye on downloads without the window open.
- **Subtitles** in the languages the video actually has.
- **Skip sponsor segments** using the community-run SponsorBlock database.
- **Download just a part** of a long video by start and end time.
- **Private, age-restricted or members-only content** through the sign-in you already have in your browser.
- **Thousands of sites,** thanks to [yt-dlp](https://github.com/yt-dlp/yt-dlp).
- **Stays up to date by itself.** Sites change; Purrloader quietly keeps up.
- **Fully local.** Downloads go straight from the site to your computer. Nothing passes through a server of ours.
- **No tracking.** No analytics, no telemetry, no accounts.
- **Light or dark.** Follows your system, or pick one in Settings.

## Install

Purrloader looks and works the same on all three. Whichever you use, the first time you open it Purrloader
downloads its free helper tools and checks each one against the checksum its publisher provides. That takes a
minute; after that you're ready.

### macOS

1. [Download Purrloader.dmg](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader.dmg).
2. Open it and drag Purrloader into Applications.
3. Open Purrloader. The helper tools are about 150 MB.

Purrloader needs macOS 15 Sequoia or later and runs natively on Apple Silicon and Intel Macs.

### Windows

1. Download the installer:
   - [Purrloader-Setup-x64.exe](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-Setup-x64.exe) for most PCs
   - [Purrloader-Setup-arm64.exe](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-Setup-arm64.exe) for Windows on Arm (Snapdragon laptops,
     Surface Pro X)

   Not sure which? Look under Settings > System > About > System type.
2. Open it. Purrloader installs for your account, without needing an administrator, adds itself to the Start
   menu and the desktop, and opens.

   The installer isn't code-signed yet, so Windows may say "Windows protected your PC". Click **More info**,
   then **Run anyway**.
3. The helper tools are about 160 MB.

Purrloader needs 64-bit Windows 10 or 11. To remove it, use Settings > Apps > Installed apps.

### Linux

There's a package for Debian and Ubuntu, an AppImage that runs on most distributions, and a plain archive. Each
comes for x64 (most PCs) and arm64; the commands below are for x64, so swap in the arm64 file name if you need
it.

**Debian, Ubuntu, Linux Mint, Pop!_OS and other Debian-based distributions**

```bash
wget https://github.com/BLCNYY/Purrloader/releases/latest/download/purrloader_amd64.deb
sudo apt install ./purrloader_amd64.deb
```

Then open Purrloader from your app menu. To update, install the newest .deb the same way. For arm64:
[purrloader_arm64.deb](https://github.com/BLCNYY/Purrloader/releases/latest/download/purrloader_arm64.deb).

**Any other distribution: AppImage**

```bash
wget https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-x86_64.AppImage
chmod +x Purrloader-x86_64.AppImage
./Purrloader-x86_64.AppImage
```

For arm64: [Purrloader-arm64.AppImage](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-arm64.AppImage). If it doesn't open, your system
may be missing FUSE: install it (`sudo apt install fuse3` on Debian and Ubuntu), or start the AppImage with
`--appimage-extract-and-run`.

**Or a plain archive**

```bash
wget https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-linux-x64.tar.gz
mkdir -p ~/.local/opt/purrloader
tar -xzf Purrloader-linux-x64.tar.gz -C ~/.local/opt/purrloader --strip-components=1
~/.local/opt/purrloader/purrloader
```

For arm64: [Purrloader-linux-arm64.tar.gz](https://github.com/BLCNYY/Purrloader/releases/latest/download/Purrloader-linux-arm64.tar.gz).

Purrloader needs a distribution from 2022 or later (glibc 2.35, as in Ubuntu 22.04) and a graphics driver with
OpenGL 3.3, which any PC from the last decade has. The helper tools are about 240 MB on Linux. On GNOME, the tray
icon needs the AppIndicator extension, which Ubuntu includes.

## How to use it

1. **Copy a link** to a video, playlist or song. Purrloader picks it up from the clipboard as soon as you switch
   to it, or you can paste it. You can also drop links, or a text file with one link per line, on the window.
2. **Choose Video or Audio**, and a quality or format if you like. Best is already selected.
3. **Press Download.** Files land in your Downloads folder. Click the folder chip to save somewhere else.

Need subtitles, sponsor skipping or just one part of a video? They're under **More**. Everything else lives in
Settings > Advanced, and you never need to go there. On Windows and Linux, Settings and History are in the ☰ menu
at the top left of the window.

<div align="center">
<img src="docs/images/playlist.png" width="49%" alt="Choosing which videos of a playlist to download">
<img src="docs/images/more-options.png" width="49%" alt="The More menu with subtitles, sponsor skipping and browser sign-in">
</div>

## FAQ

<details>
<summary><b>Which sites work?</b></summary>

Thousands. Purrloader uses [yt-dlp](https://github.com/yt-dlp/yt-dlp), which supports most video, music and
podcast sites; see its [list of supported sites](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md).
If a link isn't supported, Purrloader tells you so. Sites with DRM-protected content (most paid streaming
services) don't work, and Purrloader will never try to get around DRM.
</details>

<details>
<summary><b>Where do my files go?</b></summary>

Your Downloads folder, unless you pick another one with the folder chip next to the Download button or in
Settings > General. Finished downloads stay listed under Recent; double-click one to show it in Finder (Explorer
on Windows, your file manager on Linux). History lists everything you've downloaded: Window > History on a Mac,
the ☰ menu on Windows and Linux.
</details>

<details>
<summary><b>Why does it download extra tools?</b></summary>

Purrloader is the friendly front end; the heavy lifting is done by free, open-source tools:
[yt-dlp](https://github.com/yt-dlp/yt-dlp) reads the sites, [FFmpeg](https://ffmpeg.org) combines and converts
the media, and [Deno](https://deno.com) helps yt-dlp with some sites. They're fetched straight from their
publishers rather than bundled, so you always get current versions and Purrloader can keep them updated. Each
download is verified against its publisher's SHA-256 checksum before it's used. They live in
`~/Library/Application Support/Purrloader/bin` on a Mac, `%APPDATA%\Purrloader\bin` on Windows and
`~/.config/Purrloader/bin` on Linux.
</details>

<details>
<summary><b>How do I download private or age-restricted videos?</b></summary>

Sign in to the site in your browser as usual. Then, in Purrloader, open **More** and choose your browser under
"Sign in with my browser". Purrloader uses that sign-in for the download. If a download needs it, the error
message has a "Use Browser Sign-In" button too. Safari needs Full Disk Access for Purrloader (System Settings >
Privacy & Security); other browsers may ask for Keychain access. On Windows, Firefox works best: Chrome and Edge
keep their sign-in locked, so close them first, and even then they may refuse. On Linux, Chromium-based
browsers may ask for your keyring password.
</details>

<details>
<summary><b>A site stopped working. What now?</b></summary>

Sites change often, and the tools catch up within days. Purrloader updates them by itself, and if a site blocks
a download it updates and retries automatically. You can also update by hand in Settings > About > Update Now,
or switch to the nightly channel there for the very latest fixes. If a site still fails, it's best reported to
[yt-dlp](https://github.com/yt-dlp/yt-dlp/issues).
</details>

<details>
<summary><b>Is it safe?</b></summary>

Purrloader is open source, so anyone can read what it does. It has no analytics or tracking and doesn't send
your links or files anywhere; downloads go straight from the site to your computer. The only other connections
it makes are to GitHub (and, on a Mac, ffmpeg.martin-riedl.de) to fetch and update its helper tools, and to
GitHub once a day to see if a new Purrloader is out. Mac releases are signed and notarized by Apple. The Windows
and Linux builds aren't code-signed yet; each release lists their SHA-256 checksums in `SHA256SUMS.txt`.
</details>

<details>
<summary><b>Does it work on Apple Silicon and Intel Macs? And on Arm PCs?</b></summary>

Yes. Purrloader is a universal app, and it downloads the right helper tools for your Mac's processor. On Windows
and Linux there are separate builds for x64 and arm64, such as Windows on Arm laptops.
</details>

## Responsible use

Only download content you own or have permission to download. Respect creators, copyright and each site's
terms of service. Purrloader doesn't circumvent DRM or other technical protection measures.

## Build from source

The Mac app is native SwiftUI. You need macOS 15 or later and Xcode 26 or later.

```bash
git clone https://github.com/BLCNYY/Purrloader.git
cd Purrloader
./build.sh --run
```

`swift test` runs the tests. `./release.sh` builds a signed and notarized DMG when a Developer ID is configured.

The Windows and Linux app is written in Rust and lives in `desktop/`.
Install [Rust](https://rustup.rs) 1.92 or later. On Windows use the MSVC toolchain and Visual Studio C++ build
tools. On Linux install the development packages listed in [desktop/README.md](desktop/README.md).

```bash
cargo run --bin purrloader
cargo test --locked --all-features
cargo build --locked --release --bin purrloader
```

Native installers are built into `desktop/dist`; see [desktop/README.md](desktop/README.md) for packaging.
On a Mac, `cargo run` keeps its data in a separate folder, so it never touches the Mac app's.
See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the project is laid out and how the pieces fit together.

## Contributing

Bug reports and pull requests are welcome; [CONTRIBUTING.md](CONTRIBUTING.md) covers building, testing and code
style. If a particular site fails, please try updating the helper tools first, and report extractor problems to
[yt-dlp](https://github.com/yt-dlp/yt-dlp/issues).

## Credits

Purrloader stands on the shoulders of:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp), which does the actual downloading (Unlicense)
- [FFmpeg](https://ffmpeg.org), in [macOS builds by Martin Riedl](https://ffmpeg.martin-riedl.de) and
  [Windows and Linux builds by yt-dlp](https://github.com/yt-dlp/FFmpeg-Builds) (GPL)
- [Deno](https://deno.com), a JavaScript runtime yt-dlp uses for some sites (MIT)
- [SponsorBlock](https://sponsor.ajay.app), the community database of sponsor segments (CC BY-NC-SA 4.0)
- [egui and eframe](https://github.com/emilk/egui), for the Windows and Linux interface (MIT/Apache-2.0)
- [Inter](https://rsms.me/inter/) by Rasmus Andersson, the typeface on Windows and Linux (SIL Open Font License)

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for details.

## License

[MIT](LICENSE) © 2026 Ömer Balkan

<div align="center">
<br>
<sub>Made by <a href="https://x.com/BLCNYY">BLCNYY</a></sub>
</div>
