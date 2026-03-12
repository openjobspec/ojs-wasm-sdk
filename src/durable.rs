//! Durable execution with checkpoint-based crash recovery for WASM.
//!
//! Provides checkpoint persistence via the OJS server's checkpoint API,
//! enabling long-running browser jobs to survive page reloads.
//!
//! # Example
//!
//! ```js
//! import { DurableContext } from '@openjobspec/wasm';
//!
//! const ctx = new DurableContext("http://localhost:8080", "job-123");
//!
//! // Resume from last checkpoint
//! const state = await ctx.resume();
//! const startFrom = state ? JSON.parse(state) : 0;
//!
//! for (let i = startFrom; i < total; i++) {
//!   processItem(i);
//!   if (i % 100 === 0) {
//!     await ctx.checkpoint(JSON.stringify(i));
//!   }
//! }
//!
//! await ctx.delete_checkpoint();
//! ```

use crate::error::{OjsWasmError, Result};
use crate::transport;
use wasm_bindgen::prelude::*;

/// Context for durable job execution with checkpoint support.
///
/// Communicates with the OJS server's checkpoint API to persist
/// intermediate state that survives page reloads or process crashes.
#[wasm_bindgen]
pub struct DurableContext {
    base_url: String,
    job_id: String,
}

#[wasm_bindgen]
impl DurableContext {
    /// Create a new durable context for a specific job.
    ///
    /// `url` is the OJS server base URL (e.g., "http://localhost:8080").
    /// `job_id` is the job whose checkpoint state will be managed.
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str, job_id: &str) -> Self {
        Self {
            base_url: format!("{}/ojs/v1", url.trim_end_matches('/')),
            job_id: job_id.to_string(),
        }
    }

    /// Save a checkpoint with the given state (JSON string).
    ///
    /// Overwrites any previous checkpoint for this job.
    pub async fn checkpoint(&self, state: &str) -> std::result::Result<(), JsValue> {
        self.checkpoint_inner(state).await.map_err(JsValue::from)
    }

    /// Resume from the last checkpoint.
    ///
    /// Returns the saved state as a JSON string, or `null` if no checkpoint exists.
    pub async fn resume(&self) -> std::result::Result<JsValue, JsValue> {
        self.resume_inner().await.map_err(JsValue::from)
    }

    /// Delete the checkpoint for this job.
    pub async fn delete_checkpoint(&self) -> std::result::Result<(), JsValue> {
        self.delete_inner().await.map_err(JsValue::from)
    }

    /// Get the job ID this context is associated with.
    pub fn job_id(&self) -> String {
        self.job_id.clone()
    }
}

impl DurableContext {
    fn checkpoint_url(&self) -> String {
        format!("{}/jobs/{}/checkpoint", self.base_url, self.job_id)
    }

    async fn checkpoint_inner(&self, state: &str) -> Result<()> {
        // Validate that state is valid JSON
        let state_value: serde_json::Value = serde_json::from_str(state)
            .map_err(|e| OjsWasmError::Serialization(format!("state must be valid JSON: {}", e)))?;

        let body = serde_json::json!({ "state": state_value });
        let body_str = serde_json::to_string(&body)?;

        transport::put(&self.checkpoint_url(), &body_str).await?;
        Ok(())
    }

    async fn resume_inner(&self) -> Result<JsValue> {
        match transport::get_with_status(&self.checkpoint_url()).await {
            Ok((status, body)) => {
                if status == 404 {
                    return Ok(JsValue::NULL);
                }
                let parsed: serde_json::Value = serde_json::from_str(&body)?;
                if let Some(state) = parsed.get("state") {
                    let state_str = serde_json::to_string(state)?;
                    Ok(JsValue::from_str(&state_str))
                } else {
                    Ok(JsValue::NULL)
                }
            }
            Err(OjsWasmError::Server(ref e)) if e.code == "NOT_FOUND" => Ok(JsValue::NULL),
            Err(e) => Err(e),
        }
    }

    async fn delete_inner(&self) -> Result<()> {
        match transport::delete(&self.checkpoint_url()).await {
            Ok(_) => Ok(()),
            Err(OjsWasmError::Server(ref e)) if e.code == "NOT_FOUND" => Ok(()),
            Err(e) => Err(e),
        }
    }
}
