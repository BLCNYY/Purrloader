//! A worker owns each process until it is reaped; cancellation stops the entire tree.
use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use std::{
    io::{BufReader, Read},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct LineBuffer {
    pending: Vec<u8>,
}
impl LineBuffer {
    pub fn append(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = vec![];
        for &byte in bytes {
            if byte == b'\r' || byte == b'\n' {
                if let Some(line) = self.flush() {
                    lines.push(line);
                }
            } else {
                self.pending.push(byte);
            }
        }
        lines
    }
    pub fn flush(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned())
    }
}
pub fn command(executable: &Path, args: &[String], cwd: &Path, tool_dirs: &[&Path]) -> Command {
    let mut cmd = Command::new(executable);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONIOENCODING", "utf-8");
    let mut dirs: Vec<_> = tool_dirs.iter().map(|p| p.to_path_buf()).collect();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    if let Ok(path) = std::env::join_paths(dirs) {
        cmd.env("PATH", path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd
}
#[cfg(unix)]
fn stop_tree(pid: u32, force: bool) {
    unsafe {
        libc::kill(
            -(pid as i32),
            if force { libc::SIGKILL } else { libc::SIGINT },
        );
    }
}

/// Closing the job kills descendants even if their parent already exited.
#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl WindowsJob {
    fn attach(child: &std::process::Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            anyhow::ensure!(
                !handle.is_null(),
                "Couldn't create a helper process job: {}",
                std::io::Error::last_os_error()
            );
            let job = Self(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            anyhow::ensure!(
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32
                ) != 0,
                "Couldn't configure the helper process job: {}",
                std::io::Error::last_os_error()
            );
            anyhow::ensure!(
                AssignProcessToJobObject(handle, child.as_raw_handle()) != 0,
                "Couldn't attach the helper process: {}",
                std::io::Error::last_os_error()
            );
            Ok(job)
        }
    }
    fn stop(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}
#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Both pipes are drained concurrently; output is delivered before the exit result.
pub fn run(
    mut cmd: Command,
    cancel: Receiver<()>,
    timeout: Option<Duration>,
    mut on_line: impl FnMut(String, bool),
) -> Result<i32> {
    let mut child = cmd.spawn().context("Couldn't start the helper tool")?;
    #[cfg(windows)]
    let job = match WindowsJob::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    #[cfg(unix)]
    let pid = child.id();
    let stop = |_force: bool| {
        #[cfg(windows)]
        job.stop();
        #[cfg(unix)]
        stop_tree(pid, _force);
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    let out = child.stdout.take().context("Missing stdout")?;
    let err = child.stderr.take().context("Missing stderr")?;
    let mut readers = vec![];
    for (pipe, is_error) in [
        (Box::new(out) as Box<dyn Read + Send>, false),
        (Box::new(err) as Box<dyn Read + Send>, true),
    ] {
        let tx = tx.clone();
        readers.push(thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut buffer = [0; 8192];
            let mut lines = LineBuffer::default();
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                for line in lines.append(&buffer[..n]) {
                    if tx.send((line, is_error)).is_err() {
                        return;
                    }
                }
            }
            if let Some(line) = lines.flush() {
                let _ = tx.send((line, is_error));
            }
        }));
    }
    drop(tx);
    let start = Instant::now();
    let mut stopped = None;
    let mut exited = None;
    let mut exit_code = None;
    loop {
        while let Ok((line, err)) = rx.try_recv() {
            on_line(line, err);
        }
        if stopped.is_none()
            && (cancel.try_recv().is_ok() || timeout.is_some_and(|t| start.elapsed() >= t))
        {
            stop(false);
            stopped = Some(Instant::now());
        }
        if stopped.is_some_and(|t| t.elapsed() >= Duration::from_secs(3)) {
            stop(true);
            let _ = child.kill();
        }
        if exit_code.is_none()
            && let Some(status) = child.try_wait()?
        {
            exit_code = Some(status.code().unwrap_or(1));
            exited = Some(Instant::now());
        }
        if let Some(code) = exit_code {
            if readers.iter().all(|h| h.is_finished()) {
                for h in readers {
                    let _ = h.join();
                }
                while let Ok((line, err)) = rx.try_recv() {
                    on_line(line, err);
                }
                return Ok(if stopped.is_some() && code == 0 {
                    1
                } else {
                    code
                });
            }
            // A grandchild may retain inherited pipes after the parent exits.
            if exited.is_some_and(|t| t.elapsed() > Duration::from_secs(3)) {
                stop(true);
            }
            if exited.is_some_and(|t| t.elapsed() > Duration::from_secs(5)) {
                return Ok(code);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn collect(
    cmd: Command,
    cancel: Receiver<()>,
    timeout: Duration,
) -> Result<(i32, String, String)> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let status = run(cmd, cancel, Some(timeout), |line, error| {
        let target = if error { &mut stderr } else { &mut stdout };
        target.push_str(&line);
        target.push('\n');
    })?;
    Ok((status, stdout, stderr))
}
