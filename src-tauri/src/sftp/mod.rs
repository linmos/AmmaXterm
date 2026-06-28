//! SFTP operations over an existing SSH connection (M0: list / upload / download).
//!
//! Each call opens its own SFTP channel on the shared connection. M2 will
//! persist one `SftpSession` per connection and add a transfer queue, resume,
//! and progress reporting (FT-4, FT-7, FT-9).

use std::io::SeekFrom;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::error::{AppError, AppResult};
use crate::ssh::SshHandle;

/// Chunk size for streaming transfers (FT-4).
const CHUNK: usize = 32 * 1024;

/// One remote directory entry (FT-1, FT-8).
#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub permissions: Option<u32>,
    pub modified: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

fn sftp_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Sftp(e.to_string())
}

/// Open a fresh SFTP session on the given SSH connection.
async fn open(handle: &SshHandle) -> AppResult<SftpSession> {
    let channel = handle.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    SftpSession::new(channel.into_stream())
        .await
        .map_err(sftp_err)
}

/// List a remote directory (FT-1). Directories first, then case-insensitive name.
pub async fn list_dir(handle: &SshHandle, path: &str) -> AppResult<Vec<FileEntry>> {
    let sftp = open(handle).await?;
    let mut entries: Vec<FileEntry> = sftp
        .read_dir(path)
        .await
        .map_err(sftp_err)?
        .map(|entry| {
            let md = entry.metadata();
            FileEntry {
                name: entry.file_name(),
                is_dir: entry.file_type().is_dir(),
                is_symlink: entry.file_type().is_symlink(),
                size: md.size.unwrap_or(0),
                permissions: md.permissions,
                modified: md.mtime,
                uid: md.uid,
                gid: md.gid,
            }
        })
        .collect();

    // `read_dir` reports lstat metadata, so a symlink's `is_dir` is always false
    // (its type is "symlink"). Follow each link with a stat to learn whether the
    // target is a directory, so symlinked folders are navigable in the UI.
    // Broken links keep `is_dir = false`.
    for entry in entries.iter_mut().filter(|e| e.is_symlink) {
        let target = if path.ends_with('/') {
            format!("{path}{}", entry.name)
        } else {
            format!("{path}/{}", entry.name)
        };
        if let Ok(md) = sftp.metadata(target).await {
            entry.is_dir = md.file_type().is_dir();
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// Size of a remote file in bytes (for transfer totals, FT-4).
pub async fn remote_size(handle: &SshHandle, path: &str) -> AppResult<u64> {
    let sftp = open(handle).await?;
    let md = sftp.metadata(path).await.map_err(sftp_err)?;
    Ok(md.size.unwrap_or(0))
}

/// Stream a local file to the remote in chunks, updating `done` and stopping
/// promptly if `cancel` or `pause` is set. `offset` > 0 resumes an interrupted
/// upload from that byte (FT-4 progress/cancel, FT-7 resume).
#[allow(clippy::too_many_arguments)]
pub async fn upload_streaming(
    handle: &SshHandle,
    local_path: &str,
    remote_path: &str,
    done: &AtomicU64,
    cancel: &AtomicBool,
    pause: &AtomicBool,
    offset: u64,
) -> AppResult<()> {
    let mut local = tokio::fs::File::open(local_path).await?;
    let flags = if offset > 0 {
        OpenFlags::CREATE | OpenFlags::WRITE
    } else {
        OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE
    };
    let sftp = open(handle).await?;
    let mut remote = sftp
        .open_with_flags(remote_path, flags)
        .await
        .map_err(sftp_err)?;
    if offset > 0 {
        local.seek(SeekFrom::Start(offset)).await?;
        remote.seek(SeekFrom::Start(offset)).await?;
    }
    let mut buf = vec![0u8; CHUNK];
    loop {
        if cancel.load(Ordering::Relaxed) || pause.load(Ordering::Relaxed) {
            break;
        }
        let n = local.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        remote.write_all(&buf[..n]).await?;
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    remote.shutdown().await?;
    Ok(())
}

/// Stream a remote file to a local path in chunks. `offset` > 0 resumes an
/// interrupted download, appending from that byte (FT-4 + FT-7).
#[allow(clippy::too_many_arguments)]
pub async fn download_streaming(
    handle: &SshHandle,
    remote_path: &str,
    local_path: &str,
    done: &AtomicU64,
    cancel: &AtomicBool,
    pause: &AtomicBool,
    offset: u64,
) -> AppResult<()> {
    ensure_local_parent(local_path).await;
    let sftp = open(handle).await?;
    let mut remote = sftp
        .open_with_flags(remote_path, OpenFlags::READ)
        .await
        .map_err(sftp_err)?;
    let mut local = if offset > 0 {
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .open(local_path)
            .await?;
        f.seek(SeekFrom::Start(offset)).await?;
        remote.seek(SeekFrom::Start(offset)).await?;
        f
    } else {
        tokio::fs::File::create(local_path).await?
    };
    let mut buf = vec![0u8; CHUNK];
    loop {
        if cancel.load(Ordering::Relaxed) || pause.load(Ordering::Relaxed) {
            break;
        }
        let n = remote.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        local.write_all(&buf[..n]).await?;
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    local.shutdown().await?;
    Ok(())
}

/// Upload a local file to a remote path (FT-2).
pub async fn upload(handle: &SshHandle, local_path: &str, remote_path: &str) -> AppResult<()> {
    let bytes = tokio::fs::read(local_path).await?;
    let sftp = open(handle).await?;
    let mut file = sftp
        .open_with_flags(
            remote_path,
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await
        .map_err(sftp_err)?;
    file.write_all(&bytes).await?;
    file.shutdown().await?;
    Ok(())
}

/// Download a remote file to a local path (FT-2).
pub async fn download(handle: &SshHandle, remote_path: &str, local_path: &str) -> AppResult<()> {
    ensure_local_parent(local_path).await;
    let sftp = open(handle).await?;
    let mut file = sftp
        .open_with_flags(remote_path, OpenFlags::READ)
        .await
        .map_err(sftp_err)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    tokio::fs::write(local_path, &buf).await?;
    Ok(())
}

/// Create the parent directory tree for a local download target so a file
/// inside a downloaded folder lands even if the dir wasn't pre-created.
/// Best-effort: a failure here surfaces as the file's own create error.
async fn ensure_local_parent(local_path: &str) {
    if let Some(parent) = std::path::Path::new(local_path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
    }
}

/// One remote file to download, with its path relative to the download root.
#[derive(Serialize)]
pub struct DownloadFile {
    pub remote: String,
    pub rel: String,
}

/// A remote tree expanded for download: directories to create locally
/// (parent-before-child) and the files within them. Mirrors `expand_uploads`
/// so a whole folder can be downloaded, not just flat files (FT-5).
#[derive(Serialize)]
pub struct DownloadPlan {
    pub dirs: Vec<String>,
    pub files: Vec<DownloadFile>,
}

/// Expand selected remote paths into a download plan, recursing into directories.
/// `rel` paths use `/` and are rooted at each selected entry's own name, matching
/// the upload side so the frontend can join them onto a local destination folder.
/// Symlinks are treated as files (their target's bytes are fetched), which also
/// avoids cycles.
pub async fn expand_download(handle: &SshHandle, paths: &[String]) -> AppResult<DownloadPlan> {
    let sftp = open(handle).await?;
    let mut plan = DownloadPlan {
        dirs: Vec::new(),
        files: Vec::new(),
    };
    for p in paths {
        let base = p.trim_end_matches('/');
        let name = base.rsplit('/').next().unwrap_or(base).to_string();
        let md = sftp.metadata(p).await.map_err(sftp_err)?;
        if md.file_type().is_dir() {
            plan.dirs.push(name.clone());
            walk_download(&sftp, base, &name, &mut plan).await?;
        } else {
            plan.files.push(DownloadFile {
                remote: p.clone(),
                rel: name,
            });
        }
    }
    Ok(plan)
}

/// Recurse a remote directory, recording subdirectories (parent-before-child) and
/// files with their `prefix`-relative, forward-slashed paths.
async fn walk_download(
    sftp: &SftpSession,
    dir: &str,
    prefix: &str,
    plan: &mut DownloadPlan,
) -> AppResult<()> {
    for entry in sftp.read_dir(dir).await.map_err(sftp_err)? {
        let name = entry.file_name();
        let child = format!("{dir}/{name}");
        let rel = format!("{prefix}/{name}");
        if entry.file_type().is_dir() {
            plan.dirs.push(rel.clone());
            Box::pin(walk_download(sftp, &child, &rel, plan)).await?;
        } else {
            plan.files.push(DownloadFile { remote: child, rel });
        }
    }
    Ok(())
}

/// Create a remote directory (FT-3).
pub async fn make_dir(handle: &SshHandle, path: &str) -> AppResult<()> {
    let sftp = open(handle).await?;
    sftp.create_dir(path).await.map_err(sftp_err)
}

/// Rename or move a remote file/directory (FT-3).
pub async fn rename(handle: &SshHandle, from: &str, to: &str) -> AppResult<()> {
    let sftp = open(handle).await?;
    sftp.rename(from, to).await.map_err(sftp_err)
}

/// Change a remote file's permission bits (FT-8). Only the low 12 bits
/// (rwx + setuid/setgid/sticky) are replaced; the file-type bits are preserved.
pub async fn chmod(handle: &SshHandle, path: &str, mode: u32) -> AppResult<()> {
    let sftp = open(handle).await?;
    let mut md = sftp.metadata(path).await.map_err(sftp_err)?;
    let kept = md.permissions.unwrap_or(0) & !0o7777;
    md.permissions = Some(kept | (mode & 0o7777));
    sftp.set_metadata(path, md).await.map_err(sftp_err)
}

/// Delete a remote file, or a directory and its contents recursively (FT-3).
pub async fn remove(handle: &SshHandle, path: &str, is_dir: bool) -> AppResult<()> {
    let sftp = open(handle).await?;
    if is_dir {
        remove_dir_recursive(&sftp, path).await
    } else {
        sftp.remove_file(path).await.map_err(sftp_err)
    }
}

async fn remove_dir_recursive(sftp: &SftpSession, path: &str) -> AppResult<()> {
    let base = path.trim_end_matches('/');
    for entry in sftp.read_dir(path).await.map_err(sftp_err)? {
        let child = format!("{}/{}", base, entry.file_name());
        if entry.file_type().is_dir() {
            Box::pin(remove_dir_recursive(sftp, &child)).await?;
        } else {
            sftp.remove_file(&child).await.map_err(sftp_err)?;
        }
    }
    sftp.remove_dir(path).await.map_err(sftp_err)
}
