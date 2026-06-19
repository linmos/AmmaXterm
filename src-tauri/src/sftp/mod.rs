//! SFTP operations over an existing SSH connection (M0: list / upload / download).
//!
//! Each call opens its own SFTP channel on the shared connection. M2 will
//! persist one `SftpSession` per connection and add a transfer queue, resume,
//! and progress reporting (FT-4, FT-7, FT-9).

use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{AppError, AppResult};
use crate::ssh::SshHandle;

/// One remote directory entry (FT-1, FT-8).
#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
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
                size: md.size.unwrap_or(0),
                permissions: md.permissions,
                modified: md.mtime,
                uid: md.uid,
                gid: md.gid,
            }
        })
        .collect();

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
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
