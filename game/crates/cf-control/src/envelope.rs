//! JSON-RPC 2.0 envelope types and constants.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const METHOD_OBSERVE_FRAME: &str = "observe.frame";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(i64),
    String(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}

/// Untagged enum used by tests and tools to deserialize whichever shape arrives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    Request(JsonRpcRequest),
}

pub mod error_codes {
    pub const INVALID_PARAMS: i32 = -32602;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const COMMAND_REJECTED: i32 = -32099;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(7),
            method: "act.player.move".to_string(),
            params: serde_json::json!({"schema_version": 1, "x": 1.0}),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"act.player.move\""));
        let parsed: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.method, "act.player.move");
    }

    #[test]
    fn response_with_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(1),
            result: None,
            error: Some(JsonRpcError {
                code: error_codes::INVALID_PARAMS,
                message: "InvalidParams".to_string(),
                data: Some(serde_json::json!({
                    "reason": "schema_version_mismatch",
                    "server_version": 1,
                    "client_version": 2,
                    "fix_hint": "Upgrade cfctl or pin client schema_version: 1"
                })),
            }),
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("schema_version_mismatch"));
    }
}
