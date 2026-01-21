use thiserror::Error;

pub type Result<T> = std::result::Result<T, TimerError>;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer error: {message}")]
    Message { message: String },

    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("execution error: {0}")]
    Execution(String),
}
