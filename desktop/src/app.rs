//! The windows: main window (link field, info card, playlist, More popover, queue), Settings and History.
//! Laid out point for point like the Mac app's SwiftUI views.
use crate::{
    icons,
    platform::{self, Tray},
    settings_ui::{self, Action},
    theme::{self, W, ink},
    widgets as w,
};
use eframe::egui::{
    self, Align, Color32, Frame, Layout, Margin, Rect, RectAlign, Sense, Stroke, Ui, ViewportId,
    pos2, vec2,
};
use purrloader::{
    engine::{Engine, Job, Status},
    media::Media,
    options::Options,
    store::{Paths, Preset},
    support,
};
use std::{
    collections::BTreeSet,
    fs::File,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const TOOLS: [&str; 4] = ["yt-dlp", "ffmpeg", "ffprobe", "deno"];
const SETTINGS_SIZES: [[f32; 2]; 3] = [[520.0, 412.0], [720.0, 672.0], [520.0, 652.0]];

fn settings_id() -> ViewportId {
    ViewportId::from_hash_of("purrloader-settings")
}
fn history_id() -> ViewportId {
    ViewportId::from_hash_of("purrloader-history")
}
fn shortcut(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.replace("Ctrl+", "⌘")
            .replace("Shift+", "⇧")
            .replace("Alt+", "⌥")
            .replace("Enter", "↩")
    } else {
        keys.into()
    }
}
fn file_manager() -> &'static str {
    if cfg!(windows) {
        "Explorer"
    } else if cfg!(target_os = "macos") {
        "Finder"
    } else {
        "Files"
    }
}
fn tray_name() -> &'static str {
    if cfg!(windows) {
        "notification area"
    } else {
        "system tray"
    }
}
/// The path the way people know it: ~ for the home folder on Linux and the Mac.
fn display_path(path: &std::path::Path, home: &std::path::Path) -> String {
    if !cfg!(windows)
        && let Ok(rest) = path.strip_prefix(home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}
fn language_name(code: &str) -> String {
    let base = code.split(['-', '.', '_']).next().unwrap_or(code);
    let name = match base {
        "en" => "English",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "tr" => "Turkish",
        "ru" => "Russian",
        "uk" => "Ukrainian",
        "pl" => "Polish",
        "sv" => "Swedish",
        "no" | "nb" => "Norwegian",
        "da" => "Danish",
        "fi" => "Finnish",
        "cs" => "Czech",
        "el" => "Greek",
        "ar" => "Arabic",
        "he" => "Hebrew",
        "hi" => "Hindi",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh" => "Chinese",
        "id" => "Indonesian",
        "vi" => "Vietnamese",
        "th" => "Thai",
        _ => return code.into(),
    };
    if base == code || code.ends_with(".*") {
        name.into()
    } else {
        format!("{name} ({code})")
    }
}
fn fix_title(kind: &str) -> &'static str {
    match kind {
        "browserCookies" | "signInRequired" | "unavailable" => "Use Browser Sign-In",
        "blocked" | "unsupportedLink" => "Update and Retry",
        "folderNotWritable" => "Choose Another Folder",
        "formatUnavailable" => "Use Best Quality",
        "notStarted" => "Wait for Start",
        _ => "Retry",
    }
}
fn job_title(job: &Job) -> String {
    if job.title.trim().is_empty() {
        job.url.clone()
    } else {
        job.title.clone()
    }
}
fn history_date(date: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(date)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%-d %b %Y, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| date.into())
}

pub struct App {
    engine: Engine,
    options: Options,
    url: String,
    last_url: String,
    last_access: String,
    fetch_due: Option<Instant>,
    refetch_after_tools: bool,
    selected: BTreeSet<usize>,
    selected_for: String,
    focus_url: bool,
    more_open: bool,
    part: bool,
    picking_info_browser: bool,
    retry_browser: Option<String>,
    details: Option<String>,
    show_command: bool,
    settings_open: bool,
    settings_tab: usize,
    /// The tab whose size the Settings window has, to resize it around its centre when the tab changes.
    settings_shown: Option<usize>,
    /// Where the Settings window opens: the middle of the screen.
    settings_position: Option<egui::Pos2>,
    /// Tries left to find the Settings and History windows and give them the blurred backdrop.
    backdrop_tries: [u8; 2],
    advanced_section: usize,
    saving_setup: Option<String>,
    managing_setups: bool,
    manage_selection: Option<String>,
    manage_name: String,
    confirm_reset: bool,
    confirm_clear_archive: bool,
    archive_count: (Option<Instant>, usize),
    history_open: bool,
    history_search: String,
    history_selected: BTreeSet<String>,
    history_anchor: Option<usize>,
    confirm_clear_history: bool,
    tray: Option<Tray>,
    tray_attempted: bool,
    quit: bool,
    confirm_quit: bool,
    focus: bool,
    last_clipboard: String,
    save_due: Option<Instant>,
    icon: Arc<egui::IconData>,
    icon_texture: egui::TextureHandle,
    _lock: File,
    smoke: bool,
    frames: u32,
    screenshot: Option<PathBuf>,
    screenshot_target: ViewportId,
    screenshot_requested: bool,
    /// Smoke tests can only capture the main window, so they draw Settings or History in it.
    smoke_window: Option<ViewportId>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        paths: Paths,
        lock: File,
        smoke: bool,
        blur: bool,
    ) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::install(&cc.egui_ctx);
        #[cfg(windows)]
        theme::set_blurred(blur && platform::backdrop::apply(cc));
        #[cfg(not(windows))]
        let _ = blur;
        let wake = cc.egui_ctx.clone();
        let mut engine = Engine::load(paths, move || wake.request_repaint())?;
        let options = engine.settings.last_options.clone().unwrap_or_default();
        if !smoke {
            engine.setup_tools(engine.settings.auto_update_ytdlp, false);
            engine.check_release(false);
        }
        let args: Vec<_> = std::env::args().collect();
        let value = |flag: &str| args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone());
        let part =
            !options.extras.section_start.is_empty() || !options.extras.section_end.is_empty();
        let mut app = Self {
            engine,
            options,
            url: String::new(),
            last_url: String::new(),
            last_access: String::new(),
            fetch_due: None,
            refetch_after_tools: false,
            selected: BTreeSet::new(),
            selected_for: String::new(),
            focus_url: true,
            more_open: false,
            part,
            picking_info_browser: false,
            retry_browser: None,
            details: None,
            show_command: false,
            settings_open: false,
            settings_tab: 0,
            settings_shown: None,
            settings_position: None,
            backdrop_tries: [0; 2],
            advanced_section: 0,
            saving_setup: None,
            managing_setups: false,
            manage_selection: None,
            manage_name: String::new(),
            confirm_reset: false,
            confirm_clear_archive: false,
            archive_count: (None, 0),
            history_open: false,
            history_search: String::new(),
            history_selected: BTreeSet::new(),
            history_anchor: None,
            confirm_clear_history: false,
            tray: None,
            tray_attempted: false,
            quit: false,
            confirm_quit: false,
            focus: false,
            last_clipboard: String::new(),
            save_due: None,
            icon_texture: {
                let image =
                    image::load_from_memory(include_bytes!("../assets/icon.png"))?.into_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                cc.egui_ctx.load_texture(
                    "app-icon",
                    egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                    egui::TextureOptions::LINEAR,
                )
            },
            icon: Arc::new(eframe::icon_data::from_png_bytes(include_bytes!(
                "../assets/icon.png"
            ))?),
            _lock: lock,
            smoke,
            frames: 0,
            screenshot: value("--screenshot").map(PathBuf::from),
            screenshot_target: ViewportId::ROOT,
            screenshot_requested: false,
            smoke_window: None,
        };
        if smoke {
            app.engine.settings.did_show_welcome = true;
            if args.iter().any(|s| s == "--light") {
                app.engine.settings.appearance = "light".into();
            }
            if args.iter().any(|s| s == "--demo") {
                app.demo();
            }
            match value("--screen").as_deref() {
                Some(tab @ ("general" | "advanced" | "about")) => {
                    app.settings_open = true;
                    app.settings_tab = ["general", "advanced", "about"]
                        .iter()
                        .position(|t| *t == tab)
                        .unwrap_or(0);
                    app.screenshot_target = settings_id();
                }
                Some(section) if section.starts_with("advanced-") => {
                    app.settings_open = true;
                    app.settings_tab = 1;
                    app.advanced_section = settings_ui::SECTIONS
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(&section[9..]))
                        .unwrap_or(0);
                    app.screenshot_target = settings_id();
                }
                Some("history") => {
                    app.history_open = true;
                    app.screenshot_target = history_id();
                }
                Some("playlist") => {
                    app.engine.info = Some(Media::parse(
                        serde_json::json!({"_type":"playlist", "title":"Nature journal", "uploader":"Nature journal", "entries":[{"title":"A quiet morning in the forest","duration":212},{"title":"Along the coast","duration":180},{"title":"The mountain trail","duration":340}]}),
                        &app.url,
                    ));
                }
                Some("more") => app.more_open = true,
                Some("details") => app.details = app.engine.jobs.first().map(|j| j.id.clone()),
                Some("welcome") => app.engine.settings.did_show_welcome = false,
                Some("empty") => {
                    app.engine.jobs.clear();
                    app.url.clear();
                    app.engine.info = None;
                }
                _ => (),
            }
            if app.screenshot_target != ViewportId::ROOT {
                app.smoke_window = Some(app.screenshot_target);
                app.screenshot_target = ViewportId::ROOT;
                app.settings_open = false;
                app.history_open = false;
            }
        }
        Ok(app)
    }

    fn demo(&mut self) {
        self.url = "https://example.com/watch/forest".into();
        self.last_url = self.url.clone();
        self.last_access = serde_json::to_string(&self.options.network).unwrap();
        self.engine.info = Some(Media::parse(
            serde_json::json!({"title":"A quiet morning in the forest", "uploader":"Nature journal", "duration":212}),
            &self.url,
        ));
        let folder = self.engine.folder();
        let mut job = Job::new(
            "https://example.com/watch/coast".into(),
            Options::default(),
            folder.clone(),
        );
        job.title = "Along the coast".into();
        job.status = Status::Downloading;
        job.progress.total = 94_000_000.0;
        job.progress.downloaded = 42_000_000.0;
        job.progress.speed = 3_400_000.0;
        job.progress.eta = Some(16.0);
        job.progress.fraction = Some(0.447);
        self.engine.jobs.push(job);
        let mut job = Job::new(
            "https://example.com/watch/trail".into(),
            Options::default(),
            folder.clone(),
        );
        job.title = "The mountain trail".into();
        job.status = Status::Paused;
        job.progress.fraction = Some(0.18);
        self.engine.jobs.push(job);
        let mut job = Job::new(
            "https://example.com/watch/rain".into(),
            Options::default(),
            folder,
        );
        job.title = "Rain on the lake".into();
        job.status = Status::Finished;
        self.engine.jobs.push(job);
    }

    fn can_download(&self) -> bool {
        self.engine.tools.ready()
            && !support::web_links(&self.url).is_empty()
            && !self.engine.info.as_ref().is_some_and(|i| {
                i.playlist && self.selected.is_empty() && !self.options.playlist.no_playlist
            })
    }
    fn show(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Focus);
    }
    fn request_quit(&mut self, ctx: &egui::Context) {
        if self
            .engine
            .jobs
            .iter()
            .any(|j| j.status.active() || j.status == Status::Queued)
        {
            self.confirm_quit = true;
            self.show(ctx);
        } else {
            self.quit = true;
            ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }
    fn open_settings(&mut self, tab: usize) {
        self.settings_open = true;
        self.settings_tab = tab;
    }
    fn update_tools(&mut self) {
        if !self.smoke {
            self.engine.setup_tools(true, false);
        }
    }
    fn clear_recent(&mut self) {
        self.engine.jobs.retain(|j| !j.status.recent());
        self.engine.persist();
    }

    fn tray(&mut self, ctx: &egui::Context) {
        if self.smoke {
            return;
        }
        if self.engine.settings.show_tray_icon && !self.tray_attempted {
            self.tray_attempted = true;
            match Tray::new(ctx) {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => {
                    self.engine.error = Some(format!("The {} is unavailable: {error}", tray_name()))
                }
            }
        }
        if !self.engine.settings.show_tray_icon {
            self.tray = None;
            self.tray_attempted = false;
        }
        let events = self
            .tray
            .as_ref()
            .map(|t| t.events.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in events {
            match event.as_str() {
                "show" => self.show(ctx),
                "pause" => self.engine.pause_all(),
                "resume" => self.engine.resume_all(),
                "settings" => self.open_settings(0),
                "history" => self.history_open = true,
                "quit" => self.request_quit(ctx),
                _ => (),
            }
        }
        if let Some(tray) = &self.tray {
            let active = self.engine.active_count();
            let fraction: f32 = self
                .engine
                .jobs
                .iter()
                .filter(|j| j.status.active())
                .map(|j| j.progress.fraction.unwrap_or(0.0))
                .sum();
            let title = if active == 0 {
                "Purrloader".into()
            } else {
                format!(
                    "Purrloader · {active} downloading · {:.0}%",
                    fraction / active as f32 * 100.0
                )
            };
            let _ = tray.icon.set_tooltip(Some(title));
        }
        let activation = self.engine.paths.root.join("activate");
        if activation.exists() {
            let _ = std::fs::remove_file(activation);
            self.show(ctx);
        }
    }
    fn clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            if text != self.last_clipboard
                && !support::web_links(&text).is_empty()
                && self.url.trim().is_empty()
            {
                self.url = support::web_links(&text).join(" ");
            }
            self.last_clipboard = text;
        }
    }
    fn paste_and_fetch(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            let links = support::web_links(&text);
            if !links.is_empty() {
                self.url = links.join(" ");
                self.fetch_due = Some(Instant::now());
            }
        }
    }
    fn choose_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_directory(self.engine.folder())
            .pick_folder()
        {
            self.engine.settings.download_folder = Some(folder);
        }
    }
    fn enqueue(&mut self) {
        self.enqueue_with(|_| ());
    }
    fn enqueue_with(&mut self, adjust: impl Fn(&mut Options)) {
        if !self.can_download() {
            return;
        }
        for url in support::web_links(&self.url) {
            let mut options = self.options.clone();
            adjust(&mut options);
            let mut job = Job::new(url.clone(), options, self.engine.folder());
            if let Some(info) = &self.engine.info
                && info.url == url
            {
                job.title = info.title.clone();
                job.thumbnail = info.thumbnail.clone();
                if info.playlist && !self.options.playlist.no_playlist {
                    job.playlist_items = Some(support::compress_selection(
                        self.selected.iter().copied().collect(),
                    ));
                }
            }
            self.engine.enqueue(job);
        }
        self.url.clear();
        self.engine.cancel_info();
    }
    fn input_changes(&mut self) {
        let access = serde_json::to_string(&self.options.network).unwrap_or_default();
        if self.url != self.last_url || access != self.last_access {
            if self.url != self.last_url {
                self.options.format.exact_format.clear();
            }
            self.engine.cancel_info();
            self.selected_for.clear();
            self.last_url = self.url.clone();
            self.last_access = access;
            self.fetch_due = Some(Instant::now() + Duration::from_millis(400));
        }
        if self.refetch_after_tools && !self.engine.tools_busy {
            self.refetch_after_tools = false;
            self.fetch_due = Some(Instant::now());
        }
        if !self.smoke
            && self.fetch_due.is_some_and(|due| Instant::now() >= due)
            && self.engine.tools.ready()
        {
            self.fetch_due = None;
            let links = support::web_links(&self.url);
            if links.len() == 1 {
                self.engine
                    .fetch_info(links[0].clone(), self.options.clone());
            }
        }
        if let Some(info) = &self.engine.info
            && info.url != self.selected_for
        {
            self.selected_for = info.url.clone();
            self.selected = info.entries.iter().map(|e| e.index).collect();
        }
    }

    // MARK: - Keyboard

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers as M};
        let take = |m: M, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        if take(M::COMMAND | M::SHIFT, Key::V) {
            self.paste_and_fetch();
        } else if take(M::COMMAND | M::SHIFT, Key::U) {
            self.update_tools();
        } else if take(M::COMMAND | M::ALT, Key::O) {
            self.open_settings(1);
        } else if take(M::COMMAND | M::ALT, Key::P) {
            self.engine.pause_all();
        } else if take(M::COMMAND | M::ALT, Key::R) {
            self.engine.resume_all();
        } else if take(M::COMMAND, Key::Enter) {
            self.enqueue();
        } else if take(M::COMMAND, Key::K) {
            self.clear_recent();
        } else if take(M::COMMAND, Key::Y) {
            self.history_open = true;
        } else if take(M::COMMAND, Key::Comma) {
            self.open_settings(0);
        } else if take(M::COMMAND, Key::Q) {
            self.request_quit(ctx);
        }
    }

    // MARK: - Main window

    fn main_window(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
        self.title_strip(ui);
        self.notice(ui);
        Frame::new()
            .inner_margin(Margin {
                left: 20,
                right: 20,
                top: 0,
                bottom: 18,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 14.0;
                self.url_field(ui);
                self.link_status(ui);
                self.controls(ui);
            });
        let (rule, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(rule, 0, theme::stroke());
        self.queue(ui);
    }

    /// The 38-point strip the Mac keeps for its traffic lights; here it holds the menu button.
    fn title_strip(&mut self, ui: &mut Ui) {
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
        let rect = Rect::from_min_size(
            pos2(strip.left() + 12.0, strip.center().y - 11.0),
            vec2(26.0, 22.0),
        );
        let response = ui.interact(rect, ui.id().with("app-menu"), Sense::click());
        let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
        if open || response.hovered() {
            ui.painter().rect_filled(rect, 6, ink(0.08));
        }
        icons::paint(
            ui,
            rect,
            "lineHorizontal3",
            14.0,
            W::Medium,
            if open || response.hovered() {
                theme::text90()
            } else {
                theme::tertiary()
            },
        );
        let response = response.on_hover_text("Menu");
        w::menu(&response, RectAlign::BOTTOM_START, |ui| self.app_menu(ui));
    }
    fn app_menu(&mut self, ui: &mut Ui) {
        let item = |ui: &mut Ui, label: &str, keys: Option<&str>, enabled: bool| {
            w::menu_item(ui, label, false, keys.map(shortcut).as_deref(), enabled)
        };
        if item(ui, "Paste Link and Get Info", Some("Ctrl+Shift+V"), true) {
            self.paste_and_fetch();
        }
        if item(ui, "Download", Some("Ctrl+Enter"), self.can_download()) {
            self.enqueue();
        }
        w::menu_separator(ui);
        if item(ui, "Advanced Options…", Some("Ctrl+Alt+O"), true) {
            self.open_settings(1);
        }
        if item(ui, "Pause All", Some("Ctrl+Alt+P"), true) {
            self.engine.pause_all();
        }
        if item(ui, "Resume All", Some("Ctrl+Alt+R"), true) {
            self.engine.resume_all();
        }
        if item(ui, "Clear Recent", Some("Ctrl+K"), true) {
            self.clear_recent();
        }
        if item(
            ui,
            "Update Helper Tools",
            Some("Ctrl+Shift+U"),
            !self.engine.tools_busy,
        ) {
            self.update_tools();
        }
        w::menu_separator(ui);
        if item(ui, "History", Some("Ctrl+Y"), true) {
            self.history_open = true;
        }
        if item(ui, "Settings…", Some("Ctrl+,"), true) {
            self.open_settings(0);
        }
        if item(ui, "About Purrloader", None, true) {
            self.open_settings(2);
        }
        w::menu_separator(ui);
        if item(ui, "Quit Purrloader", Some("Ctrl+Q"), true) {
            let ctx = ui.ctx().clone();
            self.request_quit(&ctx);
        }
    }

    /// One quiet line above the link field, only when something needs attention.
    fn notice(&mut self, ui: &mut Ui) {
        enum Notice {
            Error(String),
            Preparing(String),
            SetupFailed,
            UpdateFailed,
            Update(String),
        }
        let tools_ready = self.engine.tools.ready();
        let notice = if let Some(error) = &self.engine.error {
            Notice::Error(error.clone())
        } else if !tools_ready && self.engine.tools_error.is_none() {
            Notice::Preparing(self.engine.tools_message.clone())
        } else if self.engine.tools_error.is_some() && !tools_ready {
            Notice::SetupFailed
        } else if self.engine.tools_error.is_some() {
            Notice::UpdateFailed
        } else if let Some(version) = self.engine.settings.latest_app_version.clone()
            && support::newer(&version, purrloader::VERSION)
            && self.engine.settings.dismissed_app_version.as_deref() != Some(&version)
        {
            Notice::Update(version)
        } else {
            return;
        };
        Frame::new()
            .inner_margin(Margin {
                left: 20,
                right: 20,
                top: 0,
                bottom: 12,
            })
            .show(ui, |ui| {
                Frame::new()
                    .fill(ink(0.05))
                    .corner_radius(12)
                    .inner_margin(Margin::symmetric(14, 0))
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        ui.allocate_ui_with_layout(
                            vec2(width, 40.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.set_min_size(vec2(width, 40.0));
                                let text_width = width - 190.0;
                                let text = |ui: &mut Ui, s: &str, color: Color32| {
                                    let g = theme::wrapped(ui, s, 12.0, W::Medium, color, text_width, 2);
                                    let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                                    ui.painter().galley(r.min, g, color);
                                };
                                match &notice {
                                    Notice::Error(message) => {
                                        icons::show(ui, "exclamationTriangleFill", 13.0, W::Medium, theme::warning());
                                        text(ui, message, theme::secondary());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if w::circle_button(ui, "xmark", 22.0, false, true, "Dismiss").clicked() {
                                                self.engine.error = None;
                                            }
                                        });
                                    }
                                    Notice::Preparing(message) => {
                                        w::spinner(ui, 11.0);
                                        text(ui, "Getting Purrloader ready…", theme::secondary());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(theme::rich(message, 12.0, W::Medium, theme::tertiary()));
                                        });
                                    }
                                    Notice::SetupFailed => {
                                        icons::show(ui, "exclamationTriangleFill", 13.0, W::Medium, theme::warning());
                                        text(
                                            ui,
                                            "Couldn't download Purrloader's helper tools. Check your internet connection.",
                                            theme::secondary(),
                                        );
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if w::secondary_button(ui, "Retry").clicked() && !self.smoke {
                                                self.engine.setup_tools(false, false);
                                            }
                                        });
                                    }
                                    Notice::UpdateFailed => {
                                        icons::show(ui, "exclamationTriangleFill", 13.0, W::Medium, theme::warning());
                                        text(
                                            ui,
                                            "Couldn't update Purrloader's helper tools. Downloads still work; some sites may not.",
                                            theme::secondary(),
                                        );
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if w::circle_button(ui, "xmark", 22.0, false, true, "Dismiss").clicked() {
                                                self.engine.tools_error = None;
                                            }
                                            if self.engine.tools_busy {
                                                w::spinner(ui, 16.0);
                                            } else if w::secondary_button(ui, "Try Again").clicked() {
                                                self.update_tools();
                                            }
                                        });
                                    }
                                    Notice::Update(version) => {
                                        icons::show(ui, "sparkles", 13.0, W::Medium, theme::secondary());
                                        text(ui, &format!("Purrloader {version} is available."), theme::text90());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if w::circle_button(ui, "xmark", 22.0, false, true, "Dismiss").clicked() {
                                                self.engine.settings.dismissed_app_version = Some(version.clone());
                                            }
                                            if w::secondary_button(ui, "View").clicked() {
                                                let page = self.engine.settings.latest_app_page.clone().unwrap_or_else(|| {
                                                    "https://github.com/BLCNYY/Purrloader/releases/latest".into()
                                                });
                                                let _ = open::that_detached(page);
                                            }
                                        });
                                    }
                                }
                            },
                        );
                    });
            });
    }

    fn url_field(&mut self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
        let edit_id = ui.id().with("url");
        let focused = ui.memory(|m| m.has_focus(edit_id));
        ui.painter().rect_filled(rect, 14, theme::surface());
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            14,
            Stroke::new(1.0_f32, if focused { ink(0.3) } else { theme::stroke() }),
            egui::StrokeKind::Inside,
        );
        icons::paint(
            ui,
            Rect::from_min_size(pos2(rect.left() + 16.0, rect.top()), vec2(15.0, 48.0)),
            "link",
            15.0,
            W::Medium,
            theme::tertiary(),
        );
        let clear = if self.url.is_empty() { 0.0 } else { 32.0 };
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + 41.0, rect.top()),
            pos2(rect.right() - 16.0 - clear, rect.bottom()),
        );
        let response = w::place(
            ui,
            text_rect,
            egui::TextEdit::singleline(&mut self.url)
                .id(edit_id)
                .frame(false)
                .margin(Margin::ZERO)
                .font(theme::font(15.0, W::Regular))
                .text_color(theme::ink(1.0))
                .vertical_align(Align::Center)
                .desired_width(text_rect.width())
                .hint_text(theme::rich(
                    "Paste a link, or drop links and .txt files",
                    15.0,
                    W::Regular,
                    theme::tertiary(),
                )),
        );
        if self.focus_url {
            response.request_focus();
            self.focus_url = false;
        }
        if response.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.command)
        {
            if self.engine.info.is_some() {
                self.enqueue();
            } else {
                self.fetch_due = Some(Instant::now());
            }
        }
        if !self.url.is_empty() {
            let button = Rect::from_center_size(
                pos2(rect.right() - 27.0, rect.center().y),
                vec2(22.0, 22.0),
            );
            if w::place(ui, button, |ui: &mut Ui| {
                w::circle_button(ui, "xmark", 22.0, true, true, "Clear link")
            })
            .clicked()
            {
                self.url.clear();
                self.focus_url = true;
            }
        }
    }

    fn link_status(&mut self, ui: &mut Ui) {
        if self.engine.fetching {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                w::spinner(ui, 16.0);
                ui.label(theme::rich(
                    "Getting video info…",
                    12.0,
                    W::Regular,
                    theme::secondary(),
                ));
            });
        } else if let Some(error) = self.engine.info_error.clone() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.add_space(4.0);
                icons::show(
                    ui,
                    "exclamationTriangleFill",
                    14.0,
                    W::Medium,
                    theme::warning(),
                );
                let g = theme::wrapped(
                    ui,
                    &error.message,
                    12.0,
                    W::Medium,
                    ink(0.85),
                    ui.available_width() - 190.0,
                    3,
                );
                let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                ui.painter().galley(r.min, g, theme::ink(1.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.refetch_after_tools {
                        w::spinner(ui, 16.0);
                        return;
                    }
                    let button = w::secondary_button(ui, fix_title(&error.kind));
                    if button.clicked() {
                        match error.kind.as_str() {
                            "browserCookies" | "signInRequired" | "unavailable" => {
                                self.picking_info_browser = !self.picking_info_browser
                            }
                            "blocked" | "unsupportedLink" => {
                                self.update_tools();
                                self.refetch_after_tools = !self.smoke;
                            }
                            "notStarted" => self.enqueue_with(|o| o.extras.wait_for_video = true),
                            _ => self.fetch_due = Some(Instant::now()),
                        }
                    }
                    let mut open = self.picking_info_browser;
                    let current = self.options.network.cookie_browser.clone();
                    let mut chosen = None;
                    w::popover(&button, &mut open, 280.0, |ui| {
                        chosen = browser_picker(ui, &current)
                    });
                    self.picking_info_browser = open;
                    if let Some(browser) = chosen {
                        self.picking_info_browser = false;
                        self.options.network.cookie_browser = browser;
                        self.fetch_due = Some(Instant::now());
                    }
                });
            });
        } else if let Some(info) = self.engine.info.clone() {
            self.info_card(ui, &info);
        } else {
            let count = support::web_links(&self.url).len();
            if count > 1 {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    icons::show(ui, "listBullet", 12.0, W::Medium, theme::secondary());
                    ui.label(theme::rich(
                        format!("{count} links will be added to the queue"),
                        12.0,
                        W::Regular,
                        theme::secondary(),
                    ));
                });
            }
        }
    }

    fn info_card(&mut self, ui: &mut Ui, info: &Media) {
        w::card().inner_margin(Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(14.0, 12.0);
            ui.horizontal(|ui| {
                w::thumbnail(ui, info.thumbnail.as_deref(), 120.0, 1.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;
                    let width = ui.available_width();
                    let height = 67.5;
                    let mut parts = Vec::new();
                    if !info.uploader.is_empty() {
                        parts.push(info.uploader.clone());
                    }
                    if info.playlist {
                        parts.push(match info.entries.len() {
                            1 => "1 item".into(),
                            n => format!("{n} items"),
                        });
                    }
                    if info.duration > 0.0 {
                        parts.push(support::duration(info.duration));
                    }
                    let title = theme::wrapped(
                        ui,
                        &info.title,
                        15.0,
                        W::Semibold,
                        theme::ink(1.0),
                        width,
                        2,
                    );
                    let meta = theme::line(
                        ui,
                        &parts.join("  ·  "),
                        12.0,
                        W::Regular,
                        theme::secondary(),
                        Some(width),
                    );
                    let live = info.raw["is_live"].as_bool().unwrap_or(false);
                    let custom = !self.options.format.exact_format.is_empty();
                    let content = title.size().y
                        + 5.0
                        + meta.size().y
                        + if live || custom { 23.0 } else { 0.0 };
                    ui.add_space(((height - content) / 2.0).max(0.0));
                    let (r, _) = ui.allocate_exact_size(title.size(), Sense::hover());
                    ui.painter().galley(r.min, title, theme::ink(1.0));
                    let (r, _) = ui.allocate_exact_size(meta.size(), Sense::hover());
                    ui.painter().galley(r.min, meta, theme::ink(1.0));
                    if live || custom {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if live {
                                w::tag(ui, "LIVE", theme::RED);
                            }
                            if custom {
                                w::tag(ui, "Custom format", theme::ink(1.0));
                            }
                        });
                    }
                });
            });
            if info.playlist && !info.entries.is_empty() {
                self.playlist_picker(ui, info);
            }
        });
    }

    fn playlist_picker(&mut self, ui: &mut Ui, info: &Media) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.label(theme::rich(
                    format!("{} of {} selected", self.selected.len(), info.entries.len()),
                    12.0,
                    W::Regular,
                    theme::secondary(),
                ));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if w::plain_button(ui, "None", theme::ink(1.0)).clicked() {
                        self.selected.clear();
                    }
                    if w::plain_button(ui, "Select All", theme::ink(1.0)).clicked() {
                        self.selected = info.entries.iter().map(|e| e.index).collect();
                    }
                });
            });
            let height = (info.entries.len() as f32 * 30.0).min(170.0);
            egui::ScrollArea::vertical()
                .id_salt("playlist")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for entry in &info.entries {
                        let on = self.selected.contains(&entry.index);
                        let (rect, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                        if on {
                            ui.painter().rect_filled(rect, 8, ink(0.05));
                        }
                        let color = if on {
                            theme::ink(1.0)
                        } else {
                            theme::secondary()
                        };
                        icons::paint(
                            ui,
                            Rect::from_min_size(rect.min + vec2(10.0, 0.0), vec2(14.0, 28.0)),
                            if on { "checkmarkCircleFill" } else { "circle" },
                            14.0,
                            W::Regular,
                            if on {
                                theme::ink(1.0)
                            } else {
                                theme::tertiary()
                            },
                        );
                        let index = theme::line(
                            ui,
                            &entry.index.to_string(),
                            12.0,
                            W::Regular,
                            theme::tertiary(),
                            None,
                        );
                        ui.painter().galley(
                            pos2(
                                rect.left() + 66.0 - index.size().x,
                                rect.center().y - index.size().y / 2.0,
                            ),
                            index,
                            theme::ink(1.0),
                        );
                        let duration = if entry.duration > 0.0 {
                            support::duration(entry.duration)
                        } else {
                            String::new()
                        };
                        let duration =
                            theme::line(ui, &duration, 12.0, W::Regular, theme::tertiary(), None);
                        let title = theme::line(
                            ui,
                            &entry.title,
                            12.0,
                            W::Regular,
                            color,
                            Some(rect.width() - 96.0 - duration.size().x),
                        );
                        ui.painter().galley(
                            pos2(rect.left() + 76.0, rect.center().y - title.size().y / 2.0),
                            title,
                            theme::ink(1.0),
                        );
                        ui.painter().galley(
                            pos2(
                                rect.right() - 10.0 - duration.size().x,
                                rect.center().y - duration.size().y / 2.0,
                            ),
                            duration,
                            theme::ink(1.0),
                        );
                        if response.clicked() {
                            if on {
                                self.selected.remove(&entry.index);
                            } else {
                                self.selected.insert(entry.index);
                            }
                        }
                    }
                });
        });
    }

    fn quality_heights(&self) -> Vec<u32> {
        let mut heights: Vec<u32> = self
            .engine
            .info
            .as_ref()
            .and_then(|i| i.raw["formats"].as_array())
            .map(|formats| {
                formats
                    .iter()
                    .filter_map(|f| f["height"].as_u64())
                    .map(|h| h as u32)
                    .filter(|h| *h >= 144)
                    .collect()
            })
            .unwrap_or_default();
        if heights.is_empty() {
            heights = vec![2160, 1440, 1080, 720, 480, 360];
        }
        if let Some(h) = self.options.format.max_height {
            heights.push(h);
        }
        heights.sort_unstable_by(|a, b| b.cmp(a));
        heights.dedup();
        heights
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            w::pill_segment(
                ui,
                "mode",
                &mut self.options.format.mode,
                &[("video".into(), "Video"), ("audio".into(), "Audio")],
                32.0,
            );
            if self.options.format.mode == "video" {
                let text = self
                    .options
                    .format
                    .max_height
                    .map(settings_ui::height_label)
                    .unwrap_or_else(|| "Best".into());
                let response = w::chip(ui, &text, None, true, false).on_hover_text("Highest quality to download");
                let heights = self.quality_heights();
                w::menu(&response, RectAlign::BOTTOM_START, |ui| {
                    if w::menu_item(ui, "Best", self.options.format.max_height.is_none(), None, true) {
                        self.options.format.max_height = None;
                    }
                    for h in heights {
                        if w::menu_item(
                            ui,
                            &settings_ui::height_label(h),
                            self.options.format.max_height == Some(h),
                            None,
                            true,
                        ) {
                            self.options.format.max_height = Some(h);
                        }
                    }
                });
            } else {
                let current = settings_ui::AUDIO_FORMATS
                    .iter()
                    .find(|(v, _)| *v == self.options.format.audio_format)
                    .map(|(_, t)| *t)
                    .unwrap_or("MP3");
                let response = w::chip(ui, current, None, true, false).on_hover_text(
                    "MP3 plays everywhere. Best quality keeps the original sound without converting it.",
                );
                w::menu(&response, RectAlign::BOTTOM_START, |ui| {
                    for (value, title) in [("mp3", "MP3"), ("m4a", "M4A"), ("best", "Best quality")] {
                        if w::menu_item(ui, title, self.options.format.audio_format == value, None, true) {
                            self.options.format.audio_format = value.into();
                        }
                    }
                });
            }
            let folder = self.engine.folder();
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| folder.display().to_string());
            if w::chip(ui, &name, Some("folder"), false, false)
                .on_hover_text(format!("Save to {}", folder.display()))
                .clicked()
            {
                self.choose_folder();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let can = self.can_download();
                if w::primary_button(ui, "Download", can)
                    .on_hover_text(format!("Download ({})", shortcut("Ctrl+Enter")))
                    .clicked()
                {
                    self.enqueue();
                }
                let active = self.active_more_options();
                let text = if active == 0 {
                    "More".to_owned()
                } else {
                    format!("More · {active}")
                };
                let more = w::chip(ui, &text, Some("sliders"), false, self.more_open)
                    .on_hover_text("Subtitles, sponsor skipping, part of a video, browser sign-in");
                if more.clicked() {
                    self.more_open = !self.more_open;
                }
                let mut open = self.more_open;
                let mut close = false;
                w::popover(&more, &mut open, 330.0, |ui| close = self.more_options(ui));
                self.more_open = open && !close;
            });
        });
    }
    fn active_more_options(&self) -> usize {
        let o = &self.options;
        [
            o.subtitles.enabled,
            o.extras.sponsor_block == "remove",
            self.part,
            o.network.cookie_browser != "none",
        ]
        .iter()
        .filter(|b| **b)
        .count()
    }

    /// The few extras a normal download might need. Everything else lives in Settings > Advanced.
    fn more_options(&mut self, ui: &mut Ui) -> bool {
        ui.spacing_mut().item_spacing.y = 0.0;
        let section = |ui: &mut Ui, last: bool, add: &mut dyn FnMut(&mut Ui)| {
            let response = Frame::new().inner_margin(Margin::same(14)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 8.0;
                add(ui);
            });
            if !last {
                let r = response.response.rect;
                ui.painter().line_segment(
                    [pos2(r.left(), r.bottom()), pos2(r.right(), r.bottom())],
                    Stroke::new(1.0_f32, theme::stroke()),
                );
            }
        };
        let label_row = |ui: &mut Ui, title: &str, control: &mut dyn FnMut(&mut Ui)| {
            ui.horizontal(|ui| {
                ui.label(theme::rich(title, 13.0, W::Regular, theme::text90()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| control(ui));
            });
        };
        let note = |ui: &mut Ui, text: &str| {
            theme::paragraph(ui, text, 11.0, W::Regular, theme::secondary(), 0)
        };
        let info = self.engine.info.clone();
        let options = &mut self.options;
        section(ui, false, &mut |ui| {
            label_row(ui, "Subtitles", &mut |ui| {
                w::switch(ui, &mut options.subtitles.enabled);
            });
            if options.subtitles.enabled {
                let mut choices: Vec<(String, String)> = info
                    .as_ref()
                    .map(|i| {
                        i.subtitles
                            .iter()
                            .map(|(code, auto)| {
                                let mut title = language_name(code);
                                if *auto {
                                    title.push_str(" (auto-generated)");
                                }
                                (code.clone(), title)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                if choices.is_empty() {
                    choices.push(("en.*".into(), "English".into()));
                }
                let current = options.subtitles.languages.clone();
                if !choices.iter().any(|(c, _)| *c == current) {
                    choices.push((current.clone(), language_name(&current)));
                }
                let refs: Vec<(String, &str)> = choices
                    .iter()
                    .map(|(c, t)| (c.clone(), t.as_str()))
                    .collect();
                label_row(ui, "Language", &mut |ui| {
                    w::popup_button(ui, &mut options.subtitles.languages, &refs, 0.0);
                });
                note(
                    ui,
                    if options.format.mode == "video" {
                        "Added to the video."
                    } else {
                        "Saved next to the audio file."
                    },
                );
            }
        });
        section(ui, false, &mut |ui| {
            let mut skip = options.extras.sponsor_block == "remove";
            label_row(ui, "Skip sponsor segments", &mut |ui| {
                if w::switch(ui, &mut skip).changed() {
                    options.extras.sponsor_block = if skip { "remove" } else { "off" }.into();
                }
            });
            note(
                ui,
                "Cuts out sponsor spots, self-promotion and reminders to subscribe. Works on supported sites only.",
            );
        });
        let part = &mut self.part;
        section(ui, false, &mut |ui| {
            label_row(ui, "Download only a part", &mut |ui| {
                if w::switch(ui, part).changed() && !*part {
                    options.extras.section_start.clear();
                    options.extras.section_end.clear();
                }
            });
            if *part {
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 34.0) / 2.0;
                    w::text_input(ui, &mut options.extras.section_start, "0:00", width);
                    ui.label(theme::rich("to", 13.0, W::Regular, theme::secondary()));
                    w::text_input(ui, &mut options.extras.section_end, "End", width);
                });
                note(ui, "Times like 1:30 or 1:02:45.");
            }
        });
        let mut advanced = false;
        section(ui, false, &mut |ui| {
            let mut browsers: Vec<(String, &str)> =
                vec![("none".into(), "Don't sign in"), ("—".into(), "—")];
            for b in settings_ui::cookie_browsers() {
                browsers.push(((*b).into(), settings_ui::browser_title(b)));
            }
            label_row(ui, "Sign in with my browser", &mut |ui| {
                w::popup_button(ui, &mut options.network.cookie_browser, &browsers, 0.0);
            });
            note(
                ui,
                "For private, age-restricted or members-only content you can watch in that browser.",
            );
        });
        section(ui, true, &mut |ui| {
            advanced = w::link_button(ui, "Advanced Options…").clicked();
        });
        if advanced {
            self.open_settings(1);
        }
        advanced
    }

    // MARK: - Queue

    fn queue(&mut self, ui: &mut Ui) {
        let jobs = self.engine.jobs.clone();
        if jobs.is_empty() {
            let size = ui.available_size();
            ui.allocate_ui_with_layout(size, Layout::top_down(Align::Center), |ui| {
                ui.add_space(((size.y - 104.0) / 2.0).max(0.0));
                ui.spacing_mut().item_spacing.y = 10.0;
                let (ring, _) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
                ui.painter().circle_stroke(
                    ring.center(),
                    25.25,
                    Stroke::new(1.5_f32, theme::stroke()),
                );
                icons::paint(ui, ring, "arrowDown", 20.0, W::Medium, theme::tertiary());
                ui.label(theme::rich(
                    "Paste. Purr. Saved.",
                    14.0,
                    W::Semibold,
                    theme::secondary(),
                ));
                ui.label(theme::rich(
                    "Paste a link above, or drop one anywhere in this window.",
                    12.0,
                    W::Regular,
                    theme::tertiary(),
                ));
            });
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("queue")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;
                    let current: Vec<_> = jobs.iter().filter(|j| !j.status.recent()).collect();
                    let recent: Vec<_> = jobs.iter().filter(|j| j.status.recent()).collect();
                    for job in &current {
                        self.job_row(ui, job);
                    }
                    if !recent.is_empty() {
                        if !current.is_empty() {
                            ui.add_space(10.0);
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            ui.label(theme::rich("Recent", 12.0, W::Semibold, theme::tertiary()));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(6.0);
                                if w::plain_button(ui, "Clear", theme::secondary())
                                    .on_hover_text(format!(
                                        "Remove finished downloads from this list. The files stay where they are. ({})",
                                        shortcut("Ctrl+K")
                                    ))
                                    .clicked()
                                {
                                    self.clear_recent();
                                }
                            });
                        });
                    }
                    for job in &recent {
                        self.job_row(ui, job);
                    }
                });
            });
    }

    fn job_row(&mut self, ui: &mut Ui, job: &Job) {
        let id = ui.id().with(("job", &job.id));
        let previous: Option<Rect> = ui.data(|d| d.get_temp(id));
        let row = previous.map(|rect| ui.interact(rect, id.with("row"), Sense::click()));
        let hovered = row.as_ref().is_some_and(|r| r.contains_pointer());
        let busy = self.engine.busy(&job.id);
        let active = job.status.active() || job.status == Status::Queued;
        let recent = job.status.recent();
        let mut picking = self.retry_browser.as_deref() == Some(&job.id);
        let frame = Frame::new()
            .fill(if hovered { ink(0.07) } else { ink(0.04) })
            .stroke(Stroke::new(1.0_f32, theme::stroke()))
            .corner_radius(16)
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    w::thumbnail(
                        ui,
                        job.thumbnail.as_deref(),
                        if recent { 64.0 } else { 80.0 },
                        if job.status == Status::Cancelled {
                            0.5
                        } else {
                            1.0
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.scope(|ui| {
                            if !(active || job.error.is_some()) && !hovered {
                                ui.multiply_opacity(0.6);
                            }
                            if w::circle_button(ui, "textAlignLeft", 28.0, true, true, "Details")
                                .clicked()
                            {
                                self.details = Some(job.id.clone());
                            }
                            let button = |ui: &mut Ui, icon: &str, tip: &str, enabled: bool| {
                                w::circle_button(ui, icon, 28.0, true, enabled, tip).clicked()
                            };
                            match job.status {
                                Status::Queued
                                | Status::Starting
                                | Status::Downloading
                                | Status::Processing => {
                                    if button(ui, "xmark", "Cancel", job.stop_reason.is_none()) {
                                        self.engine.stop(&job.id, Status::Cancelled);
                                    }
                                    if button(ui, "pauseFill", "Pause", job.stop_reason.is_none()) {
                                        self.engine.stop(&job.id, Status::Paused);
                                    }
                                }
                                Status::Paused => {
                                    if button(ui, "xmark", "Cancel", true) {
                                        self.engine.stop(&job.id, Status::Cancelled);
                                    }
                                    if button(ui, "playFill", "Resume", !busy) {
                                        self.engine.retry(&job.id);
                                    }
                                }
                                Status::Failed => {
                                    if button(ui, "trash", "Remove", !busy) {
                                        self.engine.remove(&job.id);
                                    }
                                    let kind = job
                                        .error
                                        .as_ref()
                                        .map(|e| e.kind.clone())
                                        .unwrap_or_default();
                                    let title = fix_title(&kind);
                                    if title != "Retry"
                                        && button(ui, "arrowClockwise", "Retry", !busy)
                                    {
                                        self.engine.retry(&job.id);
                                    }
                                    if self.engine.tools_busy
                                        && matches!(kind.as_str(), "blocked" | "unsupportedLink")
                                    {
                                        ui.add_space(8.0);
                                        w::spinner(ui, 16.0);
                                        ui.add_space(8.0);
                                    } else {
                                        let fix = w::secondary_button(ui, title);
                                        if fix.clicked() {
                                            self.job_fix(job, &kind, &mut picking);
                                        }
                                        let current = job.options.network.cookie_browser.clone();
                                        let mut chosen = None;
                                        w::popover(&fix, &mut picking, 280.0, |ui| {
                                            chosen = browser_picker(ui, &current)
                                        });
                                        if let Some(browser) = chosen {
                                            picking = false;
                                            // Remember it, so the next private video works straight away.
                                            self.options.network.cookie_browser = browser.clone();
                                            if let Some(j) =
                                                self.engine.jobs.iter_mut().find(|j| j.id == job.id)
                                            {
                                                j.options.network.cookie_browser = browser;
                                            }
                                            self.engine.retry(&job.id);
                                        }
                                    }
                                }
                                Status::Cancelled => {
                                    if button(ui, "trash", "Remove", !busy) {
                                        self.engine.remove(&job.id);
                                    }
                                    if button(ui, "arrowClockwise", "Download Again", !busy) {
                                        self.engine.retry(&job.id);
                                    }
                                }
                                Status::Finished => {
                                    if button(ui, "trash", "Remove from list", true) {
                                        self.engine.remove(&job.id);
                                    }
                                    if button(
                                        ui,
                                        "magnifyingglass",
                                        &format!("Show in {}", file_manager()),
                                        true,
                                    ) {
                                        platform::reveal(
                                            job.files.first().map(|p| p.as_path()),
                                            &job.folder,
                                        );
                                    }
                                }
                            }
                        });
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            let width = ui.available_width();
                            ui.set_width(width);
                            ui.spacing_mut().item_spacing.y = 7.0;
                            let title = theme::line(
                                ui,
                                &job_title(job),
                                13.0,
                                W::Semibold,
                                theme::ink(1.0),
                                Some(width),
                            );
                            let (r, _) =
                                ui.allocate_exact_size(vec2(width, title.size().y), Sense::hover());
                            ui.painter().galley(r.min, title, theme::ink(1.0));
                            let running = job.status.active() || job.status == Status::Queued;
                            if running {
                                w::slim_progress(ui, job.progress.fraction, width);
                            }
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let icon = match job.status {
                                    Status::Finished => Some(("checkmarkCircleFill", theme::GREEN)),
                                    Status::Failed => Some(("exclamationCircleFill", theme::RED)),
                                    Status::Paused => Some(("pauseCircleFill", theme::secondary())),
                                    Status::Cancelled => {
                                        Some(("xmarkCircleFill", theme::secondary()))
                                    }
                                    _ => None,
                                };
                                if let Some((name, color)) = icon {
                                    icons::show(ui, name, 12.0, W::Medium, color);
                                }
                                let percent =
                                    (running && job.progress.fraction.is_some()).then(|| {
                                        format!(
                                            "{}%",
                                            (job.progress.fraction.unwrap_or(0.0) * 100.0).floor()
                                        )
                                    });
                                let reserve = if percent.is_some() { 40.0 } else { 0.0 };
                                let text = job.status_text();
                                let galley = theme::wrapped(
                                    ui,
                                    &text,
                                    11.0,
                                    W::Regular,
                                    theme::secondary(),
                                    ui.available_width() - reserve,
                                    if job.error.is_some() { 2 } else { 1 },
                                );
                                let (r, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                                ui.painter().galley(r.min, galley, theme::ink(1.0));
                                if let Some(percent) = percent {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(theme::rich(percent, 11.0, W::Regular, ink(0.8)));
                                    });
                                }
                            });
                            if running
                                && job.progress.item_count.is_some_and(|n| n > 1)
                                && let Some(item) = &job.progress.title
                            {
                                let g = theme::line(
                                    ui,
                                    item,
                                    11.0,
                                    W::Regular,
                                    theme::tertiary(),
                                    Some(width),
                                );
                                let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                                ui.painter().galley(r.min, g, theme::ink(1.0));
                            }
                        });
                    });
                });
            });
        ui.data_mut(|d| d.insert_temp(id, frame.response.rect));
        if picking {
            self.retry_browser = Some(job.id.clone());
        } else if self.retry_browser.as_deref() == Some(&job.id) {
            self.retry_browser = None;
        }
        let Some(row) = row else { return };
        if row.double_clicked() {
            if job.status == Status::Finished {
                platform::reveal(job.files.first().map(|p| p.as_path()), &job.folder);
            } else {
                self.details = Some(job.id.clone());
            }
        }
        egui::Popup::context_menu(&row)
            .frame(w::menu_frame())
            .layout(Layout::top_down_justified(Align::LEFT))
            .style(|s: &mut egui::Style| s.spacing.item_spacing = egui::Vec2::ZERO)
            .show(|ui| {
                if job.status == Status::Finished
                    && !job.files.is_empty()
                    && w::menu_item(ui, "Open", false, None, true)
                {
                    let _ = open::that_detached(&job.files[0]);
                }
                if w::menu_item(
                    ui,
                    &format!("Show in {}", file_manager()),
                    false,
                    None,
                    true,
                ) {
                    platform::reveal(job.files.first().map(|p| p.as_path()), &job.folder);
                }
                if w::menu_item(ui, "Copy Link", false, None, true) {
                    ui.ctx().copy_text(job.url.clone());
                }
                if w::menu_item(ui, "Show Details", false, None, true) {
                    self.details = Some(job.id.clone());
                }
                w::menu_separator(ui);
                if w::menu_item(
                    ui,
                    "Remove from List",
                    false,
                    None,
                    !job.status.active() && !busy,
                ) {
                    self.engine.remove(&job.id);
                }
            });
    }

    fn job_fix(&mut self, job: &Job, kind: &str, picking: &mut bool) {
        match kind {
            "browserCookies" | "signInRequired" | "unavailable" => {
                *picking = !*picking;
                return;
            }
            "blocked" | "unsupportedLink" => {
                self.engine.update_and_retry(&job.id);
                return;
            }
            "folderNotWritable" => {
                let Some(folder) = rfd::FileDialog::new()
                    .set_directory(&job.folder)
                    .pick_folder()
                else {
                    return;
                };
                if let Some(j) = self.engine.jobs.iter_mut().find(|j| j.id == job.id) {
                    j.folder = folder;
                }
            }
            "formatUnavailable" => {
                if let Some(j) = self.engine.jobs.iter_mut().find(|j| j.id == job.id) {
                    j.options.format.exact_format.clear();
                    j.options.format.max_height = None;
                }
            }
            "notStarted" => {
                if let Some(j) = self.engine.jobs.iter_mut().find(|j| j.id == job.id) {
                    j.options.extras.wait_for_video = true;
                }
            }
            _ => (),
        }
        self.engine.retry(&job.id);
    }

    // MARK: - Sheets

    /// What happened during a download: its full log, and the command behind it for the curious.
    fn details_sheet(&mut self, ctx: &egui::Context) {
        let Some(job) = self
            .details
            .as_ref()
            .and_then(|id| self.engine.jobs.iter().find(|j| &j.id == id))
            .cloned()
        else {
            return;
        };
        let screen = ctx.content_rect().size();
        let size = vec2((screen.x - 48.0).min(760.0), (screen.y - 48.0).min(540.0));
        let mut done = false;
        let close = w::sheet(ctx, "details", |ui| {
            Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                ui.set_width(size.x - 32.0);
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let width = ui.available_width();
                    theme::paragraph(ui, &job_title(&job), 15.0, W::Semibold, theme::ink(1.0), 2);
                    let g = theme::wrapped(
                        ui,
                        &job.status_text(),
                        12.0,
                        W::Regular,
                        theme::secondary(),
                        width,
                        2,
                    );
                    let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                    ui.painter().galley(r.min, g, theme::ink(1.0));
                });
                let log_height = size.y - 160.0 - if self.show_command { 60.0 } else { 0.0 };
                Frame::new()
                    .fill(theme::surface())
                    .corner_radius(8)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        egui::ScrollArea::both()
                            .id_salt("log")
                            .stick_to_bottom(true)
                            .auto_shrink([false, false])
                            .max_height(log_height.max(120.0))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                for line in &job.log {
                                    let color = if line.starts_with("ERROR") {
                                        theme::RED
                                    } else if line.starts_with("WARNING") {
                                        theme::ORANGE
                                    } else {
                                        theme::text90()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(line)
                                                .font(egui::FontId::monospace(10.5))
                                                .color(color),
                                        )
                                        .selectable(true)
                                        .wrap(),
                                    );
                                }
                            });
                    });
                let line = support::command_line(&job.command, cfg!(windows));
                let (rect, response) = ui.allocate_exact_size(vec2(90.0, 16.0), Sense::click());
                let chevron = Rect::from_min_size(rect.min, vec2(9.0, 16.0));
                let angle = ui
                    .ctx()
                    .animate_bool_with_time(response.id, self.show_command, 0.15);
                icons::image("chevronRight", 9.0, W::Bold, theme::text90())
                    .rotate(angle * std::f32::consts::FRAC_PI_2, vec2(0.5, 0.5))
                    .paint_at(ui, Rect::from_center_size(chevron.center(), vec2(9.0, 9.0)));
                let g = theme::line(ui, "Command", 12.0, W::Regular, theme::text90(), None);
                ui.painter().galley(
                    pos2(rect.left() + 14.0, rect.center().y - g.size().y / 2.0),
                    g,
                    theme::ink(1.0),
                );
                if response.clicked() {
                    self.show_command = !self.show_command;
                }
                if self.show_command {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if w::circle_button(ui, "docOnDoc", 24.0, false, true, "Copy command")
                                .clicked()
                            {
                                ui.ctx().copy_text(line.clone());
                            }
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&line)
                                        .font(egui::FontId::monospace(10.5))
                                        .color(theme::text90()),
                                )
                                .selectable(true)
                                .wrap(),
                            );
                        });
                    });
                }
                ui.horizontal(|ui| {
                    if w::bezel_button(ui, "Copy Log", false, true).clicked() {
                        ui.ctx().copy_text(job.log.join("\n"));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        done = w::bezel_button(ui, "Done", true, true).clicked();
                    });
                });
            });
        });
        if close || done {
            self.details = None;
        }
    }

    /// Shown once, on first launch, while the helper tools download.
    fn welcome_sheet(&mut self, ctx: &egui::Context) {
        if self.engine.settings.did_show_welcome {
            return;
        }
        let mut start = false;
        w::sheet(ctx, "welcome", |ui| {
            Frame::new().inner_margin(Margin::symmetric(36, 30)).show(ui, |ui| {
                ui.set_width(368.0);
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 18.0;
                    ui.add(self.icon_image().fit_to_exact_size(vec2(96.0, 96.0)));
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(theme::rich("Welcome to Purrloader", 20.0, W::Semibold, theme::ink(1.0)));
                        ui.add(
                            egui::Label::new(theme::rich(
                                "Paste a link to a video and Purrloader saves it, or just its audio, to your computer.",
                                13.0,
                                W::Regular,
                                theme::secondary(),
                            ))
                            .halign(Align::Center)
                            .wrap(),
                        );
                    });
                    let (rect, _) = ui.allocate_exact_size(vec2(368.0, 44.0), Sense::hover());
                    let mut status = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Center)));
                    if let Some(error) = self.engine.tools_error.clone().filter(|_| !self.engine.tools.ready()) {
                        status.horizontal(|ui| {
                            icons::show(ui, "exclamationTriangleFill", 13.0, W::Medium, theme::warning());
                            ui.label(theme::rich(
                                "Couldn't download the helper tools. Check your internet connection.",
                                12.0,
                                W::Medium,
                                theme::warning(),
                            ))
                            .on_hover_text(error);
                        });
                        if w::secondary_button(&mut status, "Try Again").clicked() && !self.smoke {
                            self.engine.setup_tools(false, false);
                        }
                    } else if self.engine.tools.ready() {
                        status.add_space(14.0);
                        status.horizontal(|ui| {
                            ui.add_space(130.0);
                            icons::show(ui, "checkmarkCircleFill", 13.0, W::Medium, theme::GREEN);
                            ui.label(theme::rich("Ready to go.", 12.0, W::Medium, theme::GREEN));
                        });
                    } else {
                        status.add_space(10.0);
                        let (bar, _) = status.allocate_exact_size(vec2(368.0, 6.0), Sense::hover());
                        status.painter().rect_filled(bar, 3, ink(0.12));
                        let t = (status.input(|i| i.time) / 1.6).fract() as f32;
                        let left = bar.left() - 110.0 + (bar.width() + 110.0) * t;
                        let fill = Rect::from_min_max(
                            pos2(left.max(bar.left()), bar.top()),
                            pos2((left + 110.0).min(bar.right()), bar.bottom()),
                        );
                        if fill.width() > 0.0 {
                            status.painter().rect_filled(fill, 3, theme::ink(1.0));
                        }
                        status.ctx().request_repaint();
                        status.add_space(6.0);
                        status.label(theme::rich(
                            format!("Downloading free helper tools… {}", self.engine.tools_message),
                            11.0,
                            W::Regular,
                            theme::secondary(),
                        ));
                    }
                    ui.label(theme::rich(
                        "Only download content you own or have permission to download.",
                        11.0,
                        W::Regular,
                        theme::tertiary(),
                    ));
                    start = w::primary_button(ui, "Get Started", true).clicked();
                });
            });
        });
        if start {
            self.engine.settings.did_show_welcome = true;
        }
    }

    fn quit_sheet(&mut self, ctx: &egui::Context) {
        if !self.confirm_quit {
            return;
        }
        match confirm_sheet(
            ctx,
            "quit",
            &self.icon_image(),
            "Downloads are still in progress",
            "Quitting pauses them. They'll pick up where they left off the next time you open Purrloader.",
            "Quit",
            "Keep Downloading",
        ) {
            Some(true) => {
                self.quit = true;
                self.confirm_quit = false;
                ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
            }
            Some(false) => self.confirm_quit = false,
            None => (),
        }
    }
    fn icon_image(&self) -> egui::Image<'static> {
        egui::Image::from_texture(&self.icon_texture)
    }

    // MARK: - Settings

    fn settings_window(&mut self, ctx: &egui::Context) {
        let tab = self.settings_tab.min(2);
        let title = ["General", "Advanced", "About"][tab];
        let shown = self.settings_shown.unwrap_or(tab);
        if self.settings_shown.is_none() {
            self.settings_position = centered(ctx, SETTINGS_SIZES[tab]);
        }
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id("purrloader")
            .with_icon(self.icon.clone())
            .with_inner_size(SETTINGS_SIZES[shown])
            .with_transparent(theme::blurred())
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false);
        if let Some(position) = self.settings_position {
            builder = builder.with_position(position);
        }
        ctx.show_viewport_immediate(settings_id(), builder, |ctx, class| {
            title_bar_theme(ctx, &self.engine.settings.appearance.clone());
            self.backdrop(0, title);
            if shown != tab {
                // Switching tabs resizes the window around its centre instead of out from its top-left corner.
                let (old, new) = (SETTINGS_SIZES[shown], SETTINGS_SIZES[tab]);
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    let size = outer.size() + vec2(new[0] - old[0], new[1] - old[1]);
                    let position = (outer.center() - size / 2.0).round();
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
                    self.settings_position = Some(position);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new.into()));
            }
            self.settings_shown = Some(tab);
            if class == egui::ViewportClass::Embedded {
                let mut open = true;
                egui::Window::new("Settings")
                    .open(&mut open)
                    .show(ctx, |ui| self.settings_contents(ui));
                self.settings_open = open;
                return;
            }
            egui::CentralPanel::default()
                .frame(Frame::new().fill(theme::window_fill()))
                .show(ctx, |ui| self.settings_contents(ui));
            self.settings_sheets(ctx);
            if ctx.input(|i| i.viewport().close_requested()) {
                self.settings_open = false;
            }
            self.capture(ctx);
        });
    }
    fn settings_contents(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
        toolbar(ui, |ui, rect| {
            let tabs = [
                ("General", "gearshape"),
                ("Advanced", "sliders"),
                ("About", "infoCircle"),
            ];
            let total = 72.0 * 3.0 + 8.0;
            let mut x = rect.center().x - total / 2.0;
            for (i, (title, icon)) in tabs.into_iter().enumerate() {
                let tab = Rect::from_min_size(pos2(x, rect.center().y - 22.0), vec2(72.0, 44.0));
                let response = ui.interact(tab, ui.id().with(("tab", i)), Sense::click());
                let selected = self.settings_tab == i;
                if selected {
                    ui.painter().rect_filled(tab, 8, ink(0.1));
                } else if response.hovered() {
                    ui.painter().rect_filled(tab, 8, ink(0.05));
                }
                icons::paint(
                    ui,
                    Rect::from_min_size(tab.min + vec2(0.0, 4.0), vec2(72.0, 22.0)),
                    icon,
                    19.0,
                    W::Regular,
                    if selected {
                        theme::ACCENT
                    } else {
                        theme::secondary()
                    },
                );
                let g = theme::line(
                    ui,
                    title,
                    11.0,
                    W::Regular,
                    if selected {
                        theme::ink(1.0)
                    } else {
                        theme::secondary()
                    },
                    None,
                );
                ui.painter().galley(
                    pos2(tab.center().x - g.size().x / 2.0, tab.top() + 28.0),
                    g,
                    theme::ink(1.0),
                );
                if response.clicked() {
                    self.settings_tab = i;
                }
                x += 76.0;
            }
        });
        match self.settings_tab {
            0 => scroll(ui, "general", |ui| self.general(ui)),
            1 => self.advanced(ui),
            _ => scroll(ui, "about", |ui| self.about(ui)),
        }
    }

    fn general(&mut self, ui: &mut Ui) {
        form(ui, |ui| {
            let folder = self.engine.folder();
            let shown = display_path(&folder, &self.engine.paths.home);
            let mut choose = false;
            let settings = &mut self.engine.settings;
            w::section(ui, None, None, |ui, form| {
                w::row(ui, form, "Appearance", None, |ui| {
                    w::segmented(
                        ui,
                        &mut settings.appearance,
                        &[
                            ("system".into(), "System"),
                            ("light".into(), "Light"),
                            ("dark".into(), "Dark"),
                        ],
                    );
                });
            });
            w::section(ui, None, None, |ui, form| {
                w::row(ui, form, "Save downloads to", None, |ui| {
                    choose = w::bezel_button(ui, "Change…", false, true).clicked();
                    let g = theme::line(
                        ui,
                        &shown,
                        13.0,
                        W::Regular,
                        theme::secondary(),
                        Some(220.0),
                    );
                    let (r, response) = ui.allocate_exact_size(g.size(), Sense::hover());
                    ui.painter().galley(r.min, g, theme::ink(1.0));
                    response.on_hover_text(folder.display().to_string());
                });
                let label = format!("Download {} at a time", settings.max_concurrent);
                w::row(ui, form, &label, None, |ui| {
                    let mut n = settings.max_concurrent as u32;
                    if w::stepper(ui, &mut n, 1, 10, 1) {
                        settings.max_concurrent = n as usize;
                    }
                });
            });
            w::section(ui, None, None, |ui, form| {
                w::switch_row(
                    ui,
                    form,
                    "Fill in links from the clipboard",
                    &mut settings.clipboard_autofill,
                );
                w::switch_row(
                    ui,
                    form,
                    "Notify me when downloads finish or fail",
                    &mut settings.notifications,
                );
                w::switch_row(
                    ui,
                    form,
                    &format!("Show Purrloader in the {}", tray_name()),
                    &mut settings.show_tray_icon,
                );
            });
            if choose {
                self.choose_folder();
            }
        });
    }

    fn archive_count(&mut self) -> usize {
        if self
            .archive_count
            .0
            .is_none_or(|t| t.elapsed() > Duration::from_secs(2))
        {
            let count =
                std::fs::read_to_string(self.engine.paths.root.join("download-archive.txt"))
                    .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                    .unwrap_or(0);
            self.archive_count = (Some(Instant::now()), count);
        }
        self.archive_count.1
    }

    fn advanced(&mut self, ui: &mut Ui) {
        Frame::new()
            .inner_margin(Margin {
                left: 20,
                right: 20,
                top: 16,
                bottom: 0,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.label(theme::rich(
                            "These settings apply to every download.",
                            13.0,
                            W::Semibold,
                            theme::ink(1.0),
                        ));
                        ui.label(theme::rich(
                            "You don't need any of them for everyday downloads.",
                            12.0,
                            W::Regular,
                            theme::secondary(),
                        ));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if w::bezel_button(ui, "Reset to Defaults…", false, true).clicked() {
                            self.confirm_reset = true;
                        }
                        let setups = w::bezel_menu_button(ui, "Saved Setups");
                        w::menu(&setups, RectAlign::BOTTOM_END, |ui| {
                            for preset in self.engine.presets.clone() {
                                if w::menu_item(ui, &preset.name, false, None, true) {
                                    self.options = preset.options.clone();
                                }
                            }
                            if !self.engine.presets.is_empty() {
                                w::menu_separator(ui);
                            }
                            if w::menu_item(ui, "Save Current Settings…", false, None, true) {
                                self.saving_setup = Some(String::new());
                            }
                            if w::menu_item(ui, "Manage Saved Setups…", false, None, true) {
                                self.managing_setups = true;
                            }
                        });
                    });
                });
            });
        ui.add_space(14.0);
        ui.vertical_centered(|ui| {
            let names: Vec<(usize, &str)> =
                settings_ui::SECTIONS.iter().copied().enumerate().collect();
            let width: f32 = settings_ui::SECTIONS
                .iter()
                .map(|s| {
                    theme::line(ui, s, 13.0, W::Medium, theme::ink(1.0), None)
                        .size()
                        .x
                        + 30.0
                })
                .sum::<f32>()
                + 4.0;
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                w::pill_segment(ui, "section", &mut self.advanced_section, &names, 30.0);
            });
        });
        let archive_count = self.archive_count();
        let env = self.engine.tools.environment(&self.engine.paths);
        let url = support::web_links(&self.url)
            .first()
            .cloned()
            .unwrap_or_else(|| "https://example.com/video".into());
        let command = match env.download(&self.options, &url, &self.engine.folder(), None, None) {
            Ok(mut args) => {
                args.insert(0, "yt-dlp".into());
                support::command_line(&args, cfg!(windows))
            }
            Err(e) => e.to_string(),
        };
        let preview = purrloader::media::template_preview(
            &self.options.output.template,
            self.engine.info.as_ref(),
            &self.options,
            cfg!(windows),
        );
        let info = self.engine.info.clone();
        let mut cx = settings_ui::Context {
            options: &mut self.options,
            info: info.as_ref(),
            ticked: self.selected.len(),
            archive_count,
            command,
            preview,
            actions: Vec::new(),
        };
        let section = self.advanced_section;
        ui.add_space(4.0);
        scroll(ui, ("advanced", section), |ui| {
            settings_ui::show(ui, section, &mut cx)
        });
        let actions = std::mem::take(&mut cx.actions);
        for action in actions {
            match action {
                Action::ShowArchive => {
                    let archive = self.engine.paths.root.join("download-archive.txt");
                    platform::reveal(Some(&archive), &self.engine.paths.root);
                }
                Action::ClearArchive => self.confirm_clear_archive = true,
                Action::ChooseCookiesFile => {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Cookies", &["txt"])
                        .pick_file()
                    {
                        self.options.network.cookies_file = file.display().to_string();
                    }
                }
                Action::Copy(text) => ui.ctx().copy_text(text),
                Action::Open(url) => {
                    let _ = open::that_detached(url);
                }
            }
        }
    }

    fn about(&mut self, ui: &mut Ui) {
        form(ui, |ui| {
            let mut check = false;
            let release_busy = self.engine.release_busy;
            w::section(ui, None, None, |ui, form| {
                w::stack_row(ui, form, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        ui.add(self.icon_image().fit_to_exact_size(vec2(64.0, 64.0)));
                        ui.vertical(|ui| {
                            ui.add_space(7.0);
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.label(theme::rich(
                                "Purrloader",
                                17.0,
                                W::Semibold,
                                theme::ink(1.0),
                            ));
                            ui.label(theme::rich(
                                "The quiet way to save videos.",
                                13.0,
                                W::Regular,
                                theme::secondary(),
                            ));
                            ui.label(theme::rich(
                                format!("Version {}", purrloader::VERSION),
                                11.0,
                                W::Regular,
                                theme::tertiary(),
                            ));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            check = w::bezel_button(ui, "Check for Updates", false, !release_busy)
                                .clicked();
                        });
                    });
                    if let Some(version) = &self.engine.settings.latest_app_version {
                        let text = if support::newer(version, purrloader::VERSION) {
                            format!("Purrloader {version} is available.")
                        } else {
                            "Purrloader is up to date.".into()
                        };
                        ui.label(theme::rich(text, 11.0, W::Regular, theme::secondary()));
                    }
                });
            });
            if check && !self.smoke {
                self.engine.check_release(true);
            }
            let busy = self.engine.tools_busy;
            let versions = self.engine.tools.versions.clone();
            let channel_before = self.engine.settings.ytdlp_channel.clone();
            let mut update = false;
            let mut reinstall = false;
            let mut show = false;
            let message = self.engine.tools_message.clone();
            let error = self.engine.tools_error.clone();
            let settings = &mut self.engine.settings;
            w::section(
                ui,
                Some("Helper tools"),
                Some(
                    "Purrloader keeps yt-dlp up to date by itself. Nightly gets fixes for broken sites a few days before stable.",
                ),
                |ui, form| {
                    for tool in TOOLS {
                        w::row(ui, form, tool, None, |ui| {
                            if busy {
                                w::spinner(ui, 16.0);
                            } else {
                                let (text, color) = match versions.get(tool) {
                                    Some(v) => (v.as_str(), theme::secondary()),
                                    None => ("Not installed", theme::RED),
                                };
                                ui.add(
                                    egui::Label::new(theme::rich(text, 13.0, W::Regular, color))
                                        .selectable(true),
                                );
                            }
                        });
                    }
                    w::switch_row(
                        ui,
                        form,
                        "Keep yt-dlp up to date automatically",
                        &mut settings.auto_update_ytdlp,
                    );
                    w::picker_row(
                        ui,
                        form,
                        "yt-dlp channel",
                        &mut settings.ytdlp_channel,
                        &[("stable".into(), "Stable"), ("nightly".into(), "Nightly")],
                    );
                    w::stack_row(ui, form, |ui| {
                        ui.horizontal(|ui| {
                            update = w::bezel_button(
                                ui,
                                if busy { "Updating…" } else { "Update Now" },
                                false,
                                !busy,
                            )
                            .clicked();
                            reinstall =
                                w::bezel_button(ui, "Reinstall Tools", false, !busy).clicked();
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                show =
                                    w::bezel_button(ui, "Show Tools Folder", false, true).clicked();
                            });
                        });
                        let log = error.unwrap_or(message);
                        if !log.is_empty() {
                            ui.add(
                                egui::Label::new(theme::rich(
                                    log,
                                    11.0,
                                    W::Regular,
                                    theme::secondary(),
                                ))
                                .selectable(true)
                                .wrap(),
                            );
                        }
                    });
                },
            );
            if !self.smoke {
                if self.engine.settings.ytdlp_channel != channel_before || update {
                    self.engine.setup_tools(true, false);
                }
                if reinstall {
                    self.engine.setup_tools(false, true);
                }
            }
            if show {
                let _ = open::that_detached(&self.engine.paths.bin);
            }
            w::section(
                ui,
                None,
                Some(
                    "Built on yt-dlp, FFmpeg and Deno. Sponsor segments come from the SponsorBlock community.",
                ),
                |ui, form| {
                    for (title, url) in [
                        (
                            "Purrloader on GitHub",
                            "https://github.com/BLCNYY/Purrloader",
                        ),
                        (
                            "License (MIT)",
                            "https://github.com/BLCNYY/Purrloader/blob/main/LICENSE",
                        ),
                        (
                            "Third-party notices",
                            "https://github.com/BLCNYY/Purrloader/blob/main/THIRD_PARTY_NOTICES.md",
                        ),
                    ] {
                        w::stack_row(ui, form, |ui| {
                            if w::link_button(ui, title).clicked() {
                                let _ = open::that_detached(url);
                            }
                        });
                    }
                },
            );
        });
    }

    fn settings_sheets(&mut self, ctx: &egui::Context) {
        if self.confirm_reset {
            match confirm_sheet(
                ctx,
                "reset",
                &self.icon_image(),
                "Reset every advanced setting?",
                "Video, audio, subtitle, network and file name settings go back to how Purrloader came. Saved setups are kept.",
                "Reset to Defaults",
                "Cancel",
            ) {
                Some(true) => {
                    self.options = Options::default();
                    self.part = false;
                    self.confirm_reset = false;
                }
                Some(false) => self.confirm_reset = false,
                None => (),
            }
        }
        if self.confirm_clear_archive {
            match confirm_sheet(
                ctx,
                "archive",
                &self.icon_image(),
                "Clear the download archive?",
                "Items you already downloaded will be downloaded again next time.",
                "Clear Archive",
                "Cancel",
            ) {
                Some(true) => {
                    let _ =
                        std::fs::remove_file(self.engine.paths.root.join("download-archive.txt"));
                    self.archive_count = (None, 0);
                    self.confirm_clear_archive = false;
                }
                Some(false) => self.confirm_clear_archive = false,
                None => (),
            }
        }
        if let Some(mut name) = self.saving_setup.take() {
            let mut result = None;
            let close = w::sheet(ctx, "save-setup", |ui| {
                Frame::new().inner_margin(Margin::same(20)).show(ui, |ui| {
                    ui.set_width(260.0);
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        ui.add(self.icon_image().fit_to_exact_size(vec2(56.0, 56.0)));
                        ui.label(theme::rich("Save Setup", 13.0, W::Bold, theme::ink(1.0)));
                        ui.label(theme::rich(
                            "Saves every setting, including the quality and format in the main window.",
                            11.0,
                            W::Regular,
                            theme::secondary(),
                        ));
                        let response = w::text_input(ui, &mut name, "Name", 260.0);
                        if !response.has_focus() && !response.lost_focus() && ui.memory(|m| m.focused().is_none()) {
                            response.request_focus();
                        }
                        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if w::bezel_button(ui, "Cancel", false, true).clicked() {
                                result = Some(false);
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if w::bezel_button(ui, "Save", true, !name.trim().is_empty()).clicked() || enter {
                                    result = Some(true);
                                }
                            });
                        });
                    });
                });
            });
            match result {
                Some(true) if !name.trim().is_empty() => {
                    let mut options = self.options.clone();
                    options.format.exact_format.clear();
                    let name = name.trim().to_owned();
                    if let Some(preset) = self.engine.presets.iter_mut().find(|p| p.name == name) {
                        preset.options = options;
                    } else {
                        self.engine.presets.push(Preset::new(&name, options));
                    }
                    self.engine.persist();
                }
                Some(_) => (),
                None if close => (),
                None => self.saving_setup = Some(name),
            }
        }
        if self.managing_setups {
            let mut done = false;
            let mut dirty = false;
            let close = w::sheet(ctx, "manage-setups", |ui| {
                Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                    ui.set_width(408.0);
                    ui.spacing_mut().item_spacing.y = 10.0;
                    ui.label(theme::rich("Saved Setups", 13.0, W::Bold, theme::ink(1.0)));
                    Frame::new()
                        .fill(theme::black(0.15))
                        .stroke(Stroke::new(1.0_f32, ink(0.06)))
                        .corner_radius(6)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("presets")
                                .min_scrolled_height(200.0)
                                .max_height(200.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    for (i, preset) in self.engine.presets.iter().enumerate() {
                                        let (rect, response) = ui.allocate_exact_size(
                                            vec2(ui.available_width(), 24.0),
                                            Sense::click(),
                                        );
                                        let selected =
                                            self.manage_selection.as_deref() == Some(&preset.id);
                                        let fill = if selected {
                                            theme::ACCENT
                                        } else if i % 2 == 1 {
                                            ink(0.03)
                                        } else {
                                            Color32::TRANSPARENT
                                        };
                                        ui.painter().rect_filled(rect, 0, fill);
                                        let g = theme::line(
                                            ui,
                                            &preset.name,
                                            13.0,
                                            W::Regular,
                                            if selected {
                                                Color32::WHITE
                                            } else {
                                                theme::ink(1.0)
                                            },
                                            Some(rect.width() - 16.0),
                                        );
                                        ui.painter().galley(
                                            pos2(
                                                rect.left() + 8.0,
                                                rect.center().y - g.size().y / 2.0,
                                            ),
                                            g,
                                            theme::ink(1.0),
                                        );
                                        if response.clicked() {
                                            self.manage_selection = Some(preset.id.clone());
                                            self.manage_name = preset.name.clone();
                                        }
                                    }
                                });
                        });
                    let index = self
                        .manage_selection
                        .as_ref()
                        .and_then(|id| self.engine.presets.iter().position(|p| &p.id == id));
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if w::bezel_button(ui, "Delete", false, index.is_some()).clicked()
                                && let Some(i) = index
                            {
                                self.engine.presets.remove(i);
                                self.manage_selection = None;
                                self.manage_name.clear();
                                dirty = true;
                            }
                            if w::bezel_button(
                                ui,
                                "Rename",
                                false,
                                index.is_some() && !self.manage_name.trim().is_empty(),
                            )
                            .clicked()
                                && let Some(i) = index
                            {
                                self.engine.presets[i].name = self.manage_name.trim().to_owned();
                                dirty = true;
                            }
                            w::text_input(ui, &mut self.manage_name, "Name", ui.available_width());
                        });
                    });
                    ui.horizontal(|ui| {
                        if w::bezel_button(ui, "Move Up", false, index.is_some_and(|i| i > 0))
                            .clicked()
                            && let Some(i) = index
                        {
                            self.engine.presets.swap(i, i - 1);
                            dirty = true;
                        }
                        if w::bezel_button(
                            ui,
                            "Move Down",
                            false,
                            index.is_some_and(|i| i + 1 < self.engine.presets.len()),
                        )
                        .clicked()
                            && let Some(i) = index
                        {
                            self.engine.presets.swap(i, i + 1);
                            dirty = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            done = w::bezel_button(ui, "Done", true, true).clicked();
                            if w::bezel_button(ui, "Restore Built-ins", false, true).clicked() {
                                for preset in Preset::defaults() {
                                    if !self.engine.presets.iter().any(|p| p.name == preset.name) {
                                        self.engine.presets.push(preset);
                                    }
                                }
                                dirty = true;
                            }
                        });
                    });
                });
            });
            if dirty {
                self.engine.persist();
            }
            if done || close {
                self.managing_setups = false;
            }
        }
    }

    // MARK: - History

    fn history_window(&mut self, ctx: &egui::Context) {
        let builder = egui::ViewportBuilder::default()
            .with_title("History")
            .with_app_id("purrloader")
            .with_icon(self.icon.clone())
            .with_inner_size([760.0, 560.0])
            .with_min_inner_size([560.0, 360.0])
            .with_transparent(theme::blurred());
        ctx.show_viewport_immediate(history_id(), builder, |ctx, class| {
            title_bar_theme(ctx, &self.engine.settings.appearance.clone());
            self.backdrop(1, "History");
            if class == egui::ViewportClass::Embedded {
                let mut open = true;
                egui::Window::new("History")
                    .open(&mut open)
                    .show(ctx, |ui| self.history_contents(ui));
                self.history_open = open;
                return;
            }
            egui::CentralPanel::default()
                .frame(Frame::new().fill(theme::window_fill()))
                .show(ctx, |ui| self.history_contents(ui));
            if self.confirm_clear_history {
                match confirm_sheet(
                    ctx,
                    "clear-history",
                    &self.icon_image(),
                    "Clear all download history?",
                    "Your downloaded files stay where they are.",
                    "Clear History",
                    "Cancel",
                ) {
                    Some(true) => {
                        self.engine.history.clear();
                        self.history_selected.clear();
                        self.engine.persist();
                        self.confirm_clear_history = false;
                    }
                    Some(false) => self.confirm_clear_history = false,
                    None => (),
                }
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.history_open = false;
            }
            self.capture(ctx);
        });
    }

    fn redownload(&mut self, ids: &[String]) {
        for entry in self
            .engine
            .history
            .clone()
            .iter()
            .filter(|e| ids.contains(&e.id))
        {
            let mut job = Job::new(
                entry.url.clone(),
                entry.options.clone(),
                entry.folder.clone(),
            );
            job.title = entry.title.clone();
            job.thumbnail = entry.thumbnail.clone();
            self.engine.enqueue(job);
        }
    }

    fn history_contents(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
        let has_history = !self.engine.history.is_empty();
        let selection: Vec<String> = self.history_selected.iter().cloned().collect();
        toolbar(ui, |ui, rect| {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(vec2(20.0, 0.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            child.label(theme::rich("History", 15.0, W::Bold, theme::ink(1.0)));
            child.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                search_field(ui, &mut self.history_search);
                if toolbar_button(ui, "trash", "Clear History", has_history).clicked() {
                    self.confirm_clear_history = true;
                }
                if toolbar_button(
                    ui,
                    "arrowDownCircle",
                    "Download Again",
                    !selection.is_empty(),
                )
                .clicked()
                {
                    self.redownload(&selection);
                }
            });
        });
        let query = self.history_search.trim().to_lowercase();
        let entries: Vec<_> = self
            .engine
            .history
            .iter()
            .filter(|e| {
                query.is_empty()
                    || format!("{} {}", e.title, e.url)
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();
        let width = ui.available_width();
        let columns = [width - 360.0, 170.0, 190.0];
        let (head, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
        ui.painter().rect_filled(head, 0, theme::table_head());
        ui.painter().line_segment(
            [
                pos2(head.left(), head.bottom() - 0.5),
                pos2(head.right(), head.bottom() - 0.5),
            ],
            Stroke::new(1.0_f32, ink(0.08)),
        );
        let mut x = head.left();
        for (title, w) in ["Title", "Date", "Folder"].iter().zip(columns) {
            let g = theme::line(ui, title, 12.0, W::Medium, theme::secondary(), None);
            ui.painter().galley(
                pos2(x + 10.0, head.center().y - g.size().y / 2.0),
                g,
                theme::ink(1.0),
            );
            x += w;
        }
        let empty = ui.available_rect_before_wrap();
        egui::ScrollArea::vertical()
            .id_salt("history")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, entry) in entries.iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
                    let selected = self.history_selected.contains(&entry.id);
                    let fill = if selected {
                        theme::ACCENT
                    } else if index % 2 == 1 {
                        ink(0.03)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 0, fill);
                    let thumb = Rect::from_min_size(
                        pos2(rect.left() + 10.0, rect.center().y - 13.5),
                        vec2(48.0, 27.0),
                    );
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(thumb));
                    w::thumbnail(&mut child, entry.thumbnail.as_deref(), 48.0, 1.0);
                    let cells = [
                        (
                            entry.title.clone(),
                            if selected {
                                Color32::WHITE
                            } else {
                                theme::ink(1.0)
                            },
                            rect.left() + 66.0,
                            columns[0] - 76.0,
                        ),
                        (
                            history_date(&entry.date),
                            if selected {
                                Color32::WHITE
                            } else {
                                theme::secondary()
                            },
                            rect.left() + columns[0] + 10.0,
                            columns[1] - 20.0,
                        ),
                        (
                            display_path(&entry.folder, &self.engine.paths.home),
                            if selected {
                                Color32::WHITE
                            } else {
                                theme::secondary()
                            },
                            rect.left() + columns[0] + columns[1] + 10.0,
                            columns[2] - 20.0,
                        ),
                    ];
                    for (text, color, x, max) in cells {
                        let g = theme::line(ui, &text, 13.0, W::Regular, color, Some(max));
                        ui.painter().galley(
                            pos2(x, rect.center().y - g.size().y / 2.0),
                            g,
                            theme::ink(1.0),
                        );
                    }
                    if response.clicked() {
                        let (command, shift) =
                            ui.input(|i| (i.modifiers.command, i.modifiers.shift));
                        if shift && let Some(anchor) = self.history_anchor {
                            let (a, b) = (anchor.min(index), anchor.max(index));
                            self.history_selected =
                                entries[a..=b].iter().map(|e| e.id.clone()).collect();
                        } else if command {
                            if !self.history_selected.remove(&entry.id) {
                                self.history_selected.insert(entry.id.clone());
                            }
                            self.history_anchor = Some(index);
                        } else {
                            self.history_selected = [entry.id.clone()].into();
                            self.history_anchor = Some(index);
                        }
                    }
                    if response.double_clicked() {
                        platform::reveal(entry.files.first().map(|p| p.as_path()), &entry.folder);
                    }
                    if response.secondary_clicked() && !selected {
                        self.history_selected = [entry.id.clone()].into();
                    }
                    egui::Popup::context_menu(&response)
                        .frame(w::menu_frame())
                        .layout(Layout::top_down_justified(Align::LEFT))
                        .style(|s: &mut egui::Style| s.spacing.item_spacing = egui::Vec2::ZERO)
                        .show(|ui| {
                            let ids: Vec<String> = self.history_selected.iter().cloned().collect();
                            if w::menu_item(ui, "Download Again", false, None, true) {
                                self.redownload(&ids);
                            }
                            if w::menu_item(
                                ui,
                                &format!("Show in {}", file_manager()),
                                false,
                                None,
                                true,
                            ) {
                                platform::reveal(
                                    entry.files.first().map(|p| p.as_path()),
                                    &entry.folder,
                                );
                            }
                            if w::menu_item(ui, "Copy Link", false, None, true) {
                                let links: Vec<String> = self
                                    .engine
                                    .history
                                    .iter()
                                    .filter(|e| ids.contains(&e.id))
                                    .map(|e| e.url.clone())
                                    .collect();
                                ui.ctx().copy_text(links.join("\n"));
                            }
                            w::menu_separator(ui);
                            if w::menu_item(ui, "Remove from History", false, None, true) {
                                self.engine.history.retain(|e| !ids.contains(&e.id));
                                self.history_selected.clear();
                                self.engine.persist();
                            }
                        });
                }
            });
        let unavailable = if !has_history {
            Some((
                "clock",
                "No History".to_owned(),
                "Finished downloads show up here.",
            ))
        } else if entries.is_empty() {
            Some((
                "magnifyingglass",
                format!("No Results for “{}”", self.history_search.trim()),
                "Check the spelling or try a new search.",
            ))
        } else {
            None
        };
        if let Some((icon, title, text)) = unavailable {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(empty)
                    .layout(Layout::top_down(Align::Center)),
            );
            child.add_space((empty.height() / 2.0 - 50.0).max(0.0));
            child.spacing_mut().item_spacing.y = 6.0;
            icons::show(&mut child, icon, 34.0, W::Regular, theme::tertiary());
            child.label(theme::rich(title, 17.0, W::Bold, theme::ink(1.0)));
            child.label(theme::rich(text, 13.0, W::Regular, theme::secondary()));
        }
    }

    /// Gives a Settings (0) or History (1) window the blurred backdrop once it exists, on Windows 11.
    fn backdrop(&mut self, window: usize, title: &str) {
        if !theme::blurred() || self.backdrop_tries[window] >= 60 {
            return;
        }
        self.backdrop_tries[window] += 1;
        #[cfg(windows)]
        if platform::backdrop::apply_to_title(title) {
            self.backdrop_tries[window] = u8::MAX;
        }
        #[cfg(not(windows))]
        let _ = title;
    }

    /// Smoke tests save a picture of the window they were asked for.
    fn capture(&mut self, ctx: &egui::Context) {
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event
                && let Some(path) = self.screenshot.take()
            {
                let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Err(error) = image::save_buffer(
                    &path,
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Couldn't save screenshot: {error}");
                }
                if self.smoke {
                    self.quit = true;
                    ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
                }
            }
        }
    }
}

// MARK: - Shared pieces

/// The top-left corner that puts a window of `size` in the middle of the main window's screen. egui doesn't say
/// where that screen starts, so on a second screen (main window outside the first one) it centres on the main
/// window instead.
fn centered(ctx: &egui::Context, size: [f32; 2]) -> Option<egui::Pos2> {
    let (outer, monitor) = ctx.input_for(ViewportId::ROOT, |i| {
        (i.viewport().outer_rect, i.viewport().monitor_size)
    });
    let size = vec2(size[0], size[1]);
    let screen = monitor.map(|m| Rect::from_min_size(egui::Pos2::ZERO, m));
    let middle = match (screen, outer) {
        (Some(screen), Some(outer)) if screen.contains(outer.center()) => screen.center(),
        (Some(screen), None) => screen.center(),
        (_, Some(outer)) => outer.center(),
        _ => return None,
    };
    Some((middle - size / 2.0).round())
}

/// Gives this window's title bar the chosen appearance; with "system" it follows the system's.
fn title_bar_theme(ctx: &egui::Context, appearance: &str) {
    let theme = match appearance {
        "light" => egui::SystemTheme::Light,
        "dark" => egui::SystemTheme::Dark,
        _ => egui::SystemTheme::SystemDefault,
    };
    let id = egui::Id::new(("title-bar-theme", ctx.viewport_id()));
    let key = format!("{theme:?}");
    if ctx.data(|d| d.get_temp::<String>(id)).as_deref() != Some(&key) {
        ctx.data_mut(|d| d.insert_temp(id, key));
        ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(theme));
    }
}

/// The list behind "Use Browser Sign-In": pick a browser and the download retries with its sign-in.
fn browser_picker(ui: &mut Ui, current: &str) -> Option<String> {
    let mut chosen = None;
    Frame::new().inner_margin(Margin::same(14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.label(theme::rich("Sign in with my browser", 13.0, W::Semibold, theme::ink(1.0)));
        theme::paragraph(
            ui,
            "Sign in to the site in one of these browsers first. Purrloader then uses that sign-in.",
            11.0,
            W::Regular,
            theme::secondary(),
            0,
        );
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for browser in settings_ui::cookie_browsers() {
                let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    7,
                    if response.hovered() { ink(0.1) } else { ink(0.05) },
                );
                let g = theme::line(ui, settings_ui::browser_title(browser), 13.0, W::Regular, theme::text90(), None);
                ui.painter().galley(pos2(rect.left() + 10.0, rect.center().y - g.size().y / 2.0), g, theme::ink(1.0));
                if *browser == current {
                    icons::paint(
                        ui,
                        Rect::from_min_size(pos2(rect.right() - 24.0, rect.top()), vec2(12.0, 28.0)),
                        "checkmark",
                        12.0,
                        W::Semibold,
                        theme::secondary(),
                    );
                }
                if response.clicked() {
                    chosen = Some((*browser).to_owned());
                }
            }
        });
        theme::paragraph(ui, settings_ui::browser_note(), 11.0, W::Regular, theme::tertiary(), 0);
    });
    chosen
}

/// An alert-style sheet. Returns Some(true) for the main button, Some(false) to cancel.
fn confirm_sheet(
    ctx: &egui::Context,
    id: &str,
    icon: &egui::Image<'static>,
    title: &str,
    detail: &str,
    ok: &str,
    cancel: &str,
) -> Option<bool> {
    let mut result = None;
    let close = w::sheet(ctx, id, |ui| {
        Frame::new().inner_margin(Margin::same(20)).show(ui, |ui| {
            ui.set_width(260.0);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.add(icon.clone().fit_to_exact_size(vec2(56.0, 56.0)));
                ui.add(
                    egui::Label::new(theme::rich(title, 13.0, W::Bold, theme::ink(1.0)))
                        .halign(Align::Center)
                        .wrap(),
                );
                ui.add(
                    egui::Label::new(theme::rich(detail, 11.0, W::Regular, theme::secondary()))
                        .halign(Align::Center)
                        .wrap(),
                );
                ui.add_space(4.0);
                ui.vertical_centered_justified(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    if wide_bezel(ui, ok, true).clicked() {
                        result = Some(true);
                    }
                    if wide_bezel(ui, cancel, false).clicked() {
                        result = Some(false);
                    }
                });
            });
        });
    });
    if close && result.is_none() {
        result = Some(false);
    }
    result
}
fn wide_bezel(ui: &mut Ui, text: &str, default: bool) -> egui::Response {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(vec2(width, 22.0), Layout::top_down(Align::Center), |ui| {
        let r = w::bezel_button(ui, text, default, true);
        // Stretch the hit area like a full-width alert button.
        r
    })
    .inner
}

/// A window toolbar: 52 points, a hairline below.
fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui, Rect)) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, ink(0.02));
    ui.painter().line_segment(
        [
            pos2(rect.left(), rect.bottom() - 0.5),
            pos2(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(
            1.0_f32,
            theme::black(if theme::dark() { 0.35 } else { 0.12 }),
        ),
    );
    add(ui, rect);
}
fn toolbar_button(ui: &mut Ui, icon: &str, tip: &str, enabled: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(32.0, 28.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 8, ink(0.08));
    }
    icons::paint(
        ui,
        rect,
        icon,
        17.0,
        W::Regular,
        if enabled {
            theme::text90()
        } else {
            theme::tertiary()
        },
    );
    response.on_hover_text(tip)
}
fn search_field(ui: &mut Ui, text: &mut String) {
    let (rect, _) = ui.allocate_exact_size(vec2(180.0, 28.0), Sense::hover());
    ui.painter().rect_filled(rect, 8, ink(0.07));
    ui.painter().rect_stroke(
        rect,
        8,
        Stroke::new(0.5_f32, ink(0.12)),
        egui::StrokeKind::Inside,
    );
    icons::paint(
        ui,
        Rect::from_min_size(pos2(rect.left() + 8.0, rect.top()), vec2(13.0, 28.0)),
        "magnifyingglass",
        13.0,
        W::Medium,
        theme::tertiary(),
    );
    let inner = Rect::from_min_max(
        pos2(rect.left() + 26.0, rect.top()),
        pos2(rect.right() - 6.0, rect.bottom()),
    );
    let response = w::place(
        ui,
        inner,
        egui::TextEdit::singleline(text)
            .frame(false)
            .margin(Margin::ZERO)
            .font(theme::font(13.0, W::Regular))
            .text_color(theme::ink(1.0))
            .vertical_align(Align::Center)
            .hint_text(theme::rich(
                "Search history",
                13.0,
                W::Regular,
                theme::tertiary(),
            )),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            11,
            Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(10, 132, 255, 140)),
            egui::StrokeKind::Inside,
        );
    }
}
fn scroll(ui: &mut Ui, id: impl std::hash::Hash, add: impl FnOnce(&mut Ui)) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}
/// The grouped form's padding and spacing between sections.
fn form(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .inner_margin(Margin {
            left: 20,
            right: 20,
            top: 20,
            bottom: 24,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 20.0;
            add(ui);
        });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        let appearance = self.engine.settings.appearance.clone();
        title_bar_theme(ctx, &appearance);
        let dark = match appearance.as_str() {
            "light" => false,
            "dark" => true,
            _ => ctx.system_theme().is_none_or(|t| t == egui::Theme::Dark),
        };
        if self.frames == 1 || dark != theme::dark() {
            theme::apply(ctx, dark);
        }
        self.engine.poll();
        let settings_before = self.engine.settings.clone();
        let options_before = self.options.clone();
        self.tray(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.tray.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else {
                self.request_quit(ctx);
            }
        }
        let focus = ctx.input(|i| i.focused);
        if focus && !self.focus && self.engine.settings.clipboard_autofill && !self.smoke {
            self.clipboard();
        }
        self.focus = focus;
        for file in ctx.input(|i| i.raw.dropped_files.clone()) {
            let content = file
                .bytes
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .or_else(|| {
                    file.path.and_then(|p| {
                        // Link files and lists only, with a small bound; dropping media doesn't read it into memory.
                        let ext = p.extension()?.to_str()?.to_lowercase();
                        if !["url", "webloc", "txt"].contains(&ext.as_str())
                            || std::fs::metadata(&p).ok()?.len() > 65536
                        {
                            return None;
                        }
                        std::fs::read_to_string(p).ok()
                    })
                });
            if let Some(content) = content {
                let links = support::web_links(&content);
                if !links.is_empty() {
                    self.url = links.join(" ");
                }
            }
        }
        self.shortcuts(ctx);
        if let Some(window) = self.smoke_window {
            if self.frames == 1 {
                let size = if window == settings_id() {
                    SETTINGS_SIZES[self.settings_tab.min(2)]
                } else {
                    [760.0, 560.0]
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
            }
            egui::CentralPanel::default()
                .frame(Frame::new().fill(theme::window_fill()))
                .show(ctx, |ui| {
                    if window == settings_id() {
                        self.settings_contents(ui)
                    } else {
                        self.history_contents(ui)
                    }
                });
        } else {
            egui::CentralPanel::default()
                .frame(Frame::new().fill(theme::window_fill()))
                .show(ctx, |ui| self.main_window(ui));
        }
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop"),
            ));
            let rect = ctx.content_rect().shrink(9.0);
            painter.rect_stroke(
                rect,
                16,
                Stroke::new(2.0_f32, ink(0.6)),
                egui::StrokeKind::Inside,
            );
        }
        self.details_sheet(ctx);
        self.welcome_sheet(ctx);
        self.quit_sheet(ctx);
        if self.settings_open {
            self.settings_window(ctx);
        } else {
            self.settings_shown = None;
            self.settings_position = None;
            self.backdrop_tries[0] = 0;
        }
        if self.history_open {
            self.history_window(ctx);
        } else {
            self.backdrop_tries[1] = 0;
        }
        self.input_changes();
        if settings_before != self.engine.settings || options_before != self.options {
            self.engine.settings.last_options = Some(self.options.clone());
            self.save_due = Some(Instant::now() + Duration::from_millis(500));
        }
        if self.save_due.is_some_and(|due| Instant::now() >= due) {
            self.save_due = None;
            self.engine.persist();
        }
        if self.smoke && self.frames >= 8 {
            if self.screenshot.is_some() && !self.screenshot_requested {
                ctx.send_viewport_cmd_to(
                    self.screenshot_target,
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
                self.screenshot_requested = true;
            } else if self.screenshot.is_none() {
                self.quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.capture(ctx);
        // Also checks second-instance activation while the window is hidden.
        ctx.request_repaint_after(if self.smoke {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(500)
        });
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if theme::blurred() {
            [0.0; 4]
        } else {
            theme::bg().to_normalized_gamma_f32()
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine.settings.last_options = Some(self.options.clone());
        self.engine.shutdown();
    }
}
