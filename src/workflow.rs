//! # Workflow Builders
//!
//! JavaScript-callable builder functions for the three OJS workflow primitives:
//!
//! - **chain** -- Sequential execution. Each step runs after the previous completes.
//! - **group** -- Parallel fan-out. All jobs run concurrently.
//! - **batch** -- Parallel with callbacks. Like a group but fires callbacks
//!   based on collective outcome.
//!
//! These functions return plain JS objects that can be passed directly to
//! `client.workflow()`.
//!
//! # Example
//!
//! ```js
//! import init, { OJSClient, chain, group, batch } from '@openjobspec/wasm';
//!
//! await init();
//! const client = new OJSClient("http://localhost:8080");
//!
//! // Sequential pipeline
//! await client.workflow(chain([
//!   { type: "data.fetch", args: ["https://..."] },
//!   { type: "data.transform", args: ["csv"] },
//!   { type: "data.load", args: ["warehouse"] },
//! ]));
//!
//! // Parallel fan-out
//! await client.workflow(group([
//!   { type: "export.csv", args: ["rpt_123"] },
//!   { type: "export.pdf", args: ["rpt_123"] },
//! ]));
//!
//! // Batch with callbacks
//! await client.workflow(batch(
//!   [
//!     { type: "email.send", args: ["a@b.com"] },
//!     { type: "email.send", args: ["c@d.com"] },
//!   ],
//!   { on_complete: { type: "batch.report", args: [] } },
//! ));
//! ```

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Create a **chain** workflow (sequential execution).
///
/// `steps` is a JS array of job specs (objects with `type` and `args`).
/// Each step runs after the previous one completes.
///
/// Returns a workflow definition object suitable for `client.workflow()`.
#[wasm_bindgen]
pub fn chain(steps: JsValue) -> std::result::Result<JsValue, JsValue> {
    let arr: Array = steps
        .dyn_into()
        .map_err(|_| JsValue::from_str("chain() expects an array of steps"))?;

    if arr.length() == 0 {
        return Err(JsValue::from_str(
            "a chain must contain at least one step",
        ));
    }

    let obj = Object::new();
    Reflect::set(&obj, &"type".into(), &"chain".into())?;
    Reflect::set(&obj, &"steps".into(), &arr)?;
    Ok(obj.into())
}

/// Create a **group** workflow (parallel execution).
///
/// `jobs` is a JS array of job specs. All jobs execute concurrently.
///
/// Returns a workflow definition object suitable for `client.workflow()`.
#[wasm_bindgen]
pub fn group(jobs: JsValue) -> std::result::Result<JsValue, JsValue> {
    let arr: Array = jobs
        .dyn_into()
        .map_err(|_| JsValue::from_str("group() expects an array of jobs"))?;

    if arr.length() == 0 {
        return Err(JsValue::from_str(
            "a group must contain at least one job",
        ));
    }

    let obj = Object::new();
    Reflect::set(&obj, &"type".into(), &"group".into())?;
    Reflect::set(&obj, &"jobs".into(), &arr)?;
    Ok(obj.into())
}

/// Create a **batch** workflow (parallel with callbacks).
///
/// `jobs` is a JS array of job specs. `callbacks` is an object with optional
/// keys `on_complete`, `on_success`, and `on_failure`, each mapping to a
/// job spec that fires when the batch reaches that state.
///
/// Returns a workflow definition object suitable for `client.workflow()`.
#[wasm_bindgen]
pub fn batch(jobs: JsValue, callbacks: JsValue) -> std::result::Result<JsValue, JsValue> {
    let arr: Array = jobs
        .dyn_into()
        .map_err(|_| JsValue::from_str("batch() expects an array of jobs"))?;

    if arr.length() == 0 {
        return Err(JsValue::from_str(
            "a batch must contain at least one job",
        ));
    }

    // Validate that callbacks has at least one key
    let cb_obj: Object = callbacks
        .dyn_into()
        .map_err(|_| JsValue::from_str("batch() expects a callbacks object"))?;

    let has_on_complete = Reflect::has(&cb_obj, &"on_complete".into()).unwrap_or(false);
    let has_on_success = Reflect::has(&cb_obj, &"on_success".into()).unwrap_or(false);
    let has_on_failure = Reflect::has(&cb_obj, &"on_failure".into()).unwrap_or(false);

    if !has_on_complete && !has_on_success && !has_on_failure {
        return Err(JsValue::from_str(
            "a batch must have at least one callback (on_complete, on_success, or on_failure)",
        ));
    }

    let obj = Object::new();
    Reflect::set(&obj, &"type".into(), &"batch".into())?;
    Reflect::set(&obj, &"jobs".into(), &arr)?;
    Reflect::set(&obj, &"callbacks".into(), &cb_obj)?;
    Ok(obj.into())
}
