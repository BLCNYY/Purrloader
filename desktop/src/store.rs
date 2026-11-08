//! Version-one JSON compatibility and atomic, crash-safe writes.
use crate::options::Options;
use anyhow::{Context, Result, bail};
use directories::{BaseDirs, UserDirs};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub bin: PathBuf,
    pub temp: PathBuf,
    pub downloads: PathBuf,
    pub home: PathBuf,
}
impl Paths {
    pub fn discover() -> Result<Self> {
        let base = BaseDirs::new().context("Couldn't find your user folder")?;
        let name = if cfg!(target_os = "macos") {
            "Purrloader (Development)"
        } else {
            "Purrloader"
        };
        let root = std::env::var_os("PURRLOADER_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.config_dir().join(name));
        let downloads = UserDirs::new()
            .and_then(|u| u.download_dir().map(PathBuf::from))
            .unwrap_or_else(|| base.home_dir().join("Downloads"));
        Self::new(root, downloads, base.home_dir().into())
    }
    pub fn new(root: PathBuf, downloads: PathBuf, home: PathBuf) -> Result<Self> {
        let bin = root.join("bin");
        let temp = root.join("tmp");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&temp)?;
        Ok(Self {
            root,
            bin,
            temp,
            downloads,
            home,
        })
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.json"))
    }
    pub fn lock(&self) -> Result<File> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("app.lock"))?;
        fs2::FileExt::try_lock_exclusive(&file).context("Purrloader is already running")?;
        Ok(file)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T> {
    schema_version: u32,
    content: T,
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let envelope: Envelope<serde_json::Value> =
        serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "Couldn't read {}. The original file has been preserved.",
                path.display()
            )
        })?;
    if envelope.schema_version != 1 {
        bail!(
            "{} uses an unsupported schema version {}; the original file has been preserved",
            path.display(),
            envelope.schema_version
        );
    }
    serde_json::from_value(envelope.content)
        .with_context(|| format!("Couldn't read {}", path.display()))
        .map(Some)
}
pub fn save<T: Serialize>(path: &Path, content: &T) -> Result<()> {
    // A newer app may have written the file while this one was closed. Never downgrade it.
    if path.exists() {
        let _: Option<serde_json::Value> = load(path)?;
    }
    atomic_write(
        path,
        &serde_json::to_vec_pretty(&Envelope {
            schema_version: 1,
            content,
        })?,
    )
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Missing parent folder")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub download_folder: Option<PathBuf>,
    pub last_options: Option<Options>,
    pub max_concurrent: usize,
    pub notifications: bool,
    pub clipboard_autofill: bool,
    pub show_tray_icon: bool,
    pub auto_update_ytdlp: bool,
    pub ytdlp_channel: String,
    pub did_show_welcome: bool,
    pub last_app_update_check: Option<String>,
    pub latest_app_version: Option<String>,
    pub latest_app_page: Option<String>,
    pub dismissed_app_version: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            download_folder: None,
            last_options: None,
            max_concurrent: 3,
            notifications: true,
            clipboard_autofill: true,
            show_tray_icon: true,
            auto_update_ytdlp: true,
            ytdlp_channel: "stable".into(),
            did_show_welcome: false,
            last_app_update_check: None,
            latest_app_version: None,
            latest_app_page: None,
            dismissed_app_version: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub options: Options,
}
impl Preset {
    pub fn new(name: &str, mut options: Options) -> Self {
        options.format.exact_format.clear();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            options,
        }
    }
    pub fn defaults() -> Vec<Self> {
        let mut presets = vec![Self::new("Default", Options::default())];
        let mut o = Options::default();
        o.format.max_height = Some(1080);
        o.format.codec = "h264".into();
        presets.push(Self::new("1080p MP4 (H.264)", o));
        let mut o = Options::default();
        o.format.mode = "audio".into();
        o.format.audio_quality = "128K".into();
        o.extras.sponsor_block = "remove".into();
        o.output.template = "%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s".into();
        presets.push(Self::new("Podcast MP3", o));
        let mut o = Options::default();
        o.format.max_height = Some(2160);
        o.format.container = "mkv".into();
        o.subtitles.enabled = true;
        o.subtitles.languages = "all,-live_chat".into();
        o.extras.write_info_json = true;
        o.extras.write_description = true;
        o.playlist.use_archive = true;
        o.output.template =
            "%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s".into();
        presets.push(Self::new("Archive 4K MKV", o));
        let mut o = Options::default();
        o.format.mode = "audio".into();
        o.format.audio_format = "flac".into();
        presets.push(Self::new("Music FLAC", o));
        presets
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub id: String,
    pub url: String,
    pub title: String,
    pub date: String,
    pub folder: PathBuf,
    #[serde(default)]
    pub files: Vec<PathBuf>,
    pub thumbnail: Option<String>,
    pub options: Options,
}
