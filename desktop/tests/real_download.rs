//! Optional local end-to-end test: cargo test --test real_download -- --ignored
//! Requires yt-dlp, ffmpeg, ffprobe and deno on PATH. Uses only a local HTTP server.
use purrloader::{
    engine::{Engine, Job, Status},
    options::Options,
    store::Paths,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires installed media tools"]
fn real_ytdlp_downloads_and_ffmpeg_extracts_playable_audio() {
    let dir = tempfile::tempdir().unwrap();
    let locate = |name: &str| -> PathBuf {
        std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .flat_map(|dir| {
                if cfg!(windows) {
                    vec![dir.join(format!("{name}.exe"))]
                } else {
                    vec![dir.join(name)]
                }
            })
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{name} must be installed for this test"))
    };
    let ffmpeg = locate("ffmpeg");
    let ffprobe = locate("ffprobe");
    let source = dir.path().join("source.mp4");
    assert!(
        Command::new(&ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=320x180:r=24:d=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
                "-movflags",
                "+faststart"
            ])
            .arg(&source)
            .status()
            .unwrap()
            .success()
    );
    let media = std::fs::read(&source).unwrap();
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let url = format!("http://{}/source.mp4", server.local_addr().unwrap());
    let done = Arc::new(AtomicBool::new(false));
    let stop = done.clone();
    let thread = std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if let Ok((mut stream, _)) = server.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 8192];
                let n = stream.read(&mut request).unwrap_or(0);
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    media.len()
                );
                let _ = stream.write_all(headers.as_bytes());
                if !request[..n].starts_with(b"HEAD ") {
                    let _ = stream.write_all(&media);
                }
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    });
    let paths = Paths::new(
        dir.path().join("data"),
        dir.path().join("downloads"),
        dir.path().into(),
    )
    .unwrap();
    let mut engine = Engine::load(paths, || {}).unwrap();
    engine.settings.notifications = false;
    for name in purrloader::tools::NAMES {
        engine.tools.paths.insert(name.into(), locate(name));
        engine.tools.versions.insert(name.into(), "test".into());
    }
    let mut options = Options::default();
    options.format.mode = "audio".into();
    options.format.audio_format = "wav".into();
    engine.enqueue(Job::new(url, options, engine.folder()));
    let start = Instant::now();
    while !matches!(engine.jobs[0].status, Status::Finished | Status::Failed)
        && start.elapsed() < Duration::from_secs(60)
    {
        engine.poll();
        std::thread::sleep(Duration::from_millis(50));
    }
    done.store(true, Ordering::Relaxed);
    thread.join().unwrap();
    assert_eq!(
        engine.jobs[0].status,
        Status::Finished,
        "{}",
        engine.jobs[0].log.join("\n")
    );
    let file = &engine.jobs[0].files[0];
    assert_eq!(file.extension().unwrap(), "wav");
    let probe = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,codec_type",
            "-of",
            "json",
        ])
        .arg(file)
        .output()
        .unwrap();
    assert!(probe.status.success());
    let data: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(data["streams"][0]["codec_name"], "pcm_s16le");
    assert!(
        Command::new(ffmpeg)
            .args(["-v", "error", "-xerror", "-i"])
            .arg(file)
            .args(["-f", "null", "-"])
            .status()
            .unwrap()
            .success()
    );
}
