//! Tests for middleware chain operations and request creation.

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::middleware::{create_request, MiddlewareChain};

// ===========================================================================
// MiddlewareChain — construction and listing
// ===========================================================================

#[wasm_bindgen_test]
fn middleware_new_is_empty() {
    let mw = MiddlewareChain::new();
    let names = mw.list();
    let arr: js_sys::Array = names.dyn_into().unwrap();
    assert_eq!(arr.length(), 0);
}

#[wasm_bindgen_test]
fn middleware_add_single() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("auth", f);

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
    assert_eq!(arr.get(0).as_string().unwrap(), "auth");
}

#[wasm_bindgen_test]
fn middleware_add_multiple_preserves_order() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("first", f.clone());
    mw.add("second", f.clone());
    mw.add("third", f);

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 3);
    assert_eq!(arr.get(0).as_string().unwrap(), "first");
    assert_eq!(arr.get(1).as_string().unwrap(), "second");
    assert_eq!(arr.get(2).as_string().unwrap(), "third");
}

#[wasm_bindgen_test]
fn middleware_add_duplicate_name() {
    let mut mw = MiddlewareChain::new();
    let f1 = js_sys::Function::new_with_args("req", "return req");
    let f2 = js_sys::Function::new_with_args("req", "return req");
    mw.add("same-name", f1);
    mw.add("same-name", f2);

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 2, "duplicate names are allowed");
}

// ===========================================================================
// MiddlewareChain — removal
// ===========================================================================

#[wasm_bindgen_test]
fn middleware_remove_first() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("a", f.clone());
    mw.add("b", f.clone());
    mw.add("c", f);
    mw.remove("a");

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 2);
    assert_eq!(arr.get(0).as_string().unwrap(), "b");
    assert_eq!(arr.get(1).as_string().unwrap(), "c");
}

#[wasm_bindgen_test]
fn middleware_remove_middle() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("a", f.clone());
    mw.add("b", f.clone());
    mw.add("c", f);
    mw.remove("b");

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 2);
    assert_eq!(arr.get(0).as_string().unwrap(), "a");
    assert_eq!(arr.get(1).as_string().unwrap(), "c");
}

#[wasm_bindgen_test]
fn middleware_remove_last() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("a", f.clone());
    mw.add("b", f);
    mw.remove("b");

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
    assert_eq!(arr.get(0).as_string().unwrap(), "a");
}

#[wasm_bindgen_test]
fn middleware_remove_nonexistent_is_noop() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("exists", f);
    mw.remove("does-not-exist");

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 1);
}

#[wasm_bindgen_test]
fn middleware_remove_all() {
    let mut mw = MiddlewareChain::new();
    let f = js_sys::Function::new_with_args("req", "return req");
    mw.add("x", f.clone());
    mw.add("y", f);
    mw.remove("x");
    mw.remove("y");

    let arr: js_sys::Array = mw.list().dyn_into().unwrap();
    assert_eq!(arr.length(), 0);
}

// ===========================================================================
// MiddlewareChain — apply
// ===========================================================================

#[wasm_bindgen_test]
fn middleware_apply_empty_chain() {
    let mw = MiddlewareChain::new();
    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req).unwrap();

    let method = js_sys::Reflect::get(&result, &"method".into()).unwrap();
    assert_eq!(method.as_string().unwrap(), "GET");
}

#[wasm_bindgen_test]
fn middleware_apply_modifies_headers() {
    let mut mw = MiddlewareChain::new();
    let add_auth = js_sys::Function::new_with_args(
        "req",
        "req.headers['Authorization'] = 'Bearer token123'; return req;",
    );
    mw.add("auth", add_auth);

    let req = create_request("POST", "http://localhost/jobs", JsValue::NULL).unwrap();
    let result = mw.apply(req).unwrap();

    let headers = js_sys::Reflect::get(&result, &"headers".into()).unwrap();
    let auth = js_sys::Reflect::get(&headers, &"Authorization".into()).unwrap();
    assert_eq!(auth.as_string().unwrap(), "Bearer token123");
}

#[wasm_bindgen_test]
fn middleware_apply_chain_order_matters() {
    let mut mw = MiddlewareChain::new();

    let set_a = js_sys::Function::new_with_args(
        "req",
        "req.headers['X-Order'] = 'A'; return req;",
    );
    let set_b = js_sys::Function::new_with_args(
        "req",
        "req.headers['X-Order'] = 'B'; return req;",
    );

    mw.add("set-a", set_a);
    mw.add("set-b", set_b);

    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req).unwrap();

    let headers = js_sys::Reflect::get(&result, &"headers".into()).unwrap();
    let order = js_sys::Reflect::get(&headers, &"X-Order".into()).unwrap();
    assert_eq!(order.as_string().unwrap(), "B", "last middleware should win");
}

#[wasm_bindgen_test]
fn middleware_apply_accumulates_headers() {
    let mut mw = MiddlewareChain::new();

    let add_x = js_sys::Function::new_with_args(
        "req",
        "req.headers['X-First'] = '1'; return req;",
    );
    let add_y = js_sys::Function::new_with_args(
        "req",
        "req.headers['X-Second'] = '2'; return req;",
    );

    mw.add("first", add_x);
    mw.add("second", add_y);

    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req).unwrap();

    let headers = js_sys::Reflect::get(&result, &"headers".into()).unwrap();
    let h1 = js_sys::Reflect::get(&headers, &"X-First".into()).unwrap();
    let h2 = js_sys::Reflect::get(&headers, &"X-Second".into()).unwrap();
    assert_eq!(h1.as_string().unwrap(), "1");
    assert_eq!(h2.as_string().unwrap(), "2");
}

#[wasm_bindgen_test]
fn middleware_apply_null_return_fails() {
    let mut mw = MiddlewareChain::new();
    let bad = js_sys::Function::new_with_args("req", "return null;");
    mw.add("bad-mw", bad);

    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().as_string().unwrap();
    assert!(err_msg.contains("bad-mw"), "error should name the middleware");
    assert!(err_msg.contains("null/undefined"));
}

#[wasm_bindgen_test]
fn middleware_apply_undefined_return_fails() {
    let mut mw = MiddlewareChain::new();
    let bad = js_sys::Function::new_with_args("req", "");
    mw.add("noop-mw", bad);

    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn middleware_apply_throwing_function_fails() {
    let mut mw = MiddlewareChain::new();
    let throws = js_sys::Function::new_with_args("req", "throw new Error('boom');");
    mw.add("error-mw", throws);

    let req = create_request("GET", "http://localhost/test", JsValue::NULL).unwrap();
    let result = mw.apply(req);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().as_string().unwrap();
    assert!(err_msg.contains("error-mw"), "error should name the middleware");
}

// ===========================================================================
// create_request
// ===========================================================================

#[wasm_bindgen_test]
fn create_request_get() {
    let req = create_request("GET", "http://example.com/health", JsValue::NULL).unwrap();

    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    let url = js_sys::Reflect::get(&req, &"url".into()).unwrap();
    let body = js_sys::Reflect::get(&req, &"body".into()).unwrap();
    let headers = js_sys::Reflect::get(&req, &"headers".into()).unwrap();

    assert_eq!(method.as_string().unwrap(), "GET");
    assert_eq!(url.as_string().unwrap(), "http://example.com/health");
    assert!(body.is_null());
    assert!(headers.is_object());
}

#[wasm_bindgen_test]
fn create_request_post_with_body() {
    let body = JsValue::from_str(r#"{"type":"email.send","args":[]}"#);
    let req = create_request("POST", "http://example.com/jobs", body).unwrap();

    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    let req_body = js_sys::Reflect::get(&req, &"body".into()).unwrap();

    assert_eq!(method.as_string().unwrap(), "POST");
    assert!(req_body.is_string());
}

#[wasm_bindgen_test]
fn create_request_delete() {
    let req = create_request("DELETE", "http://example.com/jobs/123", JsValue::NULL).unwrap();
    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    assert_eq!(method.as_string().unwrap(), "DELETE");
}

#[wasm_bindgen_test]
fn create_request_put() {
    let req = create_request("PUT", "http://example.com/jobs/123", JsValue::NULL).unwrap();
    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    assert_eq!(method.as_string().unwrap(), "PUT");
}

#[wasm_bindgen_test]
fn create_request_patch() {
    let req = create_request("PATCH", "http://example.com/queues/default", JsValue::NULL).unwrap();
    let method = js_sys::Reflect::get(&req, &"method".into()).unwrap();
    assert_eq!(method.as_string().unwrap(), "PATCH");
}

#[wasm_bindgen_test]
fn create_request_headers_is_empty_object() {
    let req = create_request("GET", "http://example.com/", JsValue::NULL).unwrap();
    let headers = js_sys::Reflect::get(&req, &"headers".into()).unwrap();

    let keys = js_sys::Object::keys(&headers.dyn_into::<js_sys::Object>().unwrap());
    assert_eq!(keys.length(), 0, "headers should start empty");
}

#[wasm_bindgen_test]
fn create_request_url_with_query_params() {
    let url = "http://example.com/jobs?state=active&limit=10";
    let req = create_request("GET", url, JsValue::NULL).unwrap();
    let req_url = js_sys::Reflect::get(&req, &"url".into()).unwrap();
    assert_eq!(req_url.as_string().unwrap(), url);
}
