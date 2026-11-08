//! UI-independent queue. Worker threads send events; only this owner mutates persisted state.
use crate::{
    error::FriendlyError,
    media::Media,
    options::Options,
    process,
    progress::Progress,
    store::{self, History, Paths, Preset, Settings},
    tools::{self, Tools},
};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[default]
    Queued,
    Starting,
    Downloading,
    Processing,
    Paused,
    Finished,
    Failed,
    Cancelled,
}
impl Status {
    pub fn active(self) -> bool {
        matches!(self, Self::Starting | Self::Downloading | Self::Processing)
    }
    pub fn recent(self) -> bool {
        matches!(self, Self::Finished | Self::Cancelled)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Waiting",
            Self::Starting => "Starting",
            Self::Downloading => "Downloading",
            Self::Processing => "Finishing",
            Self::Paused => "Paused",
            Self::Finished => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    #[serde(default = "id")]
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub options: Options,
    pub folder: PathBuf,
    #[serde(default)]
    pub playlist_items: Option<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub error: Option<FriendlyError>,
    #[serde(default)]
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub total_bytes: f64,
    #[serde(skip)]
    pub progress: Progress,
    #[serde(skip)]
    pub log: Vec<String>,
    #[serde(skip)]
    pub command: Vec<String>,
    #[serde(skip)]
    pub stop_reason: Option<Status>,
    #[serde(skip)]
    pub did_auto_update: bool,
}
impl Job {
    pub fn new(url: String, options: Options, folder: PathBuf) -> Self {
        Self {
            id: id(),
            title: url.clone(),
            url,
            thumbnail: None,
            options,
            folder,
            playlist_items: None,
            status: Status::Queued,
            error: None,
            files: vec![],
            total_bytes: 0.0,
            progress: Progress::default(),
            log: vec![],
            command: vec![],
            stop_reason: None,
            did_auto_update: false,
        }
    }
    fn log(&mut self, line: String) {
        self.log.push(line);
        if self.log.len() > 4000 {
            self.log.drain(..self.log.len() - 4000);
        }
    }
    pub fn status_text(&self) -> String {
        if let Some(error) = &self.error {
            return error.message.clone();
        }
        if let Some(reason) = self.stop_reason {
            return format!(
                "{}…",
                if reason == Status::Paused {
                    "Pausing"
                } else {
                    "Cancelling"
                }
            );
        }
        if self.status == Status::Downloading {
            let p = &self.progress;
            let mut parts = vec![format!(
                "{} / {}",
                crate::support::bytes(p.downloaded),
                crate::support::bytes(p.total)
            )];
            if let (Some(i), Some(n)) = (p.item_index, p.item_count) {
                parts.insert(0, format!("Item {i} of {n}"));
            }
            if p.speed > 0.0 {
                parts.push(format!("{}/s", crate::support::bytes(p.speed)));
            }
            if let Some(eta) = p.eta {
                parts.push(format!("{} left", crate::support::duration(eta)));
            }
            parts.join(" · ")
        } else if self.status.active() {
            self.progress.stage.clone()
        } else {
            self.status.label().into()
        }
    }
}
struct Running {
    cancel: Sender<()>,
    thread: JoinHandle<()>,
}
enum Event {
    Line(String, String),
    Exit(String, Result<i32, String>, Vec<PathBuf>),
    Info(u64, Result<Media, String>),
    ToolsProgress(String),
    ToolsDone(Tools, Option<String>),
    Release(Result<String, String>),
}
#[derive(Clone)]
struct Sink {
    tx: Sender<Event>,
    wake: Arc<dyn Fn() + Send + Sync>,
}
impl Sink {
    fn send(&self, event: Event) {
        let _ = self.tx.send(event);
        (self.wake)();
    }
}

pub struct Engine {
    pub paths: Paths,
    pub settings: Settings,
    pub jobs: Vec<Job>,
    pub history: Vec<History>,
    pub presets: Vec<Preset>,
    pub tools: Tools,
    pub tools_busy: bool,
    pub tools_message: String,
    pub tools_error: Option<String>,
    pub info: Option<Media>,
    pub fetching: bool,
    pub info_error: Option<FriendlyError>,
    pub error: Option<String>,
    pub release_busy: bool,
    sink: Sink,
    events: Receiver<Event>,
    running: HashMap<String, Running>,
    info_worker: Option<Running>,
    retired_info: Vec<Running>,
    info_token: u64,
    update_waiters: HashSet<String>,
    shutting_down: bool,
}
impl Engine {
    pub fn load(paths: Paths, wake: impl Fn() + Send + Sync + 'static) -> Result<Self> {
        let settings: Settings = store::load(&paths.file("settings"))?.unwrap_or_default();
        let mut jobs: Vec<Job> = store::load(&paths.file("queue"))?.unwrap_or_default();
        for job in &mut jobs {
            if job.status.active() {
                job.status = Status::Queued;
            }
            job.progress.total = job.total_bytes;
        }
        // Old saves had no ids; also defend against duplicate ids in hand-edited files.
        let mut ids = HashSet::new();
        for job in &mut jobs {
            if !ids.insert(job.id.clone()) {
                job.id = id();
                ids.insert(job.id.clone());
            }
        }
        let history = store::load(&paths.file("history"))?.unwrap_or_default();
        let presets = store::load(&paths.file("presets"))?.unwrap_or_else(Preset::defaults);
        let (tx, events) = crossbeam_channel::unbounded();
        Ok(Self {
            paths,
            settings,
            jobs,
            history,
            presets,
            tools: Tools::default(),
            tools_busy: false,
            tools_message: String::new(),
            tools_error: None,
            info: None,
            fetching: false,
            info_error: None,
            error: None,
            release_busy: false,
            sink: Sink {
                tx,
                wake: Arc::new(wake),
            },
            events,
            running: HashMap::new(),
            info_worker: None,
            retired_info: vec![],
            info_token: 0,
            update_waiters: HashSet::new(),
            shutting_down: false,
        })
    }
    pub fn folder(&self) -> PathBuf {
        self.settings
            .download_folder
            .clone()
            .unwrap_or_else(|| self.paths.downloads.clone())
    }
    pub fn active_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.active()).count()
    }
    pub fn busy(&self, id: &str) -> bool {
        self.running.contains_key(id)
    }
    pub fn persist(&mut self) {
        let result = (|| -> Result<()> {
            store::save(&self.paths.file("settings"), &self.settings)?;
            let mut jobs = self.jobs.clone();
            for job in &mut jobs {
                if job.status.active() {
                    job.status = job.stop_reason.unwrap_or(Status::Queued);
                }
                job.total_bytes = job.progress.total;
            }
            store::save(&self.paths.file("queue"), &jobs)?;
            store::save(&self.paths.file("history"), &self.history)?;
            store::save(&self.paths.file("presets"), &self.presets)?;
            Ok(())
        })();
        if let Err(error) = result {
            self.error = Some(format!("Couldn't save: {error:#}"));
        }
    }
    pub fn setup_tools(&mut self, update: bool, reinstall: bool) {
        if self.tools_busy {
            return;
        }
        self.tools_busy = true;
        self.tools_error = None;
        self.tools_message = "Checking helper tools…".into();
        let paths = self.paths.clone();
        let channel = self.settings.ytdlp_channel.clone();
        let sink = self.sink.clone();
        thread::spawn(move || {
            let result = tools::bootstrap(&paths, &channel, update, reinstall, |message| {
                sink.send(Event::ToolsProgress(message))
            });
            let (tools, error) = match result {
                Ok(tools) => (tools, None),
                Err(e) => {
                    let mut tools = Tools::discover(&paths);
                    tools.refresh_versions(&paths);
                    tools
                        .paths
                        .retain(|name, _| tools.versions.contains_key(name));
                    (tools, Some(format!("{e:#}")))
                }
            };
            sink.send(Event::ToolsDone(tools, error));
        });
    }
    pub fn check_release(&mut self, force: bool) {
        if self.release_busy {
            return;
        }
        if !force
            && self
                .settings
                .last_app_update_check
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .is_some_and(|t| chrono::Utc::now().signed_duration_since(t).num_hours() < 24)
        {
            return;
        }
        self.release_busy = true;
        let sink = self.sink.clone();
        thread::spawn(move || {
            sink.send(Event::Release(
                tools::client()
                    .and_then(|client| tools::latest_tag(&client, "BLCNYY/Purrloader"))
                    .map_err(|e| e.to_string()),
            ))
        });
    }
    pub fn cancel_info(&mut self) {
        self.info_token += 1;
        self.fetching = false;
        self.info = None;
        self.info_error = None;
        if let Some(worker) = self.info_worker.take() {
            let _ = worker.cancel.send(());
            self.retired_info.push(worker);
        }
    }
    pub fn fetch_info(&mut self, url: String, options: Options) {
        self.cancel_info();
        if !self.tools.ready() {
            return;
        }
        let Some(executable) = self.tools.paths.get("yt-dlp").cloned() else {
            return;
        };
        self.fetching = true;
        let env = self.tools.environment(&self.paths);
        let args = env.info(&options, &url);
        let cmd = process::command(
            &executable,
            &args,
            &self.paths.temp,
            &[env.ffmpeg.parent().unwrap(), env.deno.parent().unwrap()],
        );
        let token = self.info_token;
        let sink = self.sink.clone();
        let (cancel, rx) = crossbeam_channel::bounded(1);
        let thread = thread::spawn(move || {
            let result = process::collect(cmd, rx, Duration::from_secs(120))
                .and_then(|(code, stdout, stderr)| {
                    anyhow::ensure!(code == 0, "{}", stderr.trim());
                    let raw = serde_json::from_str(&stdout)?;
                    Ok(Media::parse(raw, &url))
                })
                .map_err(|e| e.to_string());
            sink.send(Event::Info(token, result));
        });
        self.info_worker = Some(Running { cancel, thread });
    }
    pub fn enqueue(&mut self, job: Job) {
        self.jobs.insert(0, job);
        self.persist();
        self.schedule();
    }
    pub fn schedule(&mut self) {
        if self.shutting_down || !self.tools.ready() {
            return;
        }
        while self.running.len() < self.settings.max_concurrent.clamp(1, 10) {
            let Some(index) = self
                .jobs
                .iter()
                .rposition(|j| j.status == Status::Queued && !self.running.contains_key(&j.id))
            else {
                break;
            };
            self.start(index);
        }
    }
    fn start(&mut self, index: usize) {
        let job = &mut self.jobs[index];
        job.status = Status::Starting;
        job.error = None;
        job.stop_reason = None;
        job.progress = Progress::default();
        // New log per attempt, while paths from completed playlist items remain available.
        let log_path = self.paths.temp.join(format!("{}.paths", job.id));
        let _ = fs::remove_file(&log_path);
        let environment = self.tools.environment(&self.paths);
        let args = match environment.download(
            &job.options,
            &job.url,
            &job.folder,
            job.playlist_items.as_deref(),
            Some(&log_path),
        ) {
            Ok(args) => args,
            Err(e) => {
                job.status = Status::Failed;
                job.error = Some(FriendlyError::from_raw(&e.to_string()));
                return;
            }
        };
        let executable = self.tools.paths.get("yt-dlp").unwrap();
        job.command = vec![executable.display().to_string()];
        job.command.extend(args.clone());
        job.log(format!(
            "$ {}",
            crate::support::command_line(&job.command, cfg!(windows))
        ));
        let cmd = process::command(
            executable,
            &args,
            &self.paths.temp,
            &[
                environment.ffmpeg.parent().unwrap(),
                environment.deno.parent().unwrap(),
            ],
        );
        let id = job.id.clone();
        let sink = self.sink.clone();
        let (cancel, rx) = crossbeam_channel::bounded(1);
        let thread = thread::spawn(move || {
            let code = process::run(cmd, rx, None, |line, _| {
                sink.send(Event::Line(id.clone(), line))
            })
            .map_err(|e| e.to_string());
            let files = fs::read_to_string(&log_path)
                .unwrap_or_default()
                .lines()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            let _ = fs::remove_file(&log_path);
            sink.send(Event::Exit(id, code, files));
        });
        self.running
            .insert(job.id.clone(), Running { cancel, thread });
        self.persist();
    }
    pub fn stop(&mut self, id: &str, reason: Status) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if let Some(worker) = self.running.get(id) {
                job.stop_reason = Some(reason);
                let _ = worker.cancel.try_send(());
            } else {
                job.status = reason;
                if reason == Status::Cancelled {
                    remove_partials(job);
                }
            }
        }
        self.update_waiters.remove(id);
        self.persist();
    }
    pub fn retry(&mut self, id: &str) {
        if self.busy(id) {
            return;
        }
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = Status::Queued;
            job.error = None;
            job.stop_reason = None;
        }
        self.persist();
        self.schedule();
    }
    pub fn update_and_retry(&mut self, id: &str) {
        if self.busy(id) {
            return;
        }
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = Status::Processing;
            job.error = None;
            job.did_auto_update = true;
            job.progress.stage = "Updating helper tools, then trying again…".into();
            self.update_waiters.insert(id.into());
            self.setup_tools(true, false);
        }
    }
    pub fn pause_all(&mut self) {
        let ids: Vec<_> = self
            .jobs
            .iter()
            .filter(|j| j.status.active() || j.status == Status::Queued)
            .map(|j| j.id.clone())
            .collect();
        for id in ids {
            self.stop(&id, Status::Paused);
        }
    }
    pub fn resume_all(&mut self) {
        for job in &mut self.jobs {
            if job.status == Status::Paused {
                job.status = Status::Queued;
            }
        }
        self.persist();
        self.schedule();
    }
    pub fn remove(&mut self, id: &str) {
        if !self.busy(id) {
            self.jobs.retain(|j| j.id != id);
            self.update_waiters.remove(id);
            self.persist();
        }
    }
    pub fn poll(&mut self) {
        let mut dirty = false;
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Line(id, line) => {
                    if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
                        if job.progress.consume(&line) {
                            job.log(line);
                        }
                        if job.stop_reason.is_none() {
                            job.status = match job.progress.phase.as_str() {
                                "downloading" => Status::Downloading,
                                "processing" => Status::Processing,
                                _ => Status::Starting,
                            };
                        }
                        if job.title == job.url
                            && let Some(title) = &job.progress.title
                        {
                            job.title = title.clone();
                        }
                    }
                }
                Event::Exit(id, code, files) => {
                    if let Some(worker) = self.running.remove(&id) {
                        let _ = worker.thread.join();
                    }
                    let mut auto_retry = false;
                    if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
                        for file in files {
                            if !job.files.contains(&file) {
                                job.files.push(file);
                            }
                        }
                        if let Some(reason) = job.stop_reason.take() {
                            job.status = reason;
                            if reason == Status::Cancelled {
                                remove_partials(job);
                            }
                        } else if matches!(code, Ok(0)) {
                            job.status = Status::Finished;
                            job.progress.fraction = Some(1.0);
                            job.progress.total = job
                                .files
                                .iter()
                                .filter_map(|p| fs::metadata(p).ok())
                                .map(|m| m.len() as f64)
                                .sum();
                            self.history.insert(
                                0,
                                History {
                                    id: id.clone(),
                                    url: job.url.clone(),
                                    title: job.title.clone(),
                                    date: chrono::Utc::now().to_rfc3339(),
                                    folder: job.folder.clone(),
                                    files: job.files.clone(),
                                    thumbnail: job.thumbnail.clone(),
                                    options: job.options.clone(),
                                },
                            );
                            if self.settings.notifications {
                                notify("Downloaded", &job.title);
                            }
                        } else {
                            let raw = code
                                .err()
                                .or_else(|| job.progress.error.clone())
                                .unwrap_or_else(|| "The download stopped unexpectedly.".into());
                            let error = FriendlyError::from_raw(&raw);
                            auto_retry = error.kind == "blocked" && !job.did_auto_update;
                            job.log(raw);
                            job.status = Status::Failed;
                            job.error = Some(error);
                            if self.settings.notifications && !auto_retry {
                                notify("Couldn't download", &job.title);
                            }
                        }
                    }
                    if auto_retry {
                        self.update_and_retry(&id);
                    }
                    dirty = true;
                }
                Event::Info(token, result) => {
                    if token == self.info_token {
                        self.fetching = false;
                        match result {
                            Ok(info) => self.info = Some(info),
                            Err(raw) => self.info_error = Some(FriendlyError::from_raw(&raw)),
                        }
                    }
                }
                Event::ToolsProgress(message) => self.tools_message = message,
                Event::ToolsDone(tools, error) => {
                    self.tools = tools;
                    self.tools_busy = false;
                    self.tools_error = error;
                    self.tools_message = if self.tools.ready() {
                        "Helper tools are ready."
                    } else {
                        "Helper tools need attention."
                    }
                    .into();
                    for id in self.update_waiters.drain() {
                        if let Some(job) = self
                            .jobs
                            .iter_mut()
                            .find(|j| j.id == id && j.status == Status::Processing)
                        {
                            job.status = Status::Queued;
                        }
                    }
                    dirty = true;
                }
                Event::Release(result) => {
                    self.release_busy = false;
                    if let Ok(version) = result {
                        self.settings.latest_app_version =
                            Some(version.trim_start_matches('v').into());
                        self.settings.latest_app_page =
                            Some("https://github.com/BLCNYY/Purrloader/releases/latest".into());
                        self.settings.last_app_update_check = Some(chrono::Utc::now().to_rfc3339());
                        dirty = true;
                    }
                }
            }
        }
        self.retired_info
            .retain(|worker| !worker.thread.is_finished());
        if dirty {
            self.persist();
        }
        self.schedule();
    }
    pub fn shutdown(&mut self) {
        self.shutting_down = true;
        // Active entries are saved queued before sending cancellation, so next launch resumes them.
        self.persist();
        self.cancel_info();
        for worker in self.running.values() {
            let _ = worker.cancel.try_send(());
        }
        let start = Instant::now();
        while (self.running.values().any(|w| !w.thread.is_finished())
            || self.retired_info.iter().any(|w| !w.thread.is_finished()))
            && start.elapsed() < Duration::from_secs(9)
        {
            thread::sleep(Duration::from_millis(20));
        }
        for (_, worker) in self.running.drain() {
            if worker.thread.is_finished() {
                let _ = worker.thread.join();
            }
        }
        for worker in self.retired_info.drain(..) {
            if worker.thread.is_finished() {
                let _ = worker.thread.join();
            }
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        if !self.shutting_down {
            self.shutdown();
        }
    }
}
fn notify(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .appname("Purrloader")
            .summary(&title)
            .body(&body)
            .show();
    });
}
fn remove_partials(job: &Job) {
    // Only files named by this job, beneath its selected output folder, are candidates.
    let Ok(root) = job.folder.canonicalize() else {
        return;
    };
    for destination in &job.progress.destinations {
        let path = PathBuf::from(destination);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        let Some(parent) = path
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .filter(|p| p.starts_with(&root))
        else {
            continue;
        };
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        for suffix in [".part", ".ytdl", ".part.ytdl"] {
            let _ = fs::remove_file(parent.join(format!("{name}{suffix}")));
        }
        if let Ok(entries) = fs::read_dir(&parent) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{name}.part-Frag"))
                    && entry.path().is_file()
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}
