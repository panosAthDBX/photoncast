use thiserror::Error;

pub type Result<T> = std::result::Result<T, TimerError>;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer error: {message}")]
    Message { message: String },

    #[error("database error: {0}")]
    Database(String),

    #[error("database query error: {0}")]
    DatabaseQuery(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("execution error: {0}")]
    Execution(String),
}
