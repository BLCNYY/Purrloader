//! Publisher-verified, versioned helper installs. A running job keeps its resolved paths.
use crate::{
    command::Environment,
    process,
    store::{Paths, atomic_write},
    support::checksum_for,
};
use anyhow::{Context, Result, bail, ensure};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

pub const NAMES: [&str; 4] = ["yt-dlp", "ffmpeg", "ffprobe", "deno"];
#[derive(Debug, Clone, Default)]
pub struct Tools {
    pub paths: BTreeMap<String, PathBuf>,
    pub versions: BTreeMap<String, String>,
}
impl Tools {
    pub fn ready(&self) -> bool {
        NAMES.iter().all(|n| self.paths.contains_key(*n))
    }
    pub fn environment(&self, paths: &Paths) -> Environment {
        Environment {
            ffmpeg: self
                .paths
                .get("ffmpeg")
                .cloned()
                .unwrap_or_else(|| "ffmpeg".into()),
            deno: self
                .paths
                .get("deno")
                .cloned()
                .unwrap_or_else(|| "deno".into()),
            archive: paths.root.join("download-archive.txt"),
            home: paths.home.clone(),
            windows: cfg!(windows),
        }
    }
    pub fn discover(paths: &Paths) -> Self {
        let mut tools = Self::default();
        for name in NAMES {
            let package = if name == "ffprobe" { "ffmpeg" } else { name };
            if let Ok(build) = fs::read_to_string(paths.bin.join(package).join("current")) {
                let build = build.trim();
                if !safe_component(build) {
                    continue;
                }
                let path = paths.bin.join(package).join(build).join(executable(name));
                if path.is_file() {
                    tools.paths.insert(name.into(), path);
                }
            }
        }
        tools
    }
    pub fn refresh_versions(&mut self, paths: &Paths) {
        for name in NAMES {
            if let Some(path) = self.paths.get(name) {
                let args = vec![
                    if name.starts_with("ff") {
                        "-version"
                    } else {
                        "--version"
                    }
                    .into(),
                ];
                let cmd = process::command(path, &args, &paths.temp, &[path.parent().unwrap()]);
                if let Ok((0, output, _)) =
                    process::collect(cmd, crossbeam_channel::never(), Duration::from_secs(120))
                {
                    self.versions.insert(
                        name.into(),
                        output.lines().next().unwrap_or_default().to_string(),
                    );
                } else {
                    self.versions.remove(name);
                }
            }
        }
    }
}
fn safe_component(s: &str) -> bool {
    !s.is_empty()
        && Path::new(s).components().count() == 1
        && matches!(Path::new(s).components().next(), Some(Component::Normal(_)))
}
fn executable(name: &str) -> String {
    let arm = cfg!(target_arch = "aarch64");
    match (name, std::env::consts::OS) {
        ("yt-dlp", "windows") => if arm {
            "yt-dlp_arm64.exe"
        } else {
            "yt-dlp.exe"
        }
        .into(),
        ("yt-dlp", "linux") => if arm {
            "yt-dlp_linux_aarch64"
        } else {
            "yt-dlp_linux"
        }
        .into(),
        ("yt-dlp", _) => "yt-dlp_macos".into(),
        (_, "windows") => format!("{name}.exe"),
        _ => name.into(),
    }
}
pub fn client() -> Result<Client> {
    Client::builder()
        .user_agent(concat!("Purrloader/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .https_only(true)
        .build()
        .map_err(Into::into)
}
pub fn latest_tag(client: &Client, repository: &str) -> Result<String> {
    let response = client
        .get(format!("https://github.com/{repository}/releases/latest"))
        .send()?
        .error_for_status()?;
    let path = response.url().path();
    let (_, tag) = path
        .rsplit_once("/tag/")
        .context("Unexpected release redirect")?;
    ensure!(safe_component(tag), "Invalid release tag");
    Ok(tag.to_owned())
}
struct Source {
    archive: String,
    checksums: String,
}
fn sources(client: &Client, package: &str, channel: &str) -> Result<Vec<Source>> {
    let arm = cfg!(target_arch = "aarch64");
    let os = std::env::consts::OS;
    let (archive, checksums) = match package {
        "yt-dlp" => {
            let repo = if channel == "nightly" {
                "yt-dlp/yt-dlp-nightly-builds"
            } else {
                "yt-dlp/yt-dlp"
            };
            let tag = latest_tag(client, repo)?;
            let asset = match os {
                "windows" => {
                    if arm {
                        "yt-dlp_win_arm64.zip"
                    } else {
                        "yt-dlp_win.zip"
                    }
                }
                "linux" => {
                    if arm {
                        "yt-dlp_linux_aarch64.zip"
                    } else {
                        "yt-dlp_linux.zip"
                    }
                }
                _ => "yt-dlp_macos.zip",
            };
            let base = format!("https://github.com/{repo}/releases/download/{tag}");
            (format!("{base}/{asset}"), format!("{base}/SHA2-256SUMS"))
        }
        "ffmpeg" if os == "macos" => {
            let mut sources = vec![];
            for tool in ["ffmpeg", "ffprobe"] {
                let arch = if arm { "arm64" } else { "amd64" };
                let response = client.get(format!("https://ffmpeg.martin-riedl.de/redirect/latest/macos/{arch}/release/{tool}.zip")).send()?.error_for_status()?;
                let archive = response.url().to_string();
                sources.push(Source {
                    checksums: format!("{archive}.sha256"),
                    archive,
                });
            }
            return Ok(sources);
        }
        "ffmpeg" => {
            let asset = if os == "windows" {
                format!(
                    "ffmpeg-master-latest-{}-gpl-shared.zip",
                    if arm { "winarm64" } else { "win64" }
                )
            } else {
                format!(
                    "ffmpeg-master-latest-{}-gpl.tar.xz",
                    if arm { "linuxarm64" } else { "linux64" }
                )
            };
            let base = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest";
            (
                format!("{base}/{asset}"),
                format!("{base}/checksums.sha256"),
            )
        }
        "deno" => {
            let arch = if arm { "aarch64" } else { "x86_64" };
            let target = match os {
                "windows" => "pc-windows-msvc",
                "linux" => "unknown-linux-gnu",
                _ => "apple-darwin",
            };
            let tag = latest_tag(client, "denoland/deno")?;
            let archive = format!(
                "https://github.com/denoland/deno/releases/download/{tag}/deno-{arch}-{target}.zip"
            );
            (archive.clone(), format!("{archive}.sha256sum"))
        }
        _ => bail!("Unknown helper {package}"),
    };
    Ok(vec![Source { archive, checksums }])
}

pub fn verify_and_unpack(archive: &Path, destination: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(archive)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected.to_lowercase(),
        "The downloaded helper didn't match its published SHA-256 checksum"
    );
    fs::create_dir_all(destination)?;
    if archive.extension().is_some_and(|s| s == "zip") {
        let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let relative = entry
                .enclosed_name()
                .context("Unsafe path in helper archive")?;
            let target = destination.join(relative);
            // Symlink entries could redirect subsequent extraction outside the destination.
            ensure!(
                !entry.is_symlink(),
                "Unexpected symbolic link in ZIP archive"
            );
            if entry.is_dir() {
                fs::create_dir_all(&target)?;
                continue;
            }
            fs::create_dir_all(target.parent().context("Missing parent")?)?;
            let mut output = File::create(&target)?;
            std::io::copy(&mut entry, &mut output)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o777))?;
                }
            }
        }
    } else {
        let reader = xz2::read::XzDecoder::new(File::open(archive)?);
        let mut tar = tar::Archive::new(reader);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let kind = entry.header().entry_type();
            ensure!(
                kind.is_file() || kind.is_dir(),
                "Unexpected link or special file in helper archive"
            );
            ensure!(
                entry.unpack_in(destination)?,
                "Unsafe path in helper archive"
            );
        }
    }
    Ok(())
}

pub fn install(
    paths: &Paths,
    package: &str,
    channel: &str,
    mut progress: impl FnMut(String),
) -> Result<()> {
    let client = client()?;
    let sources = sources(&client, package, channel)?;
    let work = tempfile::Builder::new()
        .prefix("install-")
        .tempdir_in(&paths.temp)?;
    for (i, source) in sources.iter().enumerate() {
        let name = source
            .archive
            .rsplit('/')
            .next()
            .context("Missing archive name")?;
        ensure!(safe_component(name), "Invalid archive name");
        let sums = client
            .get(&source.checksums)
            .send()?
            .error_for_status()?
            .text()?;
        let expected =
            checksum_for(name, &sums).context("No published SHA-256 checksum was found")?;
        let archive = work.path().join(name);
        let mut response = client.get(&source.archive).send()?.error_for_status()?;
        let total = response.content_length().unwrap_or(0);
        let mut output = File::create(&archive)?;
        let mut buffer = [0; 65536];
        let mut written = 0u64;
        let mut last_report = std::time::Instant::now();
        loop {
            let n = response.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            output.write_all(&buffer[..n])?;
            written += n as u64;
            if last_report.elapsed() > Duration::from_millis(150) {
                progress(format!(
                    "Installing {package} · {} / {}",
                    crate::support::bytes(written as f64),
                    crate::support::bytes(total as f64)
                ));
                last_report = std::time::Instant::now();
            }
        }
        drop(output);
        progress(format!("Verifying and unpacking {package}…"));
        verify_and_unpack(
            &archive,
            &work.path().join(format!("unpacked-{i}")),
            &expected,
        )?;
        fs::remove_file(archive)?;
    }
    let name = executable(package);
    let find = |name: &str| {
        walkdir::WalkDir::new(work.path())
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.file_type().is_file() && e.file_name().to_str() == Some(name))
            .map(|e| e.path().to_path_buf())
    };
    let main = find(&name).context("The helper executable is missing from its archive")?;
    let secondary = if package == "ffmpeg" {
        Some(find(&executable("ffprobe")).context("ffprobe is missing from its archive")?)
    } else {
        None
    };
    let build = uuid::Uuid::new_v4().to_string();
    let root = paths.bin.join(package);
    fs::create_dir_all(&root)?;
    let build_dir = root.join(&build);
    let main_parent = main.parent().context("Missing executable folder")?;
    if let Some(secondary) = secondary.filter(|p| p.parent() != Some(main_parent)) {
        fs::copy(&secondary, main_parent.join(executable("ffprobe")))?;
    }
    fs::rename(main_parent, &build_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for executable in [Some(name), (package == "ffmpeg").then(|| "ffprobe".into())]
            .into_iter()
            .flatten()
        {
            fs::set_permissions(
                build_dir.join(executable),
                fs::Permissions::from_mode(0o755),
            )?;
        }
    }
    atomic_write(&root.join("current"), build.as_bytes())?;
    Ok(())
}

pub fn bootstrap(
    paths: &Paths,
    channel: &str,
    update: bool,
    reinstall: bool,
    mut progress: impl FnMut(String),
) -> Result<Tools> {
    let mut tools = Tools::discover(paths);
    for package in ["yt-dlp", "ffmpeg", "deno"] {
        let missing = !tools.paths.contains_key(package)
            || (package == "ffmpeg" && !tools.paths.contains_key("ffprobe"));
        if missing || reinstall {
            progress(format!("Installing {package}…"));
            install(paths, package, channel, &mut progress)?;
        }
    }
    tools = Tools::discover(paths);
    tools.refresh_versions(paths);
    if update && !reinstall {
        let repo = if channel == "nightly" {
            "yt-dlp/yt-dlp-nightly-builds"
        } else {
            "yt-dlp/yt-dlp"
        };
        let tag = latest_tag(&client()?, repo)?;
        if tools.versions.get("yt-dlp").is_none_or(|s| s.trim() != tag) {
            install(paths, "yt-dlp", channel, &mut progress)?;
            tools = Tools::discover(paths);
            tools.refresh_versions(paths);
        }
    }
    ensure!(
        NAMES.iter().all(|n| tools.versions.contains_key(*n)),
        "A helper could not start. Try reinstalling the helper tools."
    );
    Ok(tools)
}
