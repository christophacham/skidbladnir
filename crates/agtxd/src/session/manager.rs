use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use pty_process::{OwnedReadPty, Size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::output::SessionOutput;
use super::types::{SessionHandle, SessionInfo, SessionState, SpawnRequest};

/// Session manager for PTY process lifecycle.
///
/// Manages spawning, reading, writing, resizing, and killing PTY sessions.
/// Holds all active sessions behind `Arc<RwLock<HashMap<Uuid, SessionHandle>>>`.
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SessionHandle>>>,
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create a new SessionManager.
    ///
    /// Creates the sessions directory if it does not exist.
    pub fn new(sessions_dir: PathBuf) -> Self {
        // Best-effort directory creation; spawn will also create parent dirs for log files
        let _ = std::fs::create_dir_all(&sessions_dir);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            sessions_dir,
        }
    }

    /// Spawn a new PTY session with the given command and arguments.
    ///
    /// Returns the UUID of the newly created session.
    pub async fn spawn(&self, req: SpawnRequest) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        let output_path = self.sessions_dir.join(format!("{}.log", id));
        let output = SessionOutput::new(&output_path)
            .await
            .context("Failed to create session output")?;

        // Allocate PTY
        let (pty, pts) = pty_process::open().context("Failed to allocate PTY")?;
        pty.resize(Size::new(req.rows, req.cols))
            .context("Failed to resize PTY")?;

        // Build command
        let mut cmd = pty_process::Command::new(&req.command)
            .args(&req.args)
            .current_dir(&req.working_dir)
            .env("TERM", "xterm-256color");

        // Add extra environment variables
        for (key, val) in &req.env {
            cmd = cmd.env(key, val);
        }

        // Safety net: kill child if daemon dies
        // Safety: prctl(PR_SET_PDEATHSIG) is async-signal-safe
        unsafe {
            cmd = cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        // Spawn child process - pts is consumed, closing slave fd in parent
        let child = cmd.spawn(pts).context("Failed to spawn child process")?;
        let pid = child.id().unwrap_or(0);

        // Split PTY into read/write halves for concurrent access
        let (read_pty, write_pty) = pty.into_split();

        // Wrap output in Arc<RwLock> for shared access between reader task and manager
        let output = Arc::new(RwLock::new(output));

        // Spawn reader task to continuously capture PTY output
        let reader_output = output.clone();
        let reader_id = id;
        let reader_handle = tokio::spawn(reader_task(read_pty, reader_output, reader_id));

        // Create session handle
        let handle = SessionHandle {
            id,
            pid,
            state: SessionState::Running,
            child,
            write_pty: Mutex::new(write_pty),
            output,
            reader_handle,
            created_at: Utc::now(),
        };

        // Insert into sessions map
        self.sessions.write().await.insert(id, handle);

        tracing::info!(session_id = %id, pid = pid, "Session spawned");

        Ok(id)
    }

    /// Write input text to a session's PTY stdin, followed by a newline.
    pub async fn write(&self, id: Uuid, input: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;

        let mut write_pty = handle.write_pty.lock().await;
        write_pty
            .write_all(input.as_bytes())
            .await
            .context("Failed to write to PTY")?;
        write_pty
            .write_all(b"\n")
            .await
            .context("Failed to write newline to PTY")?;
        write_pty
            .flush()
            .await
            .context("Failed to flush PTY write")?;

        Ok(())
    }

    /// Resize a session's PTY to the given dimensions.
    pub async fn resize(&self, id: Uuid, rows: u16, cols: u16) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;

        let write_pty = handle.write_pty.lock().await;
        write_pty
            .resize(Size::new(rows, cols))
            .context("Failed to resize PTY")?;

        tracing::debug!(session_id = %id, rows = rows, cols = cols, "PTY resized");

        Ok(())
    }

    /// Get session info by ID.
    pub async fn get(&self, id: Uuid) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        let handle = sessions.get(&id)?;
        let total_bytes = handle.output.read().await.total_bytes();
        Some(SessionInfo {
            id: handle.id,
            pid: handle.pid,
            state: handle.state,
            created_at: handle.created_at,
            total_bytes,
        })
    }

    /// List all active sessions.
    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut infos = Vec::with_capacity(sessions.len());
        for handle in sessions.values() {
            let total_bytes = handle.output.read().await.total_bytes();
            infos.push(SessionInfo {
                id: handle.id,
                pid: handle.pid,
                state: handle.state,
                created_at: handle.created_at,
                total_bytes,
            });
        }
        infos
    }

    /// Get the output ring buffer contents for a session.
    pub async fn get_output(&self, id: Uuid) -> Option<Vec<u8>> {
        let output_arc = {
            let sessions = self.sessions.read().await;
            let handle = sessions.get(&id)?;
            handle.output.clone()
        };
        let guard = output_arc.read().await;
        let tail = guard.tail();
        Some(tail)
    }

    /// Kill a session: terminate the child process and clean up resources.
    pub async fn kill(&self, id: Uuid) -> anyhow::Result<()> {
        let mut handle = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(&id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?
        };

        // Kill the child process
        if let Err(e) = handle.child.kill().await {
            tracing::warn!(session_id = %id, error = %e, "Failed to kill child process");
        }

        // Wait to reap the zombie
        let _ = handle.child.wait().await;

        // Abort the reader task
        handle.reader_handle.abort();

        tracing::info!(session_id = %id, pid = handle.pid, "Session killed");

        Ok(())
    }

    /// Shut down all active sessions. Called during daemon shutdown.
    pub async fn shutdown_all(&self) {
        let mut sessions = self.sessions.write().await;
        for (id, mut handle) in sessions.drain() {
            tracing::info!(session_id = %id, pid = handle.pid, "Killing session during shutdown");

            if let Err(e) = handle.child.kill().await {
                tracing::warn!(session_id = %id, error = %e, "Failed to kill during shutdown");
            }
            let _ = handle.child.wait().await;
            handle.reader_handle.abort();
        }
    }
}

/// Background task that reads from the PTY and writes to the session output.
async fn reader_task(
    mut read_pty: OwnedReadPty,
    output: Arc<RwLock<SessionOutput>>,
    session_id: Uuid,
) {
    let mut buf = [0u8; 4096];
    loop {
        match read_pty.read(&mut buf).await {
            Ok(0) => {
                tracing::info!(session_id = %session_id, "PTY EOF");
                break;
            }
            Ok(n) => {
                let mut out = output.write().await;
                if let Err(e) = out.append(&buf[..n]).await {
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to write to session output"
                    );
                    break;
                }
            }
            Err(e) => {
                // EIO is expected when the child process exits and the PTY closes
                if e.raw_os_error() == Some(libc::EIO) {
                    tracing::info!(session_id = %session_id, "PTY closed (EIO)");
                } else {
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "PTY read error"
                    );
                }
                break;
            }
        }
    }
}
