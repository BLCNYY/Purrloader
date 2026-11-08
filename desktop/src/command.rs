//! Argument vectors only. User input is never evaluated by a shell.
use crate::options::Options;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Environment {
    pub ffmpeg: PathBuf,
    pub deno: PathBuf,
    pub archive: PathBuf,
    pub home: PathBuf,
    pub windows: bool,
}

fn pair(a: &mut Vec<String>, flag: &str, value: impl ToString) {
    a.push(flag.into());
    a.push(value.to_string());
}
fn flag(a: &mut Vec<String>, on: bool, value: &str) {
    if on {
        a.push(value.into());
    }
}

impl Environment {
    fn access(&self, o: &Options) -> Vec<String> {
        let mut a = vec![];
        flag(&mut a, o.ignore_config, "--ignore-config");
        pair(&mut a, "--ffmpeg-location", self.ffmpeg.display());
        pair(
            &mut a,
            "--js-runtimes",
            format!("deno:{}", self.deno.display()),
        );
        let n = &o.network;
        if n.cookie_browser != "none" {
            let mut browser = if n.cookie_browser == "arc" {
                "chromium"
            } else {
                &n.cookie_browser
            }
            .to_string();
            if !n.cookie_profile.trim().is_empty() {
                browser.push(':');
                browser.push_str(n.cookie_profile.trim());
            }
            pair(&mut a, "--cookies-from-browser", browser);
        } else if !n.cookies_file.trim().is_empty() {
            let path = n.cookies_file.trim();
            let expanded = if path == "~" {
                self.home.clone()
            } else if path.starts_with("~/") || path.starts_with("~\\") {
                // Preserve the separator in the input, including portable saved paths.
                PathBuf::from(format!("{}{}", self.home.display(), &path[1..]))
            } else {
                path.into()
            };
            pair(&mut a, "--cookies", expanded.display());
        }
        for (key, value) in [
            ("--proxy", &n.proxy),
            ("--impersonate", &n.impersonate),
            ("--xff", &n.xff),
        ] {
            if !value.trim().is_empty() {
                pair(&mut a, key, value.trim());
            }
        }
        if !n.user_agent.trim().is_empty() {
            pair(
                &mut a,
                "--add-headers",
                format!("User-Agent:{}", n.user_agent.trim()),
            );
        }
        a
    }
    pub fn info(&self, o: &Options, url: &str) -> Vec<String> {
        let mut a = self.access(o);
        a.extend(["-J", "--flat-playlist", "--no-warnings", "--", url].map(String::from));
        a
    }
    pub fn download(
        &self,
        o: &Options,
        url: &str,
        folder: &Path,
        selection: Option<&str>,
        paths: Option<&Path>,
    ) -> Result<Vec<String>> {
        let mut a = self.access(o);
        let f = &o.format;
        let exact = f.exact_format.trim();
        if f.mode == "audio" {
            pair(&mut a, "-f", if exact.is_empty() { "ba/b" } else { exact });
            a.push("-x".into());
            if f.audio_format != "best" {
                pair(&mut a, "--audio-format", &f.audio_format);
                if ["mp3", "m4a", "opus"].contains(&f.audio_format.as_str()) {
                    pair(&mut a, "--audio-quality", &f.audio_quality);
                }
            }
        } else {
            let limit = f
                .max_height
                .map(|h| format!("[height<={h}]"))
                .unwrap_or_default();
            pair(
                &mut a,
                "-f",
                if exact.is_empty() {
                    format!("bv*{limit}+ba/b{limit}")
                } else {
                    exact.into()
                },
            );
            let mut sort = match f.codec.as_str() {
                "h264" => vec!["vcodec:h264", "res", "acodec:m4a"],
                "hevc" => vec!["res", "vcodec:h265"],
                "vp9" => vec!["res", "vcodec:vp9"],
                "av1" => vec!["res", "vcodec:av01"],
                _ => vec![],
            };
            if f.container == "mp4" {
                if f.codec == "any" {
                    sort.push("ext:mp4:m4a");
                } else if !sort.contains(&"acodec:m4a") {
                    sort.push("acodec:m4a");
                }
            }
            if !f.allow_hdr {
                sort.insert(0, "hdr:sdr");
            }
            if !sort.is_empty() {
                pair(&mut a, "-S", sort.join(","));
            }
            if f.container != "auto" {
                pair(&mut a, "--merge-output-format", &f.container);
                pair(
                    &mut a,
                    if f.container_method == "remux" {
                        "--remux-video"
                    } else {
                        "--recode-video"
                    },
                    &f.container,
                );
            }
        }
        let p = &o.playlist;
        if let Some(s) = selection.filter(|s| !s.is_empty()) {
            a.push("--yes-playlist".into());
            pair(&mut a, "-I", s);
        } else {
            a.push(
                if p.no_playlist {
                    "--no-playlist"
                } else {
                    "--yes-playlist"
                }
                .into(),
            );
            if p.latest_count > 0 {
                pair(
                    &mut a,
                    "-I",
                    if p.reverse {
                        format!("{}:1:-1", p.latest_count)
                    } else {
                        format!("1:{}", p.latest_count)
                    },
                );
            } else if !p.range.trim().is_empty() {
                let range = p.range.trim();
                let bounds = range.split_once('-').and_then(|(a, b)| {
                    Some((a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?))
                });
                let range = if let Some((lo, hi)) = bounds {
                    if p.reverse {
                        format!("{hi}:{lo}:-1")
                    } else {
                        format!("{lo}:{hi}")
                    }
                } else {
                    range.into()
                };
                pair(&mut a, "-I", range);
            } else if p.reverse {
                pair(&mut a, "-I", "::-1");
            }
        }
        if p.use_archive {
            pair(&mut a, "--download-archive", self.archive.display());
        }
        let s = &o.subtitles;
        if s.enabled {
            a.push("--write-subs".into());
            flag(&mut a, s.include_auto_generated, "--write-auto-subs");
            pair(
                &mut a,
                "--sub-langs",
                if s.languages.trim().is_empty() {
                    "en.*"
                } else {
                    s.languages.trim()
                },
            );
            if s.convert_to_srt {
                pair(&mut a, "--convert-subs", "srt");
            }
            flag(&mut a, s.embed && f.mode == "video", "--embed-subs");
        }
        let e = &o.extras;
        for (on, value) in [
            (e.write_thumbnail, "--write-thumbnail"),
            (e.embed_thumbnail && o.cover_art(), "--embed-thumbnail"),
            (e.embed_metadata, "--embed-metadata"),
            (e.embed_chapters && o.chapters(), "--embed-chapters"),
            (e.split_chapters, "--split-chapters"),
        ] {
            flag(&mut a, on, value);
        }
        if e.sponsor_block != "off" && !e.sponsor_categories.is_empty() {
            pair(
                &mut a,
                if e.sponsor_block == "mark" {
                    "--sponsorblock-mark"
                } else {
                    "--sponsorblock-remove"
                },
                e.sponsor_categories.join(","),
            );
        }
        if !e.section_start.trim().is_empty() || !e.section_end.trim().is_empty() {
            pair(
                &mut a,
                "--download-sections",
                format!(
                    "*{}-{}",
                    if e.section_start.trim().is_empty() {
                        "0"
                    } else {
                        e.section_start.trim()
                    },
                    if e.section_end.trim().is_empty() {
                        "inf"
                    } else {
                        e.section_end.trim()
                    }
                ),
            );
        }
        if !e.chapter_regex.trim().is_empty() {
            pair(&mut a, "--download-sections", e.chapter_regex.trim());
        }
        flag(&mut a, e.force_keyframes, "--force-keyframes-at-cuts");
        flag(&mut a, e.live_from_start, "--live-from-start");
        if e.wait_for_video {
            pair(&mut a, "--wait-for-video", e.wait_interval.max(1));
        }
        flag(&mut a, e.write_description, "--write-description");
        flag(&mut a, e.write_comments, "--write-comments");
        flag(&mut a, e.write_info_json, "--write-info-json");
        let n = &o.network;
        if !n.rate_limit.trim().is_empty() {
            pair(&mut a, "-r", n.rate_limit.trim());
        }
        if n.retries != 10 {
            pair(&mut a, "-R", n.retries);
        }
        if n.concurrent_fragments > 1 {
            pair(&mut a, "-N", n.concurrent_fragments);
        }
        if n.use_aria2c {
            pair(&mut a, "--downloader", "aria2c");
        }
        pair(&mut a, "-P", folder.display());
        pair(
            &mut a,
            "-o",
            if o.output.template.trim().is_empty() {
                "%(title)s.%(ext)s"
            } else {
                &o.output.template
            },
        );
        if o.output.restrict_filenames {
            a.push("--restrict-filenames".into());
        } else {
            flag(&mut a, !self.windows, "--no-windows-filenames");
            for (pattern, replacement) in [(r"\s*:\s+", " - "), (":", "-")] {
                a.extend(
                    [
                        "--replace-in-metadata",
                        "title,playlist_title,uploader,channel",
                        pattern,
                        replacement,
                    ]
                    .map(String::from),
                );
            }
        }
        flag(&mut a, o.output.overwrite == "never", "--no-overwrites");
        flag(&mut a, o.output.overwrite == "force", "--force-overwrites");
        a.push("--newline".into());
        pair(&mut a, "--progress-delta", "0.5");
        pair(
            &mut a,
            "--progress-template",
            "download:[purrloader]%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.title)s",
        );
        pair(
            &mut a,
            "--progress-template",
            "postprocess:[purrloaderpp]%(progress.status)s|%(progress.postprocessor)s",
        );
        if let Some(paths) = paths {
            a.extend([
                "--print-to-file".into(),
                "after_move:filepath".into(),
                paths.display().to_string(),
            ]);
        }
        a.extend(
            shell_words::split(&o.extra_arguments)
                .context("Extra arguments contain an unfinished quote")?,
        );
        a.extend(["--".into(), url.into()]);
        Ok(a)
    }
}
