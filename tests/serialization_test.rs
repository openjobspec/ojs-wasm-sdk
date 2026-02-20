//! Tests for serialization edge cases, unicode handling, and JsValue interop.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::types::*;

// ===========================================================================
// Unicode and special characters in job fields
// ===========================================================================

#[wasm_bindgen_test]
fn job_with_unicode_type() {
    let json = r#"{
        "id": "u-001",
        "type": "通知.送信"
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.job_type, "通知.送信");
}

#[wasm_bindgen_test]
fn job_with_unicode_args() {
    let json = r#"{
        "id": "u-002",
        "type": "email.send",
        "args": ["ユーザー@example.com", "こんにちは"]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args[0], "ユーザー@example.com");
    assert_eq!(args[1], "こんにちは");
}

#[wasm_bindgen_test]
fn job_with_emoji_in_tags() {
    let json = r#"{
        "id": "u-003",
        "type": "test",
        "tags": ["🚀", "✅", "⚡"]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let tags = job.tags.unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], "🚀");
}

#[wasm_bindgen_test]
fn job_with_special_chars_in_queue() {
    let json = r#"{
        "id": "u-004",
        "type": "test",
        "queue": "my-queue_v2.critical"
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.queue, "my-queue_v2.critical");
}

// ===========================================================================
// Arg type diversity
// ===========================================================================

#[wasm_bindgen_test]
fn job_args_boolean_values() {
    let json = r#"{
        "id": "args-001",
        "type": "toggle",
        "args": [true, false, true]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args[0], true);
    assert_eq!(args[1], false);
}

#[wasm_bindgen_test]
fn job_args_null_values() {
    let json = r#"{
        "id": "args-002",
        "type": "process",
        "args": [null, "valid", null]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert!(args[0].is_null());
    assert_eq!(args[1], "valid");
    assert!(args[2].is_null());
}

#[wasm_bindgen_test]
fn job_args_mixed_types() {
    let json = r#"{
        "id": "args-003",
        "type": "complex",
        "args": [42, "hello", true, null, 3.14, {"key": "value"}, [1, 2]]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args.len(), 7);
    assert_eq!(args[0], 42);
    assert_eq!(args[1], "hello");
    assert_eq!(args[2], true);
    assert!(args[3].is_null());
    assert!(args[5].is_object());
    assert!(args[6].is_array());
}

#[wasm_bindgen_test]
fn job_args_deeply_nested() {
    let json = r#"{
        "id": "args-004",
        "type": "deep",
        "args": [{"a": {"b": {"c": {"d": "deep_value"}}}}]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args[0]["a"]["b"]["c"]["d"], "deep_value");
}

#[wasm_bindgen_test]
fn job_args_large_numbers() {
    let json = r#"{
        "id": "args-005",
        "type": "numbers",
        "args": [9007199254740991, -9007199254740991, 0.000001]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args.len(), 3);
}

#[wasm_bindgen_test]
fn job_args_empty_string() {
    let json = r#"{
        "id": "args-006",
        "type": "test",
        "args": ["", "", ""]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "");
}

// ===========================================================================
// Serialization roundtrips
// ===========================================================================

#[wasm_bindgen_test]
fn enqueue_options_roundtrip() {
    let opts = EnqueueOptions {
        queue: Some("test-queue".to_string()),
        priority: Some(42),
        timeout_ms: Some(30000),
        delay_until: Some("2025-01-01T00:00:00Z".to_string()),
        tags: Some(vec!["a".to_string(), "b".to_string()]),
    };

    let json = serde_json::to_string(&opts).unwrap();
    let back: EnqueueOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(opts.queue, back.queue);
    assert_eq!(opts.priority, back.priority);
    assert_eq!(opts.timeout_ms, back.timeout_ms);
    assert_eq!(opts.delay_until, back.delay_until);
    assert_eq!(opts.tags, back.tags);
}

#[wasm_bindgen_test]
fn job_full_roundtrip() {
    let job = Job {
        id: "roundtrip-001".to_string(),
        job_type: "test.roundtrip".to_string(),
        queue: "testing".to_string(),
        args: serde_json::json!(["arg1", 42, true]),
        priority: 5,
        state: Some(JobState::Active),
        attempt: 2,
        tags: Some(vec!["tag1".to_string()]),
        meta: Some(serde_json::json!({"key": "value"})),
        created_at: Some("2024-06-01T12:00:00Z".to_string()),
        enqueued_at: Some("2024-06-01T12:00:01Z".to_string()),
        started_at: Some("2024-06-01T12:00:02Z".to_string()),
        completed_at: None,
    };

    let json = serde_json::to_string(&job).unwrap();
    let back: Job = serde_json::from_str(&json).unwrap();

    assert_eq!(job.id, back.id);
    assert_eq!(job.job_type, back.job_type);
    assert_eq!(job.queue, back.queue);
    assert_eq!(job.priority, back.priority);
    assert_eq!(job.state, back.state);
    assert_eq!(job.attempt, back.attempt);
    assert_eq!(job.tags, back.tags);
    assert_eq!(job.meta, back.meta);
    assert_eq!(job.created_at, back.created_at);
    assert_eq!(job.completed_at, back.completed_at);
}

#[wasm_bindgen_test]
fn health_response_roundtrip() {
    let health = HealthResponse {
        status: "ok".to_string(),
        version: Some("1.2.3".to_string()),
        uptime_seconds: Some(86400),
    };

    let json = serde_json::to_string(&health).unwrap();
    let back: HealthResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(health.status, back.status);
    assert_eq!(health.version, back.version);
    assert_eq!(health.uptime_seconds, back.uptime_seconds);
}

#[wasm_bindgen_test]
fn workflow_response_roundtrip() {
    let wf = WorkflowResponse {
        id: "wf-rt-001".to_string(),
        workflow_type: "chain".to_string(),
        name: Some("my-pipeline".to_string()),
        state: Some(WorkflowState::Running),
        metadata: Some(WorkflowMetadata {
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            started_at: Some("2024-01-01T00:00:01Z".to_string()),
            completed_at: None,
            job_count: 3,
            completed_count: 1,
            failed_count: 0,
        }),
    };

    let json = serde_json::to_string(&wf).unwrap();
    let back: WorkflowResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(wf.id, back.id);
    assert_eq!(wf.workflow_type, back.workflow_type);
    assert_eq!(wf.name, back.name);
    assert_eq!(wf.state, back.state);
    assert_eq!(
        wf.metadata.as_ref().unwrap().job_count,
        back.metadata.as_ref().unwrap().job_count
    );
}

// ===========================================================================
// JsValue interop via serde-wasm-bindgen
// ===========================================================================

#[wasm_bindgen_test]
fn job_to_jsvalue_and_back() {
    let job = Job {
        id: "js-001".to_string(),
        job_type: "email.send".to_string(),
        queue: "default".to_string(),
        args: serde_json::json!(["test@example.com"]),
        priority: 0,
        state: Some(JobState::Pending),
        attempt: 0,
        tags: None,
        meta: None,
        created_at: None,
        enqueued_at: None,
        started_at: None,
        completed_at: None,
    };

    let js_val = serde_wasm_bindgen::to_value(&job).unwrap();
    assert!(js_val.is_object());

    let back: Job = serde_wasm_bindgen::from_value(js_val).unwrap();
    assert_eq!(back.id, "js-001");
    assert_eq!(back.job_type, "email.send");
    assert_eq!(back.state, Some(JobState::Pending));
}

#[wasm_bindgen_test]
fn enqueue_options_to_jsvalue() {
    let opts = EnqueueOptions {
        queue: Some("fast".to_string()),
        priority: Some(10),
        timeout_ms: None,
        delay_until: None,
        tags: Some(vec!["urgent".to_string()]),
    };

    let js_val = serde_wasm_bindgen::to_value(&opts).unwrap();
    assert!(js_val.is_object());

    let queue = js_sys::Reflect::get(&js_val, &"queue".into()).unwrap();
    assert_eq!(queue.as_string().unwrap(), "fast");

    let priority = js_sys::Reflect::get(&js_val, &"priority".into()).unwrap();
    assert_eq!(priority.as_f64().unwrap(), 10.0);
}

#[wasm_bindgen_test]
fn health_response_to_jsvalue() {
    let health = HealthResponse {
        status: "ok".to_string(),
        version: Some("0.1.0".to_string()),
        uptime_seconds: Some(3600),
    };

    let js_val = serde_wasm_bindgen::to_value(&health).unwrap();
    let status = js_sys::Reflect::get(&js_val, &"status".into()).unwrap();
    assert_eq!(status.as_string().unwrap(), "ok");
}

#[wasm_bindgen_test]
fn job_state_to_jsvalue() {
    let state = JobState::Completed;
    let js_val = serde_wasm_bindgen::to_value(&state).unwrap();
    assert_eq!(js_val.as_string().unwrap(), "completed");
}

#[wasm_bindgen_test]
fn workflow_state_to_jsvalue() {
    let state = WorkflowState::Failed;
    let js_val = serde_wasm_bindgen::to_value(&state).unwrap();
    assert_eq!(js_val.as_string().unwrap(), "failed");
}

// ===========================================================================
// Edge case: escaped strings
// ===========================================================================

#[wasm_bindgen_test]
fn job_with_escaped_quotes_in_args() {
    let json = r#"{
        "id": "esc-001",
        "type": "test",
        "args": ["he said \"hello\"", "line1\nline2"]
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert!(args[0].as_str().unwrap().contains("hello"));
    assert!(args[1].as_str().unwrap().contains('\n'));
}

#[wasm_bindgen_test]
fn job_with_backslash_in_type() {
    let json = r#"{
        "id": "esc-002",
        "type": "path\\to\\job"
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    assert!(job.job_type.contains('\\'));
}

// ===========================================================================
// Edge case: empty and boundary values
// ===========================================================================

#[wasm_bindgen_test]
fn job_empty_queue_string() {
    let json = r#"{
        "id": "edge-001",
        "type": "test",
        "queue": ""
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.queue, "");
}

#[wasm_bindgen_test]
fn job_zero_priority() {
    let json = r#"{
        "id": "edge-002",
        "type": "test",
        "priority": 0
    }"#;
    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.priority, 0);
}

#[wasm_bindgen_test]
fn job_max_i32_priority() {
    let json = format!(
        r#"{{"id": "edge-003", "type": "test", "priority": {}}}"#,
        i32::MAX
    );
    let job: Job = serde_json::from_str(&json).unwrap();
    assert_eq!(job.priority, i32::MAX);
}

#[wasm_bindgen_test]
fn job_min_i32_priority() {
    let json = format!(
        r#"{{"id": "edge-004", "type": "test", "priority": {}}}"#,
        i32::MIN
    );
    let job: Job = serde_json::from_str(&json).unwrap();
    assert_eq!(job.priority, i32::MIN);
}

#[wasm_bindgen_test]
fn enqueue_options_empty_tags_list() {
    let opts = EnqueueOptions {
        queue: None,
        priority: None,
        timeout_ms: None,
        delay_until: None,
        tags: Some(vec![]),
    };

    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"tags\":[]"));

    let back: EnqueueOptions = serde_json::from_str(&json).unwrap();
    assert!(back.tags.unwrap().is_empty());
}

#[wasm_bindgen_test]
fn enqueue_options_zero_timeout() {
    let opts = EnqueueOptions {
        queue: None,
        priority: None,
        timeout_ms: Some(0),
        delay_until: None,
        tags: None,
    };

    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"timeout_ms\":0"));
}

// ===========================================================================
// Queue types serialization
// ===========================================================================

#[wasm_bindgen_test]
fn queue_info_deserialization() {
    use ojs_wasm_sdk::queue::QueueInfo;

    let json = r#"{"name": "default", "paused": false, "depth": 42}"#;
    let info: QueueInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.name, "default");
    assert!(!info.paused);
    assert_eq!(info.depth, 42);
}

#[wasm_bindgen_test]
fn queue_info_defaults() {
    use ojs_wasm_sdk::queue::QueueInfo;

    let json = r#"{"name": "test"}"#;
    let info: QueueInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.name, "test");
    assert!(!info.paused);
    assert_eq!(info.depth, 0);
}

#[wasm_bindgen_test]
fn queue_stats_deserialization() {
    use ojs_wasm_sdk::queue::QueueStats;

    let json = r#"{
        "name": "critical",
        "pending": 100,
        "active": 5,
        "completed": 1000,
        "failed": 3,
        "paused": true
    }"#;
    let stats: QueueStats = serde_json::from_str(json).unwrap();
    assert_eq!(stats.name, "critical");
    assert_eq!(stats.pending, 100);
    assert_eq!(stats.active, 5);
    assert_eq!(stats.completed, 1000);
    assert_eq!(stats.failed, 3);
    assert!(stats.paused);
}

#[wasm_bindgen_test]
fn queue_stats_defaults() {
    use ojs_wasm_sdk::queue::QueueStats;

    let json = r#"{"name": "minimal"}"#;
    let stats: QueueStats = serde_json::from_str(json).unwrap();
    assert_eq!(stats.name, "minimal");
    assert_eq!(stats.pending, 0);
    assert_eq!(stats.active, 0);
    assert_eq!(stats.completed, 0);
    assert_eq!(stats.failed, 0);
    assert!(!stats.paused);
}

#[wasm_bindgen_test]
fn queue_info_roundtrip() {
    use ojs_wasm_sdk::queue::QueueInfo;

    let info = QueueInfo {
        name: "roundtrip-queue".to_string(),
        paused: true,
        depth: 999,
    };

    let json = serde_json::to_string(&info).unwrap();
    let back: QueueInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info.name, back.name);
    assert_eq!(info.paused, back.paused);
    assert_eq!(info.depth, back.depth);
}

#[wasm_bindgen_test]
fn queue_stats_roundtrip() {
    use ojs_wasm_sdk::queue::QueueStats;

    let stats = QueueStats {
        name: "roundtrip".to_string(),
        pending: 10,
        active: 3,
        completed: 500,
        failed: 2,
        paused: false,
    };

    let json = serde_json::to_string(&stats).unwrap();
    let back: QueueStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats.name, back.name);
    assert_eq!(stats.pending, back.pending);
    assert_eq!(stats.active, back.active);
    assert_eq!(stats.completed, back.completed);
    assert_eq!(stats.failed, back.failed);
    assert_eq!(stats.paused, back.paused);
}
