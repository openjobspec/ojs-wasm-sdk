use serde::{Deserialize, Serialize};
use std::fmt;
use wasm_bindgen::JsValue;

/// Main SDK error type.
#[derive(Debug)]
pub enum OjsWasmError {
    /// An error returned by the OJS server.
    Server(ServerError),
    /// HTTP transport error.
    Transport(String),
    /// Serialization / deserialization failure.
    Serialization(String),
    /// JavaScript interop error.
    Js(String),
}

impl fmt::Display for OjsWasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OjsWasmError::Server(e) => write!(f, "[{}] {}", e.code, e.message),
            OjsWasmError::Transport(e) => write!(f, "transport error: {}", e),
            OjsWasmError::Serialization(e) => write!(f, "serialization error: {}", e),
            OjsWasmError::Js(e) => write!(f, "js error: {}", e),
        }
    }
}

impl From<OjsWasmError> for JsValue {
    fn from(err: OjsWasmError) -> JsValue {
        JsValue::from_str(&err.to_string())
    }
}

impl From<serde_json::Error> for OjsWasmError {
    fn from(err: serde_json::Error) -> Self {
        OjsWasmError::Serialization(err.to_string())
    }
}

impl From<JsValue> for OjsWasmError {
    fn from(err: JsValue) -> Self {
        OjsWasmError::Js(
            err.as_string()
                .unwrap_or_else(|| format!("{:?}", err)),
        )
    }
}

/// Structured error from OJS backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
}

/// Wire format wrapper for server error responses.
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorResponse {
    pub error: ServerErrorPayload,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ServerErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
}

pub type Result<T> = std::result::Result<T, OjsWasmError>;
