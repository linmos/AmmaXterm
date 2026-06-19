//! Tauri command surface for SSH sessions and SFTP.

use std::fs;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::session::SessionManager;
use crate::sftp::FileEntry;
use crate::ssh::ConnectOptions;

/// Open an SSH session. `on_output` is a Tauri channel the backend streams
/// base64-encoded shell output through; returns the new session id.
#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    options: ConnectOptions,
    on_output: Channel<String>,
    manager: State<'_, SessionManager>,
) -> AppResult<String> {
    // App-private known_hosts (OpenSSH format) under the app config dir.
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(format!("cannot resolve app config dir: {e}")))?;
    let _ = fs::create_dir_all(&config_dir);
    let known_hosts = config_dir.join("known_hosts");

    manager.connect(options, known_hosts, on_output).await
}

/// Send user input (keystrokes / paste) to a session's shell.
#[tauri::command]
pub async fn ssh_send_input(
    id: String,
    data: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    manager.send_input(&id, data.into_bytes()).await
}

/// Notify the remote of a terminal resize (window-change).
#[tauri::command]
pub async fn ssh_resize(
    id: String,
    cols: u32,
    rows: u32,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    manager.resize(&id, cols, rows).await
}

/// Close a session.
#[tauri::command]
pub async fn ssh_disconnect(id: String, manager: State<'_, SessionManager>) -> AppResult<()> {
    manager.disconnect(&id).await
}

/// List a remote directory over SFTP (FT-1).
#[tauri::command]
pub async fn sftp_list(
    id: String,
    path: String,
    manager: State<'_, SessionManager>,
) -> AppResult<Vec<FileEntry>> {
    let handle = manager.handle(&id)?;
    crate::sftp::list_dir(&handle, &path).await
}

/// Upload a local file to the remote host over SFTP (FT-2).
#[tauri::command]
pub async fn sftp_upload(
    id: String,
    local_path: String,
    remote_path: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    let handle = manager.handle(&id)?;
    crate::sftp::upload(&handle, &local_path, &remote_path).await
}

/// Download a remote file to the local machine over SFTP (FT-2).
#[tauri::command]
pub async fn sftp_download(
    id: String,
    remote_path: String,
    local_path: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    let handle = manager.handle(&id)?;
    crate::sftp::download(&handle, &remote_path, &local_path).await
}
