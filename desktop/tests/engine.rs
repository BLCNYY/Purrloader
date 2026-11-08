#![cfg(feature = "test-helper")]
use purrloader::{
    engine::{Engine, Job, Status},
    options::Options,
    process,
    store::{self, Paths},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(
        dir.path().join("data"),
        dir.path().join("downloads"),
        dir.path().into(),
    )
    .unwrap();
    let mut engine = Engine::load(paths, || {}).unwrap();
    engine.settings.notifications = false;
    for name in purrloader::tools::NAMES {
        engine.tools.paths.insert(
            name.into(),
            PathBuf::from(env!("CARGO_BIN_EXE_purrloader-test-helper")),
        );
        engine.tools.versions.insert(name.into(), "test".into());
    }
    (dir, engine)
}
fn wait(engine: &mut Engine, condition: impl Fn(&Engine) -> bool) {
    let start = Instant::now();
    loop {
        engine.poll();
        if condition(engine) {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(12),
            "timed out: {:?}",
            engine.jobs
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn job(engine: &Engine, kind: &str) -> Job {
    Job::new(
        format!("https://test.invalid/{kind}"),
        Options::default(),
        engine.folder(),
    )
}
#[test]
fn real_process_success_records_paths_history_and_restart_state() {
    let (_dir, mut engine) = engine();
    engine.enqueue(job(&engine, "success"));
    wait(&mut engine, |e| e.jobs[0].status == Status::Finished);
    assert_eq!(engine.history.len(), 1);
    assert_eq!(
        std::fs::read(&engine.jobs[0].files[0]).unwrap(),
        b"test fixture"
    );
    assert!(
        engine.jobs[0]
            .log
            .iter()
            .any(|s| s.contains("stderr output"))
    );
    let paths = engine.paths.clone();
    engine.shutdown();
    drop(engine);
    let restored = Engine::load(paths, || {}).unwrap();
    assert_eq!(restored.jobs[0].status, Status::Finished);
    assert_eq!(restored.history.len(), 1);
}
#[test]
fn queue_limits_pause_cancel_retry_and_shutdown() {
    let (_dir, mut engine) = engine();
    engine.settings.max_concurrent = 1;
    engine.enqueue(job(&engine, "slow"));
    let first = engine.jobs[0].id.clone();
    engine.enqueue(job(&engine, "success"));
    let second = engine.jobs[0].id.clone();
    assert_eq!(engine.active_count(), 1);
    assert_eq!(engine.jobs[0].status, Status::Queued);
    engine.stop(&first, Status::Paused);
    wait(&mut engine, |e| {
        e.jobs
            .iter()
            .any(|j| j.id == second && j.status == Status::Finished)
    });
    assert_eq!(
        engine.jobs.iter().find(|j| j.id == first).unwrap().status,
        Status::Paused
    );
    engine.retry(&first);
    wait(&mut engine, |e| {
        e.jobs
            .iter()
            .any(|j| j.id == first && j.status == Status::Downloading)
    });
    engine.stop(&first, Status::Cancelled);
    wait(&mut engine, |e| !e.busy(&first));
    assert_eq!(
        engine.jobs.iter().find(|j| j.id == first).unwrap().status,
        Status::Cancelled
    );
    engine.retry(&first);
    engine.shutdown();
    let saved: Vec<Job> = store::load(&engine.paths.file("queue")).unwrap().unwrap();
    assert_eq!(
        saved.iter().find(|j| j.id == first).unwrap().status,
        Status::Queued
    );
}
#[test]
fn failed_jobs_offer_recovery_and_stale_info_cannot_replace_new_link() {
    let (_dir, mut engine) = engine();
    engine.enqueue(job(&engine, "fail"));
    wait(&mut engine, |e| e.jobs[0].status == Status::Failed);
    assert_eq!(
        engine.jobs[0].error.as_ref().unwrap().kind,
        "formatUnavailable"
    );
    assert!(engine.history.is_empty());
    engine.fetch_info("https://test.invalid/slow".into(), Options::default());
    engine.fetch_info("https://test.invalid/success".into(), Options::default());
    wait(&mut engine, |e| e.info.is_some());
    assert_eq!(
        engine.info.as_ref().unwrap().url,
        "https://test.invalid/success"
    );
    engine.cancel_info();
    assert!(engine.info.is_none());
}
#[test]
fn argv_is_literal_and_timeout_reaps_process() {
    let dir = tempfile::tempdir().unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_purrloader-test-helper"));
    let args = vec![
        "--echo".into(),
        "space and 'quotes' $HOME $(touch unsafe)".into(),
    ];
    let cmd = process::command(&exe, &args, dir.path(), &[]);
    let (code, out, _) =
        process::collect(cmd, crossbeam_channel::never(), Duration::from_secs(3)).unwrap();
    assert_eq!(code, 0);
    assert_eq!(serde_json::from_str::<Vec<String>>(&out).unwrap(), args);
    let cmd = process::command(
        &exe,
        &["-J".into(), "https://test.invalid/slow".into()],
        dir.path(),
        &[],
    );
    let start = Instant::now();
    let (code, _, _) =
        process::collect(cmd, crossbeam_channel::never(), Duration::from_millis(100)).unwrap();
    assert_ne!(code, 0);
    assert!(start.elapsed() < Duration::from_secs(8));
}

#[test]
fn cancellation_stops_grandchildren_and_releases_inherited_pipes() {
    let dir = tempfile::tempdir().unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_purrloader-test-helper"));
    let args = vec![
        "-P".into(),
        dir.path().display().to_string(),
        "https://test.invalid/tree".into(),
    ];
    let cmd = process::command(&exe, &args, dir.path(), &[]);
    let (tx, rx) = crossbeam_channel::bounded(1);
    let worker =
        std::thread::spawn(move || process::run(cmd, rx, Some(Duration::from_secs(10)), |_, _| {}));
    let start = Instant::now();
    while !dir.path().join("child.started").exists() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    tx.send(()).unwrap();
    assert_ne!(worker.join().unwrap().unwrap(), 0);
    std::thread::sleep(Duration::from_millis(1600));
    assert!(
        !dir.path().join("child.survived").exists(),
        "grandchild outlived the cancelled download"
    );
}
