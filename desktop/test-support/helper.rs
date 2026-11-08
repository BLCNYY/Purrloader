//! Deterministic yt-dlp stand-in for process/queue integration tests. Never packaged.
use std::{io::Write, path::PathBuf, time::Duration};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if args.iter().any(|a| a == "--version" || a == "-version") {
        println!("test-helper 1.0");
        return;
    }
    if args.iter().any(|a| a == "--echo") {
        println!("{}", serde_json::to_string(&args[1..]).unwrap());
        return;
    }
    if let Some(path) = value("--child-sentinel") {
        std::fs::write(format!("{path}.started"), b"started").unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        std::fs::write(format!("{path}.survived"), b"alive").unwrap();
        return;
    }
    let url = args.last().cloned().unwrap_or_default();
    if args.iter().any(|a| a == "-J") {
        if url.contains("slow") {
            std::thread::sleep(Duration::from_secs(30));
        }
        println!(r#"{{"title":"Fixture video","duration":61,"formats":[],"_type":"video"}}"#);
        return;
    }
    if url.contains("fail") {
        eprintln!("ERROR: Requested format is not available");
        std::process::exit(1);
    }
    let folder = PathBuf::from(value("-P").unwrap());
    std::fs::create_dir_all(&folder).unwrap();
    if url.contains("tree") {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--child-sentinel")
            .arg(folder.join("child"))
            .spawn()
            .unwrap();
        let _ = child.wait();
        std::thread::sleep(Duration::from_secs(30));
    }
    let output = folder.join("fixture.mp4");
    println!("[download] Destination: {}", output.display());
    println!("[purrloader]downloading|50|100|NA|10|5|Fixture video");
    std::io::stdout().flush().unwrap();
    if url.contains("slow") {
        std::thread::sleep(Duration::from_secs(30));
    }
    // Emit enough stderr to catch implementations that don't drain both pipes.
    for _ in 0..10000 {
        eprintln!("[test] stderr output");
    }
    std::fs::write(&output, b"test fixture").unwrap();
    if let Some(i) = args.iter().position(|a| a == "--print-to-file") {
        std::fs::write(&args[i + 2], format!("{}\n", output.display())).unwrap();
    }
    println!("[purrloader]finished|100|100|NA|NA|NA|Fixture video");
}
