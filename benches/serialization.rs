//! # Serialization Benchmarks
//!
//! Measures the performance of core serialization operations in the OJS WASM SDK.
//! These benchmarks run as native Rust (not WASM) to provide reliable timing.
//!
//! Run with: `cargo bench`

use ojs_wasm_sdk::types::*;
use std::time::Instant;

const ITERATIONS: u32 = 10_000;

fn bench_job_creation() {
    let start = Instant::now();
    for i in 0..ITERATIONS {
        let _req = EnqueueRequest {
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
        // Prevent optimization from eliding the work
        if i == ITERATIONS - 1 {
            std::hint::black_box(&_req);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "job_creation:       {:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn bench_job_serialization() {
    let req = EnqueueRequest {
        job_type: "email.send".to_string(),
        args: serde_json::json!(["user@example.com", "Welcome!", {"template": "onboarding"}]),
        options: Some(EnqueueOptions {
            queue: Some("critical".to_string()),
            priority: Some(10),
            timeout_ms: Some(30000),
            delay_until: None,
            tags: Some(vec!["onboarding".to_string(), "email".to_string()]),
        }),
    };

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let json = serde_json::to_string(&req).unwrap();
        if i == ITERATIONS - 1 {
            std::hint::black_box(&json);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "job_serialization:  {:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn bench_job_deserialization() {
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

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let job: Job = serde_json::from_str(json).unwrap();
        if i == ITERATIONS - 1 {
            std::hint::black_box(&job);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "job_deser:          {:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn bench_batch_serialization() {
    let batch = BatchRequest {
        jobs: (0..10)
            .map(|i| EnqueueRequest {
                job_type: format!("job.type_{}", i),
                args: serde_json::json!([i, format!("arg_{}", i)]),
                options: None,
            })
            .collect(),
    };

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let json = serde_json::to_string(&batch).unwrap();
        if i == ITERATIONS - 1 {
            std::hint::black_box(&json);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "batch_ser (10 jobs):{:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn bench_workflow_response_deserialization() {
    let json = r#"{
        "id": "wf-001",
        "type": "chain",
        "name": "data-pipeline",
        "state": "running",
        "metadata": {
            "created_at": "2024-01-15T10:00:00Z",
            "started_at": "2024-01-15T10:00:01Z",
            "job_count": 5,
            "completed_count": 2,
            "failed_count": 0
        }
    }"#;

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let wf: WorkflowResponse = serde_json::from_str(json).unwrap();
        if i == ITERATIONS - 1 {
            std::hint::black_box(&wf);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "workflow_deser:     {:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn bench_json_roundtrip() {
    let req = EnqueueRequest {
        job_type: "data.process".to_string(),
        args: serde_json::json!([1, 2, 3, "hello", {"nested": true}]),
        options: Some(EnqueueOptions {
            queue: Some("default".to_string()),
            priority: Some(5),
            timeout_ms: Some(10000),
            delay_until: Some("2024-12-01T00:00:00Z".to_string()),
            tags: Some(vec!["test".to_string()]),
        }),
    };

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let json = serde_json::to_string(&req).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        if i == ITERATIONS - 1 {
            std::hint::black_box(&_parsed);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "json_roundtrip:     {:>8} iterations in {:>8.2?}  ({:.0} ops/sec)",
        ITERATIONS,
        elapsed,
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn main() {
    println!("OJS WASM SDK — Serialization Benchmarks");
    println!("========================================");
    println!();

    bench_job_creation();
    bench_job_serialization();
    bench_job_deserialization();
    bench_batch_serialization();
    bench_workflow_response_deserialization();
    bench_json_roundtrip();

    println!();
    println!("Done.");
}
