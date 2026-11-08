//! Settings > Advanced: every yt-dlp option Purrloader exposes, grouped as in the Mac app.
use crate::{
    theme::{self, W},
    widgets::{self as w, Form},
};
use eframe::egui::{self, Align, Color32, Layout, Ui};
use purrloader::{media::Media, options::Options};

pub const SECTIONS: [&str; 6] = [
    "Format", "Playlist", "Extras", "Network", "Output", "Command",
];

/// What a section asks the app to do; the app owns files, dialogs and the clipboard.
pub enum Action {
    ShowArchive,
    ClearArchive,
    ChooseCookiesFile,
    Copy(String),
    Open(&'static str),
}

pub struct Context<'a> {
    pub options: &'a mut Options,
    pub info: Option<&'a Media>,
    pub ticked: usize,
    pub archive_count: usize,
    pub command: String,
    pub preview: String,
    pub actions: Vec<Action>,
}

pub fn cookie_browsers() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "chrome", "firefox", "brave", "edge", "chromium", "vivaldi", "opera",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "safari", "chrome", "firefox", "arc", "brave", "edge", "chromium", "vivaldi", "opera",
        ]
    } else {
        &[
            "chrome", "firefox", "brave", "edge", "chromium", "vivaldi", "opera",
        ]
    }
}
pub fn browser_title(browser: &str) -> &str {
    match browser {
        "none" => "None",
        "safari" => "Safari",
        "chrome" => "Chrome",
        "firefox" => "Firefox",
        "arc" => "Arc",
        "brave" => "Brave",
        "edge" => "Microsoft Edge",
        "chromium" => "Chromium",
        "vivaldi" => "Vivaldi",
        "opera" => "Opera",
        other => other,
    }
}
pub fn browser_note() -> &'static str {
    if cfg!(windows) {
        "Firefox works best. Chrome and Edge keep their sign-in locked on Windows, so close them first, and they may still refuse."
    } else if cfg!(target_os = "macos") {
        "Safari needs Full Disk Access for Purrloader in System Settings → Privacy & Security."
    } else {
        "Chrome, Edge and other Chromium browsers may ask for your keyring password."
    }
}
pub fn height_label(height: u32) -> String {
    if height >= 4320 {
        "8K".into()
    } else if height >= 2160 {
        "4K".into()
    } else {
        format!("{height}p")
    }
}
pub const AUDIO_FORMATS: [(&str, &str); 6] = [
    ("best", "Best quality"),
    ("mp3", "MP3"),
    ("m4a", "M4A"),
    ("opus", "OPUS"),
    ("flac", "FLAC"),
    ("wav", "WAV"),
];

fn owned(options: &[(&str, &str)]) -> Vec<(String, String)> {
    options
        .iter()
        .map(|(v, t)| ((*v).to_owned(), (*t).to_owned()))
        .collect()
}
fn picker(ui: &mut Ui, form: &mut Form, label: &str, value: &mut String, options: &[(&str, &str)]) {
    let mut list = owned(options);
    if !list.iter().any(|(v, _)| v == value) {
        list.push((value.clone(), value.clone()));
    }
    let refs: Vec<(String, &str)> = list.iter().map(|(v, t)| (v.clone(), t.as_str())).collect();
    w::picker_row(ui, form, label, value, &refs);
}
/// A borderless, right-aligned field inside a form row.
pub fn plain_field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) {
    ui.add_sized(
        [width, 22.0],
        egui::TextEdit::singleline(value)
            .frame(false)
            .font(theme::font(13.0, W::Regular))
            .text_color(theme::ink(1.0))
            .horizontal_align(Align::RIGHT)
            .vertical_align(Align::Center)
            .hint_text(theme::rich(hint, 13.0, W::Regular, theme::tertiary())),
    );
}
fn field(ui: &mut Ui, form: &mut Form, label: &str, value: &mut String, hint: &str) {
    w::row(ui, form, label, None, |ui| {
        plain_field(ui, value, hint, 260.0)
    });
}
fn mono(text: &str, size: f32, color: Color32) -> egui::RichText {
    egui::RichText::new(text)
        .font(egui::FontId::monospace(size))
        .color(color)
}

pub fn show(ui: &mut Ui, section: usize, cx: &mut Context) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 20,
            right: 20,
            top: 20,
            bottom: 24,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 20.0;
            match section {
                0 => format(ui, cx),
                1 => playlist(ui, cx),
                2 => extras(ui, cx),
                3 => network(ui, cx),
                4 => output(ui, cx),
                _ => command(ui, cx),
            }
        });
}

fn format(ui: &mut Ui, cx: &mut Context) {
    let format = &mut cx.options.format;
    w::section(ui, None, None, |ui, form| {
        w::row(ui, form, "Download", None, |ui| {
            w::segmented(
                ui,
                &mut format.mode,
                &[("video".into(), "Video"), ("audio".into(), "Audio Only")],
            );
        });
    });
    if format.mode == "video" {
        w::section(ui, Some("Video"), None, |ui, form| {
            let mut qualities: Vec<(Option<u32>, String)> = vec![(None, "Best available".into())];
            for h in [4320, 2160, 1440, 1080, 720, 480, 360] {
                qualities.push((Some(h), height_label(h)));
            }
            if let Some(h) = format.max_height
                && !qualities.iter().any(|(v, _)| *v == Some(h))
            {
                qualities.push((Some(h), height_label(h)));
            }
            let refs: Vec<_> = qualities.iter().map(|(v, t)| (*v, t.as_str())).collect();
            w::picker_row(ui, form, "Maximum quality", &mut format.max_height, &refs);
            picker(
                ui,
                form,
                "Codec preference",
                &mut format.codec,
                &[
                    ("any", "No preference"),
                    ("h264", "H.264 (most compatible)"),
                    ("hevc", "H.265 / HEVC"),
                    ("vp9", "VP9"),
                    ("av1", "AV1"),
                ],
            );
            w::switch_row(ui, form, "Allow HDR", &mut format.allow_hdr);
            picker(
                ui,
                form,
                "Container",
                &mut format.container,
                &[
                    ("auto", "Automatic"),
                    ("mp4", "MP4"),
                    ("mkv", "MKV"),
                    ("webm", "WEBM"),
                    ("mov", "MOV"),
                ],
            );
            if format.container != "auto" {
                picker(
                    ui,
                    form,
                    "Conversion",
                    &mut format.container_method,
                    &[
                        ("remux", "Remux (fast, lossless)"),
                        ("recode", "Re-encode (slow)"),
                    ],
                );
            }
        });
    } else {
        w::section(ui, Some("Audio"), None, |ui, form| {
            picker(ui, form, "Format", &mut format.audio_format, &AUDIO_FORMATS);
            if ["mp3", "m4a", "opus"].contains(&format.audio_format.as_str()) {
                picker(
                    ui,
                    form,
                    "Quality",
                    &mut format.audio_quality,
                    &[
                        ("0", "Best (VBR 0)"),
                        ("2", "High (VBR 2)"),
                        ("5", "Standard (VBR 5)"),
                        ("—", "—"),
                        ("320K", "320 kbps"),
                        ("256K", "256 kbps"),
                        ("192K", "192 kbps"),
                        ("160K", "160 kbps"),
                        ("128K", "128 kbps"),
                        ("96K", "96 kbps"),
                        ("64K", "64 kbps"),
                    ],
                );
            }
        });
    }
    let info = cx.info;
    let format = &mut cx.options.format;
    w::section(
        ui,
        Some("Exact streams"),
        Some(
            "Pick specific streams from the current link's format list (same as yt-dlp -F). Overrides the quality settings above for that link only.",
        ),
        |ui, form| {
            w::stack_row(ui, form, |ui| {
                match info.filter(|i| !i.formats.is_empty()) {
                    None => {
                        let text = if info.is_some_and(|i| i.playlist) {
                            "Stream picking is available for single videos."
                        } else {
                            "Paste a link in the main window to see its formats."
                        };
                        ui.label(theme::rich(text, 13.0, W::Regular, theme::secondary()));
                        w::text_input(
                            ui,
                            &mut format.exact_format,
                            "e.g. 137+140 or bv*+ba/b",
                            ui.available_width(),
                        );
                    }
                    Some(info) => {
                        let mut picked: Vec<String> = format
                            .exact_format
                            .split('+')
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                        egui::Frame::new()
                            .fill(theme::black(0.15))
                            .stroke(egui::Stroke::new(1.0_f32, theme::ink(0.06)))
                            .corner_radius(6)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("formats")
                                    .max_height(220.0)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        for (i, f) in info.formats.iter().enumerate() {
                                            let on = picked.contains(&f.id);
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(ui.available_width(), 24.0),
                                                egui::Sense::click(),
                                            );
                                            let fill = if on {
                                                theme::ACCENT
                                            } else if i % 2 == 1 {
                                                theme::ink(0.03)
                                            } else {
                                                Color32::TRANSPARENT
                                            };
                                            ui.painter().rect_filled(rect, 0, fill);
                                            let id = theme::line(
                                                ui,
                                                &f.id,
                                                12.0,
                                                W::Medium,
                                                if on { Color32::WHITE } else { theme::ink(1.0) },
                                                Some(70.0),
                                            );
                                            ui.painter().galley(
                                                egui::pos2(
                                                    rect.left() + 8.0,
                                                    rect.center().y - id.size().y / 2.0,
                                                ),
                                                id,
                                                theme::ink(1.0),
                                            );
                                            let label = theme::line(
                                                ui,
                                                &f.label,
                                                12.0,
                                                W::Regular,
                                                if on { Color32::WHITE } else { theme::text90() },
                                                Some(rect.width() - 96.0),
                                            );
                                            ui.painter().galley(
                                                egui::pos2(
                                                    rect.left() + 86.0,
                                                    rect.center().y - label.size().y / 2.0,
                                                ),
                                                label,
                                                theme::ink(1.0),
                                            );
                                            if response.clicked() {
                                                if on {
                                                    picked.retain(|p| p != &f.id);
                                                } else {
                                                    if picked.len() >= 2 {
                                                        picked.remove(0);
                                                    }
                                                    picked.push(f.id.clone());
                                                }
                                                format.exact_format = picked.join("+");
                                            }
                                        }
                                    });
                            });
                        ui.horizontal(|ui| {
                            let text = if format.exact_format.is_empty() {
                                "Automatic (no streams picked)".to_owned()
                            } else {
                                format!("-f {}", format.exact_format)
                            };
                            ui.label(mono(&text, 12.0, theme::secondary()));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if w::bezel_button(
                                    ui,
                                    "Clear",
                                    false,
                                    !format.exact_format.is_empty(),
                                )
                                .clicked()
                                {
                                    format.exact_format.clear();
                                }
                            });
                        });
                    }
                }
            });
        },
    );
}

fn playlist(ui: &mut Ui, cx: &mut Context) {
    let playlist = &mut cx.options.playlist;
    w::section(
        ui,
        None,
        Some("For links that point to one video inside a playlist."),
        |ui, form| {
            w::switch_row(
                ui,
                form,
                "Only download the video, not the playlist it's in",
                &mut playlist.no_playlist,
            );
        },
    );
    w::section(ui, Some("Which items"), None, |ui, form| {
        field(
            ui,
            form,
            "Range",
            &mut playlist.range,
            "All — or e.g. 5-20, or 1,3,7:10",
        );
        w::row(ui, form, "Latest", None, |ui| {
            w::stepper(ui, &mut playlist.latest_count, 0, 10_000, 1);
            let text = if playlist.latest_count == 0 {
                "All".to_owned()
            } else {
                format!("{} items", playlist.latest_count)
            };
            ui.label(theme::rich(text, 13.0, W::Regular, theme::secondary()));
        });
        w::switch_row(ui, form, "Reverse order", &mut playlist.reverse);
    });
    let count = cx.archive_count;
    let mut actions = Vec::new();
    w::section(
        ui,
        Some("Download archive"),
        Some(
            "Records every downloaded video ID so re-running a playlist or channel only fetches new items.",
        ),
        |ui, form| {
            w::switch_row(
                ui,
                form,
                "Skip items already downloaded",
                &mut playlist.use_archive,
            );
            w::row(ui, form, "Archive", None, |ui| {
                if w::bezel_button(ui, "Clear…", false, count > 0).clicked() {
                    actions.push(Action::ClearArchive);
                }
                if w::bezel_button(ui, "Show", false, count > 0).clicked() {
                    actions.push(Action::ShowArchive);
                }
                let text = if count == 1 {
                    "1 item".to_owned()
                } else {
                    format!("{count} items")
                };
                ui.label(theme::rich(text, 13.0, W::Regular, theme::secondary()));
            });
        },
    );
    cx.actions.extend(actions);
    if let Some(info) = cx.info.filter(|i| i.playlist) {
        let text = format!(
            "Items ticked in the main window take priority over the range above ({} of {} ticked).",
            cx.ticked,
            info.entries.len()
        );
        w::section(ui, None, None, |ui, form| {
            w::stack_row(ui, form, |ui| {
                theme::paragraph(ui, &text, 13.0, W::Regular, theme::secondary(), 0);
            });
        });
    }
}

fn extras(ui: &mut Ui, cx: &mut Context) {
    let options = &mut *cx.options;
    let subtitles = &mut options.subtitles;
    w::section(ui, Some("Subtitles"), None, |ui, form| {
        w::switch_row(ui, form, "Download subtitles", &mut subtitles.enabled);
        w::switch_row(
            ui,
            form,
            "Include auto-generated subtitles",
            &mut subtitles.include_auto_generated,
        );
        if subtitles.enabled {
            field(
                ui,
                form,
                "Languages",
                &mut subtitles.languages,
                "en.*, de, all",
            );
            let mut embed = if subtitles.embed { "embed" } else { "file" }.to_owned();
            picker(
                ui,
                form,
                "Save as",
                &mut embed,
                &[
                    ("embed", "Embedded in the video"),
                    ("file", "Separate files"),
                ],
            );
            subtitles.embed = embed == "embed";
            w::switch_row(ui, form, "Convert to SRT", &mut subtitles.convert_to_srt);
        }
    });
    let extras = &mut options.extras;
    w::section(
        ui,
        Some("Thumbnail & metadata"),
        Some(
            "Cover art and chapters are only embedded in formats that can hold them (not WebM or WAV).",
        ),
        |ui, form| {
            w::switch_row(
                ui,
                form,
                "Embed thumbnail as cover art",
                &mut extras.embed_thumbnail,
            );
            w::switch_row(
                ui,
                form,
                "Save thumbnail as a file",
                &mut extras.write_thumbnail,
            );
            w::switch_row(ui, form, "Embed metadata", &mut extras.embed_metadata);
            w::switch_row(ui, form, "Embed chapters", &mut extras.embed_chapters);
            w::switch_row(
                ui,
                form,
                "Split into one file per chapter",
                &mut extras.split_chapters,
            );
        },
    );
    w::section(ui, Some("SponsorBlock"), None, |ui, form| {
        picker(
            ui,
            form,
            "Segments",
            &mut extras.sponsor_block,
            &[
                ("off", "Off"),
                ("mark", "Mark as chapters"),
                ("remove", "Remove segments"),
            ],
        );
        if extras.sponsor_block != "off" {
            w::stack_row(ui, form, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(14.0, 8.0);
                    for category in [
                        "sponsor",
                        "intro",
                        "outro",
                        "selfpromo",
                        "preview",
                        "filler",
                        "interaction",
                        "music_offtopic",
                        "hook",
                    ] {
                        let mut on = extras.sponsor_categories.iter().any(|c| c == category);
                        let title: String = category
                            .replace('_', " ")
                            .split(' ')
                            .map(|word| {
                                let mut chars = word.chars();
                                chars
                                    .next()
                                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                                    .unwrap_or_default()
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        if w::checkbox(ui, &mut on, &title).changed() {
                            extras.sponsor_categories.retain(|c| c != category);
                            if on {
                                extras.sponsor_categories.push(category.into());
                            }
                        }
                    }
                });
            });
        }
    });
    w::section(
        ui,
        Some("Only part of the video"),
        Some("Start and end times are set per download in the main window's More menu."),
        |ui, form| {
            field(
                ui,
                form,
                "Chapters",
                &mut extras.chapter_regex,
                "Regex matching chapter titles",
            );
            w::switch_row(
                ui,
                form,
                "Precise cuts (re-encodes around cut points)",
                &mut extras.force_keyframes,
            );
        },
    );
    w::section(ui, Some("Live streams"), None, |ui, form| {
        w::switch_row(
            ui,
            form,
            "Record from the start",
            &mut extras.live_from_start,
        );
        w::switch_row(
            ui,
            form,
            "Wait for scheduled streams",
            &mut extras.wait_for_video,
        );
        if extras.wait_for_video {
            let label = format!("Check every {} s", extras.wait_interval);
            w::row(ui, form, &label, None, |ui| {
                w::stepper(ui, &mut extras.wait_interval, 5, 3600, 15);
            });
        }
    });
    w::section(ui, Some("Also save"), None, |ui, form| {
        w::switch_row(ui, form, "Description", &mut extras.write_description);
        w::switch_row(
            ui,
            form,
            "Comments (in the info JSON)",
            &mut extras.write_comments,
        );
        w::switch_row(ui, form, "Info JSON", &mut extras.write_info_json);
    });
}

fn network(ui: &mut Ui, cx: &mut Context) {
    let network = &mut cx.options.network;
    let mut actions = Vec::new();
    let footer = format!(
        "For private, age-restricted or members-only videos. {}",
        browser_note()
    );
    w::section(ui, Some("Sign-in"), Some(&footer), |ui, form| {
        let mut browsers: Vec<&str> = vec!["none"];
        browsers.extend(cookie_browsers());
        let current = network.cookie_browser.clone();
        if !browsers.contains(&current.as_str()) {
            browsers.push(&current);
        }
        let choices: Vec<(String, &str)> = browsers
            .iter()
            .map(|b| ((*b).to_owned(), browser_title(b)))
            .collect();
        w::picker_row(
            ui,
            form,
            "Cookies from browser",
            &mut network.cookie_browser,
            &choices,
        );
        if network.cookie_browser != "none" {
            field(
                ui,
                form,
                "Profile",
                &mut network.cookie_profile,
                "Default profile",
            );
        } else {
            w::row(ui, form, "Cookies file", None, |ui| {
                if w::bezel_button(ui, "Choose…", false, true).clicked() {
                    actions.push(Action::ChooseCookiesFile);
                }
                plain_field(
                    ui,
                    &mut network.cookies_file,
                    "cookies.txt (Netscape format)",
                    220.0,
                );
            });
        }
    });
    cx.actions.extend(actions);
    w::section(ui, Some("Connection"), None, |ui, form| {
        field(
            ui,
            form,
            "Proxy",
            &mut network.proxy,
            "socks5://127.0.0.1:1080",
        );
        field(
            ui,
            form,
            "Rate limit",
            &mut network.rate_limit,
            "Unlimited — or e.g. 5M",
        );
        let label = format!("Retries: {}", network.retries);
        w::row(ui, form, &label, None, |ui| {
            w::stepper(ui, &mut network.retries, 0, 100, 1);
        });
        let label = format!("Concurrent fragments: {}", network.concurrent_fragments);
        w::row(ui, form, &label, None, |ui| {
            w::stepper(ui, &mut network.concurrent_fragments, 1, 32, 1);
        });
        w::switch_row(
            ui,
            form,
            "Use aria2c as external downloader",
            &mut network.use_aria2c,
        );
    });
    w::section(ui, Some("Identity"), None, |ui, form| {
        picker(
            ui,
            form,
            "Impersonate browser",
            &mut network.impersonate,
            &[
                ("", "Off"),
                ("chrome", "Chrome"),
                ("edge", "Edge"),
                ("safari", "Safari"),
                ("firefox", "Firefox"),
            ],
        );
        field(
            ui,
            form,
            "Geo bypass (X-Forwarded-For)",
            &mut network.xff,
            "Off — or a country code like US, or default",
        );
        field(
            ui,
            form,
            "User agent",
            &mut network.user_agent,
            "yt-dlp default",
        );
    });
}

pub const TEMPLATE_PRESETS: [(&str, &str); 6] = [
    ("Title", "%(title)s.%(ext)s"),
    ("Title [ID]", "%(title)s [%(id)s].%(ext)s"),
    ("Uploader / Title", "%(uploader)s/%(title)s.%(ext)s"),
    (
        "Date - Title",
        "%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s",
    ),
    (
        "Playlist / 001 - Title",
        "%(playlist_title,playlist|NA)s/%(playlist_index)03d - %(title)s.%(ext)s",
    ),
    (
        "Channel / Date - Title [ID]",
        "%(channel,uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s",
    ),
];

fn output(ui: &mut Ui, cx: &mut Context) {
    let preview = cx.preview.clone();
    let output = &mut cx.options.output;
    let mut actions = Vec::new();
    w::section(ui, Some("File name"), None, |ui, form| {
        w::stack_row(ui, form, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 22.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    let response = w::bezel_menu_button(ui, "Presets");
                    w::menu(&response, egui::RectAlign::BOTTOM_END, |ui| {
                        for (name, template) in TEMPLATE_PRESETS {
                            if w::menu_item(ui, name, output.template == template, None, true) {
                                output.template = template.into();
                            }
                        }
                    });
                    w::text_input(ui, &mut output.template, "", ui.available_width());
                },
            );
        });
        w::row(ui, form, "Preview", None, |ui| {
            ui.label(mono(&preview, 12.0, theme::text90()));
        });
        w::stack_row(ui, form, |ui| {
            if w::link_button(ui, "Template fields reference").clicked() {
                actions.push(Action::Open(
                    "https://github.com/yt-dlp/yt-dlp#output-template",
                ));
            }
        });
    });
    cx.actions.extend(actions);
    w::section(ui, Some("Files"), None, |ui, form| {
        w::switch_row(
            ui,
            form,
            "Restrict file names to ASCII, no spaces",
            &mut output.restrict_filenames,
        );
        picker(
            ui,
            form,
            "Existing files",
            &mut output.overwrite,
            &[
                ("skipExisting", "Skip existing files"),
                ("never", "Never overwrite anything"),
                ("force", "Always overwrite"),
            ],
        );
    });
}

fn command(ui: &mut Ui, cx: &mut Context) {
    let config = if cfg!(windows) {
        "%APPDATA%\\yt-dlp\\config.txt"
    } else {
        "~/.config/yt-dlp/config"
    };
    let options = &mut *cx.options;
    w::section(
        ui,
        Some("Extra yt-dlp arguments"),
        Some(
            "Added after everything else, so they override the options in the other sections. Shell-style quoting works.",
        ),
        |ui, form| {
            w::stack_row(ui, form, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 66.0),
                    egui::Sense::hover(),
                );
                w::text_input_at(
                    ui,
                    rect,
                    &mut options.extra_arguments,
                    "e.g. --match-filters \"duration < 600\" --sleep-interval 5",
                    true,
                );
            });
        },
    );
    let footer = format!("Keeps {config} from changing Purrloader's output and progress handling.");
    w::section(ui, None, Some(&footer), |ui, form| {
        w::switch_row(
            ui,
            form,
            "Ignore my yt-dlp config files",
            &mut options.ignore_config,
        );
    });
    let command = cx.command.clone();
    let mut copy = false;
    w::section(
        ui,
        Some("Command preview"),
        Some(
            "Exactly what the Download button will run for the current link. Each download's full log is under Details in the queue.",
        ),
        |ui, form| {
            w::stack_row(ui, form, |ui| {
                ui.add(egui::Label::new(mono(&command, 10.5, theme::text90())).wrap());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    copy = w::bezel_button(ui, "Copy Command", false, true).clicked();
                });
            });
        },
    );
    if copy {
        cx.actions.push(Action::Copy(command));
    }
}
