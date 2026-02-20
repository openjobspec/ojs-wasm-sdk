//! Tests for error types, Display implementations, and conversions.

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::error::{OjsWasmError, ServerError};

// ===========================================================================
// OjsWasmError::Server
// ===========================================================================

#[wasm_bindgen_test]
fn error_server_display_format() {
    let err = OjsWasmError::Server(ServerError {
        code: "conflict".to_string(),
        message: "Job already exists".to_string(),
        retryable: false,
    });
    let msg = format!("{}", err);
    assert_eq!(msg, "[conflict] Job already exists");
}

#[wasm_bindgen_test]
fn error_server_retryable_flag() {
    let err = ServerError {
        code: "rate_limited".to_string(),
        message: "Too many requests".to_string(),
        retryable: true,
    };
    assert!(err.retryable);

    let err2 = ServerError {
        code: "not_found".to_string(),
        message: "Job not found".to_string(),
        retryable: false,
    };
    assert!(!err2.retryable);
}

#[wasm_bindgen_test]
fn error_server_with_empty_message() {
    let err = OjsWasmError::Server(ServerError {
        code: "internal".to_string(),
        message: "".to_string(),
        retryable: false,
    });
    let msg = format!("{}", err);
    assert_eq!(msg, "[internal] ");
}

// ===========================================================================
// OjsWasmError::Transport
// ===========================================================================

#[wasm_bindgen_test]
fn error_transport_display() {
    let err = OjsWasmError::Transport("connection refused".to_string());
    assert_eq!(format!("{}", err), "transport error: connection refused");
}

#[wasm_bindgen_test]
fn error_transport_with_http_status() {
    let err = OjsWasmError::Transport("HTTP 503: Service Unavailable".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("503"));
    assert!(msg.contains("Service Unavailable"));
}

#[wasm_bindgen_test]
fn error_transport_empty_message() {
    let err = OjsWasmError::Transport("".to_string());
    assert_eq!(format!("{}", err), "transport error: ");
}

// ===========================================================================
// OjsWasmError::Serialization
// ===========================================================================

#[wasm_bindgen_test]
fn error_serialization_display() {
    let err = OjsWasmError::Serialization("expected string, found number".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("serialization error"));
    assert!(msg.contains("expected string, found number"));
}

// ===========================================================================
// OjsWasmError::Js
// ===========================================================================

#[wasm_bindgen_test]
fn error_js_display() {
    let err = OjsWasmError::Js("TypeError: undefined is not a function".to_string());
    let msg = format!("{}", err);
    assert_eq!(msg, "js error: TypeError: undefined is not a function");
}

#[wasm_bindgen_test]
fn error_js_empty() {
    let err = OjsWasmError::Js("".to_string());
    assert_eq!(format!("{}", err), "js error: ");
}

// ===========================================================================
// From<OjsWasmError> for JsValue
// ===========================================================================

#[wasm_bindgen_test]
fn error_to_jsvalue_server() {
    let err = OjsWasmError::Server(ServerError {
        code: "bad_request".to_string(),
        message: "Invalid JSON".to_string(),
        retryable: false,
    });
    let js: JsValue = err.into();
    let s = js.as_string().unwrap();
    assert!(s.contains("bad_request"));
    assert!(s.contains("Invalid JSON"));
}

#[wasm_bindgen_test]
fn error_to_jsvalue_transport() {
    let err = OjsWasmError::Transport("network failure".to_string());
    let js: JsValue = err.into();
    assert!(js.as_string().unwrap().contains("network failure"));
}

#[wasm_bindgen_test]
fn error_to_jsvalue_serialization() {
    let err = OjsWasmError::Serialization("invalid utf-8".to_string());
    let js: JsValue = err.into();
    assert!(js.as_string().unwrap().contains("invalid utf-8"));
}

#[wasm_bindgen_test]
fn error_to_jsvalue_js() {
    let err = OjsWasmError::Js("ReferenceError".to_string());
    let js: JsValue = err.into();
    assert!(js.as_string().unwrap().contains("ReferenceError"));
}

// ===========================================================================
// From<serde_json::Error> for OjsWasmError
// ===========================================================================

#[wasm_bindgen_test]
fn error_from_serde_json() {
    let bad_json = "not valid json {{{";
    let serde_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
    let ojs_err: OjsWasmError = serde_err.into();

    match &ojs_err {
        OjsWasmError::Serialization(msg) => {
            assert!(!msg.is_empty(), "error message should not be empty");
        }
        _ => panic!("expected Serialization variant, got {:?}", ojs_err),
    }
}

// ===========================================================================
// From<JsValue> for OjsWasmError
// ===========================================================================

#[wasm_bindgen_test]
fn error_from_jsvalue_string() {
    let js = JsValue::from_str("something went wrong");
    let err: OjsWasmError = js.into();

    match &err {
        OjsWasmError::Js(msg) => assert_eq!(msg, "something went wrong"),
        _ => panic!("expected Js variant"),
    }
}

#[wasm_bindgen_test]
fn error_from_jsvalue_non_string() {
    let js = JsValue::from(42);
    let err: OjsWasmError = js.into();

    match &err {
        OjsWasmError::Js(msg) => {
            // Non-string JsValues use Debug formatting
            assert!(!msg.is_empty());
        }
        _ => panic!("expected Js variant"),
    }
}

#[wasm_bindgen_test]
fn error_from_jsvalue_null() {
    let js = JsValue::NULL;
    let err: OjsWasmError = js.into();

    match &err {
        OjsWasmError::Js(msg) => assert!(!msg.is_empty()),
        _ => panic!("expected Js variant"),
    }
}

#[wasm_bindgen_test]
fn error_from_jsvalue_undefined() {
    let js = JsValue::UNDEFINED;
    let err: OjsWasmError = js.into();

    match &err {
        OjsWasmError::Js(msg) => assert!(!msg.is_empty()),
        _ => panic!("expected Js variant"),
    }
}

#[wasm_bindgen_test]
fn error_from_jsvalue_boolean() {
    let js = JsValue::from(false);
    let err: OjsWasmError = js.into();

    match &err {
        OjsWasmError::Js(_) => {} // just verify it's the right variant
        _ => panic!("expected Js variant"),
    }
}

// ===========================================================================
// ServerError — serde
// ===========================================================================

#[wasm_bindgen_test]
fn server_error_deserialization_full() {
    let json = r#"{"code": "internal_error", "message": "Unexpected failure", "retryable": true}"#;
    let err: ServerError = serde_json::from_str(json).unwrap();
    assert_eq!(err.code, "internal_error");
    assert_eq!(err.message, "Unexpected failure");
    assert!(err.retryable);
}

#[wasm_bindgen_test]
fn server_error_retryable_defaults_false() {
    let json = r#"{"code": "not_found", "message": "Job not found"}"#;
    let err: ServerError = serde_json::from_str(json).unwrap();
    assert!(!err.retryable, "retryable should default to false");
}

#[wasm_bindgen_test]
fn server_error_serialization_roundtrip() {
    let err = ServerError {
        code: "timeout".to_string(),
        message: "Request timed out".to_string(),
        retryable: true,
    };

    let json = serde_json::to_string(&err).unwrap();
    let back: ServerError = serde_json::from_str(&json).unwrap();
    assert_eq!(err.code, back.code);
    assert_eq!(err.message, back.message);
    assert_eq!(err.retryable, back.retryable);
}

#[wasm_bindgen_test]
fn server_error_clone() {
    let err = ServerError {
        code: "test".to_string(),
        message: "test message".to_string(),
        retryable: true,
    };
    let cloned = err.clone();
    assert_eq!(err.code, cloned.code);
    assert_eq!(err.message, cloned.message);
    assert_eq!(err.retryable, cloned.retryable);
}

// ===========================================================================
// Error debug representation
// ===========================================================================

#[wasm_bindgen_test]
fn error_debug_format() {
    let err = OjsWasmError::Transport("test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("Transport"));
    assert!(debug.contains("test"));
}

#[wasm_bindgen_test]
fn server_error_debug_format() {
    let err = ServerError {
        code: "err".to_string(),
        message: "msg".to_string(),
        retryable: false,
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("ServerError"));
}
