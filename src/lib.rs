//! # OJS WASM SDK
//!
//! A browser-native OJS client compiled to WebAssembly via wasm-bindgen.
//! Provides job enqueue, batch enqueue, get, cancel, workflow, and health
//! check operations using the browser Fetch API.
//!
//! ## Modules
//!
//! - [`transport`] -- Browser `window.fetch` based HTTP transport.
//! - [`service_worker`] -- Service Worker client with background sync and push notifications.
//! - [`edge`] -- Edge runtime clients (Cloudflare Workers, Deno Deploy, Vercel Edge).
//! - [`workflow`] -- Workflow builder functions (chain, group, batch).
//! - [`middleware`] -- Request/response middleware chain.
//! - [`retry`] -- Retry policy configuration.
//! - [`queue`] -- Queue management operations.

pub mod edge;
pub mod error;
pub mod middleware;
pub mod queue;
pub mod retry;
pub mod service_worker;
pub mod transport;
pub mod types;
pub mod workflow;

use error::{OjsWasmError, Result};
use types::{
    BatchRequest, BatchResponse, EnqueueRequest, HealthResponse, JobResponse, WorkflowResponse,
};
use wasm_bindgen::prelude::*;

const BASE_PATH: &str = "/ojs/v1";

/// OJS client for enqueueing and managing jobs from the browser.
///
/// Uses `window.fetch` under the hood. For Service Worker or edge runtime
/// contexts, use [`ServiceWorkerClient`](service_worker::ServiceWorkerClient)
/// or one of the edge clients in the [`edge`] module.
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
    pub async fn enqueue(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_inner(job_type, args, None)
            .await
            .map_err(JsValue::from)
    }

    /// Enqueue a single job with options.
    ///
    /// `options` is a JS object with optional fields: `queue`, `priority`,
    /// `timeout_ms`, `delay_until`, `tags`.
    ///
    /// ```js
    /// await client.enqueue_with_options("email.send", ["user@example.com"], {
    ///   queue: "critical",
    ///   priority: 10,
    ///   tags: ["onboarding"],
    /// });
    /// ```
    pub async fn enqueue_with_options(
        &self,
        job_type: &str,
        args: JsValue,
        options: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        let opts: types::EnqueueOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&format!("invalid options: {}", e)))?;
        self.enqueue_inner(job_type, args, Some(opts))
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
    pub async fn enqueue_batch(
        &self,
        jobs: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
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

    /// Create and start a workflow.
    ///
    /// Pass a workflow definition built with `chain()`, `group()`, or `batch()`
    /// from the `workflow` module.
    ///
    /// ```js
    /// import { chain } from '@openjobspec/wasm';
    ///
    /// const status = await client.workflow(chain(
    ///   { type: "data.fetch", args: ["https://..."] },
    ///   { type: "data.transform", args: ["csv"] },
    /// ));
    /// ```
    pub async fn workflow(
        &self,
        definition: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.workflow_inner(definition)
            .await
            .map_err(JsValue::from)
    }

    /// Get the status of a workflow by ID.
    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.get_workflow_inner(workflow_id)
            .await
            .map_err(JsValue::from)
    }

    /// Health check.
    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.health_inner().await.map_err(JsValue::from)
    }

    /// List all queues.
    pub async fn list_queues(&self) -> std::result::Result<JsValue, JsValue> {
        let qm = queue::QueueManager::new(&self.base_url);
        qm.list_queues().await.map_err(JsValue::from)
    }

    /// Get statistics for a specific queue.
    pub async fn queue_stats(&self, queue_name: &str) -> std::result::Result<JsValue, JsValue> {
        let qm = queue::QueueManager::new(&self.base_url);
        qm.queue_stats(queue_name).await.map_err(JsValue::from)
    }

    /// Pause a queue.
    pub async fn pause_queue(&self, queue_name: &str) -> std::result::Result<(), JsValue> {
        let qm = queue::QueueManager::new(&self.base_url);
        qm.pause_queue(queue_name).await.map_err(JsValue::from)
    }

    /// Resume a paused queue.
    pub async fn resume_queue(&self, queue_name: &str) -> std::result::Result<(), JsValue> {
        let qm = queue::QueueManager::new(&self.base_url);
        qm.resume_queue(queue_name).await.map_err(JsValue::from)
    }
}

// ---------------------------------------------------------------------------
// Internal implementations (Result-based for ergonomics)
// ---------------------------------------------------------------------------

impl OJSClient {
    async fn enqueue_inner(
        &self,
        job_type: &str,
        args: JsValue,
        options: Option<types::EnqueueOptions>,
    ) -> Result<JsValue> {
        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
            options,
        };

        let body = serde_json::to_string(&req)?;
        let url = format!("{}/jobs", self.base_url);
        let resp_text = transport::post(&url, &body).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn enqueue_batch_inner(&self, jobs: JsValue) -> Result<JsValue> {
        #[derive(serde::Deserialize)]
        struct JsJob {
            #[serde(rename = "type")]
            job_type: String,
            args: serde_json::Value,
            #[serde(default)]
            options: Option<types::EnqueueOptions>,
        }

        let js_jobs: Vec<JsJob> = serde_wasm_bindgen::from_value(jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let batch = BatchRequest {
            jobs: js_jobs
                .into_iter()
                .map(|j| EnqueueRequest {
                    job_type: j.job_type,
                    args: j.args,
                    options: j.options,
                })
                .collect(),
        };

        let body = serde_json::to_string(&batch)?;
        let url = format!("{}/jobs/batch", self.base_url);
        let resp_text = transport::post(&url, &body).await?;
        let resp: BatchResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = transport::get(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn cancel_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = transport::delete(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn workflow_inner(&self, definition: JsValue) -> Result<JsValue> {
        let wire: serde_json::Value = serde_wasm_bindgen::from_value(definition)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let body = serde_json::to_string(&wire)?;
        let url = format!("{}/workflows", self.base_url);
        let resp_text = transport::post(&url, &body).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_workflow_inner(&self, workflow_id: &str) -> Result<JsValue> {
        let url = format!("{}/workflows/{}", self.base_url, workflow_id);
        let resp_text = transport::get(&url).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn health_inner(&self) -> Result<JsValue> {
        let url = format!("{}/health", self.base_url);
        let resp_text = transport::get(&url).await?;
        let resp: HealthResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }
}
