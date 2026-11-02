//! Error codes and the error object (docs/specs/RPC.md §1).

use serde_json::{Value, json};

/// JSON-RPC error codes used by the server.
pub mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL: i64 = -32603;
    pub const FAILED: i64 = -32000;
    pub const UNAUTHORIZED: i64 = -32001;
    pub const RATE_LIMITED: i64 = -32002;
    pub const TOO_LARGE: i64 = -32003;
    pub const TOO_MANY_CONNECTIONS: i64 = -32004;
    pub const TIMEOUT: i64 = -32005;
}

/// The `error` member of a failed response.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> RpcError {
        RpcError {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// A call that ran and failed; `kind` says what kind of failure.
    pub fn failed(kind: &str, message: impl Into<String>) -> RpcError {
        RpcError {
            code: code::FAILED,
            message: message.into(),
            data: Some(json!({ "kind": kind })),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> RpcError {
        RpcError::new(code::INVALID_PARAMS, message)
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut e = json!({"code": self.code, "message": self.message});
        if let Some(d) = &self.data {
            e["data"] = d.clone();
        }
        e
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

pub(crate) fn success(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub(crate) fn failure(id: &Value, error: &RpcError) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": error.to_json()})
}
