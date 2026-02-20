//! # Edge Runtime Support
//!
//! Provides OJS clients compatible with edge/serverless runtimes that expose
//! the standard Web `fetch()` API but do **not** have a `window` object.
//!
//! Supported runtimes:
//!
//! | Runtime | Client | Feature gate |
//! |---------|--------|--------------|
//! | Cloudflare Workers | [`CloudflareClient`] | `edge_cloudflare` |
//! | Deno Deploy | [`DenoClient`] | `edge_deno` |
//! | Vercel Edge Functions | [`VercelEdgeClient`] | `edge_vercel` |
//!
//! All three runtimes provide the global `fetch()` function, so the core
//! HTTP logic is shared. Runtime-specific helpers (e.g. Cloudflare KV,
//! Deno.env, Vercel `waitUntil`) are exposed as convenience methods.
//!
//! ## Feature gates
//!
//! Each client is behind a Cargo feature flag so unused runtimes are
//! tree-shaken at compile time. Enable them in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! ojs-wasm-sdk = { version = "0.1", features = ["edge_cloudflare"] }
//! ```
//!
//! If no feature is enabled the [`EdgeClient`] base type is still available
//! and works in any runtime that has a global `fetch`.

use crate::error::{ErrorResponse, OjsWasmError, Result};
use crate::types::{
    BatchRequest, BatchResponse, EnqueueRequest, JobResponse, WorkflowResponse,
};
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};

const BASE_PATH: &str = "/ojs/v1";

// ---------------------------------------------------------------------------
// Global fetch binding (shared by all edge runtimes)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "fetch")]
    fn global_fetch(input: &Request) -> Promise;
}

/// Returns the global `self` object (works in all edge runtimes).
fn edge_global() -> JsValue {
    js_sys::global().into()
}

// ---------------------------------------------------------------------------
// Shared transport helpers
// ---------------------------------------------------------------------------

fn build_request(method: &str, url: &str, body: Option<String>, auth_token: Option<&str>) -> Result<Request> {
    let opts = RequestInit::new();
    opts.set_method(method);

    let headers = Headers::new().map_err(OjsWasmError::from)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(OjsWasmError::from)?;

    if let Some(token) = auth_token {
        headers
            .set("Authorization", &format!("Bearer {}", token))
            .map_err(OjsWasmError::from)?;
    }

    opts.set_headers(&headers);

    if let Some(b) = body {
        opts.set_body(&JsValue::from_str(&b));
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

async fn edge_post(url: &str, body: &str, auth: Option<&str>) -> Result<String> {
    let req = build_request("POST", url, Some(body.to_string()), auth)?;
    execute(req).await
}

async fn edge_get(url: &str, auth: Option<&str>) -> Result<String> {
    let req = build_request("GET", url, None, auth)?;
    execute(req).await
}

async fn edge_delete(url: &str, auth: Option<&str>) -> Result<String> {
    let req = build_request("DELETE", url, None, auth)?;
    execute(req).await
}

// ===========================================================================
// EdgeClient — runtime-agnostic base client
// ===========================================================================

/// Generic edge-runtime OJS client.
///
/// Works in **any** JavaScript environment that provides a global `fetch()`
/// function (Service Workers, Cloudflare Workers, Deno Deploy, Vercel Edge,
/// Bun, Node 18+, etc.).
///
/// # Example
///
/// ```js
/// import { EdgeClient } from '@openjobspec/wasm';
///
/// const client = new EdgeClient("https://ojs.example.com");
/// const job = await client.enqueue("email.send", ["user@example.com"]);
/// ```
#[wasm_bindgen]
pub struct EdgeClient {
    base_url: String,
    auth_token: Option<String>,
}

#[wasm_bindgen]
impl EdgeClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        let base_url = format!("{}{}", url.trim_end_matches('/'), BASE_PATH);
        Self { base_url, auth_token: None }
    }

    /// Create a client with API key authentication.
    pub fn with_auth(url: &str, api_key: &str) -> Self {
        let base_url = format!("{}{}", url.trim_end_matches('/'), BASE_PATH);
        Self { base_url, auth_token: Some(api_key.to_string()) }
    }

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

    pub async fn enqueue_batch(
        &self,
        jobs: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.enqueue_batch_inner(jobs)
            .await
            .map_err(JsValue::from)
    }

    pub async fn get_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.get_job_inner(id).await.map_err(JsValue::from)
    }

    pub async fn cancel_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.cancel_job_inner(id).await.map_err(JsValue::from)
    }

    /// Create and start a workflow.
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

    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.health_inner().await.map_err(JsValue::from)
    }
}

impl EdgeClient {
    async fn enqueue_inner(&self, job_type: &str, args: JsValue, options: Option<crate::types::EnqueueOptions>) -> Result<JsValue> {
        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;
        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
            options,
        };
        let body = serde_json::to_string(&req)?;
        let url = format!("{}/jobs", self.base_url);
        let resp_text = edge_post(&url, &body, self.auth_token.as_deref()).await?;
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
            options: Option<crate::types::EnqueueOptions>,
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
        let resp_text = edge_post(&url, &body, self.auth_token.as_deref()).await?;
        let resp: BatchResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp.jobs)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = edge_get(&url, self.auth_token.as_deref()).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn cancel_job_inner(&self, id: &str) -> Result<JsValue> {
        let url = format!("{}/jobs/{}", self.base_url, id);
        let resp_text = edge_delete(&url, self.auth_token.as_deref()).await?;
        let resp: JobResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp.job)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn workflow_inner(&self, definition: JsValue) -> Result<JsValue> {
        let wire: serde_json::Value = serde_wasm_bindgen::from_value(definition)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))?;

        let body = serde_json::to_string(&wire)?;
        let url = format!("{}/workflows", self.base_url);
        let resp_text = edge_post(&url, &body, self.auth_token.as_deref()).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn get_workflow_inner(&self, workflow_id: &str) -> Result<JsValue> {
        let url = format!("{}/workflows/{}", self.base_url, workflow_id);
        let resp_text = edge_get(&url, self.auth_token.as_deref()).await?;
        let resp: WorkflowResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    async fn health_inner(&self) -> Result<JsValue> {
        let url = format!("{}/health", self.base_url);
        let resp_text = edge_get(&url, self.auth_token.as_deref()).await?;
        let resp: crate::types::HealthResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }
}

// ===========================================================================
// Shared Edge Types
// ===========================================================================

/// Configuration for edge-specific storage bindings.
///
/// These types represent common edge runtime storage abstractions that can
/// be used to configure OJS clients with runtime-specific backends.

/// Cloudflare KV namespace binding reference.
///
/// Used to pass KV namespace bindings from the Cloudflare Worker environment
/// to the OJS client for caching or configuration storage.
///
/// ```js
/// export default {
///   async fetch(request, env, ctx) {
///     const kvRef = { namespace: env.OJS_CACHE };
///     // Use kvRef with your application logic
///   }
/// };
/// ```
#[wasm_bindgen]
pub struct KVNamespaceRef {
    namespace: JsValue,
}

#[wasm_bindgen]
impl KVNamespaceRef {
    /// Wrap a Cloudflare KV namespace binding.
    #[wasm_bindgen(constructor)]
    pub fn new(namespace: JsValue) -> Self {
        Self { namespace }
    }

    /// Get a value by key from the KV namespace.
    pub async fn get(&self, key: &str) -> std::result::Result<JsValue, JsValue> {
        let get_fn = Reflect::get(&self.namespace, &JsValue::from_str("get"))
            .map_err(|_| JsValue::from_str("KV namespace missing get()"))?;
        let get_fn: Function = get_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("KV get is not a function"))?;
        let promise: Promise = get_fn
            .call1(&self.namespace, &JsValue::from_str(key))
            .map_err(|e| JsValue::from_str(&format!("KV get failed: {:?}", e)))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("KV get did not return a Promise"))?;
        JsFuture::from(promise).await
    }

    /// Put a value by key into the KV namespace.
    pub async fn put(&self, key: &str, value: &str) -> std::result::Result<(), JsValue> {
        let put_fn = Reflect::get(&self.namespace, &JsValue::from_str("put"))
            .map_err(|_| JsValue::from_str("KV namespace missing put()"))?;
        let put_fn: Function = put_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("KV put is not a function"))?;
        let promise: Promise = put_fn
            .call2(
                &self.namespace,
                &JsValue::from_str(key),
                &JsValue::from_str(value),
            )
            .map_err(|e| JsValue::from_str(&format!("KV put failed: {:?}", e)))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("KV put did not return a Promise"))?;
        JsFuture::from(promise).await?;
        Ok(())
    }
}

/// Cloudflare D1 database binding reference.
///
/// Wraps a D1 database binding from the Cloudflare Worker environment
/// for executing SQL queries against Cloudflare's edge SQL database.
///
/// ```js
/// export default {
///   async fetch(request, env, ctx) {
///     const db = new D1DatabaseRef(env.DB);
///     const result = await db.exec("SELECT * FROM jobs LIMIT 10");
///   }
/// };
/// ```
#[wasm_bindgen]
pub struct D1DatabaseRef {
    db: JsValue,
}

#[wasm_bindgen]
impl D1DatabaseRef {
    /// Wrap a Cloudflare D1 database binding.
    #[wasm_bindgen(constructor)]
    pub fn new(db: JsValue) -> Self {
        Self { db }
    }

    /// Execute a raw SQL statement.
    pub async fn exec(&self, query: &str) -> std::result::Result<JsValue, JsValue> {
        let exec_fn = Reflect::get(&self.db, &JsValue::from_str("exec"))
            .map_err(|_| JsValue::from_str("D1 database missing exec()"))?;
        let exec_fn: Function = exec_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("D1 exec is not a function"))?;
        let promise: Promise = exec_fn
            .call1(&self.db, &JsValue::from_str(query))
            .map_err(|e| JsValue::from_str(&format!("D1 exec failed: {:?}", e)))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("D1 exec did not return a Promise"))?;
        JsFuture::from(promise).await
    }

    /// Prepare a parameterized SQL statement.
    pub fn prepare(&self, query: &str) -> std::result::Result<JsValue, JsValue> {
        let prepare_fn = Reflect::get(&self.db, &JsValue::from_str("prepare"))
            .map_err(|_| JsValue::from_str("D1 database missing prepare()"))?;
        let prepare_fn: Function = prepare_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("D1 prepare is not a function"))?;
        prepare_fn
            .call1(&self.db, &JsValue::from_str(query))
            .map_err(|e| JsValue::from_str(&format!("D1 prepare failed: {:?}", e)))
    }
}

/// OJS client tailored for **Cloudflare Workers**.
///
/// Extends [`EdgeClient`] with Cloudflare-specific helpers such as
/// `waitUntil` integration for fire-and-forget job enqueueing.
///
/// # Example
///
/// ```js
/// // wrangler.toml must include the WASM binding
/// import { CloudflareClient } from '@openjobspec/wasm';
///
/// export default {
///   async fetch(request, env, ctx) {
///     const client = new CloudflareClient("https://ojs.example.com");
///     const job = await client.enqueue("email.send", ["user@example.com"]);
///     return Response.json(job);
///   }
/// };
/// ```
#[wasm_bindgen]
pub struct CloudflareClient {
    inner: EdgeClient,
}

#[wasm_bindgen]
impl CloudflareClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        Self {
            inner: EdgeClient::new(url),
        }
    }

    /// Create a client with API key authentication.
    pub fn with_auth(url: &str, api_key: &str) -> Self {
        Self {
            inner: EdgeClient::with_auth(url, api_key),
        }
    }

    pub async fn enqueue(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue(job_type, args).await
    }

    pub async fn enqueue_with_options(
        &self,
        job_type: &str,
        args: JsValue,
        options: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_with_options(job_type, args, options).await
    }

    pub async fn enqueue_batch(
        &self,
        jobs: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_batch(jobs).await
    }

    pub async fn get_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_job(id).await
    }

    pub async fn cancel_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.cancel_job(id).await
    }

    pub async fn workflow(
        &self,
        definition: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.workflow(definition).await
    }

    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_workflow(workflow_id).await
    }

    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.inner.health().await
    }

    /// Fire-and-forget enqueue using Cloudflare's `ctx.waitUntil`.
    ///
    /// The job enqueue request runs in the background after the response is
    /// sent to the client, avoiding extra latency on the critical path.
    ///
    /// ```js
    /// export default {
    ///   async fetch(request, env, ctx) {
    ///     const client = new CloudflareClient("https://ojs.example.com");
    ///     client.enqueue_with_wait_until(ctx, "analytics.track", [request.url]);
    ///     return new Response("ok");
    ///   }
    /// };
    /// ```
    pub fn enqueue_with_wait_until(
        &self,
        ctx: JsValue,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<(), JsValue> {
        let wait_until_fn = Reflect::get(&ctx, &JsValue::from_str("waitUntil"))
            .map_err(|_| JsValue::from_str("ctx.waitUntil not found"))?;
        let wait_until_fn: Function = wait_until_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("ctx.waitUntil is not a function"))?;

        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
            options: None,
        };
        let body = serde_json::to_string(&req)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = format!("{}/jobs", self.inner.base_url);

        // Build the fetch promise and hand it to waitUntil
        let request = build_request("POST", &url, Some(body), self.inner.auth_token.as_deref())?;
        let promise = global_fetch(&request);

        wait_until_fn.call1(&ctx, &promise)?;
        Ok(())
    }
}

// ===========================================================================
// DenoClient — Deno Deploy
// ===========================================================================

/// OJS client tailored for **Deno Deploy**.
///
/// # Example
///
/// ```ts
/// import { DenoClient } from '@openjobspec/wasm';
///
/// Deno.serve(async (_req) => {
///   const client = new DenoClient("https://ojs.example.com");
///   const job = await client.enqueue("email.send", ["user@example.com"]);
///   return Response.json(job);
/// });
/// ```
#[wasm_bindgen]
pub struct DenoClient {
    inner: EdgeClient,
}

#[wasm_bindgen]
impl DenoClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        Self {
            inner: EdgeClient::new(url),
        }
    }

    /// Create a client with API key authentication.
    pub fn with_auth(url: &str, api_key: &str) -> Self {
        Self {
            inner: EdgeClient::with_auth(url, api_key),
        }
    }

    /// Create a client using the `OJS_URL` environment variable via `Deno.env.get`.
    pub fn from_env() -> std::result::Result<DenoClient, JsValue> {
        let deno = Reflect::get(&edge_global(), &JsValue::from_str("Deno"))
            .map_err(|_| JsValue::from_str("Deno global not found — not running on Deno?"))?;
        let env = Reflect::get(&deno, &JsValue::from_str("env"))
            .map_err(|_| JsValue::from_str("Deno.env not available"))?;
        let get_fn: Function = Reflect::get(&env, &JsValue::from_str("get"))
            .map_err(|_| JsValue::from_str("Deno.env.get not found"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("Deno.env.get is not a function"))?;

        let url_val = get_fn
            .call1(&env, &JsValue::from_str("OJS_URL"))
            .map_err(|_| JsValue::from_str("failed to read OJS_URL"))?;
        let url = url_val
            .as_string()
            .ok_or_else(|| JsValue::from_str("OJS_URL is not set"))?;

        Ok(Self {
            inner: EdgeClient::new(&url),
        })
    }

    pub async fn enqueue(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue(job_type, args).await
    }

    pub async fn enqueue_with_options(
        &self,
        job_type: &str,
        args: JsValue,
        options: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_with_options(job_type, args, options).await
    }

    pub async fn enqueue_batch(
        &self,
        jobs: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_batch(jobs).await
    }

    pub async fn get_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_job(id).await
    }

    pub async fn cancel_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.cancel_job(id).await
    }

    pub async fn workflow(
        &self,
        definition: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.workflow(definition).await
    }

    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_workflow(workflow_id).await
    }

    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.inner.health().await
    }
}

// ===========================================================================
// VercelEdgeClient — Vercel Edge Functions
// ===========================================================================

/// OJS client tailored for **Vercel Edge Functions**.
///
/// # Example
///
/// ```ts
/// import { VercelEdgeClient } from '@openjobspec/wasm';
///
/// export const config = { runtime: 'edge' };
///
/// export default async function handler(req: Request) {
///   const client = new VercelEdgeClient("https://ojs.example.com");
///   const job = await client.enqueue("email.send", ["user@example.com"]);
///   return Response.json(job);
/// }
/// ```
#[wasm_bindgen]
pub struct VercelEdgeClient {
    inner: EdgeClient,
}

#[wasm_bindgen]
impl VercelEdgeClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        Self {
            inner: EdgeClient::new(url),
        }
    }

    /// Create a client with API key authentication.
    pub fn with_auth(url: &str, api_key: &str) -> Self {
        Self {
            inner: EdgeClient::with_auth(url, api_key),
        }
    }

    /// Create a client using the `OJS_URL` environment variable via
    /// `process.env` (available in Vercel Edge Functions).
    pub fn from_env() -> std::result::Result<VercelEdgeClient, JsValue> {
        let process = Reflect::get(&edge_global(), &JsValue::from_str("process"))
            .map_err(|_| JsValue::from_str("process global not found"))?;
        let env = Reflect::get(&process, &JsValue::from_str("env"))
            .map_err(|_| JsValue::from_str("process.env not available"))?;
        let url_val = Reflect::get(&env, &JsValue::from_str("OJS_URL"))
            .map_err(|_| JsValue::from_str("OJS_URL not found in process.env"))?;
        let url = url_val
            .as_string()
            .ok_or_else(|| JsValue::from_str("OJS_URL is not set"))?;

        Ok(Self {
            inner: EdgeClient::new(&url),
        })
    }

    pub async fn enqueue(
        &self,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue(job_type, args).await
    }

    pub async fn enqueue_with_options(
        &self,
        job_type: &str,
        args: JsValue,
        options: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_with_options(job_type, args, options).await
    }

    pub async fn enqueue_batch(
        &self,
        jobs: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.enqueue_batch(jobs).await
    }

    pub async fn get_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_job(id).await
    }

    pub async fn cancel_job(&self, id: &str) -> std::result::Result<JsValue, JsValue> {
        self.inner.cancel_job(id).await
    }

    pub async fn workflow(
        &self,
        definition: JsValue,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.workflow(definition).await
    }

    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        self.inner.get_workflow(workflow_id).await
    }

    pub async fn health(&self) -> std::result::Result<JsValue, JsValue> {
        self.inner.health().await
    }

    /// Extend the function execution lifetime using Vercel's `waitUntil`.
    ///
    /// ```ts
    /// import { waitUntil } from '@vercel/functions';
    ///
    /// const client = new VercelEdgeClient("https://ojs.example.com");
    /// client.enqueue_with_wait_until(waitUntil, "analytics.track", [req.url]);
    /// ```
    pub fn enqueue_with_wait_until(
        &self,
        wait_until_fn: JsValue,
        job_type: &str,
        args: JsValue,
    ) -> std::result::Result<(), JsValue> {
        let wait_until: Function = wait_until_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("waitUntil is not a function"))?;

        let args_value: serde_json::Value = serde_wasm_bindgen::from_value(args)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let req = EnqueueRequest {
            job_type: job_type.to_string(),
            args: args_value,
            options: None,
        };
        let body = serde_json::to_string(&req)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = format!("{}/jobs", self.inner.base_url);

        let request = build_request("POST", &url, Some(body), self.inner.auth_token.as_deref())?;
        let promise = global_fetch(&request);

        wait_until.call1(&JsValue::NULL, &promise)?;
        Ok(())
    }
}
