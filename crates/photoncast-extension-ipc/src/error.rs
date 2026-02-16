use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("rpc error {code}: {message}")]
    RpcError { code: i32, message: String },
    #[error("invalid rpc message: {0}")]
    InvalidMessage(String),
    #[error("response channel closed")]
    ResponseChannelClosed,
    #[error("request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("connection closed")]
    Disconnected,
}

impl IpcError {
    #[must_use]
    pub const fn rpc_code(&self) -> i32 {
        match self {
            Self::RpcError { code, .. } => *code,
            Self::InvalidMessage(_) => -32600,
            _ => -32000,
        }
    }

    #[must_use]
    pub fn rpc_message(&self) -> String {
        match self {
            Self::RpcError { message, .. } => message.clone(),
            _ => self.to_string(),
        }
    }
}
