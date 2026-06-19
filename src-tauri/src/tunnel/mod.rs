//! SSH port forwarding / tunnels (PF-1..PF-7).
//!
//! Each tunnel runs as an independent task over an existing session's shared SSH
//! `Handle`. **PF-7 (P0):** local listeners always bind the loopback address
//! `127.0.0.1` — never `0.0.0.0` — so a forward is not unintentionally exposed
//! to the LAN. Per-connection work is spawned so one stalled connection never
//! blocks the listener.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::{AppError, AppResult};
use crate::ssh::{RemoteForwards, SshHandle};

/// Loopback bind address enforced for local listeners (PF-7).
const LOOPBACK: &str = "127.0.0.1";

/// A tunnel definition from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelSpec {
    /// "local" (-L) | "dynamic" (-D, SOCKS5) | "remote" (-R).
    pub kind: String,
    /// Local listen port (for local/dynamic).
    #[serde(default)]
    pub listen_port: u16,
    /// Forward destination host (local/remote target).
    #[serde(default)]
    pub dest_host: String,
    /// Forward destination port (local/remote target).
    #[serde(default)]
    pub dest_port: u16,
    /// Remote (-R) only: bind the server listener to 0.0.0.0 (LAN-exposed)
    /// instead of 127.0.0.1. Off by default (PF-3 safe default).
    #[serde(default)]
    pub expose: bool,
}

#[derive(Default)]
struct Metrics {
    conns: AtomicU64,
    bytes_up: AtomicU64,
    bytes_down: AtomicU64,
}

struct Tunnel {
    spec: TunnelSpec,
    session_id: String,
    task: tokio::task::JoinHandle<()>,
    metrics: Arc<Metrics>,
    cancel: Arc<AtomicBool>,
}

/// Snapshot of a tunnel for the management panel (PF-5).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TunnelInfo {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub listen_host: String,
    pub listen_port: u16,
    pub dest_host: String,
    pub dest_port: u16,
    pub conns: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
}

/// Active tunnels across all sessions, managed as Tauri state.
#[derive(Default)]
pub struct TunnelManager {
    tunnels: Mutex<HashMap<String, Tunnel>>,
    counter: AtomicU64,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a tunnel over `handle` (the session's shared SSH connection) and
    /// register it. Binding happens here so an in-use port surfaces immediately
    /// as a clear error (PF-6).
    pub async fn open(
        &self,
        session_id: String,
        spec: TunnelSpec,
        handle: Arc<SshHandle>,
        remote_forwards: RemoteForwards,
    ) -> AppResult<String> {
        let metrics = Arc::new(Metrics::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let task = match spec.kind.as_str() {
            "local" => {
                if spec.dest_host.is_empty() || spec.dest_port == 0 {
                    return Err(AppError::Other(
                        "local forward needs a destination host and port".into(),
                    ));
                }
                let listener = bind(spec.listen_port).await?;
                spawn_local(listener, handle, spec.clone(), metrics.clone())
            }
            "dynamic" => {
                let listener = bind(spec.listen_port).await?;
                spawn_dynamic(listener, handle, metrics.clone())
            }
            "remote" => {
                if spec.dest_host.is_empty() || spec.dest_port == 0 {
                    return Err(AppError::Other(
                        "remote forward needs a destination host and port".into(),
                    ));
                }
                let bind_addr = if spec.expose { "0.0.0.0" } else { "127.0.0.1" };
                // Register the client-side target before asking the server to listen.
                remote_forwards
                    .lock()
                    .unwrap()
                    .insert(spec.listen_port, (spec.dest_host.clone(), spec.dest_port));
                if let Err(e) = handle
                    .tcpip_forward(bind_addr, spec.listen_port as u32)
                    .await
                {
                    remote_forwards.lock().unwrap().remove(&spec.listen_port);
                    return Err(AppError::Other(format!(
                        "remote forward request was denied by the server: {e}"
                    )));
                }
                spawn_remote_cleanup(
                    handle,
                    remote_forwards,
                    bind_addr.to_string(),
                    spec.listen_port,
                    cancel.clone(),
                )
            }
            other => {
                return Err(AppError::Other(format!("unknown tunnel type '{other}'")));
            }
        };

        let id = format!("tunnel-{}", self.counter.fetch_add(1, Ordering::Relaxed));
        self.tunnels.lock().unwrap().insert(
            id.clone(),
            Tunnel {
                spec,
                session_id,
                task,
                metrics,
                cancel,
            },
        );
        Ok(id)
    }

    pub fn close(&self, id: &str) {
        if let Some(t) = self.tunnels.lock().unwrap().remove(id) {
            stop(&t);
        }
    }

    /// Close every tunnel belonging to a session (called when it disconnects).
    pub fn close_for_session(&self, session_id: &str) {
        let mut guard = self.tunnels.lock().unwrap();
        let ids: Vec<String> = guard
            .iter()
            .filter(|(_, t)| t.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(t) = guard.remove(&id) {
                stop(&t);
            }
        }
    }

    pub fn list(&self) -> Vec<TunnelInfo> {
        self.tunnels
            .lock()
            .unwrap()
            .iter()
            .map(|(id, t)| TunnelInfo {
                id: id.clone(),
                session_id: t.session_id.clone(),
                kind: t.spec.kind.clone(),
                listen_host: if t.spec.kind == "remote" && t.spec.expose {
                    "0.0.0.0".to_string()
                } else {
                    LOOPBACK.to_string()
                },
                listen_port: t.spec.listen_port,
                dest_host: t.spec.dest_host.clone(),
                dest_port: t.spec.dest_port,
                conns: t.metrics.conns.load(Ordering::Relaxed),
                bytes_up: t.metrics.bytes_up.load(Ordering::Relaxed),
                bytes_down: t.metrics.bytes_down.load(Ordering::Relaxed),
            })
            .collect()
    }
}

/// Stop a tunnel: signal cancel. Remote (-R) tunnels self-clean on the flag
/// (cancelling the server-side forward); local/dynamic just have their listener
/// task aborted.
fn stop(t: &Tunnel) {
    t.cancel.store(true, Ordering::Relaxed);
    if t.spec.kind != "remote" {
        t.task.abort();
    }
}

/// Remote forward (-R): the server does the listening, so this task only waits
/// for cancel, then asks the server to stop and drops the registry entry.
fn spawn_remote_cleanup(
    handle: Arc<SshHandle>,
    remote_forwards: RemoteForwards,
    bind_addr: String,
    bind_port: u16,
    cancel: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !cancel.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let _ = handle
            .cancel_tcpip_forward(bind_addr, bind_port as u32)
            .await;
        remote_forwards.lock().unwrap().remove(&bind_port);
    })
}

/// Bind a loopback TCP listener, mapping common failures to clear errors (PF-6/7).
async fn bind(port: u16) -> AppResult<TcpListener> {
    TcpListener::bind((LOOPBACK, port)).await.map_err(|e| {
        let msg = match e.kind() {
            std::io::ErrorKind::AddrInUse => {
                format!("local port {port} is already in use")
            }
            std::io::ErrorKind::PermissionDenied => {
                format!("permission denied binding local port {port}")
            }
            _ => format!("could not bind 127.0.0.1:{port}: {e}"),
        };
        AppError::Other(msg)
    })
}

/// Local forward (-L): accept loopback connections and splice each to a
/// direct-tcpip channel to `dest_host:dest_port` on the SSH server (PF-1).
fn spawn_local(
    listener: TcpListener,
    handle: Arc<SshHandle>,
    spec: TunnelSpec,
    metrics: Arc<Metrics>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (mut tcp, peer) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let handle = handle.clone();
            let metrics = metrics.clone();
            let dest_host = spec.dest_host.clone();
            let dest_port = spec.dest_port as u32;
            metrics.conns.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move {
                let channel = handle
                    .channel_open_direct_tcpip(
                        dest_host,
                        dest_port,
                        peer.ip().to_string(),
                        peer.port() as u32,
                    )
                    .await;
                if let Ok(channel) = channel {
                    let mut stream = channel.into_stream();
                    if let Ok((up, down)) =
                        tokio::io::copy_bidirectional(&mut tcp, &mut stream).await
                    {
                        metrics.bytes_up.fetch_add(up, Ordering::Relaxed);
                        metrics.bytes_down.fetch_add(down, Ordering::Relaxed);
                    }
                }
            });
        }
    })
}

/// Dynamic forward (-D): a minimal SOCKS5 proxy (no auth, CONNECT only) that
/// opens a direct-tcpip channel per requested target (PF-2).
fn spawn_dynamic(
    listener: TcpListener,
    handle: Arc<SshHandle>,
    metrics: Arc<Metrics>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (tcp, peer) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let handle = handle.clone();
            let metrics = metrics.clone();
            metrics.conns.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move {
                let _ = handle_socks5(tcp, peer, handle, metrics).await;
            });
        }
    })
}

/// SOCKS5 reply with a failure code and a zeroed BND address.
async fn socks5_fail(tcp: &mut tokio::net::TcpStream, code: u8) -> std::io::Result<()> {
    tcp.write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
}

async fn handle_socks5(
    mut tcp: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    handle: Arc<SshHandle>,
    metrics: Arc<Metrics>,
) -> std::io::Result<()> {
    // Greeting: VER, NMETHODS, METHODS… → reply "no authentication required".
    let mut head = [0u8; 2];
    tcp.read_exact(&mut head).await?;
    if head[0] != 0x05 {
        return Ok(());
    }
    let mut methods = vec![0u8; head[1] as usize];
    tcp.read_exact(&mut methods).await?;
    tcp.write_all(&[0x05, 0x00]).await?;

    // Request: VER, CMD, RSV, ATYP, DST.ADDR, DST.PORT.
    let mut req = [0u8; 4];
    tcp.read_exact(&mut req).await?;
    if req[0] != 0x05 {
        return Ok(());
    }
    let host = match req[3] {
        0x01 => {
            let mut a = [0u8; 4];
            tcp.read_exact(&mut a).await?;
            std::net::Ipv4Addr::from(a).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            tcp.read_exact(&mut len).await?;
            let mut dom = vec![0u8; len[0] as usize];
            tcp.read_exact(&mut dom).await?;
            String::from_utf8_lossy(&dom).into_owned()
        }
        0x04 => {
            let mut a = [0u8; 16];
            tcp.read_exact(&mut a).await?;
            std::net::Ipv6Addr::from(a).to_string()
        }
        _ => {
            socks5_fail(&mut tcp, 0x08).await?; // address type not supported
            return Ok(());
        }
    };
    let mut port = [0u8; 2];
    tcp.read_exact(&mut port).await?;
    let port = u16::from_be_bytes(port);

    if req[1] != 0x01 {
        socks5_fail(&mut tcp, 0x07).await?; // command not supported (CONNECT only)
        return Ok(());
    }

    match handle
        .channel_open_direct_tcpip(host, port as u32, peer.ip().to_string(), peer.port() as u32)
        .await
    {
        Ok(channel) => {
            tcp.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;
            let mut stream = channel.into_stream();
            if let Ok((up, down)) = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await {
                metrics.bytes_up.fetch_add(up, Ordering::Relaxed);
                metrics.bytes_down.fetch_add(down, Ordering::Relaxed);
            }
        }
        Err(_) => {
            socks5_fail(&mut tcp, 0x05).await?; // connection refused
        }
    }
    Ok(())
}
