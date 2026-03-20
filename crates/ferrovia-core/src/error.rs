use thiserror::Error;

/// Result alias for ferrovia operations.
pub type Result<T> = std::result::Result<T, FerroviaError>;

/// Library errors.
#[derive(Debug, Error)]
pub enum FerroviaError {
    #[error("parse error at byte {position}: {message}")]
    Parse { position: usize, message: String },
    #[error("unsupported plugin: {0}")]
    UnsupportedPlugin(String),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}
