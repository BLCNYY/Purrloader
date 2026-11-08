use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct Progress {
    pub phase: String,
    pub stage: String,
    pub fraction: Option<f32>,
    pub downloaded: f64,
    pub total: f64,
    pub speed: f64,
    pub eta: Option<f64>,
    pub item_index: Option<usize>,
    pub item_count: Option<usize>,
    pub title: Option<String>,
    pub destinations: Vec<String>,
    pub error: Option<String>,
    finished_bytes: f64,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            phase: "starting".into(),
            stage: "Starting…".into(),
            fraction: None,
            downloaded: 0.0,
            total: 0.0,
            speed: 0.0,
            eta: None,
            item_index: None,
            item_count: None,
            title: None,
            destinations: vec![],
            error: None,
            finished_bytes: 0.0,
        }
    }
}
fn number(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0)
}
impl Progress {
    /// False means a machine-readable line that should stay out of the human log.
    pub fn consume(&mut self, line: &str) -> bool {
        if let Some(payload) = line.strip_prefix("[purrloader]") {
            let parts: Vec<_> = payload.splitn(7, '|').collect();
            if parts.len() < 7 {
                return false;
            }
            let downloaded = number(parts[1]).unwrap_or(0.0);
            let total = number(parts[2]).or_else(|| number(parts[3])).unwrap_or(0.0);
            self.downloaded = self.finished_bytes + downloaded;
            self.total = self.finished_bytes + total;
            self.speed = number(parts[4]).unwrap_or(0.0);
            self.eta = number(parts[5]);
            self.title = (parts[6] != "NA").then(|| parts[6].to_string());
            self.phase = "downloading".into();
            self.stage.clear();
            self.fraction = (self.total > 0.0).then(|| {
                let f = (self.downloaded / self.total).clamp(0.0, 1.0);
                if let (Some(i), Some(n)) = (self.item_index, self.item_count) {
                    ((i.saturating_sub(1) as f64 + f) / n.max(1) as f64).clamp(0.0, 1.0) as f32
                } else {
                    f as f32
                }
            });
            if parts[0] == "finished" {
                self.finished_bytes += downloaded;
            }
            return false;
        }
        if let Some(payload) = line.strip_prefix("[purrloaderpp]started|") {
            self.phase = "processing".into();
            self.fraction = None;
            self.stage = match payload {
                "Merger" => "Combining video and audio…",
                "ExtractAudio" => "Converting audio…",
                "VideoRemuxer" => "Changing the video container…",
                "VideoConvertor" => "Converting video…",
                "EmbedSubtitle" => "Adding subtitles…",
                "EmbedThumbnail" => "Adding cover art…",
                "Metadata" => "Adding details…",
                "SponsorBlock" | "ModifyChapters" => "Skipping sponsor segments…",
                _ => "Finishing…",
            }
            .into();
            return false;
        }
        if line.starts_with("[purrloaderpp]") {
            return false;
        }
        if let Some(e) = line.strip_prefix("ERROR:") {
            self.error = Some(e.trim().into());
        }
        if let Some(d) = line.strip_prefix("[download] Destination: ") {
            self.destinations.push(d.into());
        }
        static ITEM: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"Downloading item (\d+) of (\d+)").unwrap());
        if let Some(c) = ITEM.captures(line) {
            self.item_index = c[1].parse().ok();
            self.item_count = c[2].parse().ok();
            self.finished_bytes = 0.0;
        }
        if line.starts_with("[Merger]") {
            self.phase = "processing".into();
            self.stage = "Combining video and audio…".into();
        } else if line.contains("has already been") {
            self.stage = "Already downloaded".into();
        } else if line.starts_with("[wait]") {
            self.stage = "Waiting for the stream to start…".into();
        } else if self.phase == "starting" && line.contains("playlist") {
            self.stage = "Reading playlist…".into();
        } else if self.phase == "starting" && line.contains("Downloading") {
            self.stage = "Reading video details…".into();
        }
        true
    }
}
