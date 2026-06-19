//! Tauri command surface for SSH sessions, SFTP, saved sites, and secrets.

use std::fs;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::secrets::{self, SecretKind};
use crate::session::SessionManager;
use crate::sftp::FileEntry;
use crate::ssh::{AuthCredential, ConnectOptions, ConnectRequest, HostKeyPrompts};
use crate::store::{AuthMethod, Site, SiteInput, SiteStore};

/// Resolve (and ensure) the app config directory.
fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(format!("cannot resolve app config dir: {e}")))?;
    let _ = fs::create_dir_all(&dir);
    Ok(dir)
}

// --- SSH sessions ---

/// Open an SSH session from ad-hoc options (Quick Connect, password auth).
#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    options: ConnectOptions,
    on_output: Channel<String>,
    manager: State<'_, SessionManager>,
    prompts: State<'_, HostKeyPrompts>,
) -> AppResult<String> {
    let known_hosts = config_dir(&app)?.join("known_hosts");
    let prompts = prompts.inner().clone();
    manager
        .connect(app, prompts, options.into_request(), known_hosts, on_output)
        .await
}

/// Open an SSH session from a saved site, resolving its secret from the keychain.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn site_connect(
    app: AppHandle,
    site_id: String,
    cols: u32,
    rows: u32,
    on_output: Channel<String>,
    store: State<'_, SiteStore>,
    manager: State<'_, SessionManager>,
    prompts: State<'_, HostKeyPrompts>,
) -> AppResult<String> {
    let site = store.get(&site_id)?;
    let known_hosts = config_dir(&app)?.join("known_hosts");

    let auth = match site.auth {
        AuthMethod::Password => AuthCredential::Password(
            secrets::get(SecretKind::Password, &site_id)?
                .ok_or_else(|| AppError::Auth("no saved password for this site".into()))?,
        ),
        AuthMethod::PublicKey { key_path } => AuthCredential::PublicKey {
            key_path,
            passphrase: secrets::get(SecretKind::Passphrase, &site_id)?,
        },
        AuthMethod::KeyboardInteractive => AuthCredential::KeyboardInteractive(
            secrets::get(SecretKind::Password, &site_id)?
                .ok_or_else(|| AppError::Auth("no saved secret for this site".into()))?,
        ),
        AuthMethod::Agent => {
            return Err(AppError::Other(
                "SSH agent auth is not supported yet".into(),
            ))
        }
    };

    let req = ConnectRequest {
        host: site.host,
        port: site.port,
        username: site.username,
        auth,
        cols,
        rows,
    };
    manager
        .connect(app, prompts.inner().clone(), req, known_hosts, on_output)
        .await
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

/// Resolve a pending host-key prompt with the user's trust decision (TM-6).
#[tauri::command]
pub fn host_key_decision(request_id: String, trust: bool, prompts: State<'_, HostKeyPrompts>) {
    prompts.resolve(&request_id, trust);
}

// --- SFTP ---

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

/// Create a remote directory (FT-3).
#[tauri::command]
pub async fn sftp_mkdir(
    id: String,
    path: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    let handle = manager.handle(&id)?;
    crate::sftp::make_dir(&handle, &path).await
}

/// Rename or move a remote file/directory (FT-3).
#[tauri::command]
pub async fn sftp_rename(
    id: String,
    from: String,
    to: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    let handle = manager.handle(&id)?;
    crate::sftp::rename(&handle, &from, &to).await
}

/// Delete a remote file or directory (recursive for dirs) (FT-3).
#[tauri::command]
pub async fn sftp_delete(
    id: String,
    path: String,
    is_dir: bool,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    let handle = manager.handle(&id)?;
    crate::sftp::remove(&handle, &path, is_dir).await
}

// --- Saved sites (SM-1) ---

/// List all saved sites.
#[tauri::command]
pub fn site_list(store: State<'_, SiteStore>) -> Vec<Site> {
    store.list()
}

/// Create a new site; returns it with its assigned id.
#[tauri::command]
pub fn site_add(input: SiteInput, store: State<'_, SiteStore>) -> AppResult<Site> {
    store.add(input)
}

/// Update an existing site.
#[tauri::command]
pub fn site_update(id: String, input: SiteInput, store: State<'_, SiteStore>) -> AppResult<Site> {
    store.update(&id, input)
}

/// Delete a site and its stored secrets.
#[tauri::command]
pub fn site_delete(id: String, store: State<'_, SiteStore>) -> AppResult<()> {
    store.delete(&id)?;
    let _ = secrets::delete_all(&id);
    Ok(())
}

// --- Secrets (AK-1) ---

/// Store/replace a site's password in the OS keychain.
#[tauri::command]
pub fn site_set_password(site_id: String, password: String) -> AppResult<()> {
    secrets::set(SecretKind::Password, &site_id, &password)
}

/// Store/replace a site's private-key passphrase in the OS keychain.
#[tauri::command]
pub fn site_set_passphrase(site_id: String, passphrase: String) -> AppResult<()> {
    secrets::set(SecretKind::Passphrase, &site_id, &passphrase)
}
