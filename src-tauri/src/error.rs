use serde::Serialize;

/// Categorized application errors.
///
/// Each variant maps to a stable `kind` string that is sent to the frontend
/// alongside a human-readable message, so the UI can show specific, actionable
/// errors (DNS / timeout / auth / host-key / tunnel ...) per PRD §6.4.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("connection failed: {0}")]
    Connect(String),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("host key rejected: {0}")]
    HostKey(String),

    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),

    #[error("SFTP error: {0}")]
    Sftp(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Stable machine-readable category used by the frontend to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Connect(_) => "connect",
            AppError::Auth(_) => "auth",
            AppError::HostKey(_) => "host_key",
            AppError::Ssh(_) => "ssh",
            AppError::Sftp(_) => "sftp",
            AppError::Io(_) => "io",
            AppError::SessionNotFound(_) => "session_not_found",
            AppError::Other(_) => "other",
        }
    }
}

#[derive(Serialize)]
struct ErrorPayload {
    kind: &'static str,
    message: String,
}

/// Serialize as `{ "kind": "...", "message": "..." }` for the frontend.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ErrorPayload {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
