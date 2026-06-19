//! SSH connection, authentication, interactive PTY shell, host-key
//! verification (with an interactive prompt), and the per-session streaming actor.
//!
//! Each session runs as an independent async task so one failing session never
//! affects the others (PRD §6.4). The connection `Handle` is shared (`Arc`) so
//! additional channels (e.g. SFTP) can be opened on the same connection.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse, Msg};
use russh::keys::ssh_key;
use russh::ChannelMsg;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

use crate::error::{AppError, AppResult};

/// Shared SSH connection handle; used to open the shell and SFTP channels.
pub(crate) type SshHandle = Handle<ClientHandler>;

/// Per-connection registry of active remote forwards (-R, PF-3): maps the
/// server-side bind port to the client-side target `(host, port)`. Populated by
/// the tunnel manager; read by the handler when the server opens a forwarded
/// channel back to us.
pub(crate) type RemoteForwards = Arc<Mutex<HashMap<u16, (String, u16)>>>;

/// Quick-connect options coming straight from the frontend (password auth).
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct ConnectOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub cols: u32,
    pub rows: u32,
}

impl ConnectOptions {
    pub fn into_request(self) -> ConnectRequest {
        ConnectRequest {
            host: self.host,
            port: self.port,
            username: self.username,
            auth: AuthCredential::Password(self.password),
            jumps: Vec::new(),
            cols: self.cols,
            rows: self.rows,
        }
    }
}

/// A credential resolved at connect time (secrets already fetched). TM-2.
pub(crate) enum AuthCredential {
    Password(String),
    PublicKey {
        key_path: String,
        passphrase: Option<String>,
    },
    KeyboardInteractive(String),
}

/// One hop in a ProxyJump chain (TM-9): a resolved jump host the connection
/// tunnels through before reaching the final target.
pub(crate) struct HopRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthCredential,
}

/// A fully-resolved connection request. `jumps` (if any) are dialed in order,
/// each tunnelled through the previous, with the final target reached over the
/// last jump (TM-9 ProxyJump).
pub(crate) struct ConnectRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthCredential,
    pub jumps: Vec<HopRequest>,
    pub cols: u32,
    pub rows: u32,
}

/// Commands sent from the Tauri command layer to a running session actor.
pub(crate) enum SessionCommand {
    Input(Vec<u8>),
    Resize {
        cols: u32,
        rows: u32,
    },
    /// Begin appending shell output to a local file (TM-12).
    StartLog(PathBuf),
    /// Stop logging and flush the file.
    StopLog,
    Close,
}

/// Registry of in-flight host-key prompts, keyed by request id. Managed as
/// Tauri state so the `host_key_decision` command can resolve a pending prompt.
#[derive(Clone, Default)]
pub struct HostKeyPrompts(Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>);

impl HostKeyPrompts {
    fn insert(&self, id: String, tx: oneshot::Sender<bool>) {
        self.0.lock().unwrap().insert(id, tx);
    }
    fn remove(&self, id: &str) {
        self.0.lock().unwrap().remove(id);
    }
    /// Resolve a pending prompt with the user's trust decision.
    pub fn resolve(&self, id: &str, trust: bool) {
        if let Some(tx) = self.0.lock().unwrap().remove(id) {
            let _ = tx.send(trust);
        }
    }
}

/// Lets the host-key handler prompt the frontend and await the user's decision.
/// Cloneable so each hop in a ProxyJump chain gets its own prompter (TM-9).
#[derive(Clone)]
pub(crate) struct HostKeyPrompter {
    pub app: AppHandle,
    pub prompts: HostKeyPrompts,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HostKeyPromptPayload {
    request_id: String,
    host: String,
    port: u16,
    fingerprint: String,
    changed: bool,
}

impl HostKeyPrompter {
    /// Emit a prompt to the frontend and await trust/reject (120s timeout → reject).
    async fn ask(&self, host: &str, port: u16, fingerprint: &str, changed: bool) -> bool {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.prompts.insert(request_id.clone(), tx);
        let _ = self.app.emit(
            "ssh://host-key-prompt",
            HostKeyPromptPayload {
                request_id: request_id.clone(),
                host: host.to_string(),
                port,
                fingerprint: fingerprint.to_string(),
                changed,
            },
        );
        match tokio::time::timeout(Duration::from_secs(120), rx).await {
            Ok(Ok(trust)) => trust,
            _ => {
                self.prompts.remove(&request_id);
                false
            }
        }
    }
}

/// Shared slot the handler uses to report a host-key problem back to `open_shell`.
#[derive(Clone, Default)]
struct HostKeyReport(Arc<Mutex<Option<String>>>);

/// russh client event handler with interactive `known_hosts` verification (TM-6).
///
/// With a `prompter` (normal app use) the user confirms unknown/changed keys;
/// without one (headless/tests) it falls back to trust-on-first-use and refuses
/// changed keys.
pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    known_hosts: PathBuf,
    prompter: Option<HostKeyPrompter>,
    report: HostKeyReport,
    remote_forwards: RemoteForwards,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string();

        match russh::keys::check_known_hosts_path(
            &self.host,
            self.port,
            server_public_key,
            &self.known_hosts,
        ) {
            // Known host, key matches.
            Ok(true) => Ok(true),
            // Unknown host: ask the user (or trust-on-first-use when headless).
            Ok(false) => {
                let trust = match &self.prompter {
                    Some(p) => p.ask(&self.host, self.port, &fingerprint, false).await,
                    None => true,
                };
                if trust {
                    let _ = russh::keys::known_hosts::learn_known_hosts_path(
                        &self.host,
                        self.port,
                        server_public_key,
                        &self.known_hosts,
                    );
                    Ok(true)
                } else {
                    *self.report.0.lock().unwrap() = Some(format!(
                        "Host key for {}:{} was not trusted (fingerprint {fingerprint}).",
                        self.host, self.port
                    ));
                    Ok(false)
                }
            }
            // Key changed: ask the user, defaulting to reject; never auto-accept headless.
            Err(russh::keys::Error::KeyChanged { line }) => {
                let trust = match &self.prompter {
                    Some(p) => p.ask(&self.host, self.port, &fingerprint, true).await,
                    None => false,
                };
                if trust {
                    let _ = forget_host(&self.known_hosts, &self.host, self.port);
                    let _ = russh::keys::known_hosts::learn_known_hosts_path(
                        &self.host,
                        self.port,
                        server_public_key,
                        &self.known_hosts,
                    );
                    Ok(true)
                } else {
                    *self.report.0.lock().unwrap() = Some(format!(
                        "Host key for {}:{} CHANGED (known_hosts line {line}); new fingerprint \
                         {fingerprint}. Rejected — possible man-in-the-middle.",
                        self.host, self.port
                    ));
                    Ok(false)
                }
            }
            Err(e) => {
                *self.report.0.lock().unwrap() = Some(format!(
                    "Host key verification failed for {}:{}: {e}",
                    self.host, self.port
                ));
                Ok(false)
            }
        }
    }

    /// Server opened a remote-forward channel (-R, PF-3): splice it to a fresh
    /// TCP connection to the client-side target registered for this bind port.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<Msg>,
        _connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let target = self
            .remote_forwards
            .lock()
            .unwrap()
            .get(&(connected_port as u16))
            .cloned();
        if let Some((host, port)) = target {
            tokio::spawn(async move {
                if let Ok(mut tcp) = tokio::net::TcpStream::connect((host.as_str(), port)).await {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                }
            });
        }
        Ok(())
    }
}

/// Rewrite `known_hosts`, dropping entries for `host[:port]`. Entries written by
/// `learn_known_hosts_path` use plain (un-hashed) host fields, so a plain match
/// is sufficient for keys this app recorded.
fn forget_host(path: &PathBuf, host: &str, port: u16) -> std::io::Result<()> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    let plain = host.to_string();
    let with_port = format!("[{host}]:{port}");
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| {
            let first = line.split_whitespace().next().unwrap_or("");
            !first.split(',').any(|h| h == plain || h == with_port)
        })
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    std::fs::write(path, out)
}

/// Connect, verify the host key, authenticate, and open an interactive shell.
///
/// `keepalive_secs` (0 = off) sets the SSH keepalive interval; if the peer stops
/// answering, russh drops the connection and the session emits `ssh://closed`,
/// which the frontend can act on to reconnect (TM-8).
///
/// When `req.jumps` is non-empty the connection is tunnelled through each jump
/// host in order (TM-9 ProxyJump). The returned `Vec<SshHandle>` holds the jump
/// connections, which the caller must keep alive for the session's lifetime
/// (dropping them tears down the tunnel underneath the target).
pub(crate) async fn open_shell(
    req: &ConnectRequest,
    known_hosts: PathBuf,
    prompter: Option<HostKeyPrompter>,
    keepalive_secs: u32,
) -> AppResult<(
    SshHandle,
    russh::Channel<Msg>,
    RemoteForwards,
    Vec<SshHandle>,
)> {
    let mut cfg = client::Config::default();
    if keepalive_secs > 0 {
        cfg.keepalive_interval = Some(Duration::from_secs(keepalive_secs as u64));
    }
    let config = Arc::new(cfg);
    let remote_forwards: RemoteForwards = Arc::new(Mutex::new(HashMap::new()));

    // Walk jump hosts (if any), then the final target. Each node is dialed over
    // a direct-tcpip channel on the previous node's handle; the first directly.
    let mut jump_handles: Vec<SshHandle> = Vec::new();
    let last = req.jumps.len(); // index of the target node in the virtual chain
    let mut prev: Option<SshHandle> = None;

    for idx in 0..=last {
        let (host, port, username, auth) = if idx < last {
            let j = &req.jumps[idx];
            (j.host.as_str(), j.port, j.username.as_str(), &j.auth)
        } else {
            (
                req.host.as_str(),
                req.port,
                req.username.as_str(),
                &req.auth,
            )
        };

        // Tunnel through the previous hop, or dial directly for the first node.
        let over = match prev.as_ref() {
            Some(h) => Some(
                h.channel_open_direct_tcpip(host.to_string(), port as u32, "127.0.0.1", 0)
                    .await
                    .map_err(|e| {
                        AppError::Connect(format!(
                            "could not open jump channel to {host}:{port}: {e}"
                        ))
                    })?,
            ),
            None => None,
        };

        // Only the final target needs the real remote-forward registry (-R).
        let rf = if idx == last {
            remote_forwards.clone()
        } else {
            Arc::new(Mutex::new(HashMap::new()))
        };

        let mut handle = connect_node(
            config.clone(),
            host,
            port,
            known_hosts.clone(),
            prompter.clone(),
            rf,
            over,
        )
        .await?;
        authenticate(&mut handle, username, auth).await?;

        if let Some(old) = prev.replace(handle) {
            jump_handles.push(old);
        }
    }

    let handle = prev.expect("chain always contains the target node");
    let channel = handle.channel_open_session().await?;
    channel
        .request_pty(false, "xterm-256color", req.cols, req.rows, 0, 0, &[])
        .await?;
    channel.request_shell(true).await?;

    Ok((handle, channel, remote_forwards, jump_handles))
}

/// Connect (TCP or over a jump channel) and verify the host key for one node in
/// the chain. Auth is performed separately by the caller.
async fn connect_node(
    config: Arc<client::Config>,
    host: &str,
    port: u16,
    known_hosts: PathBuf,
    prompter: Option<HostKeyPrompter>,
    remote_forwards: RemoteForwards,
    over: Option<russh::Channel<Msg>>,
) -> AppResult<SshHandle> {
    let report = HostKeyReport::default();
    let handler = ClientHandler {
        host: host.to_string(),
        port,
        known_hosts,
        prompter,
        report: report.clone(),
        remote_forwards,
    };

    let result = match over {
        Some(channel) => client::connect_stream(config, channel.into_stream(), handler).await,
        None => client::connect(config, (host, port), handler).await,
    };
    result.map_err(|e| {
        if let Some(msg) = report.0.lock().unwrap().take() {
            AppError::HostKey(msg)
        } else {
            AppError::Connect(e.to_string())
        }
    })
}

/// Authenticate `handle` as `username` using the requested method (TM-2).
async fn authenticate(
    handle: &mut SshHandle,
    username: &str,
    auth: &AuthCredential,
) -> AppResult<()> {
    match auth {
        AuthCredential::Password(password) => {
            let ok = handle
                .authenticate_password(username.to_string(), password.clone())
                .await?
                .success();
            if !ok {
                return Err(AppError::Auth("password authentication failed".into()));
            }
        }
        AuthCredential::PublicKey {
            key_path,
            passphrase,
        } => {
            let key = russh::keys::load_secret_key(key_path, passphrase.as_deref())
                .map_err(|e| AppError::Auth(format!("could not load private key: {e}")))?;
            let hash = handle.best_supported_rsa_hash().await?.flatten();
            let key = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            let ok = handle
                .authenticate_publickey(username.to_string(), key)
                .await?
                .success();
            if !ok {
                return Err(AppError::Auth("public-key authentication failed".into()));
            }
        }
        AuthCredential::KeyboardInteractive(secret) => {
            let mut response = handle
                .authenticate_keyboard_interactive_start(username.to_string(), None)
                .await?;
            loop {
                match response {
                    KeyboardInteractiveAuthResponse::Success => break,
                    KeyboardInteractiveAuthResponse::Failure { .. } => {
                        return Err(AppError::Auth(
                            "keyboard-interactive authentication failed".into(),
                        ));
                    }
                    KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                        let answers = prompts.iter().map(|_| secret.clone()).collect();
                        response = handle
                            .authenticate_keyboard_interactive_respond(answers)
                            .await?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Per-session actor: forwards user input/resize to the SSH shell channel and
/// streams shell output back to the frontend (base64 over a Tauri `Channel`).
/// Emits `ssh://closed` with the session id when the shell ends (SM-2, §6.4).
pub(crate) async fn run_session(
    mut channel: russh::Channel<Msg>,
    mut rx: mpsc::Receiver<SessionCommand>,
    output: Channel<String>,
    app: AppHandle,
    id: String,
) {
    use std::io::Write;

    let b64 = base64::engine::general_purpose::STANDARD;
    let mut log: Option<std::io::BufWriter<std::fs::File>> = None;

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
                    Some(SessionCommand::StartLog(path)) => {
                        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                            Ok(f) => log = Some(std::io::BufWriter::new(f)),
                            Err(_) => log = None,
                        }
                    }
                    Some(SessionCommand::StopLog) => {
                        if let Some(mut w) = log.take() {
                            let _ = w.flush();
                        }
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
                        if let Some(w) = log.as_mut() {
                            let _ = w.write_all(&data[..]);
                        }
                        if output.send(b64.encode(&data[..])).is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                        if let Some(w) = log.as_mut() {
                            let _ = w.write_all(&data[..]);
                        }
                        let _ = output.send(b64.encode(&data[..]));
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }

    if let Some(mut w) = log.take() {
        let _ = w.flush();
    }
    let _ = app.emit("ssh://closed", &id);
}

#[cfg(test)]
mod tests {
    //! M0 live verification against a Docker OpenSSH server.
    //!
    //! Start the target first:
    //!   docker run -d --name ammax-sshd -e PASSWORD_ACCESS=true \
    //!     -e USER_NAME=amos -e USER_PASSWORD=ammax123 -p 2222:2222 \
    //!     lscr.io/linuxserver/openssh-server:latest
    //! Then run:
    //!   cargo test --manifest-path src-tauri/Cargo.toml --lib ssh::tests -- --ignored --nocapture

    use super::*;
    use std::time::Duration;

    fn test_request() -> ConnectRequest {
        ConnectRequest {
            host: "127.0.0.1".into(),
            port: 2222,
            username: "amos".into(),
            auth: AuthCredential::Password("ammax123".into()),
            jumps: Vec::new(),
            cols: 80,
            rows: 24,
        }
    }

    #[tokio::test]
    #[ignore = "requires the Docker OpenSSH container on 127.0.0.1:2222"]
    async fn m0_shell_sftp_and_hostkey() {
        let known_hosts = std::env::temp_dir().join("ammax_test_known_hosts");
        let _ = std::fs::remove_file(&known_hosts);

        // 1) Connect (TOFU learns the key — no prompter in tests) + open a shell.
        let (handle, mut channel, _fwd, _jumps) =
            open_shell(&test_request(), known_hosts.clone(), None, 0)
                .await
                .expect("open_shell should succeed against the Docker SSH server");

        // 2) Shell streaming: send a command and read until the marker comes back.
        channel
            .data_bytes(b"echo M0_MARKER_OK\n".to_vec())
            .await
            .expect("send input");
        let mut out = String::new();
        let saw_marker = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match channel.wait().await {
                    Some(ChannelMsg::Data { ref data }) => {
                        out.push_str(&String::from_utf8_lossy(&data[..]));
                        if out.contains("M0_MARKER_OK") {
                            return true;
                        }
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => return false,
                    _ => {}
                }
            }
        })
        .await
        .unwrap_or(false);
        assert!(
            saw_marker,
            "shell did not echo the marker; output so far:\n{out}"
        );

        // 3) SFTP on the SAME connection: list + upload/download round-trip.
        let entries = crate::sftp::list_dir(&handle, ".")
            .await
            .expect("sftp list_dir");
        println!("[m0] sftp list '.' -> {} entries", entries.len());

        let up = std::env::temp_dir().join("ammax_up.txt");
        let down = std::env::temp_dir().join("ammax_down.txt");
        let payload = "hello-from-ammaxterm-m0";
        std::fs::write(&up, payload).unwrap();
        crate::sftp::upload(&handle, up.to_str().unwrap(), "ammax_remote.txt")
            .await
            .expect("sftp upload");
        crate::sftp::download(&handle, "ammax_remote.txt", down.to_str().unwrap())
            .await
            .expect("sftp download");
        assert_eq!(
            std::fs::read_to_string(&down).unwrap(),
            payload,
            "SFTP round-trip content mismatch"
        );

        // 4) Reconnect: the learned key must verify (match path, no re-learn).
        let (handle2, _ch2, _fwd2, _jumps2) =
            open_shell(&test_request(), known_hosts.clone(), None, 0)
                .await
                .expect("reconnect should pass host-key verification");
        let kh = std::fs::read_to_string(&known_hosts).unwrap();
        let key_lines = kh
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .count();
        assert_eq!(
            key_lines, 1,
            "known_hosts should hold exactly one learned key (matched, not re-learned):\n{kh}"
        );

        drop(handle2);
        drop(handle);
        let _ = std::fs::remove_file(&up);
        let _ = std::fs::remove_file(&down);
        println!("[m0] OK: shell streaming + SFTP round-trip + host-key TOFU/reconnect");
    }
}
