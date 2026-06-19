//! SFTP transfer queue with live progress and cancel/retry (FT-4).
//!
//! Each transfer runs as its own task that streams in chunks (see
//! `sftp::upload_streaming` / `download_streaming`), updating a shared byte
//! counter the frontend polls via `transfer_list`. Pause + resume are FT-7
//! (resumable transfers) and not implemented here; cancel + retry are.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::error::AppResult;
use crate::session::SessionManager;
use crate::ssh::SshHandle;

/// A queued/active/finished transfer.
struct Transfer {
    name: String,
    direction: &'static str, // "upload" | "download"
    session_id: String,
    local_path: String,
    remote_path: String,
    total: u64,
    done: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    status: Arc<Mutex<(String, Option<String>)>>, // (state, error)
}

/// Snapshot for the transfer panel.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferInfo {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub direction: String,
    pub total: u64,
    pub done: u64,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct TransferManager {
    items: Mutex<HashMap<String, Arc<Transfer>>>,
    counter: AtomicU64,
}

impl TransferManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> String {
        format!("xfer-{}", self.counter.fetch_add(1, Ordering::Relaxed))
    }

    /// Queue an upload and start it; returns the transfer id.
    pub async fn enqueue_upload(
        &self,
        handle: Arc<SshHandle>,
        session_id: String,
        local_path: String,
        remote_path: String,
    ) -> AppResult<String> {
        let total = tokio::fs::metadata(&local_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        let name = base_name(&local_path);
        let t = Arc::new(Transfer {
            name,
            direction: "upload",
            session_id,
            local_path,
            remote_path,
            total,
            done: Arc::new(AtomicU64::new(0)),
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(("active".into(), None))),
        });
        let id = self.next_id();
        self.items.lock().unwrap().insert(id.clone(), t.clone());
        spawn_run(t, handle, false);
        Ok(id)
    }

    /// Queue a download and start it; returns the transfer id.
    pub async fn enqueue_download(
        &self,
        handle: Arc<SshHandle>,
        session_id: String,
        remote_path: String,
        local_path: String,
    ) -> AppResult<String> {
        let total = crate::sftp::remote_size(&handle, &remote_path)
            .await
            .unwrap_or(0);
        let name = base_name(&remote_path);
        let t = Arc::new(Transfer {
            name,
            direction: "download",
            session_id,
            local_path,
            remote_path,
            total,
            done: Arc::new(AtomicU64::new(0)),
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(("active".into(), None))),
        });
        let id = self.next_id();
        self.items.lock().unwrap().insert(id.clone(), t.clone());
        spawn_run(t, handle, false);
        Ok(id)
    }

    pub fn cancel(&self, id: &str) {
        if let Some(t) = self.items.lock().unwrap().get(id) {
            t.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Pause an active transfer (FT-7); progress is kept for a later resume.
    pub fn pause(&self, id: &str) {
        if let Some(t) = self.items.lock().unwrap().get(id) {
            t.pause.store(true, Ordering::Relaxed);
        }
    }

    /// Resume a paused transfer, continuing from the bytes already transferred.
    pub async fn resume(&self, id: &str, manager: &SessionManager) -> AppResult<()> {
        self.restart(id, manager, true).await
    }

    /// Retry a canceled/errored transfer, continuing from any partial bytes.
    pub async fn retry(&self, id: &str, manager: &SessionManager) -> AppResult<()> {
        self.restart(id, manager, true).await
    }

    async fn restart(&self, id: &str, manager: &SessionManager, resume: bool) -> AppResult<()> {
        let t = match self.items.lock().unwrap().get(id) {
            Some(t) => t.clone(),
            None => return Ok(()),
        };
        let handle = manager.handle(&t.session_id)?;
        t.cancel.store(false, Ordering::Relaxed);
        t.pause.store(false, Ordering::Relaxed);
        *t.status.lock().unwrap() = ("active".into(), None);
        spawn_run(t, handle, resume);
        Ok(())
    }

    /// Remove a finished entry from the list (no effect on active transfers).
    pub fn clear(&self, id: &str) {
        let mut guard = self.items.lock().unwrap();
        if let Some(t) = guard.get(id) {
            if t.status.lock().unwrap().0 == "active" {
                return;
            }
        }
        guard.remove(id);
    }

    pub fn list(&self) -> Vec<TransferInfo> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .map(|(id, t)| {
                let (state, error) = t.status.lock().unwrap().clone();
                TransferInfo {
                    id: id.clone(),
                    session_id: t.session_id.clone(),
                    name: t.name.clone(),
                    direction: t.direction.to_string(),
                    total: t.total,
                    done: t.done.load(Ordering::Relaxed),
                    status: state,
                    error,
                }
            })
            .collect()
    }
}

const MAX_RETRIES: u32 = 3;

/// Bytes already durably present on the receiving side, used as the resume
/// offset: the remote file size for uploads, the local file size for downloads.
async fn resume_offset(t: &Transfer, handle: &SshHandle) -> u64 {
    if t.direction == "upload" {
        crate::sftp::remote_size(handle, &t.remote_path)
            .await
            .unwrap_or(0)
    } else {
        tokio::fs::metadata(&t.local_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0)
    }
}

/// Spawn the transfer task: streams with auto-retry (resuming from the already
/// transferred bytes) and honours cancel/pause. Records the final status.
fn spawn_run(t: Arc<Transfer>, handle: Arc<SshHandle>, resume: bool) {
    tauri::async_runtime::spawn(async move {
        let mut attempt = 0u32;
        loop {
            let offset = if attempt == 0 && !resume {
                0
            } else {
                resume_offset(&t, &handle).await
            };
            t.done.store(offset, Ordering::Relaxed);

            let result = if t.direction == "upload" {
                crate::sftp::upload_streaming(
                    &handle,
                    &t.local_path,
                    &t.remote_path,
                    &t.done,
                    &t.cancel,
                    &t.pause,
                    offset,
                )
                .await
            } else {
                crate::sftp::download_streaming(
                    &handle,
                    &t.remote_path,
                    &t.local_path,
                    &t.done,
                    &t.cancel,
                    &t.pause,
                    offset,
                )
                .await
            };

            // Stop reasons take priority over the stream's Ok/Err.
            if t.cancel.load(Ordering::Relaxed) {
                *t.status.lock().unwrap() = ("canceled".into(), None);
                return;
            }
            if t.pause.load(Ordering::Relaxed) {
                *t.status.lock().unwrap() = ("paused".into(), None);
                return;
            }
            match result {
                Ok(()) => {
                    *t.status.lock().unwrap() = ("done".into(), None);
                    return;
                }
                Err(e) => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        *t.status.lock().unwrap() = ("error".into(), Some(e.to_string()));
                        return;
                    }
                    // Exponential backoff before resuming.
                    tokio::time::sleep(std::time::Duration::from_secs(1u64 << attempt)).await;
                }
            }
        }
    });
}

fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}
