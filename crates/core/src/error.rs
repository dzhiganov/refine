use serde::Serialize;
use ts_rs::TS;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Add an API key for {0} in Settings.")]
    MissingKey(String),
    #[error("Pick a model for {0} in Settings.")]
    NoModel(String),
    #[error("{provider} answered with an error ({status}): {message}")]
    Provider { provider: String, status: u16, message: String },
    #[error("Couldn't reach {0}. Check the address and that it's running.")]
    Network(String),
    #[error("The model's answer didn't match the expected format: {0}")]
    InvalidOutput(String),
    #[error("{0}")]
    Invalid(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("Couldn't access the system keychain: {0}")]
    Secret(String),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// What the UI receives when a command fails: a stable `kind` to branch on and a readable message.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AppErrorKind {
    MissingKey,
    NoModel,
    Provider,
    Network,
    InvalidOutput,
    Invalid,
    NotFound,
    Internal,
}

impl Error {
    pub fn kind(&self) -> AppErrorKind {
        match self {
            Error::MissingKey(_) => AppErrorKind::MissingKey,
            Error::NoModel(_) => AppErrorKind::NoModel,
            Error::Provider { .. } => AppErrorKind::Provider,
            Error::Network(_) => AppErrorKind::Network,
            Error::InvalidOutput(_) => AppErrorKind::InvalidOutput,
            Error::Invalid(_) => AppErrorKind::Invalid,
            Error::NotFound(_) => AppErrorKind::NotFound,
            Error::Storage(_) | Error::Secret(_) | Error::Io(_) | Error::Json(_) => AppErrorKind::Internal,
        }
    }
}

impl From<Error> for AppError {
    fn from(e: Error) -> Self {
        AppError { kind: e.kind(), message: e.to_string() }
    }
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        AppError { kind: self.kind(), message: self.to_string() }.serialize(s)
    }
}
