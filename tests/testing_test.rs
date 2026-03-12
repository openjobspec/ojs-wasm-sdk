//! Tests for the testing (fake store) module.

use ojs_wasm_sdk::testing::FakeStore;

#[test]
fn test_fake_store_basic_operations() {
    let store = FakeStore::new();

    // Record some enqueues
    let id1 = store.record_enqueue("email.send", r#"["user@example.com"]"#, None);
    let id2 = store.record_enqueue("report.gen", r#"[42]"#, Some("reports".into()));

    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert_ne!(id1, id2);

    assert_eq!(store.enqueued_count(), 2);
}

#[test]
fn test_fake_store_drain() {
    let store = FakeStore::new();
    store.record_enqueue("task.a", "[]", None);
    store.record_enqueue("task.b", "[]", None);
    store.record_enqueue("task.c", "[]", None);

    let processed = store.drain();
    assert_eq!(processed, 3);

    // Draining again should process 0 (already completed)
    let again = store.drain();
    assert_eq!(again, 0);
}

#[test]
fn test_fake_store_clear() {
    let store = FakeStore::new();
    store.record_enqueue("task.a", "[]", None);
    store.record_enqueue("task.b", "[]", None);
    assert_eq!(store.enqueued_count(), 2);

    store.clear();
    assert_eq!(store.enqueued_count(), 0);
}

#[test]
fn test_fake_store_enqueued_types() {
    let store = FakeStore::new();
    store.record_enqueue("email.send", "[]", None);
    store.record_enqueue("sms.send", "[]", None);
    store.record_enqueue("email.send", "[]", None);

    let types_json = store.enqueued_types();
    assert!(types_json.contains("\"email.send\""));
    assert!(types_json.contains("\"sms.send\""));
}

#[test]
fn test_fake_store_unique_ids() {
    let store = FakeStore::new();
    let mut ids = Vec::new();
    for _ in 0..100 {
        ids.push(store.record_enqueue("task", "[]", None));
    }

    // All IDs should be unique
    let unique_count = {
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    assert_eq!(unique_count, 100);
}

#[test]
fn test_fake_store_custom_queue() {
    let store = FakeStore::new();
    store.record_enqueue("task.a", "[]", Some("high-priority".into()));
    store.record_enqueue("task.b", "[]", Some("low-priority".into()));
    assert_eq!(store.enqueued_count(), 2);
}
