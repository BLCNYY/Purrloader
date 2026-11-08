# Contributing

Thanks for helping. Purrloader aims to be the simplest way for people to save a video or its audio,
so the bar for new UI in the main window is high. Power features belong in Settings > Advanced.

## Build and run

You need macOS 15 or later and Xcode 26 or later (for the Swift 6 toolchain and the Icon Composer icon).

```bash
./build.sh --run       # build into ./build and launch
./build.sh --install   # build, copy to /Applications, launch
```

Set `PURRLOADER_SIGN_IDENTITY` to a signing identity from your keychain if you want macOS to keep permissions
(like Downloads folder access) across rebuilds. Without it the build is ad-hoc signed.

## Windows and Linux

The Windows and Linux app in `desktop/` is a port of the Mac app with the same features, words and look. A change
to one usually belongs in the other too: its core mirrors `Sources/PurrloaderCore`, and
`desktop/tests/fixtures/commands.json` holds the yt-dlp argument cases both must agree on.
Use Rust 1.92 or later and the platform prerequisites in [desktop/README.md](desktop/README.md).

```bash
cargo run --bin purrloader       # macOS development uses an isolated data folder
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked --release --bin purrloader
```

The `test-helper` feature builds a deterministic subprocess fixture for integration tests. It is excluded from
release packages. Tests cover yt-dlp arguments, persistence, checksum verification, path traversal,
process cancellation, queue scheduling and restart behavior. `--smoke-test` opens an offline window with
temporary data, renders a few frames, then exits. Add `--demo --screenshot /path/to/smoke.png` for visual QA.

Package on the target OS with `desktop/packaging/package.py`. CI tests Windows/Linux on x64 and arm64.

## Test

```bash
swift test
```

Tests cover the non-UI library, `PurrloaderCore`: command building, progress parsing, error mapping and so on.
If you change how options turn into yt-dlp arguments, add or update a test in `Tests/PurrloaderCoreTests`.

## Style

- Format with the bundled config before sending a pull request: `swift format -i -r Sources Tests`
- Swift 6 language mode with strict concurrency. No `DispatchQueue`; use async/await.
- Plain language in the UI. Words like "remux", "codec" or "yt-dlp" only appear in Settings > Advanced.
- Comments explain why, not what.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the code is laid out.

## A site stopped working?

Purrloader doesn't contain any site-specific code; [yt-dlp](https://github.com/yt-dlp/yt-dlp) does all the
reading. So:

1. Update yt-dlp first: Settings > About > Update Now. Try the nightly channel too, as fixes land there first.
2. If the site still fails, it's an extractor bug. Please report it to
   [yt-dlp](https://github.com/yt-dlp/yt-dlp/issues), not here, following their template.
3. Open an issue here only if yt-dlp on its own works but Purrloader doesn't.
