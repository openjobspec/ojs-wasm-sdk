//! # Retry Configuration
//!
//! Retry policy types for job enqueue requests. These map directly to the
//! OJS retry extension specification.
//!
//! # Example
//!
//! ```js
//! import init, { OJSClient, RetryPolicy } from '@openjobspec/wasm';
//!
//! const client = new OJSClient("http://localhost:8080");
//!
//! // Create a retry policy
//! const retry = RetryPolicy.exponential(5, 1000, 60000);
//!
//! await client.enqueue_with_options("email.send", ["user@example.com"], {
//!   retry: retry.to_object(),
//! });
//! ```

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

/// Retry policy configuration for job enqueue requests.
#[wasm_bindgen]
pub struct RetryPolicy {
    max_attempts: u32,
    backoff_type: String,
    initial_delay_ms: u32,
    max_delay_ms: u32,
}

#[wasm_bindgen]
impl RetryPolicy {
    /// Create an exponential backoff retry policy.
    ///
    /// - `max_attempts`: Maximum number of retry attempts
    /// - `initial_delay_ms`: Initial delay between retries in milliseconds
    /// - `max_delay_ms`: Maximum delay cap in milliseconds
    pub fn exponential(max_attempts: u32, initial_delay_ms: u32, max_delay_ms: u32) -> Self {
        Self {
            max_attempts,
            backoff_type: "exponential".to_string(),
            initial_delay_ms,
            max_delay_ms,
        }
    }

    /// Create a fixed-interval retry policy.
    pub fn fixed(max_attempts: u32, delay_ms: u32) -> Self {
        Self {
            max_attempts,
            backoff_type: "fixed".to_string(),
            initial_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
        }
    }

    /// Create a linear backoff retry policy.
    pub fn linear(max_attempts: u32, initial_delay_ms: u32, max_delay_ms: u32) -> Self {
        Self {
            max_attempts,
            backoff_type: "linear".to_string(),
            initial_delay_ms,
            max_delay_ms,
        }
    }

    /// Convert to a JS object suitable for use in enqueue options.
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        Reflect::set(
            &obj,
            &"max_attempts".into(),
            &JsValue::from(self.max_attempts),
        )?;

        let backoff = Object::new();
        Reflect::set(
            &backoff,
            &"type".into(),
            &JsValue::from_str(&self.backoff_type),
        )?;
        Reflect::set(
            &backoff,
            &"initial_ms".into(),
            &JsValue::from(self.initial_delay_ms),
        )?;
        Reflect::set(
            &backoff,
            &"max_ms".into(),
            &JsValue::from(self.max_delay_ms),
        )?;

        Reflect::set(&obj, &"backoff".into(), &backoff)?;
        Ok(obj.into())
    }
}
