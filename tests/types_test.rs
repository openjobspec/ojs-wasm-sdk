//! Tests for core type serialization, deserialization, and edge cases.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::types::*;

// ===========================================================================
// JobState — exhaustive variant coverage
// ===========================================================================

#[wasm_bindgen_test]
fn job_state_all_variants_roundtrip() {
    let variants = vec![
        JobState::Pending,
        JobState::Scheduled,
        JobState::Available,
        JobState::Active,
        JobState::Completed,
        JobState::Retryable,
        JobState::Cancelled,
        JobState::Discarded,
    ];

    for state in variants {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: JobState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized, "roundtrip failed for {:?}", state);
    }
}

#[wasm_bindgen_test]
fn job_state_invalid_value_fails() {
    let result = serde_json::from_str::<JobState>("\"unknown_state\"");
    assert!(result.is_err(), "should reject unknown state");
}

#[wasm_bindgen_test]
fn job_state_empty_string_fails() {
    let result = serde_json::from_str::<JobState>("\"\"");
    assert!(result.is_err(), "should reject empty string");
}

#[wasm_bindgen_test]
fn job_state_number_fails() {
    let result = serde_json::from_str::<JobState>("42");
    assert!(result.is_err(), "should reject numeric value");
}

#[wasm_bindgen_test]
fn job_state_null_fails() {
    let result = serde_json::from_str::<JobState>("null");
    assert!(result.is_err(), "should reject null");
}

#[wasm_bindgen_test]
fn job_state_clone_and_eq() {
    let state = JobState::Active;
    let cloned = state.clone();
    assert_eq!(state, cloned);
}

// ===========================================================================
// Job — comprehensive field testing
// ===========================================================================

#[wasm_bindgen_test]
fn job_with_all_timestamps() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000010",
        "type": "data.process",
        "queue": "default",
        "args": [],
        "priority": 0,
        "state": "completed",
        "attempt": 3,
        "created_at": "2024-01-15T10:00:00Z",
        "enqueued_at": "2024-01-15T10:00:01Z",
        "started_at": "2024-01-15T10:00:02Z",
        "completed_at": "2024-01-15T10:00:05Z"
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert!(job.created_at.is_some());
    assert!(job.enqueued_at.is_some());
    assert!(job.started_at.is_some());
    assert!(job.completed_at.is_some());
    assert_eq!(job.completed_at.unwrap(), "2024-01-15T10:00:05Z");
}

#[wasm_bindgen_test]
fn job_with_meta_field() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000011",
        "type": "email.send",
        "meta": {"source": "api", "version": 2, "nested": {"key": "value"}}
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    let meta = job.meta.unwrap();
    assert_eq!(meta["source"], "api");
    assert_eq!(meta["version"], 2);
    assert_eq!(meta["nested"]["key"], "value");
}

#[wasm_bindgen_test]
fn job_with_null_args() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000012",
        "type": "cleanup"
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert!(job.args.is_null(), "missing args should default to null");
}

#[wasm_bindgen_test]
fn job_with_numeric_args() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000013",
        "type": "calculate",
        "args": [1, 2.5, -3, 0]
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args.len(), 4);
    assert_eq!(args[0], 1);
    assert_eq!(args[2], -3);
}

#[wasm_bindgen_test]
fn job_with_nested_object_args() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000014",
        "type": "notify",
        "args": [{"user": "alice", "channels": ["email", "sms"]}, true]
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    let args = job.args.as_array().unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0]["user"], "alice");
    assert_eq!(args[0]["channels"][1], "sms");
    assert_eq!(args[1], true);
}

#[wasm_bindgen_test]
fn job_with_empty_tags() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000015",
        "type": "test",
        "tags": []
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    let tags = job.tags.unwrap();
    assert!(tags.is_empty());
}

#[wasm_bindgen_test]
fn job_negative_priority() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000016",
        "type": "low.priority",
        "priority": -5
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.priority, -5);
}

#[wasm_bindgen_test]
fn job_high_attempt_count() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000017",
        "type": "resilient",
        "attempt": 999
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.attempt, 999);
}

#[wasm_bindgen_test]
fn job_ignores_unknown_fields() {
    let json = r#"{
        "id": "019012ab-cdef-7000-8000-000000000018",
        "type": "test",
        "unknown_field": "should be ignored",
        "another_unknown": 42
    }"#;

    let job: Job = serde_json::from_str(json).unwrap();
    assert_eq!(job.id, "019012ab-cdef-7000-8000-000000000018");
    assert_eq!(job.job_type, "test");
}

#[wasm_bindgen_test]
fn job_missing_id_fails() {
    let json = r#"{"type": "test"}"#;
    let result = serde_json::from_str::<Job>(json);
    assert!(result.is_err(), "should fail without id");
}

#[wasm_bindgen_test]
fn job_missing_type_fails() {
    let json = r#"{"id": "019012ab-cdef-7000-8000-000000000019"}"#;
    let result = serde_json::from_str::<Job>(json);
    assert!(result.is_err(), "should fail without type");
}

#[wasm_bindgen_test]
fn job_serialization_skips_none_fields() {
    let job = Job {
        id: "test-id".to_string(),
        job_type: "test".to_string(),
        queue: "default".to_string(),
        args: serde_json::Value::Null,
        priority: 0,
        state: None,
        attempt: 0,
        tags: None,
        meta: None,
        created_at: None,
        enqueued_at: None,
        started_at: None,
        completed_at: None,
    };

    let json = serde_json::to_string(&job).unwrap();
    assert!(!json.contains("state"));
    assert!(!json.contains("tags"));
    assert!(!json.contains("meta"));
    assert!(!json.contains("created_at"));
    assert!(!json.contains("completed_at"));
}

// ===========================================================================
// EnqueueOptions — field combinations
// ===========================================================================

#[wasm_bindgen_test]
fn enqueue_options_partial_fields() {
    let opts = EnqueueOptions {
        queue: Some("high".to_string()),
        priority: None,
        timeout_ms: Some(5000),
        delay_until: None,
        tags: None,
    };

    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"queue\":\"high\""));
    assert!(json.contains("\"timeout_ms\":5000"));
    assert!(!json.contains("priority"));
    assert!(!json.contains("delay_until"));
    assert!(!json.contains("tags"));
}

#[wasm_bindgen_test]
fn enqueue_options_all_fields() {
    let opts = EnqueueOptions {
        queue: Some("critical".to_string()),
        priority: Some(100),
        timeout_ms: Some(60000),
        delay_until: Some("2025-06-01T00:00:00Z".to_string()),
        tags: Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
    };

    let json = serde_json::to_string(&opts).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["queue"], "critical");
    assert_eq!(parsed["priority"], 100);
    assert_eq!(parsed["timeout_ms"], 60000);
    assert_eq!(parsed["delay_until"], "2025-06-01T00:00:00Z");
    assert_eq!(parsed["tags"].as_array().unwrap().len(), 3);
}

#[wasm_bindgen_test]
fn enqueue_options_deserialization() {
    let json = r#"{"queue":"fast","priority":5}"#;
    let opts: EnqueueOptions = serde_json::from_str(json).unwrap();
    assert_eq!(opts.queue.unwrap(), "fast");
    assert_eq!(opts.priority.unwrap(), 5);
    assert!(opts.timeout_ms.is_none());
    assert!(opts.tags.is_none());
}

// ===========================================================================
// EnqueueRequest
// ===========================================================================

#[wasm_bindgen_test]
fn enqueue_request_without_options() {
    let req = EnqueueRequest {
        job_type: "test.job".to_string(),
        args: serde_json::json!([1, "hello"]),
        options: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "test.job");
    assert_eq!(parsed["args"][0], 1);
    assert_eq!(parsed["args"][1], "hello");
    assert!(parsed.get("options").is_none());
}

#[wasm_bindgen_test]
fn enqueue_request_with_empty_args() {
    let req = EnqueueRequest {
        job_type: "noop".to_string(),
        args: serde_json::json!([]),
        options: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"args\":[]"));
}

// ===========================================================================
// BatchRequest
// ===========================================================================

#[wasm_bindgen_test]
fn batch_request_single_job() {
    let batch = BatchRequest {
        jobs: vec![EnqueueRequest {
            job_type: "solo".to_string(),
            args: serde_json::json!(["only"]),
            options: None,
        }],
    };

    let json = serde_json::to_string(&batch).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["jobs"][0]["type"], "solo");
}

#[wasm_bindgen_test]
fn batch_request_empty_jobs() {
    let batch = BatchRequest { jobs: vec![] };
    let json = serde_json::to_string(&batch).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jobs"].as_array().unwrap().len(), 0);
}

// ===========================================================================
// BatchResponse
// ===========================================================================

#[wasm_bindgen_test]
fn batch_response_deserialization() {
    let json = r#"{
        "jobs": [
            {"id": "j1", "type": "a", "queue": "default"},
            {"id": "j2", "type": "b", "queue": "default"}
        ],
        "count": 2
    }"#;

    let resp: BatchResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.jobs.len(), 2);
    assert_eq!(resp.count, 2);
    assert_eq!(resp.jobs[0].id, "j1");
    assert_eq!(resp.jobs[1].id, "j2");
}

#[wasm_bindgen_test]
fn batch_response_count_defaults_to_zero() {
    let json = r#"{
        "jobs": [{"id": "j1", "type": "a"}]
    }"#;

    let resp: BatchResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.count, 0);
    assert_eq!(resp.jobs.len(), 1);
}

// ===========================================================================
// JobResponse
// ===========================================================================

#[wasm_bindgen_test]
fn job_response_wraps_job() {
    let json = r#"{
        "job": {
            "id": "019012ab-cdef-7000-8000-000000000020",
            "type": "email.send",
            "queue": "critical",
            "state": "pending"
        }
    }"#;

    let resp: JobResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.job.id, "019012ab-cdef-7000-8000-000000000020");
    assert_eq!(resp.job.job_type, "email.send");
    assert_eq!(resp.job.state, Some(JobState::Pending));
}

// ===========================================================================
// HealthResponse
// ===========================================================================

#[wasm_bindgen_test]
fn health_response_with_large_uptime() {
    let json = r#"{"status": "ok", "uptime_seconds": 8640000}"#;
    let health: HealthResponse = serde_json::from_str(json).unwrap();
    assert_eq!(health.uptime_seconds.unwrap(), 8640000);
}

#[wasm_bindgen_test]
fn health_response_serialization_skips_none() {
    let health = HealthResponse {
        status: "ok".to_string(),
        version: None,
        uptime_seconds: None,
    };

    let json = serde_json::to_string(&health).unwrap();
    assert_eq!(json, r#"{"status":"ok"}"#);
}

// ===========================================================================
// WorkflowState
// ===========================================================================

#[wasm_bindgen_test]
fn workflow_state_all_variants() {
    let states = vec![
        (WorkflowState::Pending, "\"pending\""),
        (WorkflowState::Running, "\"running\""),
        (WorkflowState::Completed, "\"completed\""),
        (WorkflowState::Failed, "\"failed\""),
        (WorkflowState::Cancelled, "\"cancelled\""),
    ];

    for (state, expected) in &states {
        let json = serde_json::to_string(state).unwrap();
        assert_eq!(&json, expected);
        let back: WorkflowState = serde_json::from_str(expected).unwrap();
        assert_eq!(state, &back);
    }
}

#[wasm_bindgen_test]
fn workflow_state_invalid_fails() {
    let result = serde_json::from_str::<WorkflowState>("\"paused\"");
    assert!(result.is_err());
}

// ===========================================================================
// WorkflowResponse
// ===========================================================================

#[wasm_bindgen_test]
fn workflow_response_minimal() {
    let json = r#"{"id": "wf-minimal", "type": "chain"}"#;
    let wf: WorkflowResponse = serde_json::from_str(json).unwrap();
    assert_eq!(wf.id, "wf-minimal");
    assert_eq!(wf.workflow_type, "chain");
    assert!(wf.name.is_none());
    assert!(wf.state.is_none());
    assert!(wf.metadata.is_none());
}

#[wasm_bindgen_test]
fn workflow_response_with_all_metadata() {
    let json = r#"{
        "id": "wf-full",
        "type": "group",
        "name": "parallel-exports",
        "state": "completed",
        "metadata": {
            "created_at": "2024-01-01T00:00:00Z",
            "started_at": "2024-01-01T00:00:01Z",
            "completed_at": "2024-01-01T00:05:00Z",
            "job_count": 10,
            "completed_count": 10,
            "failed_count": 0
        }
    }"#;

    let wf: WorkflowResponse = serde_json::from_str(json).unwrap();
    assert_eq!(wf.state, Some(WorkflowState::Completed));
    let meta = wf.metadata.unwrap();
    assert!(meta.created_at.is_some());
    assert!(meta.started_at.is_some());
    assert!(meta.completed_at.is_some());
    assert_eq!(meta.job_count, 10);
    assert_eq!(meta.completed_count, 10);
    assert_eq!(meta.failed_count, 0);
}

// ===========================================================================
// WorkflowMetadata
// ===========================================================================

#[wasm_bindgen_test]
fn workflow_metadata_defaults_to_zero() {
    let json = r#"{}"#;
    let meta: WorkflowMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.job_count, 0);
    assert_eq!(meta.completed_count, 0);
    assert_eq!(meta.failed_count, 0);
    assert!(meta.created_at.is_none());
}

#[wasm_bindgen_test]
fn workflow_metadata_partial_counts() {
    let json = r#"{"job_count": 5, "completed_count": 2, "failed_count": 1}"#;
    let meta: WorkflowMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.job_count, 5);
    assert_eq!(meta.completed_count, 2);
    assert_eq!(meta.failed_count, 1);
}
