//! Persisted options retain the original camelCase JSON shape.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Format {
    pub mode: String,
    pub max_height: Option<u32>,
    pub audio_format: String,
    pub audio_quality: String,
    pub container: String,
    pub container_method: String,
    pub codec: String,
    #[serde(rename = "allowHDR")]
    pub allow_hdr: bool,
    pub exact_format: String,
}
impl Default for Format {
    fn default() -> Self {
        Self {
            mode: "video".into(),
            max_height: None,
            audio_format: "mp3".into(),
            audio_quality: "0".into(),
            container: "mp4".into(),
            container_method: "remux".into(),
            codec: "any".into(),
            allow_hdr: true,
            exact_format: "".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Playlist {
    pub no_playlist: bool,
    pub range: String,
    pub reverse: bool,
    pub latest_count: u32,
    pub use_archive: bool,
}
impl Default for Playlist {
    fn default() -> Self {
        Self {
            no_playlist: false,
            range: "".into(),
            reverse: false,
            latest_count: 0,
            use_archive: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Subtitles {
    pub enabled: bool,
    pub include_auto_generated: bool,
    pub languages: String,
    pub embed: bool,
    #[serde(rename = "convertToSRT")]
    pub convert_to_srt: bool,
}
impl Default for Subtitles {
    fn default() -> Self {
        Self {
            enabled: false,
            include_auto_generated: true,
            languages: "en.*".into(),
            embed: true,
            convert_to_srt: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Extras {
    pub write_thumbnail: bool,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    pub split_chapters: bool,
    pub sponsor_block: String,
    pub sponsor_categories: Vec<String>,
    pub section_start: String,
    pub section_end: String,
    pub chapter_regex: String,
    pub force_keyframes: bool,
    pub live_from_start: bool,
    pub wait_for_video: bool,
    pub wait_interval: u32,
    pub write_description: bool,
    pub write_comments: bool,
    #[serde(rename = "writeInfoJSON")]
    pub write_info_json: bool,
}
impl Default for Extras {
    fn default() -> Self {
        Self {
            write_thumbnail: false,
            embed_thumbnail: true,
            embed_metadata: true,
            embed_chapters: true,
            split_chapters: false,
            sponsor_block: "off".into(),
            sponsor_categories: vec!["sponsor".into(), "selfpromo".into(), "interaction".into()],
            section_start: "".into(),
            section_end: "".into(),
            chapter_regex: "".into(),
            force_keyframes: false,
            live_from_start: false,
            wait_for_video: false,
            wait_interval: 60,
            write_description: false,
            write_comments: false,
            write_info_json: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Network {
    pub cookie_browser: String,
    pub cookie_profile: String,
    pub cookies_file: String,
    pub proxy: String,
    pub rate_limit: String,
    pub retries: u32,
    pub concurrent_fragments: u32,
    pub use_aria2c: bool,
    pub impersonate: String,
    pub xff: String,
    pub user_agent: String,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            cookie_browser: "none".into(),
            cookie_profile: "".into(),
            cookies_file: "".into(),
            proxy: "".into(),
            rate_limit: "".into(),
            retries: 10,
            concurrent_fragments: 1,
            use_aria2c: false,
            impersonate: "".into(),
            xff: "".into(),
            user_agent: "".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Output {
    pub template: String,
    pub restrict_filenames: bool,
    pub overwrite: String,
}
impl Default for Output {
    fn default() -> Self {
        Self {
            template: "%(title)s.%(ext)s".into(),
            restrict_filenames: false,
            overwrite: "skipExisting".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub format: Format,
    pub playlist: Playlist,
    pub subtitles: Subtitles,
    pub extras: Extras,
    pub network: Network,
    pub output: Output,
    pub extra_arguments: String,
    pub ignore_config: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            format: Default::default(),
            playlist: Default::default(),
            subtitles: Default::default(),
            extras: Default::default(),
            network: Default::default(),
            output: Default::default(),
            extra_arguments: String::new(),
            ignore_config: true,
        }
    }
}

impl<'de> Deserialize<'de> for Options {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(deserializer)?;
        let mut o = Self {
            format: serde_json::from_value(v["format"].clone()).unwrap_or_default(),
            playlist: serde_json::from_value(v["playlist"].clone()).unwrap_or_default(),
            subtitles: serde_json::from_value(v["subtitles"].clone()).unwrap_or_default(),
            extras: serde_json::from_value(v["extras"].clone()).unwrap_or_default(),
            network: serde_json::from_value(v["network"].clone()).unwrap_or_default(),
            output: serde_json::from_value(v["output"].clone()).unwrap_or_default(),
            ..Self::default()
        };
        if let Some(s) = v["extraArguments"].as_str() {
            o.extra_arguments = s.into();
        }
        if let Some(b) = v["ignoreConfig"].as_bool() {
            o.ignore_config = b;
        }
        o.validate();
        Ok(o)
    }
}

impl Options {
    pub fn validate(&mut self) {
        let f = &self.format;
        if !["video", "audio"].contains(&f.mode.as_str())
            || !["best", "mp3", "m4a", "opus", "flac", "wav"].contains(&f.audio_format.as_str())
            || !["auto", "mp4", "mkv", "webm", "mov"].contains(&f.container.as_str())
            || !["remux", "recode"].contains(&f.container_method.as_str())
            || !["any", "h264", "hevc", "vp9", "av1"].contains(&f.codec.as_str())
        {
            self.format = Format::default();
        }
        if !["off", "mark", "remove"].contains(&self.extras.sponsor_block.as_str()) {
            self.extras = Extras::default();
        }
        if ![
            "none", "safari", "chrome", "firefox", "arc", "brave", "edge", "chromium", "vivaldi",
            "opera",
        ]
        .contains(&self.network.cookie_browser.as_str())
        {
            self.network = Network::default();
        }
        if !["skipExisting", "never", "force"].contains(&self.output.overwrite.as_str()) {
            self.output = Output::default();
        }
        self.network.concurrent_fragments = self.network.concurrent_fragments.clamp(1, 32);
        self.extras.wait_interval = self.extras.wait_interval.max(1);
    }
    pub fn extension(&self) -> &str {
        if self.format.mode == "audio" {
            if self.format.audio_format == "best" {
                "m4a"
            } else {
                &self.format.audio_format
            }
        } else if self.format.container == "auto" {
            "mp4"
        } else {
            &self.format.container
        }
    }
    pub fn cover_art(&self) -> bool {
        if self.format.mode == "audio" {
            self.format.audio_format != "wav"
        } else {
            ["mp4", "mkv", "mov"].contains(&self.format.container.as_str())
        }
    }
    pub fn chapters(&self) -> bool {
        if self.format.mode == "audio" {
            self.format.audio_format != "wav"
        } else {
            self.format.container != "webm"
        }
    }
}
