//! # Service Worker Support
//!
//! Provides an OJS client that operates within a Service Worker context,
//! where there is no `window` object. Includes background sync integration
//! for offline job enqueueing and push notification support for job
//! completion events.

use crate::error::{OjsWasmError, Result};
use crate::types::{
    BatchJobInput, BatchRequest, BatchResponse, EnqueueRequest, JobResponse, WorkflowResponse,
};
use js_sys::{Function, Object, Promise, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};

const BASE_PATH: &str = "/ojs/v1";
const SYNC_TAG_PREFIX: &str = "ojs-enqueue-";
const SYNC_LEASE_MS: f64 = 5.0 * 60.0 * 1000.0;

#[wasm_bindgen(inline_js = r#"
const OJS_SYNC_DB = "openjobspec-background-sync";
const OJS_SYNC_STORE = "pending-jobs";

function ojsSyncError(error, fallback) {
  if (error instanceof Error) return error;
  if (error && typeof error.message === "string") return new Error(error.message);
  return new Error(fallback);
}

function ojsOpenSyncDatabase() {
  if (!globalThis.indexedDB) {
    return Promise.reject(new Error("IndexedDB is unavailable; durable Background Sync storage is required"));
  }

  return new Promise((resolve, reject) => {
    const request = globalThis.indexedDB.open(OJS_SYNC_DB, 1);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(OJS_SYNC_STORE)) {
        database.createObjectStore(OJS_SYNC_STORE);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(ojsSyncError(request.error, "failed to open Background Sync database"));
    request.onblocked = () => reject(new Error("Background Sync database upgrade is blocked"));
  });
}

async function ojsSyncTransaction(mode, operation) {
  const database = await ojsOpenSyncDatabase();
  try {
    return await new Promise((resolve, reject) => {
      let transaction;
      try {
        transaction = database.transaction(OJS_SYNC_STORE, mode);
      } catch (error) {
        reject(ojsSyncError(error, "failed to create Background Sync transaction"));
        return;
      }

      let result;
      let failure;
      const fail = (error, fallback) => {
        failure = ojsSyncError(error, fallback);
        try {
          transaction.abort();
        } catch (abortError) {
          reject(ojsSyncError(abortError, failure.message));
        }
      };

      transaction.oncomplete = () => resolve(result);
      transaction.onabort = () => reject(
        failure || ojsSyncError(transaction.error, "Background Sync transaction aborted")
      );
      transaction.onerror = () => {
        failure ||= ojsSyncError(transaction.error, "Background Sync transaction failed");
      };

      try {
        operation(
          transaction.objectStore(OJS_SYNC_STORE),
          (value) => { result = value; },
          fail,
        );
      } catch (error) {
        fail(error, "Background Sync storage operation failed");
      }
    });
  } finally {
    database.close();
  }
}

export async function ojsBackgroundSyncPut(tag, recordJson) {
  const record = JSON.parse(recordJson);
  return ojsSyncTransaction("readwrite", (store, setResult, fail) => {
    const request = store.put(record, tag);
    request.onsuccess = () => setResult(true);
    request.onerror = () => fail(request.error, `failed to persist pending job ${tag}`);
  });
}

export async function ojsBackgroundSyncAcquire(tag, leaseId, now, leaseMs) {
  return ojsSyncTransaction("readwrite", (store, setResult, fail) => {
    const request = store.get(tag);
    request.onerror = () => fail(request.error, `failed to read pending job ${tag}`);
    request.onsuccess = () => {
      const record = request.result;
      if (record === undefined) {
        fail(new Error(`no pending job for tag: ${tag}`), "pending job not found");
        return;
      }
      if (record.lease_id && Number(record.lease_until) > now) {
        fail(new Error(`pending job is already leased: ${tag}`), "pending job already leased");
        return;
      }

      record.lease_id = leaseId;
      record.lease_until = now + leaseMs;
      const putRequest = store.put(record, tag);
      putRequest.onerror = () => fail(putRequest.error, `failed to lease pending job ${tag}`);
      putRequest.onsuccess = () => setResult(JSON.stringify(record));
    };
  });
}

export async function ojsBackgroundSyncRelease(tag, leaseId) {
  return ojsSyncTransaction("readwrite", (store, setResult, fail) => {
    const request = store.get(tag);
    request.onerror = () => fail(request.error, `failed to read pending job ${tag}`);
    request.onsuccess = () => {
      const record = request.result;
      if (record === undefined || record.lease_id !== leaseId) {
        setResult(false);
        return;
      }

      delete record.lease_id;
      delete record.lease_until;
      const putRequest = store.put(record, tag);
      putRequest.onerror = () => fail(putRequest.error, `failed to release pending job ${tag}`);
      putRequest.onsuccess = () => setResult(true);
    };
  });
}

export async function ojsBackgroundSyncDelete(tag, leaseId) {
  return ojsSyncTransaction("readwrite", (store, setResult, fail) => {
    const request = store.get(tag);
    request.onerror = () => fail(request.error, `failed to read pending job ${tag}`);
    request.onsuccess = () => {
      const record = request.result;
      if (record === undefined) {
        setResult(false);
        return;
      }
      if (record.lease_id !== leaseId) {
        fail(new Error(`pending job lease changed before deletion: ${tag}`), "pending job lease changed");
        return;
      }

      const deleteRequest = store.delete(tag);
      deleteRequest.onerror = () => fail(deleteRequest.error, `failed to delete pending job ${tag}`);
      deleteRequest.onsuccess = () => setResult(true);
    };
  });
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = ojsBackgroundSyncPut)]
    fn idb_put_pending(tag: &str, record_json: &str) -> Promise;

    #[wasm_bindgen(js_name = ojsBackgroundSyncAcquire)]
    fn idb_acquire_pending(tag: &str, lease_id: &str, now: f64, lease_ms: f64) -> Promise;

    #[wasm_bindgen(js_name = ojsBackgroundSyncRelease)]
    fn idb_release_pending(tag: &str, lease_id: &str) -> Promise;

    #[wasm_bindgen(js_name = ojsBackgroundSyncDelete)]
    fn idb_delete_pending(tag: &str, lease_id: &str) -> Promise;
}

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
    crate::error::interpret_response(resp.ok(), resp.status(), body)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPendingJob {
    #[serde(flatten)]
    pending: PendingJob,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lease_until: Option<f64>,
}

/// Prefix used for every Background Sync tag created by this SDK.
///
/// Use this exported value when filtering Service Worker `sync` events rather
/// than duplicating the prefix in application code.
#[wasm_bindgen]
pub fn background_sync_tag_prefix() -> String {
    SYNC_TAG_PREFIX.to_string()
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
        self.enqueue_inner(job_type, args, None)
            .await
            .map_err(JsValue::from)
    }

    /// Enqueue a single job with options.
    pub async fn enqueue_with_options(
        &self,
        job_type: &str,
        args: JsValue,
        options: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        let opts: crate::types::EnqueueOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&format!("invalid options: {}", e)))?;
        self.enqueue_inner(job_type, args, Some(opts))
            .await
            .map_err(JsValue::from)
    }

    /// Enqueue multiple jobs in a single batch request.
    pub async fn enqueue_batch(&self, jobs: JsValue) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_batch_inner(jobs).await.map_err(JsValue::from)
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
    pub async fn workflow(&self, definition: JsValue) -> std::result::Result<JsValue, JsValue> {
        self.workflow_inner(definition).await.map_err(JsValue::from)
    }

    /// Get the status of a workflow by ID.
    pub async fn get_workflow(&self, workflow_id: &str) -> std::result::Result<JsValue, JsValue> {
        self.get_workflow_inner(workflow_id)
            .await
            .map_err(JsValue::from)
    }

    /// Health check.
    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.health_inner().await.map_err(JsValue::from)
    }

    // -- Background Sync helpers --------------------------------------------

    /// Register a Background Sync tag for deferred job enqueue.
    ///
    /// Call this when the network is unavailable. The browser will fire
    /// a `sync` event with the returned tag once connectivity is restored. The
    /// pending job is committed to IndexedDB before registration, so it
    /// survives Service Worker termination. If `SyncManager` is unavailable,
    /// this method returns an explicit error and no tag.
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
    /// const syncPrefix = background_sync_tag_prefix();
    /// self.addEventListener('sync', (event) => {
    ///   if (event.tag.startsWith(syncPrefix)) {
    ///     event.waitUntil(client.process_sync(event.tag));
    ///   }
    /// });
    /// ```
    pub async fn process_sync(&self, tag: &str) -> std::result::Result<JsValue, JsValue> {
        self.process_sync_inner(tag).await.map_err(JsValue::from)
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
    async fn enqueue_inner(
        &self,
        job_type: &str,
        args: JsValue,
        options: Option<crate::types::EnqueueOptions>,
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
        let resp_text = sw_post(&url, &body).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn enqueue_batch_inner(&self, jobs: JsValue) -> Result<JsValue> {
        let js_jobs: Vec<BatchJobInput> = serde_wasm_bindgen::from_value(jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let batch = BatchRequest {
            jobs: js_jobs.into_iter().map(EnqueueRequest::from).collect(),
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

    async fn workflow_inner(&self, definition: JsValue) -> Result<JsValue> {
        let wire: serde_json::Value = serde_wasm_bindgen::from_value(definition)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let body = serde_json::to_string(&wire)?;
        let url = format!("{}/workflows", self.base_url);
        let resp_text = sw_post(&url, &body).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_workflow_inner(&self, workflow_id: &str) -> Result<JsValue> {
        let url = format!("{}/workflows/{}", self.base_url, workflow_id);
        let resp_text = sw_get(&url).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn health_inner(&self) -> Result<JsValue> {
        let url = format!("{}/health", self.base_url);
        let resp_text = sw_get(&url).await?;
        let resp: crate::types::HealthResponse = serde_json::from_str(&resp_text)?;

        serde_wasm_bindgen::to_value(&resp).map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    // -- Background Sync internals ------------------------------------------

    async fn register_sync_inner(&self, job_type: &str, args: JsValue) -> Result<JsValue> {
        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;
        let (sync, register_fn) = sync_register_function()?;

        let pending = PendingJob {
            job_type: job_type.to_string(),
            args: args_value,
            created_at: js_sys::Date::now(),
        };
        let stored = StoredPendingJob {
            pending,
            lease_id: None,
            lease_until: None,
        };

        let tag = new_sync_tag();
        let pending_json = serde_json::to_string(&stored)?;
        await_storage(
            idb_put_pending(&tag, &pending_json),
            "failed to persist pending Background Sync job",
        )
        .await?;

        let promise: Promise = register_fn
            .call1(&sync, &JsValue::from_str(&tag))
            .map_err(|error| {
                OjsWasmError::Js(format!(
                    "Background Sync registration failed for persisted tag '{}': {}",
                    tag,
                    js_error_message(&error)
                ))
            })?
            .dyn_into()
            .map_err(|_| {
                OjsWasmError::Js(format!(
                    "sync.register did not return a Promise; pending tag '{}' remains persisted",
                    tag
                ))
            })?;
        JsFuture::from(promise).await.map_err(|error| {
            OjsWasmError::Js(format!(
                "Background Sync registration failed for persisted tag '{}': {}",
                tag,
                js_error_message(&error)
            ))
        })?;

        Ok(JsValue::from_str(&tag))
    }

    async fn process_sync_inner(&self, tag: &str) -> Result<JsValue> {
        if !tag.starts_with(SYNC_TAG_PREFIX) {
            return Err(OjsWasmError::Validation(format!(
                "invalid Background Sync tag: {}",
                tag
            )));
        }

        let lease_id = new_unique_suffix();
        let pending_val = await_storage(
            idb_acquire_pending(tag, &lease_id, js_sys::Date::now(), SYNC_LEASE_MS),
            "failed to acquire pending Background Sync job",
        )
        .await?;
        let enqueue_result = async {
            let pending_json = pending_val
                .as_string()
                .ok_or_else(|| OjsWasmError::Serialization("pending job is not JSON".into()))?;
            let stored: StoredPendingJob = serde_json::from_str(&pending_json)?;
            let req = EnqueueRequest {
                job_type: stored.pending.job_type,
                args: stored.pending.args,
                options: None,
            };
            let body = serde_json::to_string(&req)?;
            let url = format!("{}/jobs", self.base_url);
            let resp_text = sw_post(&url, &body).await?;
            serde_json::from_str::<JobResponse>(&resp_text).map_err(OjsWasmError::from)
        }
        .await;

        let resp = match enqueue_result {
            Ok(response) => response,
            Err(error) => {
                let release_result = await_storage(
                    idb_release_pending(tag, &lease_id),
                    "failed to release pending Background Sync lease",
                )
                .await;
                return match release_result {
                    Ok(_) => Err(error),
                    Err(release_error) => Err(OjsWasmError::Js(format!(
                        "{}; additionally failed to release lease: {}",
                        error, release_error
                    ))),
                };
            }
        };

        await_storage(
            idb_delete_pending(tag, &lease_id),
            "enqueue succeeded but pending Background Sync cleanup failed",
        )
        .await?;
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
        let short_id: String = job_id.chars().take(8).collect();
        let body_text = format!("{} ({})", job_type, short_id);

        // Build notification options: { body, tag, data: { jobId, jobType, state } }
        let options = Object::new();
        Reflect::set(&options, &"body".into(), &JsValue::from_str(&body_text))
            .map_err(OjsWasmError::from)?;
        Reflect::set(
            &options,
            &"tag".into(),
            &JsValue::from_str(&format!("ojs-job-{}", job_id)),
        )
        .map_err(OjsWasmError::from)?;

        let data = Object::new();
        Reflect::set(&data, &"jobId".into(), &JsValue::from_str(job_id))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&data, &"jobType".into(), &JsValue::from_str(job_type))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&data, &"state".into(), &JsValue::from_str(state))
            .map_err(OjsWasmError::from)?;
        Reflect::set(&options, &"data".into(), &data).map_err(OjsWasmError::from)?;

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

fn sync_register_function() -> Result<(JsValue, Function)> {
    let registration = Reflect::get(&sw_global(), &JsValue::from_str("registration"))
        .map_err(OjsWasmError::from)?;
    if registration.is_undefined() || registration.is_null() {
        return Err(OjsWasmError::Js(
            "Background Sync API unavailable: no ServiceWorkerRegistration".into(),
        ));
    }

    let sync =
        Reflect::get(&registration, &JsValue::from_str("sync")).map_err(OjsWasmError::from)?;
    if sync.is_undefined() || sync.is_null() {
        return Err(OjsWasmError::Js(
            "Background Sync API unavailable: SyncManager is not supported".into(),
        ));
    }

    let register_fn =
        Reflect::get(&sync, &JsValue::from_str("register")).map_err(OjsWasmError::from)?;
    let register_fn = register_fn
        .dyn_into()
        .map_err(|_| OjsWasmError::Js("sync.register is not a function".into()))?;
    Ok((sync, register_fn))
}

fn new_sync_tag() -> String {
    format!("{}{}", SYNC_TAG_PREFIX, new_unique_suffix())
}

fn new_unique_suffix() -> String {
    let timestamp = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * (u32::MAX as f64 + 1.0)) as u32;
    format!("{}-{:08x}", timestamp, random)
}

async fn await_storage(promise: Promise, context: &str) -> Result<JsValue> {
    JsFuture::from(promise)
        .await
        .map_err(|error| OjsWasmError::Js(format!("{}: {}", context, js_error_message(&error))))
}

fn js_error_message(error: &JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            Reflect::get(error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{:?}", error))
}
