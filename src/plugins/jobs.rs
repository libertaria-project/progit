// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Async job scheduler for plugin long-running commands.
//!
//! Mirrors the SDK event model (`PluginEvent::Job*`) on the host side.
//! Each job runs in a background thread; stdout/stderr are streamed
//! line-by-line through an MPSC channel so the TUI can poll them
//! without blocking the render loop.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

/// Unique job identifier.
pub type JobId = String;

/// Events emitted by a running or completed job.
#[derive(Debug, Clone)]
pub enum JobEvent {
    Started {
        job_id: JobId,
        plugin: String,
        command: String,
    },
    Progress {
        job_id: JobId,
        percent: Option<u8>,
        message: String,
    },
    LogLine {
        job_id: JobId,
        line: String,
        stream: String,
    },
    OutputChunk {
        job_id: JobId,
        chunk: String,
    },
    Cancelled {
        job_id: JobId,
        reason: String,
    },
    Completed {
        job_id: JobId,
        exit_code: i32,
        output: String,
    },
    Failed {
        job_id: JobId,
        error: String,
    },
}

/// Current state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// In-memory record for an active or finished job.
pub struct ActiveJob {
    pub id: JobId,
    pub plugin: String,
    pub command: String,
    pub args: Vec<String>,
    pub state: JobState,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: Option<i32>,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
}

/// Host-side job scheduler.
pub struct JobManager {
    jobs: HashMap<JobId, ActiveJob>,
    next_id: u64,
    event_tx: Sender<JobEvent>,
    event_rx: Receiver<JobEvent>,
}

impl JobManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            jobs: HashMap::new(),
            next_id: 0,
            event_tx: tx,
            event_rx: rx,
        }
    }

    /// Spawn a background job. Returns the job id immediately.
    pub fn spawn_job(
        &mut self,
        plugin: &str,
        command: &str,
        args: Vec<String>,
        cwd: Option<&std::path::Path>,
    ) -> JobId {
        let id = format!("{}:{}", plugin, self.next_id);
        self.next_id += 1;

        let job = ActiveJob {
            id: id.clone(),
            plugin: plugin.to_string(),
            command: command.to_string(),
            args: args.clone(),
            state: JobState::Pending,
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: None,
            start_time: Instant::now(),
            end_time: None,
        };
        self.jobs.insert(id.clone(), job);

        let tx = self.event_tx.clone();
        let job_id = id.clone();
        let cmd = command.to_string();
        let plugin_name = plugin.to_string();
        let cwd = cwd.map(|p| p.to_path_buf());

        thread::spawn(move || {
            let _ = tx.send(JobEvent::Started {
                job_id: job_id.clone(),
                plugin: plugin_name.clone(),
                command: cmd.clone(),
            });

            let mut child_cmd = Command::new(&cmd);
            child_cmd.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
            if let Some(ref dir) = cwd {
                child_cmd.current_dir(dir);
            }

            let mut child = match child_cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(JobEvent::Failed {
                        job_id: job_id.clone(),
                        error: format!("Failed to spawn '{}': {}", cmd, e),
                    });
                    return;
                }
            };

            // Stream stdout
            if let Some(stdout) = child.stdout.take() {
                let tx2 = tx.clone();
                let jid = job_id.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let _ = tx2.send(JobEvent::LogLine {
                                job_id: jid.clone(),
                                line,
                                stream: "stdout".to_string(),
                            });
                        }
                    }
                });
            }

            // Stream stderr
            if let Some(stderr) = child.stderr.take() {
                let tx2 = tx.clone();
                let jid = job_id.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let _ = tx2.send(JobEvent::LogLine {
                                job_id: jid.clone(),
                                line,
                                stream: "stderr".to_string(),
                            });
                        }
                    }
                });
            }

            match child.wait() {
                Ok(status) => {
                    let code = status.code().unwrap_or(-1);
                    let _ = tx.send(JobEvent::Completed {
                        job_id: job_id.clone(),
                        exit_code: code,
                        output: String::new(), // individual lines already streamed
                    });
                }
                Err(e) => {
                    let _ = tx.send(JobEvent::Failed {
                        job_id: job_id.clone(),
                        error: format!("Wait error: {}", e),
                    });
                }
            }
        });

        id
    }

    /// Non-blocking poll of all pending job events.
    pub fn poll_events(&mut self) -> Vec<JobEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            self.apply_event(&evt);
            events.push(evt);
        }
        events
    }

    fn apply_event(&mut self, evt: &JobEvent) {
        match evt {
            JobEvent::Started { job_id, .. } => {
                if let Some(job) = self.jobs.get_mut(job_id) {
                    job.state = JobState::Running;
                }
            }
            JobEvent::LogLine {
                job_id, line, stream, ..
            } => {
                if let Some(job) = self.jobs.get_mut(job_id) {
                    if stream == "stderr" {
                        job.stderr.push(line.clone());
                    } else {
                        job.stdout.push(line.clone());
                    }
                }
            }
            JobEvent::Completed { job_id, exit_code, .. } => {
                if let Some(job) = self.jobs.get_mut(job_id) {
                    job.state = JobState::Completed;
                    job.exit_code = Some(*exit_code);
                    job.end_time = Some(Instant::now());
                }
            }
            JobEvent::Failed { job_id, .. } => {
                if let Some(job) = self.jobs.get_mut(job_id) {
                    job.state = JobState::Failed;
                    job.end_time = Some(Instant::now());
                }
            }
            JobEvent::Cancelled { job_id, .. } => {
                if let Some(job) = self.jobs.get_mut(job_id) {
                    job.state = JobState::Cancelled;
                    job.end_time = Some(Instant::now());
                }
            }
            _ => {}
        }
    }

    /// Jobs currently in `Running` state.
    pub fn active_jobs(&self) -> Vec<&ActiveJob> {
        self.jobs
            .values()
            .filter(|j| j.state == JobState::Running)
            .collect()
    }

    /// Lookup a job by id.
    pub fn get_job(&self, id: &str) -> Option<&ActiveJob> {
        self.jobs.get(id)
    }

    /// Total jobs tracked (including finished).
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_manager_starts_empty() {
        let manager = JobManager::new();
        assert_eq!(manager.job_count(), 0);
        assert!(manager.active_jobs().is_empty());
    }

    #[test]
    fn spawn_job_returns_id_and_emits_started() {
        let mut manager = JobManager::new();
        let id = manager.spawn_job("test", "echo", vec!["hello".into()], None);
        assert!(!id.is_empty());
        assert_eq!(manager.job_count(), 1);

        // Poll until we get at least the Started event
        let mut found_started = false;
        for _ in 0..50 {
            let evts = manager.poll_events();
            if evts.iter().any(|e| matches!(e, JobEvent::Started { .. })) {
                found_started = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(found_started, "Expected JobEvent::Started");
    }

    #[test]
    fn echo_job_completes_with_output() {
        let mut manager = JobManager::new();
        let id = manager.spawn_job("test", "echo", vec!["plugin-line".into()], None);

        let mut completed = false;
        let mut saw_line = false;
        for _ in 0..100 {
            for evt in manager.poll_events() {
                match evt {
                    JobEvent::LogLine { job_id, line, .. } if job_id == id => {
                        if line.contains("plugin-line") {
                            saw_line = true;
                        }
                    }
                    JobEvent::Completed { job_id, .. } if job_id == id => {
                        completed = true;
                    }
                    _ => {}
                }
            }
            if completed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(completed, "Job should complete");
        assert!(saw_line, "Should see stdout line");

        let job = manager.get_job(&id).unwrap();
        assert_eq!(job.state, JobState::Completed);
        assert_eq!(job.exit_code, Some(0));
    }
}
