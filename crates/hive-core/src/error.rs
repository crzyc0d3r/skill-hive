//! Error type shared by every Hive service.

use std::path::PathBuf;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, HiveError>;

#[derive(Debug, thiserror::Error)]
pub enum HiveError {
    #[error("{0}")]
    Message(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("provider '{provider}' failed: {message}")]
    Provider { provider: String, message: String },

    #[error("command `{command}` failed with status {status}: {stderr}")]
    Command {
        command: String,
        status: i32,
        stderr: String,
    },

    #[error("timed out after {0} seconds")]
    Timeout(u64),
}

impl HiveError {
    pub fn msg(message: impl Into<String>) -> Self {
        HiveError::Message(message.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        HiveError::Invalid(message.into())
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        HiveError::NotFound(what.into())
    }

    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        HiveError::Io {
            path: path.into(),
            source,
        }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        HiveError::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// A short machine-readable code useful for the UI.
    pub fn code(&self) -> &'static str {
        match self {
            HiveError::Message(_) => "error",
            HiveError::NotFound(_) => "not_found",
            HiveError::AlreadyExists(_) => "already_exists",
            HiveError::Invalid(_) => "invalid",
            HiveError::Cancelled => "cancelled",
            HiveError::Io { .. } => "io",
            HiveError::Db(_) => "db",
            HiveError::Json(_) => "json",
            HiveError::Yaml(_) => "yaml",
            HiveError::Http(_) => "http",
            HiveError::Provider { .. } => "provider",
            HiveError::Command { .. } => "command",
            HiveError::Timeout(_) => "timeout",
        }
    }
}

impl From<std::io::Error> for HiveError {
    fn from(source: std::io::Error) -> Self {
        HiveError::Io {
            path: PathBuf::new(),
            source,
        }
    }
}

impl From<tokio::task::JoinError> for HiveError {
    fn from(err: tokio::task::JoinError) -> Self {
        HiveError::Message(format!("background task failed: {err}"))
    }
}

/// Extension trait to attach a path to io errors.
pub trait IoContext<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoContext<T> for std::result::Result<T, std::io::Error> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|source| HiveError::io(path, source))
    }
}
