//! Tauri command surface for SSH sessions, SFTP, saved sites, and secrets.

use std::fs;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::importer::{self, ImportedSite};
use crate::secrets::{self, SecretKind};
use crate::session::SessionManager;
use crate::settings::{Settings, SettingsStore};
use crate::sftp::FileEntry;
use crate::ssh::{AuthCredential, ConnectOptions, ConnectRequest, HostKeyPrompts};
use crate::store::{AuthMethod, Site, SiteInput, SiteStore};
use crate::tunnel::{TunnelInfo, TunnelManager, TunnelSpec};

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
    settings: State<'_, SettingsStore>,
) -> AppResult<String> {
    let known_hosts = config_dir(&app)?.join("known_hosts");
    let prompts = prompts.inner().clone();
    let keepalive = settings.get().keepalive_secs;
    manager
        .connect(
            app,
            prompts,
            options.into_request(),
            known_hosts,
            keepalive,
            on_output,
        )
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
    settings: State<'_, SettingsStore>,
) -> AppResult<String> {
    let site = store.get(&site_id)?;
    let known_hosts = config_dir(&app)?.join("known_hosts");
    let keepalive = settings.get().keepalive_secs;

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
        .connect(
            app,
            prompts.inner().clone(),
            req,
            known_hosts,
            keepalive,
            on_output,
        )
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

/// Close a session (and tear down any tunnels bound to it).
#[tauri::command]
pub async fn ssh_disconnect(
    id: String,
    manager: State<'_, SessionManager>,
    tunnels: State<'_, TunnelManager>,
) -> AppResult<()> {
    tunnels.close_for_session(&id);
    manager.disconnect(&id).await
}

/// Start logging a session's output to a local file (TM-12).
#[tauri::command]
pub async fn session_start_log(
    id: String,
    path: String,
    manager: State<'_, SessionManager>,
) -> AppResult<()> {
    manager.start_log(&id, std::path::PathBuf::from(path)).await
}

/// Stop logging a session's output.
#[tauri::command]
pub async fn session_stop_log(id: String, manager: State<'_, SessionManager>) -> AppResult<()> {
    manager.stop_log(&id).await
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

// --- Import / export (SM-7, SM-8) ---

/// Resolve the user's home directory across platforms.
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}

/// Parse an OpenSSH `config` file into review candidates (SM-7). When `path` is
/// omitted, the default `~/.ssh/config` is used.
#[tauri::command]
pub fn import_ssh_config(path: Option<String>) -> AppResult<Vec<ImportedSite>> {
    let path = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => home_dir()
            .ok_or_else(|| AppError::Other("cannot resolve home directory".into()))?
            .join(".ssh")
            .join("config"),
    };
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::Other(format!("cannot read {}: {e}", path.display())))?;
    Ok(importer::parse_openssh_config(&text))
}

/// Read an AmmaXterm backup file into review candidates (SM-8 restore).
#[tauri::command]
pub fn import_sites_backup(path: String) -> AppResult<Vec<ImportedSite>> {
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::Other(format!("cannot read {path}: {e}")))?;
    importer::read_backup(&text)
}

/// Export all saved sites to a backup file (SM-8). No secrets are written.
#[tauri::command]
pub fn export_sites(path: String, store: State<'_, SiteStore>) -> AppResult<()> {
    store.export_to(std::path::Path::new(&path))
}

// --- Port forwarding / tunnels (PF-1..PF-7) ---

/// Open a tunnel over an active session; returns the tunnel id.
#[tauri::command]
pub async fn tunnel_open(
    session_id: String,
    spec: TunnelSpec,
    manager: State<'_, SessionManager>,
    tunnels: State<'_, TunnelManager>,
) -> AppResult<String> {
    let handle = manager.handle(&session_id)?;
    tunnels.open(session_id, spec, handle).await
}

/// Close a single tunnel.
#[tauri::command]
pub fn tunnel_close(id: String, tunnels: State<'_, TunnelManager>) {
    tunnels.close(&id);
}

/// List all active tunnels (for the management panel, PF-5).
#[tauri::command]
pub fn tunnel_list(tunnels: State<'_, TunnelManager>) -> Vec<TunnelInfo> {
    tunnels.list()
}

// --- Settings (TM-11, ST-1, ST-2) ---

/// Read the global settings.
#[tauri::command]
pub fn settings_get(settings: State<'_, SettingsStore>) -> Settings {
    settings.get()
}

/// Replace the global settings; returns the persisted value.
#[tauri::command]
pub fn settings_set(value: Settings, settings: State<'_, SettingsStore>) -> AppResult<Settings> {
    settings.set(value)
}
