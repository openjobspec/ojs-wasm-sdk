//! # Service Worker Support
//!
//! Provides an OJS client that operates within a Service Worker context,
//! where there is no `window` object. Includes background sync integration
//! for offline job enqueueing and push notification support for job
//! completion events.

use crate::error::{ErrorResponse, OjsWasmError, Result};
use crate::types::{BatchRequest, BatchResponse, EnqueueRequest, JobResponse};
use js_sys::{Function, Object, Promise, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};

const BASE_PATH: &str = "/ojs/v1";
const SYNC_TAG_PREFIX: &str = "ojs-enqueue-";

// ---------------------------------------------------------------------------
// Global-scope fetch (works in Service Workers, Worklets, etc.)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "fetch")]
    fn global_fetch(input: &Request) -> Promise;
}

/// Returns the global `self` object (works in Service Workers, Workers, etc.).
fn sw_global() -> JsValue {
    js_sys::global().into()
}

fn build_request(method: &str, url: &str, body: Option<String>) -> Result<Request> {
    let opts = RequestInit::new();
    opts.set_method(method);

    let headers = Headers::new().map_err(OjsWasmError::from)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(OjsWasmError::from)?;
    opts.set_headers(&headers);

    if let Some(b) = body {
        let js_body = JsValue::from_str(&b);
        opts.set_body(&js_body);
    }

    Request::new_with_str_and_init(url, &opts).map_err(OjsWasmError::from)
}

async fn execute(request: Request) -> Result<String> {
    let resp_value = JsFuture::from(global_fetch(&request))
        .await
        .map_err(OjsWasmError::from)?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| OjsWasmError::Transport("response is not a Response".into()))?;

    let text = JsFuture::from(resp.text().map_err(OjsWasmError::from)?)
        .await
        .map_err(OjsWasmError::from)?;

    let body = text.as_string().unwrap_or_default();

    if !resp.ok() {
        if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            return Err(OjsWasmError::Server(crate::error::ServerError {
                code: err_resp.error.code,
                message: err_resp.error.message,
                retryable: err_resp.error.retryable,
            }));
        }
        return Err(OjsWasmError::Transport(format!(
            "HTTP {}: {}",
            resp.status(),
            body
        )));
    }

    Ok(body)
}

async fn sw_post(url: &str, body: &str) -> Result<String> {
    let request = build_request("POST", url, Some(body.to_string()))?;
    execute(request).await
}

async fn sw_get(url: &str) -> Result<String> {
    let request = build_request("GET", url, None)?;
    execute(request).await
}

async fn sw_delete(url: &str) -> Result<String> {
    let request = build_request("DELETE", url, None)?;
    execute(request).await
}

// ---------------------------------------------------------------------------
// Pending job storage for offline sync
// ---------------------------------------------------------------------------

/// A serializable record of a job that was enqueued while offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingJob {
    pub job_type: String,
    pub args: serde_json::Value,
    pub created_at: f64,
}

// ---------------------------------------------------------------------------
// ServiceWorkerClient
// ---------------------------------------------------------------------------

/// OJS client designed for the Service Worker global scope.
///
/// Unlike [`OJSClient`](crate::OJSClient) which relies on `window.fetch`,
/// this client uses the global `fetch()` function available in Service
/// Workers, Shared Workers, and other non-window contexts.
///
/// # Example
///
/// ```js
/// // Inside a Service Worker script
/// import { ServiceWorkerClient } from '@openjobspec/wasm';
///
/// const client = new ServiceWorkerClient("https://api.example.com");
/// const job = await client.enqueue("email.send", ["user@example.com"]);
/// ```
#[wasm_bindgen]
pub struct ServiceWorkerClient {
    base_url: String,
}

#[wasm_bindgen]
impl ServiceWorkerClient {
    /// Create a new Service Worker–scoped OJS client.
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        let base_url = format!("{}{}", url.trim_end_matches('/'), BASE_PATH);
        Self { base_url }
    }

    /// Enqueue a single job using the global `fetch()`.
    pub async fn enqueue(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_inner(job_type, args)
            .await
            .map_err(JsValue::from)
    }

    /// Enqueue multiple jobs in a single batch request.
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

    /// Health check.
    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.health_inner().await.map_err(JsValue::from)
    }

    // -- Background Sync helpers --------------------------------------------

    /// Register a Background Sync tag for deferred job enqueue.
    ///
    /// Call this when the network is unavailable. The browser will fire
    /// a `sync` event with the returned tag once connectivity is restored.
    ///
    /// ```js
    /// // In your main page / worker registration:
    /// await client.register_sync("email.send", ["user@example.com"]);
    /// ```
    pub async fn register_sync(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.register_sync_inner(job_type, args)
            .await
            .map_err(JsValue::from)
    }

    /// Process all pending sync jobs for the given `sync` event tag.
    ///
    /// Call this from your Service Worker `sync` event handler:
    ///
    /// ```js
    /// self.addEventListener('sync', (event) => {
    ///   if (event.tag.startsWith('ojs-enqueue-')) {
    ///     event.waitUntil(client.process_sync(event.tag));
    ///   }
    /// });
    /// ```
    pub async fn process_sync(
        &self,
        tag: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.process_sync_inner(tag)
            .await
            .map_err(JsValue::from)
    }

    // -- Push Notification helpers ------------------------------------------

    /// Show a push notification when a job completes.
    ///
    /// Designed to be called from a `push` event handler:
    ///
    /// ```js
    /// self.addEventListener('push', (event) => {
    ///   const data = event.data.json();
    ///   event.waitUntil(
    ///     client.notify_job_completed(data.job_id, data.job_type, data.state)
    ///   );
    /// });
    /// ```
    pub async fn notify_job_completed(
        &self,
        job_id: &str,
        job_type: &str,
        state: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.notify_job_completed_inner(job_id, job_type, state)
            .await
            .map_err(JsValue::from)
    }
}

// ---------------------------------------------------------------------------
// Internal implementations
// ---------------------------------------------------------------------------

impl ServiceWorkerClient {
    async fn enqueue_inner(&self, job_type: &str, args: JsValue) -> Result<JsValue> {
        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
        };

        let body = serde_json::to_string(&req)?;
        let url = format!("{}/jobs", self.base_url);
        let resp_text = sw_post(&url, &body).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn enqueue_batch_inner(&self, jobs: JsValue) -> Result<JsValue> {
        #[derive(Deserialize)]
        struct JsJob {
            #[serde(rename = "type")]
            job_type: String,
            args: serde_json::Value,
        }

        let js_jobs: Vec<JsJob> = serde_wasm_bindgen::from_value(jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

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
        let resp_text = sw_post(&url, &body).await?;
        let resp: BatchResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = sw_get(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn cancel_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = sw_delete(&url).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn health_inner(&self) -> Result<JsValue> {
        let url = format!("{}/health", self.base_url);
        let resp_text = sw_get(&url).await?;
        let resp: crate::types::HealthResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    // -- Background Sync internals ------------------------------------------

    async fn register_sync_inner(&self, job_type: &str, args: JsValue) -> Result<JsValue> {
        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let pending = PendingJob {
            job_type: job_type.to_string(),
            args: args_value,
            created_at: js_sys::Date::now(),
        };

        let tag = format!("{}{}", SYNC_TAG_PREFIX, js_sys::Date::now() as u64);

        // Store the pending job in the global scope so the sync handler
        // can retrieve it later. We attach it to `self.__ojs_pending`.
        let pending_json = serde_json::to_string(&pending)?;
        let storage = get_or_create_pending_storage()?;
        Reflect::set(
            &storage,
            &JsValue::from_str(&tag),
            &JsValue::from_str(&pending_json),
        )
        .map_err(OjsWasmError::from)?;

        // Request background sync via the SyncManager
        let registration = Reflect::get(&sw_global(), &JsValue::from_str("registration"))
            .map_err(OjsWasmError::from)?;

        if !registration.is_undefined() {
            let sync = Reflect::get(&registration, &JsValue::from_str("sync"))
                .map_err(OjsWasmError::from)?;

            if !sync.is_undefined() {
                let register_fn = Reflect::get(&sync, &JsValue::from_str("register"))
                    .map_err(OjsWasmError::from)?;
                let register_fn: Function = register_fn
                    .dyn_into()
                    .map_err(|_| OjsWasmError::Js("sync.register is not a function".into()))?;
                let promise: Promise = register_fn
                    .call1(&sync, &JsValue::from_str(&tag))
                    .map_err(OjsWasmError::from)?
                    .dyn_into()
                    .map_err(|_| OjsWasmError::Js("sync.register did not return a Promise".into()))?;
                JsFuture::from(promise).await.map_err(OjsWasmError::from)?;
            }
        }

        Ok(JsValue::from_str(&tag))
    }

    async fn process_sync_inner(&self, tag: &str) -> Result<JsValue> {
        let storage = get_or_create_pending_storage()?;

        let pending_val = Reflect::get(&storage, &JsValue::from_str(tag))
            .map_err(OjsWasmError::from)?;

        if pending_val.is_undefined() {
            return Err(OjsWasmError::Js(format!("no pending job for tag: {}", tag)));
        }

        let pending_json = pending_val
            .as_string()
            .ok_or_else(|| OjsWasmError::Serialization("pending job is not a string".into()))?;

        let pending: PendingJob = serde_json::from_str(&pending_json)?;

        // Enqueue the deferred job
        let req = EnqueueRequest {
            job_type: pending.job_type,
            args: pending.args,
        };
        let body = serde_json::to_string(&req)?;
        let url = format!("{}/jobs", self.base_url);
        let resp_text = sw_post(&url, &body).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        // Remove from pending storage
        Reflect::delete_property(
            &storage.dyn_into::<Object>().map_err(OjsWasmError::from)?,
            &JsValue::from_str(tag).into(),
        )
        .map_err(OjsWasmError::from)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    // -- Push Notification internals ----------------------------------------

    async fn notify_job_completed_inner(
        &self,
        job_id: &str,
        job_type: &str,
        state: &str,
    ) -> Result<JsValue> {
        let title = format!("Job {}", state);
        let body_text = format!("{} ({})", job_type, &job_id[..8.min(job_id.len())]);

        // Build notification options: { body, tag, data: { jobId, jobType, state } }
        let options = Object::new();
        Reflect::set(&options, &"body".into(), &JsValue::from_str(&body_text))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&options, &"tag".into(), &JsValue::from_str(&format!("ojs-job-{}", job_id)))
            .map_err(OjsWasmError::from)?;

        let data = Object::new();
        Reflect::set(&data, &"jobId".into(), &JsValue::from_str(job_id))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&data, &"jobType".into(), &JsValue::from_str(job_type))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&data, &"state".into(), &JsValue::from_str(state))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&options, &"data".into(), &data)
            .map_err(OjsWasmError::from)?;

        // Call self.registration.showNotification(title, options)
        let registration = Reflect::get(&sw_global(), &JsValue::from_str("registration"))
            .map_err(OjsWasmError::from)?;

        if registration.is_undefined() {
            return Err(OjsWasmError::Js(
                "not running in a Service Worker context (no registration)".into(),
            ));
        }

        let show_fn = Reflect::get(&registration, &JsValue::from_str("showNotification"))
            .map_err(OjsWasmError::from)?;
        let show_fn: Function = show_fn
            .dyn_into()
            .map_err(|_| OjsWasmError::Js("showNotification is not a function".into()))?;

        let promise: Promise = show_fn
            .call2(&registration, &JsValue::from_str(&title), &options)
            .map_err(OjsWasmError::from)?
            .dyn_into()
            .map_err(|_| OjsWasmError::Js("showNotification did not return a Promise".into()))?;

        JsFuture::from(promise).await.map_err(OjsWasmError::from)?;

        Ok(JsValue::TRUE)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get or create the `self.__ojs_pending` object used to store pending jobs.
fn get_or_create_pending_storage() -> Result<JsValue> {
    let key = JsValue::from_str("__ojs_pending");
    let existing = Reflect::get(&sw_global(), &key).map_err(OjsWasmError::from)?;

    if existing.is_undefined() || existing.is_null() {
        let obj = Object::new();
        Reflect::set(&sw_global(), &key, &obj).map_err(OjsWasmError::from)?;
        Ok(obj.into())
    } else {
        Ok(existing)
    }
}
