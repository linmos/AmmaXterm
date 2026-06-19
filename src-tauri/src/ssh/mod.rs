//! SSH connection, interactive PTY shell, host-key verification, and the
//! per-session streaming actor.
//!
//! Each session runs as an independent async task so one failing session never
//! affects the others (PRD §6.4). The connection `Handle` is shared (`Arc`) so
//! additional channels (e.g. SFTP) can be opened on the same connection.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::Engine;
use russh::client::{self, Handle, Msg};
use russh::keys::ssh_key;
use russh::ChannelMsg;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};

/// Shared SSH connection handle; used to open the shell and SFTP channels.
pub(crate) type SshHandle = Handle<ClientHandler>;

/// Options needed to open an SSH session. (M0: password auth only; public-key
/// and keyboard-interactive arrive with credential management in M1, TM-2.)
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct ConnectOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub cols: u32,
    pub rows: u32,
}

/// Commands sent from the Tauri command layer to a running session actor.
pub(crate) enum SessionCommand {
    Input(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

/// Shared slot the handler uses to report a host-key problem back to `connect`,
/// since `check_server_key` can only signal accept/reject via a bool.
#[derive(Clone, Default)]
struct HostKeyReport(Arc<Mutex<Option<String>>>);

/// russh client event handler with `known_hosts` verification (TM-6).
pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    known_hosts: PathBuf,
    report: HostKeyReport,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match russh::keys::check_known_hosts_path(
            &self.host,
            self.port,
            server_public_key,
            &self.known_hosts,
        ) {
            // Known host, key matches.
            Ok(true) => Ok(true),
            // Unknown host: trust on first use and record it.
            // TODO(M1, TM-6): prompt the user with the fingerprint before trusting.
            Ok(false) => {
                let _ = russh::keys::known_hosts::learn_known_hosts_path(
                    &self.host,
                    self.port,
                    server_public_key,
                    &self.known_hosts,
                );
                Ok(true)
            }
            // Key changed or other verification error: refuse (fail closed).
            Err(e) => {
                let fp = server_public_key.fingerprint(ssh_key::HashAlg::Sha256);
                let msg = match &e {
                    russh::keys::Error::KeyChanged { line } => format!(
                        "Host key for {}:{} CHANGED (known_hosts line {line}). \
                         New key fingerprint {fp}. Possible man-in-the-middle — refusing to connect.",
                        self.host, self.port
                    ),
                    other => format!(
                        "Host key verification failed for {}:{}: {other}",
                        self.host, self.port
                    ),
                };
                *self.report.0.lock().unwrap() = Some(msg);
                Ok(false)
            }
        }
    }
}

/// Connect, verify the host key, authenticate, open an interactive shell, and
/// spawn the session actor. Returns the command sender and the shared handle.
pub(crate) async fn connect(
    opts: &ConnectOptions,
    known_hosts: PathBuf,
    output: Channel<String>,
) -> AppResult<(mpsc::Sender<SessionCommand>, Arc<SshHandle>)> {
    let (handle, channel) = open_shell(opts, known_hosts).await?;
    let (tx, rx) = mpsc::channel(64);
    tauri::async_runtime::spawn(run_session(channel, rx, output));
    Ok((tx, Arc::new(handle)))
}

async fn open_shell(
    opts: &ConnectOptions,
    known_hosts: PathBuf,
) -> AppResult<(SshHandle, russh::Channel<Msg>)> {
    let config = Arc::new(client::Config::default());
    let report = HostKeyReport::default();
    let handler = ClientHandler {
        host: opts.host.clone(),
        port: opts.port,
        known_hosts,
        report: report.clone(),
    };

    let mut handle = match client::connect(config, (opts.host.as_str(), opts.port), handler).await {
        Ok(h) => h,
        Err(e) => {
            // Prefer the specific host-key message if the handshake failed on it.
            if let Some(msg) = report.0.lock().unwrap().take() {
                return Err(AppError::HostKey(msg));
            }
            return Err(AppError::Connect(e.to_string()));
        }
    };

    let auth = handle
        .authenticate_password(opts.username.clone(), opts.password.clone())
        .await?;
    if !auth.success() {
        return Err(AppError::Auth("password authentication failed".into()));
    }

    let channel = handle.channel_open_session().await?;
    channel
        .request_pty(false, "xterm-256color", opts.cols, opts.rows, 0, 0, &[])
        .await?;
    channel.request_shell(true).await?;

    Ok((handle, channel))
}

/// Per-session actor: forwards user input/resize to the SSH shell channel and
/// streams shell output back to the frontend (base64 over a Tauri `Channel`).
/// The connection stays alive via the `Arc<SshHandle>` held by `SessionManager`.
async fn run_session(
    mut channel: russh::Channel<Msg>,
    mut rx: mpsc::Receiver<SessionCommand>,
    output: Channel<String>,
) {
    let b64 = base64::engine::general_purpose::STANDARD;

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    Some(SessionCommand::Input(bytes)) => {
                        if channel.data_bytes(bytes).await.is_err() {
                            break;
                        }
                    }
                    Some(SessionCommand::Resize { cols, rows }) => {
                        let _ = channel.window_change(cols, rows, 0, 0).await;
                    }
                    Some(SessionCommand::Close) | None => {
                        let _ = channel.eof().await;
                        break;
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { ref data }) => {
                        if output.send(b64.encode(&data[..])).is_err() {
                            break; // frontend channel gone
                        }
                    }
                    Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                        let _ = output.send(b64.encode(&data[..]));
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }
}
