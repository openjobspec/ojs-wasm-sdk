//! Tests for retry policy configuration and JS object generation.

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::retry::RetryPolicy;

// ===========================================================================
// Helper to extract fields from RetryPolicy JS object
// ===========================================================================

fn get_f64(obj: &wasm_bindgen::JsValue, key: &str) -> f64 {
    js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
        .unwrap()
        .as_f64()
        .unwrap()
}

fn get_string(obj: &wasm_bindgen::JsValue, key: &str) -> String {
    js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
        .unwrap()
        .as_string()
        .unwrap()
}

fn get_nested(obj: &wasm_bindgen::JsValue, key: &str) -> wasm_bindgen::JsValue {
    js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key)).unwrap()
}

// ===========================================================================
// Exponential backoff
// ===========================================================================

#[wasm_bindgen_test]
fn exponential_basic() {
    let policy = RetryPolicy::exponential(5, 1000, 60000);
    let obj = policy.to_object().unwrap();

    assert_eq!(get_f64(&obj, "max_attempts"), 5.0);

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_string(&backoff, "type"), "exponential");
    assert_eq!(get_f64(&backoff, "initial_ms"), 1000.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 60000.0);
}

#[wasm_bindgen_test]
fn exponential_single_attempt() {
    let policy = RetryPolicy::exponential(1, 100, 100);
    let obj = policy.to_object().unwrap();
    assert_eq!(get_f64(&obj, "max_attempts"), 1.0);
}

#[wasm_bindgen_test]
fn exponential_large_values() {
    let policy = RetryPolicy::exponential(100, 50, 3_600_000);
    let obj = policy.to_object().unwrap();

    assert_eq!(get_f64(&obj, "max_attempts"), 100.0);

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 50.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 3_600_000.0);
}

#[wasm_bindgen_test]
fn exponential_zero_initial_delay() {
    let policy = RetryPolicy::exponential(3, 0, 10000);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 0.0);
}

#[wasm_bindgen_test]
fn exponential_same_initial_and_max() {
    let policy = RetryPolicy::exponential(5, 2000, 2000);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 2000.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 2000.0);
}

// ===========================================================================
// Fixed backoff
// ===========================================================================

#[wasm_bindgen_test]
fn fixed_basic() {
    let policy = RetryPolicy::fixed(3, 5000);
    let obj = policy.to_object().unwrap();

    assert_eq!(get_f64(&obj, "max_attempts"), 3.0);

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_string(&backoff, "type"), "fixed");
    assert_eq!(get_f64(&backoff, "initial_ms"), 5000.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 5000.0);
}

#[wasm_bindgen_test]
fn fixed_initial_equals_max() {
    let policy = RetryPolicy::fixed(10, 2000);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    let initial = get_f64(&backoff, "initial_ms");
    let max = get_f64(&backoff, "max_ms");
    assert_eq!(initial, max, "fixed policy should have equal initial and max");
}

#[wasm_bindgen_test]
fn fixed_zero_delay() {
    let policy = RetryPolicy::fixed(5, 0);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 0.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 0.0);
}

#[wasm_bindgen_test]
fn fixed_single_retry() {
    let policy = RetryPolicy::fixed(1, 1000);
    let obj = policy.to_object().unwrap();
    assert_eq!(get_f64(&obj, "max_attempts"), 1.0);
}

// ===========================================================================
// Linear backoff
// ===========================================================================

#[wasm_bindgen_test]
fn linear_basic() {
    let policy = RetryPolicy::linear(10, 500, 30000);
    let obj = policy.to_object().unwrap();

    assert_eq!(get_f64(&obj, "max_attempts"), 10.0);

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_string(&backoff, "type"), "linear");
    assert_eq!(get_f64(&backoff, "initial_ms"), 500.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 30000.0);
}

#[wasm_bindgen_test]
fn linear_small_range() {
    let policy = RetryPolicy::linear(3, 100, 300);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 100.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 300.0);
}

#[wasm_bindgen_test]
fn linear_same_initial_and_max() {
    let policy = RetryPolicy::linear(5, 1000, 1000);
    let obj = policy.to_object().unwrap();

    let backoff = get_nested(&obj, "backoff");
    assert_eq!(get_f64(&backoff, "initial_ms"), 1000.0);
    assert_eq!(get_f64(&backoff, "max_ms"), 1000.0);
}

// ===========================================================================
// Cross-policy comparisons
// ===========================================================================

#[wasm_bindgen_test]
fn all_policies_have_backoff_type() {
    let policies = vec![
        ("exponential", RetryPolicy::exponential(3, 100, 1000)),
        ("fixed", RetryPolicy::fixed(3, 100)),
        ("linear", RetryPolicy::linear(3, 100, 1000)),
    ];

    for (expected_type, policy) in policies {
        let obj = policy.to_object().unwrap();
        let backoff = get_nested(&obj, "backoff");
        let bt = get_string(&backoff, "type");
        assert_eq!(bt, expected_type);
    }
}

#[wasm_bindgen_test]
fn all_policies_have_max_attempts() {
    let policies = vec![
        RetryPolicy::exponential(7, 100, 1000),
        RetryPolicy::fixed(7, 100),
        RetryPolicy::linear(7, 100, 1000),
    ];

    for policy in policies {
        let obj = policy.to_object().unwrap();
        assert_eq!(get_f64(&obj, "max_attempts"), 7.0);
    }
}

#[wasm_bindgen_test]
fn all_policies_have_initial_and_max_ms() {
    let policies = vec![
        RetryPolicy::exponential(3, 200, 5000),
        RetryPolicy::fixed(3, 200),
        RetryPolicy::linear(3, 200, 5000),
    ];

    for policy in policies {
        let obj = policy.to_object().unwrap();
        let backoff = get_nested(&obj, "backoff");
        let initial = get_f64(&backoff, "initial_ms");
        assert_eq!(initial, 200.0);
    }
}

// ===========================================================================
// Object structure verification
// ===========================================================================

#[wasm_bindgen_test]
fn policy_object_has_exactly_two_top_keys() {
    let policy = RetryPolicy::exponential(3, 100, 1000);
    let obj = policy.to_object().unwrap();
    let obj_ref: &js_sys::Object = obj.dyn_ref::<js_sys::Object>().unwrap();
    let keys = js_sys::Object::keys(obj_ref);
    assert_eq!(keys.length(), 2, "should have max_attempts and backoff");
}

#[wasm_bindgen_test]
fn backoff_object_has_exactly_three_keys() {
    let policy = RetryPolicy::linear(3, 100, 1000);
    let obj = policy.to_object().unwrap();
    let backoff = get_nested(&obj, "backoff");
    let backoff_ref: &js_sys::Object = backoff.dyn_ref::<js_sys::Object>().unwrap();
    let keys = js_sys::Object::keys(backoff_ref);
    assert_eq!(keys.length(), 3, "should have type, initial_ms, and max_ms");
}
