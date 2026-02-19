//! # OJS WASM SDK
//!
//! A browser-native OJS client compiled to WebAssembly via wasm-bindgen.
//! Provides job enqueue, batch enqueue, get, cancel, and health check
//! operations using the browser Fetch API.
//!
//! ## Modules
//!
//! - [`transport`] — Browser `window.fetch` based HTTP transport.
//! - [`service_worker`] — Service Worker client with background sync and push notifications.
//! - [`edge`] — Edge runtime clients (Cloudflare Workers, Deno Deploy, Vercel Edge).

pub mod edge;
pub mod error;
pub mod service_worker;
pub mod transport;
pub mod types;

use error::{OjsWasmError, Result};
use types::{BatchRequest, BatchResponse, EnqueueRequest, HealthResponse, JobResponse};
use wasm_bindgen::prelude::*;

const BASE_PATH: &str = "/ojs/v1";

/// OJS client for enqueueing and managing jobs from the browser.
#[wasm_bindgen]
pub struct OJSClient {
    base_url: String,
}

#[wasm_bindgen]
impl OJSClient {
    /// Create a new OJS client pointing at the given server URL.
    ///
    /// # Example
    /// ```js
    /// const client = new OJSClient("http://localhost:8080");
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        let base_url = format!("{}{}", url.trim_end_matches('/'), BASE_PATH);
        Self { base_url }
    }

    /// Enqueue a single job.
    ///
    /// `args` should be a JS array, e.g. `["user@example.com", "Hello"]`.
    /// Returns the created job object.
    pub async fn enqueue(&self, job_type: &str, args: JsValue) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_inner(job_type, args)
            .await
            .map_err(JsValue::from)
    }

    /// Enqueue multiple jobs in a single batch request.
    ///
    /// `jobs` should be a JS array of objects with `type` and `args` fields:
    /// ```js
    /// await client.enqueue_batch([
    ///   { type: "email.send", args: ["a@b.com"] },
    ///   { type: "report.generate", args: [42] },
    /// ]);
    /// ```
    pub async fn enqueue_batch(&self, jobs: JsValue) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_batch_inner(jobs)
            .await
            .map_err(JsValue::from)
    }

    /// Get a job by ID.
    pub async fn get_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.get_job_inner(id).await.map_err(JsValue::from)
    }

    /// Cancel a job by ID.
    pub async fn cancel_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.cancel_job_inner(id).await.map_err(JsValue::from)
    }

    /// Health check.
    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.health_inner().await.map_err(JsValue::from)
    }
}

// ---------------------------------------------------------------------------
// Internal implementations (Result-based for ergonomics)
// ---------------------------------------------------------------------------

impl OJSClient {
    async fn enqueue_inner(&self, job_type: &str, args: JsValue) -> Result<JsValue> {
        let args_value: serde_json::Value =
            serde_wasm_bindgen::from_value(args).map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
        };

        let body = serde_json::to_string(&req)?;
        let url = format!("{}/jobs", self.base_url);
        let resp_text = transport::post(&url, &body).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn enqueue_batch_inner(&self, jobs: JsValue) -> Result<JsValue> {
        #[derive(serde::Deserialize)]
        struct JsJob {
            #[serde(rename = "type")]
            job_type: String,
            args: serde_json::Value,
        }

        let js_jobs: Vec<JsJob> =
            serde_wasm_bindgen::from_value(jobs).map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let batch = BatchRequest {
            jobs: js_jobs
                .into_iter()
                .map(|j| EnqueueRequest {
                    job_type: j.job_type,
                    args: j.args,
                })
                .collect(),
        };

        let body = serde_json::to_string(&batch)?;
        let url = format!("{}/jobs/batch", self.base_url);
        let resp_text = transport::post(&url, &body).await?;
        let resp: BatchResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.jobs).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = transport::get(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn cancel_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = transport::delete(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn health_inner(&self) -> Result<JsValue> {
        let url = format!("{}/health", self.base_url);
        let resp_text = transport::get(&url).await?;
        let resp: HealthResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }
}
