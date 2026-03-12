//! Tests for the durable execution module.

use ojs_wasm_sdk::durable::DurableContext;

#[test]
fn test_durable_context_creation() {
    let ctx = DurableContext::new("http://localhost:8080", "job-123");
    assert_eq!(ctx.job_id(), "job-123");
}

#[test]
fn test_durable_context_url_normalization() {
    // Trailing slash should be stripped
    let ctx = DurableContext::new("http://localhost:8080/", "job-456");
    assert_eq!(ctx.job_id(), "job-456");
}

#[test]
fn test_durable_context_different_jobs() {
    let ctx1 = DurableContext::new("http://localhost:8080", "job-1");
    let ctx2 = DurableContext::new("http://localhost:8080", "job-2");
    assert_ne!(ctx1.job_id(), ctx2.job_id());
}
