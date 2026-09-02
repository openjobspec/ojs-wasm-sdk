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
    /// Client-side validation error.
    Validation(String),
}

impl fmt::Display for OjsWasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OjsWasmError::Server(e) => write!(f, "[{}] {}", e.code, e.message),
            OjsWasmError::Transport(e) => write!(f, "transport error: {}", e),
            OjsWasmError::Serialization(e) => write!(f, "serialization error: {}", e),
            OjsWasmError::Js(e) => write!(f, "js error: {}", e),
            OjsWasmError::Validation(e) => write!(f, "validation error: {}", e),
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
        OjsWasmError::Js(err.as_string().unwrap_or_else(|| format!("{:?}", err)))
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

/// Map an HTTP response body to a [`Result`] using the OJS transport rule.
///
/// This is the single source of truth shared by every transport (window
/// `fetch`, Service Worker/global `fetch`, and edge `fetch`). Keeping it a pure
/// function — independent of `web-sys` — lets the error semantics be unit-tested
/// on the native target.
///
/// - On success (`ok == true`) the body is returned verbatim.
/// - On failure, a structured OJS error body maps to [`OjsWasmError::Server`];
///   otherwise the status and raw body map to [`OjsWasmError::Transport`].
pub(crate) fn interpret_response(ok: bool, status: u16, body: String) -> Result<String> {
    if !ok {
        if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            return Err(OjsWasmError::Server(ServerError {
                code: err_resp.error.code,
                message: err_resp.error.message,
                retryable: err_resp.error.retryable,
            }));
        }
        return Err(OjsWasmError::Transport(format!(
            "HTTP {}: {}",
            status, body
        )));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpret_success_returns_body_verbatim() {
        let body = r#"{"job":{"id":"1"}}"#.to_string();
        assert_eq!(interpret_response(true, 200, body.clone()).unwrap(), body);
    }

    #[test]
    fn interpret_structured_error_maps_to_server() {
        let body = r#"{"error":{"code":"conflict","message":"exists","retryable":false}}"#;
        match interpret_response(false, 409, body.to_string()) {
            Err(OjsWasmError::Server(e)) => {
                assert_eq!(e.code, "conflict");
                assert_eq!(e.message, "exists");
                assert!(!e.retryable);
            }
            other => panic!("expected Server error, got {:?}", other),
        }
    }

    #[test]
    fn interpret_structured_error_preserves_retryable() {
        let body = r#"{"error":{"code":"unavailable","message":"try later","retryable":true}}"#;
        match interpret_response(false, 503, body.to_string()) {
            Err(OjsWasmError::Server(e)) => assert!(e.retryable),
            other => panic!("expected Server error, got {:?}", other),
        }
    }

    #[test]
    fn interpret_non_json_error_maps_to_transport() {
        match interpret_response(false, 502, "bad gateway".to_string()) {
            Err(OjsWasmError::Transport(msg)) => {
                assert_eq!(msg, "HTTP 502: bad gateway");
            }
            other => panic!("expected Transport error, got {:?}", other),
        }
    }
}
