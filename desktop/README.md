# Rust desktop app

Purrloader for Windows 10/11 and Linux, on x64 and arm64. The interface and application logic are Rust.
macOS builds here are for development; the released Mac app remains SwiftUI.

## Build

Install Rust 1.92 or later with rustup. Windows needs the MSVC C++ build tools and Windows SDK.
On Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libssl-dev libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev \
  libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev liblzma-dev
```

From the repository root:

```sh
cargo run --locked --bin purrloader
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo build --locked --release --bin purrloader
```

The default build includes the app only. `test-helper` enables a deterministic helper executable for integration
tests. It must never be included in packages. `VERSION` is the display and installer version; Cargo's package
version is independent metadata. Commit `Cargo.lock` when dependencies change.

Linux needs OpenGL 3.3, a running X11 or Wayland session, and glibc 2.35 or later. Native file dialogs use
xdg-desktop-portal. Tray integration uses StatusNotifier over D-Bus; GNOME needs the AppIndicator extension.
The .deb declares graphics and desktop dependencies. Windows packages statically link the Visual C++ runtime.

## Local verification

```sh
cargo run --locked --bin purrloader -- --smoke-test --demo --screenshot /tmp/purrloader.png
```

Smoke mode uses temporary data, makes no network requests, skips the tray and closes automatically after
rendering. On headless Linux use `dbus-run-session -- xvfb-run -a` and `LIBGL_ALWAYS_SOFTWARE=1`.
`--version` prints the version without starting a window. For interactive development, set
`PURRLOADER_DATA_DIR` to an empty directory to avoid touching your normal queue.

## Data

Settings, the queue, history and saved setups live in `%APPDATA%\Purrloader` or `~/.config/Purrloader`.
Paused downloads stay paused; interrupted ones resume once the helper tools are checked. A damaged file or one
from a newer version produces an error and is left untouched. Uninstalling keeps app data and downloads.

## Look

The windows are drawn to match the Mac app point for point: the same colours, sizes and controls. Inter
(`assets/fonts`, SIL Open Font License) stands in for SF Pro, and `src/icons.rs` draws look-alikes of the SF
Symbols, since Apple's fonts and symbols may only be used on Apple platforms. `src/theme.rs` holds the colours
and fonts, `src/widgets.rs` the controls. `--smoke-test --demo --screen <name> --screenshot <file>` renders one
screen for comparison (`main`, `more`, `playlist`, `details`, `welcome`, `general`, `advanced`, `about`,
`history`, or `advanced-<section>`).

## Packaging

Build packages on the target operating system. CI uses native x64 and arm64 runners for both platforms.
Linux additionally needs `dpkg-deb`, `squashfs-tools` and `desktop-file-utils`. Windows needs NSIS on PATH or
installed in `Program Files (x86)/NSIS`.

```sh
cargo install cargo-about --locked --version 0.9.3 --features cli
cargo about generate desktop/packaging/licenses.hbs -o THIRD_PARTY_LICENSES.html
# Linux x64 example; use --arch arm64 for an ARM binary.
python3 desktop/packaging/package.py --arch x64 --binary target/release/purrloader \
  --licenses THIRD_PARTY_LICENSES.html
# Windows: same command with python and target/release/purrloader.exe.
```

Use `--format archive`, `deb`, `appimage` or `nsis` to build one format. Artifacts go into `desktop/dist`:

| Platform | x64 | arm64 |
| --- | --- | --- |
| Windows | `Purrloader-Setup-x64.exe` | `Purrloader-Setup-arm64.exe` |
| Debian | `purrloader_amd64.deb` | `purrloader_arm64.deb` |
| AppImage | `Purrloader-x86_64.AppImage` | `Purrloader-arm64.AppImage` |
| Archive | `Purrloader-linux-x64.tar.gz` | `Purrloader-linux-arm64.tar.gz` |

The packaging script validates the binary's PE/ELF architecture. AppImage tooling is fetched from its publisher
and verified against the release asset's SHA-256 digest before execution. The Windows installer is per-user,
creates Start Menu/desktop shortcuts, and offers to open the app. Signing is not configured for these platforms.
A tag release uploads packages and `SHA256SUMS.txt` through the existing draft-release workflow.
