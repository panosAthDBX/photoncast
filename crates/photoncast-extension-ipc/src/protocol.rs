use serde::{Deserialize, Serialize};

use crate::IpcError;

pub const RPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcRequest {
    #[must_use]
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorData>,
}

impl RpcResponse {
    #[must_use]
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    #[must_use]
    pub fn error(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(RpcErrorData {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorData {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcNotification {
    #[must_use]
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcMessage {
    Request(RpcRequest),
    Response(RpcResponse),
    Notification(RpcNotification),
}

impl RpcMessage {
    pub fn parse_line(line: &str) -> Result<Self, IpcError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(IpcError::InvalidMessage("empty message".to_string()));
        }
        let message = serde_json::from_str::<RpcMessage>(trimmed)?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization_roundtrip() {
        let message =
            RpcMessage::Request(RpcRequest::new(42, "ping", serde_json::json!({"value": 1})));

        let json = serde_json::to_string(&message).expect("should serialize rpc message");
        let parsed = RpcMessage::parse_line(&json).expect("should parse serialized message");

        match parsed {
            RpcMessage::Request(request) => {
                assert_eq!(request.jsonrpc, RPC_VERSION);
                assert_eq!(request.id, 42);
                assert_eq!(request.method, "ping");
                assert_eq!(request.params, serde_json::json!({"value": 1}));
            },
            _ => panic!("expected request message after roundtrip"),
        }
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        let error = RpcMessage::parse_line("{ this is not valid json }")
            .expect_err("expected invalid json parse to fail");

        assert!(matches!(error, IpcError::Json(_)));
    }

    #[test]
    fn test_parse_empty_message_returns_error() {
        let error =
            RpcMessage::parse_line("   ").expect_err("expected empty message parse to fail");

        match error {
            IpcError::InvalidMessage(message) => {
                assert!(message.contains("empty message"));
            },
            other => panic!("expected InvalidMessage error, got: {other}"),
        }
    }
}
