//! Tests for workflow builder functions (chain, group, batch).

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::workflow::{batch, chain, group};

// ===========================================================================
// Helper to build a job spec JS object
// ===========================================================================

fn make_job(job_type: &str, args: &[&str]) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"type".into(), &JsValue::from_str(job_type)).unwrap();
    let arr = js_sys::Array::new();
    for a in args {
        arr.push(&JsValue::from_str(a));
    }
    js_sys::Reflect::set(&obj, &"args".into(), &arr).unwrap();
    obj.into()
}

fn make_jobs_array(specs: &[(&str, &[&str])]) -> JsValue {
    let arr = js_sys::Array::new();
    for (job_type, args) in specs {
        arr.push(&make_job(job_type, args));
    }
    arr.into()
}

// ===========================================================================
// Chain — basic
// ===========================================================================

#[wasm_bindgen_test]
fn chain_single_step() {
    let steps = make_jobs_array(&[("email.send", &["user@test.com"])]);
    let result = chain(steps).unwrap();

    let wf_type = js_sys::Reflect::get(&result, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "chain");

    let steps_val = js_sys::Reflect::get(&result, &"steps".into()).unwrap();
    let arr: js_sys::Array = steps_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
}

#[wasm_bindgen_test]
fn chain_preserves_step_fields() {
    let steps = make_jobs_array(&[("data.fetch", &["https://example.com"])]);
    let result = chain(steps).unwrap();

    let steps_val = js_sys::Reflect::get(&result, &"steps".into()).unwrap();
    let arr: js_sys::Array = steps_val.dyn_into().unwrap();
    let first = arr.get(0);
    let step_type = js_sys::Reflect::get(&first, &"type".into()).unwrap();
    assert_eq!(step_type.as_string().unwrap(), "data.fetch");

    let step_args = js_sys::Reflect::get(&first, &"args".into()).unwrap();
    let args_arr: js_sys::Array = step_args.dyn_into().unwrap();
    assert_eq!(args_arr.get(0).as_string().unwrap(), "https://example.com");
}

#[wasm_bindgen_test]
fn chain_five_steps() {
    let steps = make_jobs_array(&[
        ("step.1", &[]),
        ("step.2", &[]),
        ("step.3", &[]),
        ("step.4", &[]),
        ("step.5", &[]),
    ]);
    let result = chain(steps).unwrap();

    let steps_val = js_sys::Reflect::get(&result, &"steps".into()).unwrap();
    let arr: js_sys::Array = steps_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 5);
}

// ===========================================================================
// Chain — error cases
// ===========================================================================

#[wasm_bindgen_test]
fn chain_boolean_fails() {
    let result = chain(JsValue::TRUE);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn chain_number_fails() {
    let result = chain(JsValue::from(3.14));
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn chain_object_fails() {
    let obj = js_sys::Object::new();
    let result = chain(obj.into());
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn chain_error_message_for_empty() {
    let arr = js_sys::Array::new();
    let result = chain(arr.into());
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(msg.contains("at least one step"), "got: {}", msg);
}

#[wasm_bindgen_test]
fn chain_error_message_for_non_array() {
    let result = chain(JsValue::from_str("invalid"));
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(msg.contains("array"), "got: {}", msg);
}

// ===========================================================================
// Group — basic
// ===========================================================================

#[wasm_bindgen_test]
fn group_single_job() {
    let jobs = make_jobs_array(&[("export.csv", &["report_1"])]);
    let result = group(jobs).unwrap();

    let wf_type = js_sys::Reflect::get(&result, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "group");

    let jobs_val = js_sys::Reflect::get(&result, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
}

#[wasm_bindgen_test]
fn group_preserves_job_fields() {
    let jobs = make_jobs_array(&[("export.pdf", &["doc_123"])]);
    let result = group(jobs).unwrap();

    let jobs_val = js_sys::Reflect::get(&result, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_val.dyn_into().unwrap();
    let first = arr.get(0);
    let jt = js_sys::Reflect::get(&first, &"type".into()).unwrap();
    assert_eq!(jt.as_string().unwrap(), "export.pdf");
}

#[wasm_bindgen_test]
fn group_many_jobs() {
    let jobs = make_jobs_array(&[
        ("resize.small", &["img.png"]),
        ("resize.medium", &["img.png"]),
        ("resize.large", &["img.png"]),
        ("resize.thumb", &["img.png"]),
    ]);
    let result = group(jobs).unwrap();

    let jobs_val = js_sys::Reflect::get(&result, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 4);
}

// ===========================================================================
// Group — error cases
// ===========================================================================

#[wasm_bindgen_test]
fn group_null_fails() {
    let result = group(JsValue::NULL);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn group_undefined_fails() {
    let result = group(JsValue::UNDEFINED);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn group_string_fails() {
    let result = group(JsValue::from_str("not array"));
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn group_error_message_for_empty() {
    let arr = js_sys::Array::new();
    let result = group(arr.into());
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(msg.contains("at least one job"), "got: {}", msg);
}

#[wasm_bindgen_test]
fn group_error_message_for_non_array() {
    let result = group(JsValue::from(99));
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(msg.contains("array"), "got: {}", msg);
}

// ===========================================================================
// Batch — basic
// ===========================================================================

fn make_callbacks(keys: &[&str]) -> JsValue {
    let callbacks = js_sys::Object::new();
    for key in keys {
        let cb = js_sys::Object::new();
        js_sys::Reflect::set(&cb, &"type".into(), &JsValue::from_str(&format!("cb.{}", key)))
            .unwrap();
        js_sys::Reflect::set(&cb, &"args".into(), &js_sys::Array::new()).unwrap();
        js_sys::Reflect::set(&callbacks, &JsValue::from_str(key), &cb).unwrap();
    }
    callbacks.into()
}

#[wasm_bindgen_test]
fn batch_with_on_complete_only() {
    let jobs = make_jobs_array(&[("email.send", &["a@b.com"])]);
    let callbacks = make_callbacks(&["on_complete"]);

    let result = batch(jobs, callbacks).unwrap();
    let wf_type = js_sys::Reflect::get(&result, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "batch");
}

#[wasm_bindgen_test]
fn batch_with_on_success_only() {
    let jobs = make_jobs_array(&[("email.send", &["a@b.com"])]);
    let callbacks = make_callbacks(&["on_success"]);

    let result = batch(jobs, callbacks);
    assert!(result.is_ok(), "batch should accept on_success only");
}

#[wasm_bindgen_test]
fn batch_with_on_failure_only() {
    let jobs = make_jobs_array(&[("email.send", &["a@b.com"])]);
    let callbacks = make_callbacks(&["on_failure"]);

    let result = batch(jobs, callbacks);
    assert!(result.is_ok(), "batch should accept on_failure only");
}

#[wasm_bindgen_test]
fn batch_result_structure() {
    let jobs = make_jobs_array(&[("a", &[]), ("b", &[])]);
    let callbacks = make_callbacks(&["on_complete", "on_failure"]);

    let result = batch(jobs, callbacks).unwrap();

    // Verify type
    let wf_type = js_sys::Reflect::get(&result, &"type".into()).unwrap();
    assert_eq!(wf_type.as_string().unwrap(), "batch");

    // Verify jobs array
    let jobs_val = js_sys::Reflect::get(&result, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 2);

    // Verify callbacks object
    let cb_val = js_sys::Reflect::get(&result, &"callbacks".into()).unwrap();
    assert!(js_sys::Reflect::has(&cb_val, &"on_complete".into()).unwrap());
    assert!(js_sys::Reflect::has(&cb_val, &"on_failure".into()).unwrap());
    assert!(!js_sys::Reflect::has(&cb_val, &"on_success".into()).unwrap());
}

#[wasm_bindgen_test]
fn batch_many_jobs_single_callback() {
    let specs: Vec<(&str, &[&str])> = (0..10)
        .map(|i| {
            // Use leaked strings to get 'static lifetime
            let s: &'static str = Box::leak(format!("job.{}", i).into_boxed_str());
            (s, &[] as &[&str])
        })
        .collect();
    let jobs = make_jobs_array(&specs);
    let callbacks = make_callbacks(&["on_complete"]);

    let result = batch(jobs, callbacks).unwrap();
    let jobs_val = js_sys::Reflect::get(&result, &"jobs".into()).unwrap();
    let arr: js_sys::Array = jobs_val.dyn_into().unwrap();
    assert_eq!(arr.length(), 10);
}

// ===========================================================================
// Batch — error cases
// ===========================================================================

#[wasm_bindgen_test]
fn batch_null_jobs_fails() {
    let callbacks = make_callbacks(&["on_complete"]);
    let result = batch(JsValue::NULL, callbacks);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn batch_undefined_jobs_fails() {
    let callbacks = make_callbacks(&["on_complete"]);
    let result = batch(JsValue::UNDEFINED, callbacks);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn batch_null_callbacks_fails() {
    let jobs = make_jobs_array(&[("test", &[])]);
    let result = batch(jobs, JsValue::NULL);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn batch_undefined_callbacks_fails() {
    let jobs = make_jobs_array(&[("test", &[])]);
    let result = batch(jobs, JsValue::UNDEFINED);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn batch_number_callbacks_fails() {
    let jobs = make_jobs_array(&[("test", &[])]);
    let result = batch(jobs, JsValue::from(123));
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn batch_error_no_callbacks_message() {
    let jobs = make_jobs_array(&[("test", &[])]);
    let callbacks = js_sys::Object::new();
    let result = batch(jobs, callbacks.into());
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(
        msg.contains("at least one callback"),
        "got: {}",
        msg
    );
}

#[wasm_bindgen_test]
fn batch_error_empty_jobs_message() {
    let arr = js_sys::Array::new();
    let callbacks = make_callbacks(&["on_complete"]);
    let result = batch(arr.into(), callbacks);
    let err = result.unwrap_err();
    let msg = err.as_string().unwrap();
    assert!(msg.contains("at least one job"), "got: {}", msg);
}

#[wasm_bindgen_test]
fn batch_callbacks_with_unrecognized_keys_and_no_valid() {
    let jobs = make_jobs_array(&[("test", &[])]);
    let callbacks = js_sys::Object::new();
    js_sys::Reflect::set(
        &callbacks,
        &"on_unknown".into(),
        &JsValue::from_str("ignored"),
    )
    .unwrap();

    let result = batch(jobs, callbacks.into());
    assert!(
        result.is_err(),
        "batch should fail when no valid callback keys present"
    );
}
