//! Integration tests: end-to-end request/response serialization pipeline.
//!
//! These tests verify the full flow that the client executes internally:
//!   build request types → serialize to JSON → parse server response JSON → typed structs
//!
//! No actual HTTP calls are made; we test the serialization boundary that sits
//! between the SDK and the OJS HTTP API.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::types::*;

// ===========================================================================
// Enqueue flow: build request → serialize → mock response → deserialize
// ===========================================================================

#[wasm_bindgen_test]
fn integration_enqueue_roundtrip() {
    // 1. Build the request the client would send
    let request = EnqueueRequest {
        job_type: "email.send".to_string(),
        args: serde_json::json!(["user@example.com", "Welcome!"]),
        options: Some(EnqueueOptions {
            queue: Some("critical".to_string()),
            priority: Some(10),
            timeout_ms: Some(30000),
            delay_until: None,
            tags: Some(vec!["onboarding".to_string()]),
        }),
    };

    // 2. Serialize to JSON (what gets sent over the wire)
    let wire_json = serde_json::to_string(&request).unwrap();

    // 3. Verify the wire format matches OJS HTTP API expectations
    let wire: serde_json::Value = serde_json::from_str(&wire_json).unwrap();
    assert_eq!(wire["type"], "email.send");
    assert_eq!(wire["args"][0], "user@example.com");
    assert_eq!(wire["args"][1], "Welcome!");
    assert_eq!(wire["options"]["queue"], "critical");
    assert_eq!(wire["options"]["priority"], 10);
    assert_eq!(wire["options"]["timeout_ms"], 30000);
    assert!(wire["options"].get("delay_until").is_none());
    assert_eq!(wire["options"]["tags"][0], "onboarding");

    // 4. Simulate server response (what the OJS server returns)
    let server_response = r#"{
        "job": {
            "id": "019012ab-cdef-7000-8000-000000000001",
            "type": "email.send",
            "queue": "critical",
            "args": ["user@example.com", "Welcome!"],
            "priority": 10,
            "state": "pending",
            "attempt": 0,
            "tags": ["onboarding"],
            "created_at": "2024-01-15T10:30:00Z",
            "enqueued_at": "2024-01-15T10:30:00Z"
        }
    }"#;

    // 5. Deserialize the response (what the client does internally)
    let resp: JobResponse = serde_json::from_str(server_response).unwrap();

    // 6. Verify the deserialized job matches expectations
    assert_eq!(resp.job.id, "019012ab-cdef-7000-8000-000000000001");
    assert_eq!(resp.job.job_type, "email.send");
    assert_eq!(resp.job.queue, "critical");
    assert_eq!(resp.job.priority, 10);
    assert_eq!(resp.job.state, Some(JobState::Pending));
    assert_eq!(resp.job.attempt, 0);
    assert_eq!(resp.job.tags.as_ref().unwrap(), &["onboarding"]);
    assert!(resp.job.created_at.is_some());
    assert!(resp.job.completed_at.is_none());
}

// ===========================================================================
// Batch enqueue flow
// ===========================================================================

#[wasm_bindgen_test]
fn integration_batch_enqueue_roundtrip() {
    // 1. Build batch request
    let batch = BatchRequest {
        jobs: vec![
            EnqueueRequest {
                job_type: "email.send".to_string(),
                args: serde_json::json!(["a@b.com"]),
                options: None,
            },
            EnqueueRequest {
                job_type: "sms.send".to_string(),
                args: serde_json::json!(["+1234567890", "Hello"]),
                options: Some(EnqueueOptions {
                    queue: Some("sms".to_string()),
                    ..EnqueueOptions::default()
                }),
            },
        ],
    };

    // 2. Serialize
    let wire_json = serde_json::to_string(&batch).unwrap();
    let wire: serde_json::Value = serde_json::from_str(&wire_json).unwrap();
    assert_eq!(wire["jobs"].as_array().unwrap().len(), 2);
    assert_eq!(wire["jobs"][0]["type"], "email.send");
    assert_eq!(wire["jobs"][1]["type"], "sms.send");
    assert_eq!(wire["jobs"][1]["options"]["queue"], "sms");

    // 3. Mock server response
    let server_response = r#"{
        "jobs": [
            {"id": "job-001", "type": "email.send", "queue": "default", "state": "pending"},
            {"id": "job-002", "type": "sms.send", "queue": "sms", "state": "pending"}
        ],
        "count": 2
    }"#;

    // 4. Deserialize
    let resp: BatchResponse = serde_json::from_str(server_response).unwrap();
    assert_eq!(resp.count, 2);
    assert_eq!(resp.jobs.len(), 2);
    assert_eq!(resp.jobs[0].id, "job-001");
    assert_eq!(resp.jobs[1].queue, "sms");
}

// ===========================================================================
// Get job flow (server → client)
// ===========================================================================

#[wasm_bindgen_test]
fn integration_get_job_response_all_states() {
    let states = [
        ("scheduled", JobState::Scheduled),
        ("available", JobState::Available),
        ("pending", JobState::Pending),
        ("active", JobState::Active),
        ("completed", JobState::Completed),
        ("retryable", JobState::Retryable),
        ("cancelled", JobState::Cancelled),
        ("discarded", JobState::Discarded),
    ];

    for (state_str, expected_state) in &states {
        let response = format!(
            r#"{{"job": {{"id": "j-1", "type": "test", "state": "{}"}}}}"#,
            state_str
        );
        let resp: JobResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            resp.job.state,
            Some(expected_state.clone()),
            "failed for state: {}",
            state_str
        );
    }
}

// ===========================================================================
// Health check flow
// ===========================================================================

#[wasm_bindgen_test]
fn integration_health_check_roundtrip() {
    let server_response = r#"{"status": "ok", "version": "1.0.0", "uptime_seconds": 86400}"#;
    let health: HealthResponse = serde_json::from_str(server_response).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.version.unwrap(), "1.0.0");
    assert_eq!(health.uptime_seconds.unwrap(), 86400);
}

// ===========================================================================
// Workflow flow: request → serialize → response → deserialize
// ===========================================================================

#[wasm_bindgen_test]
fn integration_workflow_chain_roundtrip() {
    // Simulate the JSON the chain() builder would produce
    let workflow_request = serde_json::json!({
        "type": "chain",
        "steps": [
            {"type": "data.fetch", "args": ["https://api.example.com"]},
            {"type": "data.transform", "args": ["csv"]},
            {"type": "data.load", "args": ["warehouse"]}
        ]
    });

    // Verify it serializes cleanly
    let wire = serde_json::to_string(&workflow_request).unwrap();
    assert!(wire.contains("\"type\":\"chain\""));
    assert!(wire.contains("data.fetch"));

    // Mock server response
    let server_response = r#"{
        "id": "wf-pipeline-001",
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

    let resp: WorkflowResponse = serde_json::from_str(server_response).unwrap();
    assert_eq!(resp.id, "wf-pipeline-001");
    assert_eq!(resp.workflow_type, "chain");
    assert_eq!(resp.state, Some(WorkflowState::Running));

    let meta = resp.metadata.unwrap();
    assert_eq!(meta.job_count, 3);
    assert_eq!(meta.completed_count, 1);
    assert_eq!(meta.failed_count, 0);
}

#[wasm_bindgen_test]
fn integration_workflow_group_response() {
    let server_response = r#"{
        "id": "wf-export-001",
        "type": "group",
        "state": "completed",
        "metadata": {
            "job_count": 3,
            "completed_count": 3,
            "failed_count": 0,
            "completed_at": "2024-01-15T10:05:00Z"
        }
    }"#;

    let resp: WorkflowResponse = serde_json::from_str(server_response).unwrap();
    assert_eq!(resp.state, Some(WorkflowState::Completed));
    let meta = resp.metadata.unwrap();
    assert_eq!(meta.completed_count, meta.job_count);
    assert!(meta.completed_at.is_some());
}

#[wasm_bindgen_test]
fn integration_workflow_batch_response() {
    let server_response = r#"{
        "id": "wf-batch-001",
        "type": "batch",
        "state": "failed",
        "metadata": {
            "job_count": 10,
            "completed_count": 8,
            "failed_count": 2
        }
    }"#;

    let resp: WorkflowResponse = serde_json::from_str(server_response).unwrap();
    assert_eq!(resp.state, Some(WorkflowState::Failed));
    let meta = resp.metadata.unwrap();
    assert_eq!(meta.failed_count, 2);
    assert_eq!(meta.completed_count + meta.failed_count, meta.job_count);
}

// ===========================================================================
// Error response flow
// ===========================================================================

#[wasm_bindgen_test]
fn integration_server_error_response() {
    use ojs_wasm_sdk::error::ServerError;

    // Simulate server error responses for common cases
    let cases = vec![
        (
            r#"{"code": "not_found", "message": "Job not found", "retryable": false}"#,
            "not_found",
            false,
        ),
        (
            r#"{"code": "rate_limited", "message": "Too many requests", "retryable": true}"#,
            "rate_limited",
            true,
        ),
        (
            r#"{"code": "conflict", "message": "Job already exists", "retryable": false}"#,
            "conflict",
            false,
        ),
        (
            r#"{"code": "internal_error", "message": "Unexpected failure", "retryable": true}"#,
            "internal_error",
            true,
        ),
    ];

    for (json, expected_code, expected_retryable) in cases {
        let err: ServerError = serde_json::from_str(json).unwrap();
        assert_eq!(err.code, expected_code);
        assert_eq!(err.retryable, expected_retryable);
    }
}

// ===========================================================================
// Queue management flow
// ===========================================================================

#[wasm_bindgen_test]
fn integration_queue_list_response() {
    use ojs_wasm_sdk::queue::QueueInfo;

    let server_response = r#"[
        {"name": "default", "paused": false, "depth": 42},
        {"name": "critical", "paused": false, "depth": 5},
        {"name": "maintenance", "paused": true, "depth": 0}
    ]"#;

    let queues: Vec<QueueInfo> = serde_json::from_str(server_response).unwrap();
    assert_eq!(queues.len(), 3);
    assert_eq!(queues[0].name, "default");
    assert!(!queues[0].paused);
    assert_eq!(queues[0].depth, 42);
    assert!(queues[2].paused);
}

#[wasm_bindgen_test]
fn integration_queue_stats_response() {
    use ojs_wasm_sdk::queue::QueueStats;

    let server_response = r#"{
        "name": "critical",
        "pending": 100,
        "active": 5,
        "completed": 10000,
        "failed": 12,
        "paused": false
    }"#;

    let stats: QueueStats = serde_json::from_str(server_response).unwrap();
    assert_eq!(stats.name, "critical");
    assert_eq!(stats.pending, 100);
    assert_eq!(stats.active, 5);
    assert_eq!(stats.completed, 10000);
    assert_eq!(stats.failed, 12);
}

// ===========================================================================
// End-to-end: full client pipeline simulation
// ===========================================================================

#[wasm_bindgen_test]
fn integration_full_pipeline_enqueue_then_get() {
    // Step 1: Client builds and serializes an enqueue request
    let enqueue_req = EnqueueRequest {
        job_type: "report.generate".to_string(),
        args: serde_json::json!([2024, "quarterly"]),
        options: Some(EnqueueOptions {
            queue: Some("reports".to_string()),
            priority: Some(5),
            timeout_ms: Some(120000),
            delay_until: None,
            tags: Some(vec!["finance".to_string(), "q4".to_string()]),
        }),
    };

    let request_body = serde_json::to_string(&enqueue_req).unwrap();

    // Step 2: Server receives and responds with the created job
    let enqueue_response = r#"{
        "job": {
            "id": "019012ab-cdef-7000-8000-000000000099",
            "type": "report.generate",
            "queue": "reports",
            "args": [2024, "quarterly"],
            "priority": 5,
            "state": "pending",
            "attempt": 0,
            "tags": ["finance", "q4"],
            "created_at": "2024-06-15T14:00:00Z",
            "enqueued_at": "2024-06-15T14:00:00Z"
        }
    }"#;

    let created: JobResponse = serde_json::from_str(enqueue_response).unwrap();
    let job_id = created.job.id.clone();

    // Step 3: Client later fetches the job (now active)
    let get_response = format!(
        r#"{{
            "job": {{
                "id": "{}",
                "type": "report.generate",
                "queue": "reports",
                "args": [2024, "quarterly"],
                "priority": 5,
                "state": "active",
                "attempt": 1,
                "tags": ["finance", "q4"],
                "created_at": "2024-06-15T14:00:00Z",
                "enqueued_at": "2024-06-15T14:00:00Z",
                "started_at": "2024-06-15T14:00:01Z"
            }}
        }}"#,
        job_id
    );

    let active: JobResponse = serde_json::from_str(&get_response).unwrap();
    assert_eq!(active.job.id, job_id);
    assert_eq!(active.job.state, Some(JobState::Active));
    assert_eq!(active.job.attempt, 1);
    assert!(active.job.started_at.is_some());

    // Step 4: Job completes
    let completed_response = format!(
        r#"{{
            "job": {{
                "id": "{}",
                "type": "report.generate",
                "queue": "reports",
                "state": "completed",
                "attempt": 1,
                "completed_at": "2024-06-15T14:01:30Z"
            }}
        }}"#,
        job_id
    );

    let completed: JobResponse = serde_json::from_str(&completed_response).unwrap();
    assert_eq!(completed.job.state, Some(JobState::Completed));
    assert!(completed.job.completed_at.is_some());

    // Verify request body is valid JSON the server would accept
    let parsed_request: serde_json::Value = serde_json::from_str(&request_body).unwrap();
    assert_eq!(parsed_request["type"], "report.generate");
    assert_eq!(parsed_request["args"][0], 2024);
}
