//! "Edit remote file" (FT-11): download a remote file to a local temp copy,
//! open it in an external editor, then watch the copy — and on every save ask
//! the user whether to upload it back to the remote (MobaXterm-style round-trip
//! edit, with an explicit confirm step rather than a silent push).
//!
//! The watcher polls the temp file's mtime (no extra filesystem-notify crate)
//! and, on change, emits `sftp://edit-changed`; the frontend confirms and then
//! calls back into `upload_saved`. The watcher runs for the life of the session:
//! when the session goes away (disconnect or reconnect, which mints a new id)
//! the next poll can't resolve a handle, so the task stops and cleans up its
//! temp dir.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::session::SessionManager;

/// Tracks remote files currently open for editing, keyed by `id\0remote_path`,
/// so re-opening the same file reuses its temp copy (preserving unsaved edits)
/// instead of spawning a second watcher.
#[derive(Default)]
pub struct EditManager {
    active: Mutex<HashMap<String, PathBuf>>,
}

impl EditManager {
    fn key(id: &str, remote: &str) -> String {
        format!("{id}\u{0}{remote}")
    }
    fn get(&self, id: &str, remote: &str) -> Option<PathBuf> {
        self.active
            .lock()
            .unwrap()
            .get(&Self::key(id, remote))
            .cloned()
    }
    fn insert(&self, id: &str, remote: &str, local: PathBuf) {
        self.active
            .lock()
            .unwrap()
            .insert(Self::key(id, remote), local);
    }
    fn remove(&self, id: &str, remote: &str) {
        self.active.lock().unwrap().remove(&Self::key(id, remote));
    }
}

/// Emitted when a temp copy is saved locally, so the panel can ask the user
/// whether to upload it back (they then call `upload_saved`).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditChangedPayload {
    id: String,
    remote_path: String,
    name: String,
}

/// The trailing path component, used as the temp filename and for editor titles.
fn base_name(remote_path: &str) -> &str {
    remote_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("file")
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Download `remote_path` to a temp copy, open it in `editor` (OS default when
/// empty), and start watching it for saves. Re-opening a file already being
/// edited just re-launches the editor on its existing copy.
pub async fn open_for_edit(
    app: AppHandle,
    id: String,
    remote_path: String,
    editor: String,
) -> AppResult<()> {
    let handle = app.state::<SessionManager>().handle(&id)?;
    let name = base_name(&remote_path).to_string();

    // Re-editing an open file: keep the existing copy (unsaved edits intact) and
    // just bring the editor back to it.
    if let Some(existing) = app.state::<EditManager>().get(&id, &remote_path) {
        if existing.exists() {
            launch_editor(&app, &existing, &editor)?;
            return Ok(());
        }
        app.state::<EditManager>().remove(&id, &remote_path);
    }

    // Each edit gets its own temp dir so distinct files of the same name (or two
    // sessions) never collide, while the basename is preserved for the editor.
    let dir = app
        .path()
        .temp_dir()
        .map_err(|e| AppError::Other(format!("cannot resolve temp dir: {e}")))?
        .join("ammaxterm-edit")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir)?;
    let local = dir.join(&name);
    let local_str = local.to_string_lossy().to_string();

    crate::sftp::download(&handle, &remote_path, &local_str).await?;

    app.state::<EditManager>()
        .insert(&id, &remote_path, local.clone());
    launch_editor(&app, &local, &editor)?;
    spawn_watcher(app, id, remote_path, name, local);
    Ok(())
}

/// Open `path` in the configured editor, or the OS default association when the
/// editor string is blank.
fn launch_editor(app: &AppHandle, path: &Path, editor: &str) -> AppResult<()> {
    let editor = editor.trim();
    if editor.is_empty() {
        return app
            .opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| AppError::Other(format!("open in editor: {e}")));
    }
    let mut tokens = tokenize(editor);
    if tokens.is_empty() {
        return Err(AppError::Other("editor command is empty".into()));
    }
    let program = tokens.remove(0);
    let file = path.to_string_lossy().to_string();
    // Substitute a `{file}` placeholder if present, else append the path.
    let mut substituted = false;
    let args: Vec<String> = tokens
        .into_iter()
        .map(|t| {
            if t.contains("{file}") {
                substituted = true;
                t.replace("{file}", &file)
            } else {
                t
            }
        })
        .collect();
    let mut cmd = std::process::Command::new(&program);
    cmd.args(&args);
    if !substituted {
        cmd.arg(&file);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| AppError::Other(format!("launch editor '{program}': {e}")))
}

/// Split a command string into tokens, honouring double-quoted spans so a
/// program path with spaces stays one token. Good enough for an editor command
/// like `code -w` or `"C:\\Program Files\\…\\notepad++.exe" -multiInst`.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    out.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        out.push(cur);
    }
    out
}

/// Upload the temp copy of an in-progress edit back to the remote. Called by the
/// frontend once the user confirms a detected save (`sftp://edit-changed`).
pub async fn upload_saved(app: AppHandle, id: String, remote_path: String) -> AppResult<()> {
    let local = app
        .state::<EditManager>()
        .get(&id, &remote_path)
        .ok_or_else(|| AppError::Other("this file is no longer open for editing".into()))?;
    let handle = app.state::<SessionManager>().handle(&id)?;
    crate::sftp::upload(&handle, &local.to_string_lossy(), &remote_path).await
}

/// Poll the temp copy and, whenever it is saved, ask the frontend to confirm an
/// upload. Runs until the session is gone or the file is deleted, then cleans up
/// the temp dir.
fn spawn_watcher(app: AppHandle, id: String, remote_path: String, name: String, local: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let mut last = mtime(&local);
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            // Session gone (disconnect / reconnect → new id) → stop watching.
            if app.state::<SessionManager>().handle(&id).is_err() {
                break;
            }
            // The editor (or the user) removed the temp file → done.
            let Some(current) = mtime(&local) else { break };

            if last.map(|l| current > l).unwrap_or(true) {
                last = Some(current);
                // Ask the user whether to push this save; the upload itself runs
                // via `upload_saved` only if they confirm.
                let _ = app.emit(
                    "sftp://edit-changed",
                    EditChangedPayload {
                        id: id.clone(),
                        remote_path: remote_path.clone(),
                        name: name.clone(),
                    },
                );
            }
        }
        app.state::<EditManager>().remove(&id, &remote_path);
        if let Some(parent) = local.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_extracts_trailing_component() {
        assert_eq!(base_name("/etc/nginx/nginx.conf"), "nginx.conf");
        assert_eq!(base_name("file.txt"), "file.txt");
        assert_eq!(base_name("/trailing/slash/"), "slash");
        assert_eq!(base_name("/"), "file");
    }

    #[test]
    fn tokenize_handles_quotes_and_args() {
        assert_eq!(tokenize("code -w"), vec!["code", "-w"]);
        assert_eq!(
            tokenize("\"C:\\Program Files\\app.exe\" -n {file}"),
            vec!["C:\\Program Files\\app.exe", "-n", "{file}"]
        );
        assert!(tokenize("   ").is_empty());
    }
}
