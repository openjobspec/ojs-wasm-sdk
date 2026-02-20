use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::OJSClient;
use ojs_wasm_sdk::workflow::{chain, group, batch};

// ===========================================================================
// OJSClient construction
// ===========================================================================

#[wasm_bindgen_test]
fn test_client_creation() {
    let client = OJSClient::new("http://localhost:8080");
    drop(client);
}

#[wasm_bindgen_test]
fn test_client_trailing_slash() {
    let client = OJSClient::new("http://localhost:8080/");
    drop(client);
}

#[wasm_bindgen_test]
fn test_client_various_urls() {
    let _c1 = OJSClient::new("https://ojs.example.com");
    let _c2 = OJSClient::new("http://127.0.0.1:9090");
    let _c3 = OJSClient::new("https://ojs.example.com:443/");
}

// ===========================================================================
// Type serialization / deserialization
// ===========================================================================

#[wasm_bindgen_test]
fn test_job_state_serialization() {
    use ojs_wasm_sdk::types::JobState;

    let states = vec![
        (JobState::Pending, "\"pending\""),
        (JobState::Scheduled, "\"scheduled\""),
        (JobState::Available, "\"available\""),
        (JobState::Active, "\"active\""),
        (JobState::Completed, "\"completed\""),
        (JobState::Retryable, "\"retryable\""),
        (JobState::Cancelled, "\"cancelled\""),
        (JobState::Discarded, "\"discarded\""),
    ];

    for (state, expected_json) in &states {
        let json = serde_json::to_string(state).unwrap();
        assert_eq!(&json, expected_json, "serialization of {:?}", state);
    }
}

#[wasm_bindgen_test]
fn test_job_state_deserialization() {
    use ojs_wasm_sdk::types::JobState;

    let json = "\"completed\"";
    let state: JobState = serde_json::from_str(json).unwrap();
    assert_eq!(state, JobState::Completed);
}

#[wasm_bindgen_test]
fn test_job_deserialization_full() {
    use ojs_wasm_sdk::types::Job;

    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000001",
        "type": "email.send",
        "queue": "critical",
        "args": ["user@example.com", "Welcome!"],
        "priority": 10,
        "state": "active",
        "attempt": 1,
        "tags": ["onboarding", "welcome"],
        "created_at": "2024-01-15T10:30:00Z",
        "enqueued_at": "2024-01-15T10:30:00Z",
        "started_at": "2024-01-15T10:30:01Z"
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.id, "019012ab-cdef-7000-8000-000000000001");
    assert_eq!(job.job_type, "email.send");
    assert_eq!(job.queue, "critical");
    assert_eq!(job.priority, 10);
    assert_eq!(job.state, Some(ojs_wasm_sdk::types::JobState::Active));
    assert_eq!(job.attempt, 1);
    assert_eq!(job.tags.as_ref().unwrap().len(), 2);
    assert!(job.created_at.is_some());
    assert!(job.started_at.is_some());
    assert!(job.completed_at.is_none());
}

#[wasm_bindgen_test]
fn test_job_deserialization_minimal() {
    use ojs_wasm_sdk::types::Job;

    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000002",
        "type": "report.generate"
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.id, "019012ab-cdef-7000-8000-000000000002");
    assert_eq!(job.job_type, "report.generate");
    assert_eq!(job.queue, "default");
    assert_eq!(job.priority, 0);
    assert_eq!(job.attempt, 0);
    assert!(job.state.is_none());
    assert!(job.tags.is_none());
}

#[wasm_bindgen_test]
fn test_job_roundtrip() {
    use ojs_wasm_sdk::types::Job;

    let json = r#"{
        "id": "019012ab-0000-7000-8000-000000000003",
        "type": "data.process",
        "queue": "default",
        "args": [1, 2, 3],
        "priority": 5,
        "state": "pending",
        "attempt": 0
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&job).unwrap();
    let job2: Job = serde_json::from_str(&serialized).unwrap();

    assert_eq!(job.id, job2.id);
    assert_eq!(job.job_type, job2.job_type);
    assert_eq!(job.queue, job2.queue);
    assert_eq!(job.priority, job2.priority);
    assert_eq!(job.state, job2.state);
}

#[wasm_bindgen_test]
fn test_enqueue_options_serialization() {
    use ojs_wasm_sdk::types::EnqueueOptions;

    let opts = EnqueueOptions {
        queue: Some("critical".to_string()),
        priority: Some(10),
        timeout_ms: Some(30000),
        delay_until: Some("2024-12-01T00:00:00Z".to_string()),
        tags: Some(vec!["urgent".to_string(), "email".to_string()]),
    };

    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"queue\":\"critical\""));
    assert!(json.contains("\"priority\":10"));
    assert!(json.contains("\"timeout_ms\":30000"));
    assert!(json.contains("\"tags\""));
}

#[wasm_bindgen_test]
fn test_enqueue_options_skip_none() {
    use ojs_wasm_sdk::types::EnqueueOptions;

    let opts = EnqueueOptions::default();
    let json = serde_json::to_string(&opts).unwrap();
    assert_eq!(json, "{}");
}

#[wasm_bindgen_test]
fn test_enqueue_request_serialization() {
    use ojs_wasm_sdk::types::{EnqueueRequest, EnqueueOptions};

    let req = EnqueueRequest {
        job_type: "email.send".to_string(),
        args: serde_json::json!(["user@example.com", "Hello!"]),
        options: Some(EnqueueOptions {
            queue: Some("mail".to_string()),
            priority: None,
            timeout_ms: None,
            delay_until: None,
            tags: None,
        }),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"type\":\"email.send\""));
    assert!(json.contains("\"args\":[\"user@example.com\",\"Hello!\"]"));
    assert!(json.contains("\"queue\":\"mail\""));
}

#[wasm_bindgen_test]
fn test_batch_request_serialization() {
    use ojs_wasm_sdk::types::{BatchRequest, EnqueueRequest};

    let batch_req = BatchRequest {
        jobs: vec![
            EnqueueRequest {
                job_type: "email.send".to_string(),
                args: serde_json::json!(["a@b.com"]),
                options: None,
            },
            EnqueueRequest {
                job_type: "sms.send".to_string(),
                args: serde_json::json!(["+1234567890"]),
                options: None,
            },
        ],
    };

    let json = serde_json::to_string(&batch_req).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jobs"].as_array().unwrap().len(), 2);
}

#[wasm_bindgen_test]
fn test_health_response_deserialization() {
    use ojs_wasm_sdk::types::HealthResponse;

    let json = r#"{"status": "ok", "version": "0.1.0", "uptime_seconds": 3600}"#;
    let health: HealthResponse = serde_json::from_str(json).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, Some("0.1.0".to_string()));
    assert_eq!(health.uptime_seconds, Some(3600));
}

#[wasm_bindgen_test]
fn test_health_response_minimal() {
    use ojs_wasm_sdk::types::HealthResponse;

    let json = r#"{"status": "ok"}"#;
    let health: HealthResponse = serde_json::from_str(json).unwrap();
    assert_eq!(health.status, "ok");
    assert!(health.version.is_none());
    assert!(health.uptime_seconds.is_none());
}

#[wasm_bindgen_test]
fn test_workflow_response_deserialization() {
    use ojs_wasm_sdk::types::{WorkflowResponse, WorkflowState};

    let json = r#"{
        "id": "wf-001",
        "type": "chain",
        "name": "data-pipeline",
        "state": "running",
        "metadata": {
            "created_at": "2024-01-15T10:00:00Z",
            "started_at": "2024-01-15T10:00:01Z",
            "job_count": 3,
            "completed_count": 1,
            "failed_count": 0
        }
    }"#;

    let wf: WorkflowResponse = serde_json::from_str(json).unwrap();
    assert_eq!(wf.id, "wf-001");
    assert_eq!(wf.workflow_type, "chain");
    assert_eq!(wf.name, Some("data-pipeline".to_string()));
    assert_eq!(wf.state, Some(WorkflowState::Running));
    let meta = wf.metadata.unwrap();
    assert_eq!(meta.job_count, 3);
    assert_eq!(meta.completed_count, 1);
    assert_eq!(meta.failed_count, 0);
}

// ===========================================================================
// Error types
// ===========================================================================

#[wasm_bindgen_test]
fn test_error_display_server() {
    use ojs_wasm_sdk::error::{OjsWasmError, ServerError};

    let err = OjsWasmError::Server(ServerError {
        code: "not_found".to_string(),
        message: "Job not found".to_string(),
        retryable: false,
    });
    let msg = format!("{}", err);
    assert!(msg.contains("not_found"));
    assert!(msg.contains("Job not found"));
}

#[wasm_bindgen_test]
fn test_error_display_transport() {
    use ojs_wasm_sdk::error::OjsWasmError;

    let err = OjsWasmError::Transport("connection refused".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("transport error"));
    assert!(msg.contains("connection refused"));
}

#[wasm_bindgen_test]
fn test_error_display_serialization() {
    use ojs_wasm_sdk::error::OjsWasmError;

    let err = OjsWasmError::Serialization("invalid JSON".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("serialization error"));
}

#[wasm_bindgen_test]
fn test_error_to_jsvalue() {
    use ojs_wasm_sdk::error::OjsWasmError;

    let err = OjsWasmError::Transport("timeout".to_string());
    let js: JsValue = err.into();
    assert!(js.as_string().unwrap().contains("timeout"));
}

#[wasm_bindgen_test]
fn test_server_error_deserialization() {
    use ojs_wasm_sdk::error::ServerError;

    let json = r#"{"code": "rate_limited", "message": "Too many requests", "retryable": true}"#;
    let err: ServerError = serde_json::from_str(json).unwrap();
    assert_eq!(err.code, "rate_limited");
    assert_eq!(err.message, "Too many requests");
    assert!(err.retryable);
}

// ===========================================================================
// Workflow builder: chain
// ===========================================================================

#[wasm_bindgen_test]
fn test_chain_valid() {
    let steps = js_sys::Array::new();
    let step = js_sys::Object::new();
    js_sys::Reflect::set(&step, &"type".into(), &"email.send".into()).unwrap();
    let args = js_sys::Array::new();
    args.push(&"user@example.com".into());
    js_sys::Reflect::set(&step, &"args".into(), &args).unwrap();
    steps.push(&step);

    let result = chain(steps.into());
    assert!(result.is_ok(), "chain() should succeed with valid input");

    let obj = result.unwrap();
    let wf_type = js_sys::Reflect::get(&obj, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "chain");
}

#[wasm_bindgen_test]
fn test_chain_multiple_steps() {
    let steps = js_sys::Array::new();
    for name in &["data.fetch", "data.transform", "data.load"] {
        let step = js_sys::Object::new();
        js_sys::Reflect::set(&step, &"type".into(), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(&step, &"args".into(), &js_sys::Array::new()).unwrap();
        steps.push(&step);
    }

    let result = chain(steps.into());
    assert!(result.is_ok());

    let obj = result.unwrap();
    let steps_arr = js_sys::Reflect::get(&obj, &"steps".into()).unwrap();
    let arr: js_sys::Array = steps_arr.dyn_into().unwrap();
    assert_eq!(arr.length(), 3);
}

#[wasm_bindgen_test]
fn test_chain_empty_array_fails() {
    let steps = js_sys::Array::new();
    let result = chain(steps.into());
    assert!(result.is_err(), "chain() should fail with empty array");
}

#[wasm_bindgen_test]
fn test_chain_non_array_fails() {
    let result = chain(JsValue::from_str("not an array"));
    assert!(result.is_err(), "chain() should fail with non-array");
}

#[wasm_bindgen_test]
fn test_chain_null_fails() {
    let result = chain(JsValue::NULL);
    assert!(result.is_err(), "chain() should fail with null");
}

#[wasm_bindgen_test]
fn test_chain_undefined_fails() {
    let result = chain(JsValue::UNDEFINED);
    assert!(result.is_err(), "chain() should fail with undefined");
}

// ===========================================================================
// Workflow builder: group
// ===========================================================================

#[wasm_bindgen_test]
fn test_group_valid() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"export.csv".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let result = group(jobs.into());
    assert!(result.is_ok(), "group() should succeed with valid input");

    let obj = result.unwrap();
    let wf_type = js_sys::Reflect::get(&obj, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "group");
}

#[wasm_bindgen_test]
fn test_group_multiple_jobs() {
    let jobs = js_sys::Array::new();
    for name in &["export.csv", "export.pdf", "export.xlsx"] {
        let job = js_sys::Object::new();
        js_sys::Reflect::set(&job, &"type".into(), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
        jobs.push(&job);
    }

    let result = group(jobs.into());
    assert!(result.is_ok());

    let obj = result.unwrap();
    let jobs_arr = js_sys::Reflect::get(&obj, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_arr.dyn_into().unwrap();
    assert_eq!(arr.length(), 3);
}

#[wasm_bindgen_test]
fn test_group_empty_array_fails() {
    let jobs = js_sys::Array::new();
    let result = group(jobs.into());
    assert!(result.is_err(), "group() should fail with empty array");
}

#[wasm_bindgen_test]
fn test_group_non_array_fails() {
    let result = group(JsValue::from(42));
    assert!(result.is_err(), "group() should fail with non-array");
}

// ===========================================================================
// Workflow builder: batch
// ===========================================================================

#[wasm_bindgen_test]
fn test_batch_valid() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"email.send".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let callbacks = js_sys::Object::new();
    let on_complete = js_sys::Object::new();
    js_sys::Reflect::set(&on_complete, &"type".into(), &"batch.report".into()).unwrap();
    js_sys::Reflect::set(&on_complete, &"args".into(), &js_sys::Array::new()).unwrap();
    js_sys::Reflect::set(&callbacks, &"on_complete".into(), &on_complete).unwrap();

    let result = batch(jobs.into(), callbacks.into());
    assert!(result.is_ok(), "batch() should succeed with valid input");

    let obj = result.unwrap();
    let wf_type = js_sys::Reflect::get(&obj, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "batch");
}

#[wasm_bindgen_test]
fn test_batch_with_on_failure() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"email.send".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let callbacks = js_sys::Object::new();
    let on_failure = js_sys::Object::new();
    js_sys::Reflect::set(&on_failure, &"type".into(), &"batch.alert".into()).unwrap();
    js_sys::Reflect::set(&on_failure, &"args".into(), &js_sys::Array::new()).unwrap();
    js_sys::Reflect::set(&callbacks, &"on_failure".into(), &on_failure).unwrap();

    let result = batch(jobs.into(), callbacks.into());
    assert!(result.is_ok(), "batch() should accept on_failure callback");
}

#[wasm_bindgen_test]
fn test_batch_with_all_callbacks() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"process".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let callbacks = js_sys::Object::new();
    for key in &["on_complete", "on_success", "on_failure"] {
        let cb = js_sys::Object::new();
        js_sys::Reflect::set(&cb, &"type".into(), &JsValue::from_str(&format!("cb.{}", key))).unwrap();
        js_sys::Reflect::set(&cb, &"args".into(), &js_sys::Array::new()).unwrap();
        js_sys::Reflect::set(&callbacks, &JsValue::from_str(key), &cb).unwrap();
    }

    let result = batch(jobs.into(), callbacks.into());
    assert!(result.is_ok());

    let obj = result.unwrap();
    let cb_val = js_sys::Reflect::get(&obj, &"callbacks".into()).unwrap();
    assert!(js_sys::Reflect::has(&cb_val.into(), &"on_complete".into()).unwrap());
}

#[wasm_bindgen_test]
fn test_batch_no_callbacks_fails() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"email.send".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let callbacks = js_sys::Object::new();
    let result = batch(jobs.into(), callbacks.into());
    assert!(result.is_err(), "batch() should fail with no callbacks");
}

#[wasm_bindgen_test]
fn test_batch_empty_jobs_fails() {
    let jobs = js_sys::Array::new();
    let callbacks = js_sys::Object::new();
    let on_complete = js_sys::Object::new();
    js_sys::Reflect::set(&on_complete, &"type".into(), &"report".into()).unwrap();
    js_sys::Reflect::set(&callbacks, &"on_complete".into(), &on_complete).unwrap();

    let result = batch(jobs.into(), callbacks.into());
    assert!(result.is_err(), "batch() should fail with empty jobs array");
}

#[wasm_bindgen_test]
fn test_batch_non_object_callbacks_fails() {
    let jobs = js_sys::Array::new();
    let job = js_sys::Object::new();
    js_sys::Reflect::set(&job, &"type".into(), &"process".into()).unwrap();
    js_sys::Reflect::set(&job, &"args".into(), &js_sys::Array::new()).unwrap();
    jobs.push(&job);

    let result = batch(jobs.into(), JsValue::from_str("not an object"));
    assert!(result.is_err(), "batch() should fail with non-object callbacks");
}

// ===========================================================================
// Edge client construction
// ===========================================================================

#[wasm_bindgen_test]
fn test_edge_client_creation() {
    use ojs_wasm_sdk::edge::EdgeClient;
    let client = EdgeClient::new("https://ojs.example.com");
    drop(client);
}

#[wasm_bindgen_test]
fn test_edge_client_with_auth() {
    use ojs_wasm_sdk::edge::EdgeClient;
    let client = EdgeClient::with_auth("https://ojs.example.com", "my-api-key");
    drop(client);
}

#[wasm_bindgen_test]
fn test_cloudflare_client_creation() {
    use ojs_wasm_sdk::edge::CloudflareClient;
    let client = CloudflareClient::new("https://ojs.example.com");
    drop(client);
}

#[wasm_bindgen_test]
fn test_cloudflare_client_with_auth() {
    use ojs_wasm_sdk::edge::CloudflareClient;
    let client = CloudflareClient::with_auth("https://ojs.example.com", "secret");
    drop(client);
}

#[wasm_bindgen_test]
fn test_deno_client_creation() {
    use ojs_wasm_sdk::edge::DenoClient;
    let client = DenoClient::new("https://ojs.example.com");
    drop(client);
}

#[wasm_bindgen_test]
fn test_vercel_client_creation() {
    use ojs_wasm_sdk::edge::VercelEdgeClient;
    let client = VercelEdgeClient::new("https://ojs.example.com");
    drop(client);
}

// ===========================================================================
// ServiceWorkerClient construction
// ===========================================================================

#[wasm_bindgen_test]
fn test_service_worker_client_creation() {
    use ojs_wasm_sdk::service_worker::ServiceWorkerClient;
    let client = ServiceWorkerClient::new("https://api.example.com");
    drop(client);
}

// ===========================================================================
// Middleware chain
// ===========================================================================

#[wasm_bindgen_test]
fn test_middleware_chain_creation() {
    use ojs_wasm_sdk::middleware::MiddlewareChain;
    let chain = MiddlewareChain::new();
    let names = chain.list();
    let arr: js_sys::Array = names.dyn_into().unwrap();
    assert_eq!(arr.length(), 0);
}

#[wasm_bindgen_test]
fn test_middleware_add_and_list() {
    use ojs_wasm_sdk::middleware::MiddlewareChain;

    let mut mw = MiddlewareChain::new();
    let identity = js_sys::Function::new_with_args("req", "return req");
    mw.add("test-mw", identity);

    let names = mw.list();
    let arr: js_sys::Array = names.dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
    assert_eq!(arr.get(0).as_string().unwrap(), "test-mw");
}

#[wasm_bindgen_test]
fn test_middleware_remove() {
    use ojs_wasm_sdk::middleware::MiddlewareChain;

    let mut mw = MiddlewareChain::new();
    let identity = js_sys::Function::new_with_args("req", "return req");
    mw.add("mw-a", identity.clone());
    mw.add("mw-b", identity);
    mw.remove("mw-a");

    let names = mw.list();
    let arr: js_sys::Array = names.dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
    assert_eq!(arr.get(0).as_string().unwrap(), "mw-b");
}

#[wasm_bindgen_test]
fn test_middleware_apply() {
    use ojs_wasm_sdk::middleware::{MiddlewareChain, create_request};

    let mut mw = MiddlewareChain::new();
    let add_header = js_sys::Function::new_with_args(
        "req",
        "req.headers['X-Custom'] = 'test'; return req;",
    );
    mw.add("custom-header", add_header);

    let req = create_request("POST", "http://example.com/jobs", JsValue::NULL).unwrap();
    let result = mw.apply(req);
    assert!(result.is_ok());

    let modified = result.unwrap();
    let headers = js_sys::Reflect::get(&modified, &"headers".into()).unwrap();
    let custom = js_sys::Reflect::get(&headers, &"X-Custom".into()).unwrap();
    assert_eq!(custom.as_string().unwrap(), "test");
}

#[wasm_bindgen_test]
fn test_create_request() {
    use ojs_wasm_sdk::middleware::create_request;

    let req = create_request("GET", "http://example.com/health", JsValue::NULL).unwrap();
    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    let url = js_sys::Reflect::get(&req, &"url".into()).unwrap();
    assert_eq!(method.as_string().unwrap(), "GET");
    assert_eq!(url.as_string().unwrap(), "http://example.com/health");
}

// ===========================================================================
// Retry policy
// ===========================================================================

#[wasm_bindgen_test]
fn test_retry_policy_exponential() {
    use ojs_wasm_sdk::retry::RetryPolicy;

    let policy = RetryPolicy::exponential(5, 1000, 60000);
    let obj = policy.to_object().unwrap();

    let max = js_sys::Reflect::get(&obj, &"max_attempts".into()).unwrap();
    assert_eq!(max.as_f64().unwrap(), 5.0);

    let backoff = js_sys::Reflect::get(&obj, &"backoff".into()).unwrap();
    let bt = js_sys::Reflect::get(&backoff, &"type".into()).unwrap();
    assert_eq!(bt.as_string().unwrap(), "exponential");

    let initial = js_sys::Reflect::get(&backoff, &"initial_ms".into()).unwrap();
    assert_eq!(initial.as_f64().unwrap(), 1000.0);

    let max_delay = js_sys::Reflect::get(&backoff, &"max_ms".into()).unwrap();
    assert_eq!(max_delay.as_f64().unwrap(), 60000.0);
}

#[wasm_bindgen_test]
fn test_retry_policy_fixed() {
    use ojs_wasm_sdk::retry::RetryPolicy;

    let policy = RetryPolicy::fixed(3, 5000);
    let obj = policy.to_object().unwrap();

    let backoff = js_sys::Reflect::get(&obj, &"backoff".into()).unwrap();
    let bt = js_sys::Reflect::get(&backoff, &"type".into()).unwrap();
    assert_eq!(bt.as_string().unwrap(), "fixed");
}

#[wasm_bindgen_test]
fn test_retry_policy_linear() {
    use ojs_wasm_sdk::retry::RetryPolicy;

    let policy = RetryPolicy::linear(10, 500, 30000);
    let obj = policy.to_object().unwrap();

    let max = js_sys::Reflect::get(&obj, &"max_attempts".into()).unwrap();
    assert_eq!(max.as_f64().unwrap(), 10.0);

    let backoff = js_sys::Reflect::get(&obj, &"backoff".into()).unwrap();
    let bt = js_sys::Reflect::get(&backoff, &"type".into()).unwrap();
    assert_eq!(bt.as_string().unwrap(), "linear");
}
