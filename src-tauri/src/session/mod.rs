//! In-memory registry of active SSH sessions, managed as Tauri state.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use russh::Disconnect;
use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::ssh::{self, ConnectRequest, SessionCommand, SshHandle};

/// One active session: the shell command channel plus the shared SSH handle
/// (used to open SFTP channels on the same connection).
struct Session {
    commands: mpsc::Sender<SessionCommand>,
    handle: Arc<SshHandle>,
}

/// Tracks all active sessions by id.
#[derive(Default)]
pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
    counter: AtomicU64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a new SSH session, spawn its streaming actor, and register it;
    /// returns the session id.
    pub async fn connect(
        &self,
        app: AppHandle,
        req: ConnectRequest,
        known_hosts: PathBuf,
        output: Channel<String>,
    ) -> AppResult<String> {
        let (handle, channel) = ssh::open_shell(&req, known_hosts).await?;
        let id = format!("session-{}", self.counter.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel(64);
        tauri::async_runtime::spawn(ssh::run_session(channel, rx, output, app, id.clone()));
        self.sessions.lock().unwrap().insert(
            id.clone(),
            Session {
                commands: tx,
                handle: Arc::new(handle),
            },
        );
        Ok(id)
    }

    /// Clone the command sender for a session (lock is never held across await).
    fn sender(&self, id: &str) -> AppResult<mpsc::Sender<SessionCommand>> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|s| s.commands.clone())
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))
    }

    /// Clone the shared SSH handle for a session (for SFTP and other channels).
    pub fn handle(&self, id: &str) -> AppResult<Arc<SshHandle>> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|s| s.handle.clone())
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))
    }

    pub async fn send_input(&self, id: &str, bytes: Vec<u8>) -> AppResult<()> {
        self.sender(id)?
            .send(SessionCommand::Input(bytes))
            .await
            .map_err(|_| AppError::Other("session is closed".into()))
    }

    pub async fn resize(&self, id: &str, cols: u32, rows: u32) -> AppResult<()> {
        self.sender(id)?
            .send(SessionCommand::Resize { cols, rows })
            .await
            .map_err(|_| AppError::Other("session is closed".into()))
    }

    pub async fn disconnect(&self, id: &str) -> AppResult<()> {
        let session = self.sessions.lock().unwrap().remove(id);
        if let Some(session) = session {
            let _ = session.commands.send(SessionCommand::Close).await;
            let _ = session
                .handle
                .disconnect(Disconnect::ByApplication, "", "")
                .await;
        }
        Ok(())
    }
}
