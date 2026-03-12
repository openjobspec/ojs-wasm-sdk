//! Testing utilities for the OJS WASM SDK.
//!
//! Provides a fake in-memory job store for unit testing browser applications
//! that use OJS, without requiring a running OJS server.
//!
//! # Example
//!
//! ```js
//! import { FakeStore } from '@openjobspec/wasm';
//!
//! const store = new FakeStore();
//! store.record_enqueue("email.send", JSON.stringify(["user@example.com"]));
//! store.assert_enqueued("email.send");
//! store.drain();
//! store.assert_completed("email.send");
//! ```

use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

/// A recorded job in the fake store.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FakeJob {
    id: String,
    job_type: String,
    queue: String,
    args: String,
    state: String,
    attempt: u32,
}

struct FakeStoreInner {
    enqueued: Vec<FakeJob>,
    performed: Vec<FakeJob>,
    next_id: u64,
}

/// In-memory fake job store for testing.
///
/// Records enqueued jobs and provides assertion helpers,
/// mirroring the testing module pattern used across all OJS SDKs.
#[wasm_bindgen]
pub struct FakeStore {
    inner: Arc<Mutex<FakeStoreInner>>,
}

#[wasm_bindgen]
impl FakeStore {
    /// Create a new fake store.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(FakeStoreInner {
                enqueued: Vec::new(),
                performed: Vec::new(),
                next_id: 0,
            })),
        }
    }

    /// Record a job enqueue. Returns the fake job ID.
    ///
    /// `args_json` should be the JSON-serialized args string.
    /// `queue` is optional (defaults to "default").
    pub fn record_enqueue(
        &self,
        job_type: &str,
        args_json: &str,
        queue: Option<String>,
    ) -> String {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let id = format!("fake-{:06}", inner.next_id);
        let job = FakeJob {
            id: id.clone(),
            job_type: job_type.to_string(),
            queue: queue.unwrap_or_else(|| "default".to_string()),
            args: args_json.to_string(),
            state: "available".to_string(),
            attempt: 0,
        };
        inner.enqueued.push(job);
        id
    }

    /// Assert that at least one job of the given type was enqueued.
    ///
    /// Throws a JS error if the assertion fails.
    pub fn assert_enqueued(&self, job_type: &str) -> Result<(), JsValue> {
        let inner = self.inner.lock().unwrap();
        let found = inner.enqueued.iter().any(|j| j.job_type == job_type);
        if !found {
            let types: Vec<&str> = inner.enqueued.iter().map(|j| j.job_type.as_str()).collect();
            return Err(JsValue::from_str(&format!(
                "Expected at least one enqueued job of type '{}', found none. Enqueued types: {:?}",
                job_type, types
            )));
        }
        Ok(())
    }

    /// Assert that a specific count of jobs of the given type were enqueued.
    pub fn assert_enqueued_count(&self, job_type: &str, expected: u32) -> Result<(), JsValue> {
        let inner = self.inner.lock().unwrap();
        let count = inner
            .enqueued
            .iter()
            .filter(|j| j.job_type == job_type)
            .count() as u32;
        if count != expected {
            return Err(JsValue::from_str(&format!(
                "Expected {} enqueued job(s) of type '{}', found {}",
                expected, job_type, count
            )));
        }
        Ok(())
    }

    /// Assert that NO job of the given type was enqueued.
    pub fn refute_enqueued(&self, job_type: &str) -> Result<(), JsValue> {
        let inner = self.inner.lock().unwrap();
        let count = inner
            .enqueued
            .iter()
            .filter(|j| j.job_type == job_type)
            .count();
        if count > 0 {
            return Err(JsValue::from_str(&format!(
                "Expected no enqueued jobs of type '{}', but found {}",
                job_type, count
            )));
        }
        Ok(())
    }

    /// Assert that at least one job of the given type was completed.
    pub fn assert_completed(&self, job_type: &str) -> Result<(), JsValue> {
        let inner = self.inner.lock().unwrap();
        let found = inner
            .performed
            .iter()
            .any(|j| j.job_type == job_type && j.state == "completed");
        if !found {
            return Err(JsValue::from_str(&format!(
                "Expected a completed job of type '{}', found none",
                job_type
            )));
        }
        Ok(())
    }

    /// Process all available (enqueued) jobs, marking them as completed.
    ///
    /// Returns the number of jobs processed.
    pub fn drain(&self) -> u32 {
        let mut inner = self.inner.lock().unwrap();
        let mut processed = 0u32;
        let mut completed_jobs = Vec::new();

        for job in inner.enqueued.iter_mut() {
            if job.state == "available" {
                job.state = "completed".to_string();
                job.attempt += 1;
                completed_jobs.push(job.clone());
                processed += 1;
            }
        }

        inner.performed.extend(completed_jobs);
        processed
    }

    /// Return the total number of enqueued jobs.
    pub fn enqueued_count(&self) -> u32 {
        self.inner.lock().unwrap().enqueued.len() as u32
    }

    /// Return all enqueued job types as a JSON array string.
    pub fn enqueued_types(&self) -> String {
        let inner = self.inner.lock().unwrap();
        let types: Vec<&str> = inner.enqueued.iter().map(|j| j.job_type.as_str()).collect();
        serde_json::to_string(&types).unwrap_or_else(|_| "[]".to_string())
    }

    /// Clear all enqueued and performed jobs.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.enqueued.clear();
        inner.performed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_assert() {
        let store = FakeStore::new();
        store.record_enqueue("email.send", "[]", None);
        store.record_enqueue("email.send", "[]", None);
        store.record_enqueue("report.gen", "[]", Some("reports".into()));

        // Direct state verification (JsValue-based assert_* methods panic on non-wasm)
        let inner = store.inner.lock().unwrap();
        assert!(inner.enqueued.iter().any(|j| j.job_type == "email.send"));
        assert!(inner.enqueued.iter().any(|j| j.job_type == "report.gen"));
        assert!(!inner.enqueued.iter().any(|j| j.job_type == "payment.process"));
        assert_eq!(inner.enqueued.iter().filter(|j| j.job_type == "email.send").count(), 2);
        drop(inner);
        assert_eq!(store.enqueued_count(), 3);
    }

    #[test]
    fn test_drain() {
        let store = FakeStore::new();
        store.record_enqueue("email.send", "[]", None);
        store.record_enqueue("report.gen", "[]", None);

        let processed = store.drain();
        assert_eq!(processed, 2);

        let inner = store.inner.lock().unwrap();
        assert!(inner.performed.iter().any(|j| j.job_type == "email.send" && j.state == "completed"));
        assert!(inner.performed.iter().any(|j| j.job_type == "report.gen" && j.state == "completed"));
    }

    #[test]
    fn test_clear() {
        let store = FakeStore::new();
        store.record_enqueue("email.send", "[]", None);
        assert_eq!(store.enqueued_count(), 1);
        store.clear();
        assert_eq!(store.enqueued_count(), 0);
    }

    #[test]
    fn test_assert_fails_correctly() {
        let store = FakeStore::new();
        // On non-wasm targets, JsValue methods panic, so we test the
        // internal logic by verifying the store state directly.
        let inner = store.inner.lock().unwrap();
        assert!(!inner.enqueued.iter().any(|j| j.job_type == "missing"));
        assert!(inner.performed.is_empty());
    }

    #[test]
    fn test_enqueued_types() {
        let store = FakeStore::new();
        store.record_enqueue("a", "[]", None);
        store.record_enqueue("b", "[]", None);
        let types = store.enqueued_types();
        assert!(types.contains("\"a\""));
        assert!(types.contains("\"b\""));
    }
}
