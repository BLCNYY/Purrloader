use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Entry {
    pub index: usize,
    pub title: String,
    pub duration: f64,
}
#[derive(Debug, Clone)]
pub struct Format {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone)]
pub struct Media {
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub thumbnail: Option<String>,
    pub duration: f64,
    pub playlist: bool,
    pub entries: Vec<Entry>,
    pub formats: Vec<Format>,
    pub subtitles: Vec<(String, bool)>,
    pub raw: Value,
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

/// A filename preview; yt-dlp remains the authority for full template expressions.
pub fn template_preview(
    template: &str,
    media: Option<&Media>,
    options: &crate::options::Options,
    windows: bool,
) -> String {
    use std::sync::LazyLock;
    use unicode_normalization::UnicodeNormalization;
    static FIELDS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"%\(([^)]+)\)(0?\d*)([sd])").unwrap());
    let mut values = serde_json::json!({"id":"a1b2c3", "title":"Example Video Title", "uploader":"Example Channel", "channel":"Example Channel", "playlist_title":"Example Playlist", "playlist_index":1, "upload_date":"20240131"});
    if let Some(media) = media {
        if let Some(object) = media.raw.as_object() {
            values.as_object_mut().unwrap().extend(object.clone());
        }
        if media.playlist {
            values["playlist_title"] = media.title.clone().into();
            if let Some(entry) = media.raw["entries"]
                .as_array()
                .and_then(|v| v.iter().find_map(Value::as_object))
            {
                values.as_object_mut().unwrap().extend(entry.clone());
            }
        }
    }
    values["ext"] = options.extension().into();
    FIELDS
        .replace_all(template, |capture: &regex::Captures<'_>| {
            let (expression, fallback) = capture[1].split_once('|').unwrap_or((&capture[1], "NA"));
            let (keys, date_format) = expression
                .split_once('>')
                .map(|(k, f)| (k, Some(f)))
                .unwrap_or((expression, None));
            let raw = keys
                .split(',')
                .find_map(|key| values.get(key).filter(|v| !v.is_null()));
            let mut value = match raw {
                Some(Value::String(s)) => s.clone(),
                Some(v) if v.is_number() => v.to_string(),
                _ => fallback.into(),
            };
            if let Some(format) = date_format
                && let Ok(date) = chrono::NaiveDate::parse_from_str(&value, "%Y%m%d")
            {
                // Invalid chrono formatting directives must not panic the UI.
                let items: Vec<_> = chrono::format::StrftimeItems::new(format).collect();
                if !items
                    .iter()
                    .any(|i| matches!(i, chrono::format::Item::Error))
                {
                    value = date.format_with_items(items.iter()).to_string();
                }
            }
            if &capture[3] == "d" && capture[2].starts_with('0') {
                let width = capture[2].parse::<usize>().unwrap_or(0).min(64);
                value = format!("{value:0>width$}");
            }
            value = value
                .replace('/', "⧸")
                .replace(": ", " - ")
                .replace(':', "-");
            if options.output.restrict_filenames {
                value = value
                    .nfkd()
                    .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || "-_.[]()".contains(c) {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
            } else if windows {
                for (from, to) in [
                    ('\\', "⧹"),
                    ('|', "｜"),
                    ('*', "＊"),
                    ('?', "？"),
                    ('<', "＜"),
                    ('>', "＞"),
                    ('"', "＂"),
                ] {
                    value = value.replace(from, to);
                }
            }
            value
        })
        .into_owned()
}
impl Media {
    pub fn parse(raw: Value, url: &str) -> Self {
        let title = raw["title"].as_str().unwrap_or(url).to_string();
        let thumbnail = raw["thumbnail"]
            .as_str()
            .or_else(|| raw["thumbnails"].as_array()?.last()?["url"].as_str())
            .map(String::from);
        let entries = raw["entries"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.is_object())
                    .map(|(i, e)| Entry {
                        index: i + 1,
                        title: e["title"].as_str().unwrap_or("Unavailable video").into(),
                        duration: e["duration"].as_f64().unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let formats = raw["formats"]
            .as_array()
            .map(|formats| {
                formats
                    .iter()
                    .rev()
                    .filter_map(|f| {
                        let id = f["format_id"].as_str()?;
                        let video = f["vcodec"].as_str().is_some_and(|v| v != "none");
                        let audio = f["acodec"].as_str().is_some_and(|v| v != "none");
                        if !video && !audio {
                            return None;
                        }
                        let resolution = if video {
                            format!("{}p", f["height"].as_u64().unwrap_or_default())
                        } else {
                            "audio".into()
                        };
                        Some(Format {
                            id: id.into(),
                            label: format!(
                                "{id} · {resolution} · {} · {} {}",
                                text(&f["ext"]),
                                text(&f["vcodec"]),
                                text(&f["acodec"])
                            ),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut subtitles = Vec::new();
        for (key, auto) in [("subtitles", false), ("automatic_captions", true)] {
            if let Some(tracks) = raw[key].as_object() {
                for code in tracks.keys() {
                    if code != "live_chat" && !subtitles.iter().any(|(c, _)| c == code) {
                        subtitles.push((code.clone(), auto));
                    }
                }
            }
        }
        Self {
            url: url.into(),
            title,
            uploader: raw["uploader"]
                .as_str()
                .or(raw["channel"].as_str())
                .unwrap_or_default()
                .into(),
            thumbnail,
            duration: raw["duration"].as_f64().unwrap_or_default(),
            playlist: raw["_type"] == "playlist",
            entries,
            formats,
            subtitles,
            raw,
        }
    }
}
