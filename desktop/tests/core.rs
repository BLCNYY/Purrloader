use purrloader::{
    command::Environment, error::FriendlyError, options::Options, progress::Progress, store,
    support,
};
use serde::Deserialize;

fn env(windows: bool) -> Environment {
    Environment {
        ffmpeg: "/tools/ffmpeg".into(),
        deno: "/tools/deno".into(),
        archive: "/data/download-archive.txt".into(),
        home: "/home/person".into(),
        windows,
    }
}
#[derive(Deserialize)]
struct Case {
    name: String,
    options: Options,
    platform: String,
    selection: Option<String>,
    args: Vec<String>,
    info: Vec<String>,
}
#[test]
fn commands_match_golden_cases() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/commands.json")).unwrap();
    for case in cases {
        let env = env(case.platform == "win32");
        let args = env
            .download(
                &case.options,
                "https://example.com/video",
                "/downloads".as_ref(),
                case.selection.as_deref(),
                Some("/data/paths".as_ref()),
            )
            .unwrap();
        assert_eq!(args, case.args, "{}", case.name);
        assert_eq!(
            env.info(&case.options, "https://example.com/video"),
            case.info,
            "{} info",
            case.name
        );
    }
}
#[test]
fn bad_extra_quotes_are_reported_and_urls_cannot_be_flags() {
    let mut o = Options {
        extra_arguments: "--match-filters 'unfinished".into(),
        ..Options::default()
    };
    assert!(
        env(false)
            .download(&o, "--exec=bad", "/out".as_ref(), None, None)
            .is_err()
    );
    o.extra_arguments.clear();
    let args = env(false)
        .download(&o, "--exec=bad", "/out".as_ref(), None, None)
        .unwrap();
    assert_eq!(&args[args.len() - 2..], &["--", "--exec=bad"]);
}
#[test]
fn option_groups_recover_independently() {
    let o: Options = serde_json::from_value(serde_json::json!({"format":{"mode":"hologram"},"output":{"template":"%(id)s.%(ext)s"},"network":{"retries":3},"subtitles":{"enabled":true}})).unwrap();
    assert_eq!(o.format, Options::default().format);
    assert_eq!(o.output.template, "%(id)s.%(ext)s");
    assert_eq!(o.network.retries, 3);
    assert!(o.subtitles.enabled);
    let o: Options = serde_json::from_value(
        serde_json::json!({"format":{"mode":"audio","audioQuality":8},"extras":null}),
    )
    .unwrap();
    assert_eq!(o, Options::default());
    let defaults = Options::default();
    assert_eq!(
        serde_json::from_str::<Options>(&serde_json::to_string(&defaults).unwrap()).unwrap(),
        defaults
    );
}
#[test]
fn progress_handles_streams_playlists_unknowns_and_bars_in_titles() {
    let mut p = Progress::default();
    assert!(!p.consume("[purrloader]finished|800|800|NA|NA|NA|Clip"));
    p.consume("[purrloader]downloading|100|200|NA|50|2|Clip | part two");
    assert_eq!(p.downloaded, 900.0);
    assert_eq!(p.total, 1000.0);
    assert_eq!(p.fraction, Some(0.9));
    assert_eq!(p.title.as_deref(), Some("Clip | part two"));
    p.consume("[download] Downloading item 2 of 4");
    p.consume("[purrloader]downloading|50|NA|100|NA|NA|Second");
    assert_eq!(p.fraction, Some(0.375));
    assert_eq!(p.eta, None);
    p.consume("[purrloaderpp]started|Merger");
    assert_eq!(p.phase, "processing");
    assert_eq!(p.fraction, None);
    p.consume("ERROR: Video unavailable");
    assert_eq!(p.error.as_deref(), Some("Video unavailable"));
    p.consume("[purrloader]downloading|NaN|inf|NA|NA|NA|NA");
    assert_eq!(p.fraction, None);
    assert_eq!(p.title, None);
}
#[test]
fn line_buffer_preserves_split_utf8_and_flushes_crlf() {
    let mut buffer = purrloader::process::LineBuffer::default();
    let bytes = "Çay\r\nKedi".as_bytes();
    assert!(buffer.append(&bytes[..1]).is_empty());
    assert_eq!(buffer.append(&bytes[1..]), ["Çay"]);
    assert_eq!(buffer.flush().as_deref(), Some("Kedi"));
    assert_eq!(buffer.flush(), None);
}
#[test]
fn friendly_errors_cover_windows_and_linux_failures() {
    for (raw, kind) in [
        ("Failed to decrypt with DPAPI", "browserCookies"),
        ("Private video. Sign in", "signInRequired"),
        ("HTTP Error 403", "blocked"),
        ("[WinError 5] Access is denied", "folderNotWritable"),
        ("[Errno 28] No space left on device", "diskFull"),
        ("getaddrinfo failed", "network"),
        ("Requested format is not available", "formatUnavailable"),
        ("This live event will begin in 3 hours", "notStarted"),
    ] {
        let error = FriendlyError::from_raw(raw);
        assert_eq!(error.kind, kind);
        assert!(!error.fix_label().is_empty());
    }
}
#[test]
fn helpers_parse_untrusted_metadata() {
    assert_eq!(
        support::compress_selection(vec![9, 1, 2, 2, 10, 0]),
        "1:2,9:10"
    );
    assert_eq!(
        support::web_links("watch https://example.com/v. and ftp://bad"),
        ["https://example.com/v"]
    );
    let hash = "ab".repeat(32);
    assert_eq!(
        support::checksum_for("deno.zip", &format!("{hash} *deno.zip\n")),
        Some(hash.clone())
    );
    assert_eq!(
        support::checksum_for("other.zip", &format!("{hash} *deno.zip\n")),
        None
    );
    assert!(support::newer("v1.10", "1.9"));
    assert!(!support::newer("1.0", "1.0.0"));
    let media = purrloader::media::Media::parse(
        serde_json::json!({"_type":"playlist","entries":[null,{"title":"Second"}],"formats":[{"format_id":"storyboard","vcodec":"none","acodec":"none"}]}),
        "https://example.com",
    );
    assert_eq!(media.entries[0].index, 2);
    assert!(media.formats.is_empty());
}
#[test]
fn persistence_is_atomic_and_preserves_future_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("settings.json");
    std::fs::write(&file, include_str!("fixtures/settings.json")).unwrap();
    let mut settings: store::Settings = store::load(&file).unwrap().unwrap();
    assert_eq!(settings.last_options, Some(Options::default()));
    settings.max_concurrent = 5;
    store::save(&file, &settings).unwrap();
    assert_eq!(
        store::load::<store::Settings>(&file)
            .unwrap()
            .unwrap()
            .max_concurrent,
        5
    );
    let future = br#"{"schemaVersion":2,"content":{"keep":"me"}}"#;
    std::fs::write(&file, future).unwrap();
    assert!(store::load::<store::Settings>(&file).is_err());
    assert!(store::save(&file, &settings).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), future);
}
#[test]
fn archive_checksum_and_traversal_are_checked_before_install() {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("tool.zip");
    let write_zip = |name: &str| {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"tool").unwrap();
        zip.finish().unwrap();
        format!("{:x}", Sha256::digest(std::fs::read(&archive).unwrap()))
    };
    let hash = write_zip("bin/tool");
    assert!(
        purrloader::tools::verify_and_unpack(
            &archive,
            &dir.path().join("bad-hash"),
            &"00".repeat(32)
        )
        .is_err()
    );
    assert!(!dir.path().join("bad-hash").exists());
    purrloader::tools::verify_and_unpack(&archive, &dir.path().join("good"), &hash).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("good/bin/tool")).unwrap(),
        b"tool"
    );
    let hash = write_zip("../escape");
    assert!(
        purrloader::tools::verify_and_unpack(&archive, &dir.path().join("unsafe"), &hash).is_err()
    );
    assert!(!dir.path().join("escape").exists());
}

#[test]
fn windows_command_display_uses_literal_powershell_arguments() {
    let args = [
        "C:\\Program Files\\yt-dlp.exe",
        "say \"hi\"",
        "it's literal $HOME",
        "",
    ]
    .map(String::from);
    assert_eq!(
        support::command_line(&args, true),
        "& 'C:\\Program Files\\yt-dlp.exe' 'say \"hi\"' 'it''s literal $HOME' ''"
    );
    assert_eq!(
        shell_words::split(&support::command_line(&args, false)).unwrap(),
        args
    );
}

#[test]
fn filename_preview_handles_dates_padding_and_platform_characters() {
    use purrloader::media::{Media, template_preview};
    let mut options = Options::default();
    assert_eq!(
        template_preview(
            "%(playlist_index)03d - %(upload_date>%Y-%m-%d)s.%(ext)s",
            None,
            &options,
            false
        ),
        "001 - 2024-01-31.mp4"
    );
    let media = Media::parse(
        serde_json::json!({"title":"Çay: A/B?"}),
        "https://example.com",
    );
    assert_eq!(
        template_preview("%(title)s", Some(&media), &options, true),
        "Çay - A⧸B？"
    );
    options.output.restrict_filenames = true;
    assert_eq!(
        template_preview("%(title)s", Some(&media), &options, false),
        "Cay_-_A_B_"
    );
    assert_eq!(
        template_preview("%(missing|fallback)s", None, &options, false),
        "fallback"
    );
}
