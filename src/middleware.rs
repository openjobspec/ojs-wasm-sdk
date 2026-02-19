//! # Middleware Support
//!
//! Provides a middleware chain for the WASM SDK client. Middleware functions
//! can intercept requests before they are sent and responses after they arrive.
//!
//! # Example
//!
//! ```js
//! import init, { OJSClient } from '@openjobspec/wasm';
//!
//! const client = new OJSClient("http://localhost:8080");
//!
//! // Add auth middleware
//! client.use_middleware("auth", (req) => {
//!   req.headers["Authorization"] = "Bearer my-api-key";
//!   return req;
//! });
//!
//! // Add logging middleware
//! client.use_middleware("logger", (req) => {
//!   console.log(`[OJS] ${req.method} ${req.url}`);
//!   return req;
//! });
//! ```

use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::prelude::*;

/// MiddlewareChain holds registered middleware functions.
#[wasm_bindgen]
pub struct MiddlewareChain {
    middlewares: Vec<(String, Function)>,
}

#[wasm_bindgen]
impl MiddlewareChain {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware function with a name.
    pub fn add(&mut self, name: &str, handler: Function) {
        self.middlewares.push((name.to_string(), handler));
    }

    /// Remove a middleware by name.
    pub fn remove(&mut self, name: &str) {
        self.middlewares.retain(|(n, _)| n != name);
    }

    /// List registered middleware names.
    pub fn list(&self) -> JsValue {
        let arr = Array::new();
        for (name, _) in &self.middlewares {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    /// Apply all middleware to a request object.
    /// The request object has { method, url, headers, body } fields.
    pub fn apply(&self, request: JsValue) -> Result<JsValue, JsValue> {
        let mut req = request;
        for (name, handler) in &self.middlewares {
            let result = handler.call1(&JsValue::NULL, &req);
            match result {
                Ok(modified) => {
                    if modified.is_undefined() || modified.is_null() {
                        return Err(JsValue::from_str(&format!(
                            "middleware '{}' returned null/undefined — must return the request object",
                            name
                        )));
                    }
                    req = modified;
                }
                Err(e) => {
                    return Err(JsValue::from_str(&format!(
                        "middleware '{}' threw: {:?}",
                        name, e
                    )));
                }
            }
        }
        Ok(req)
    }
}

/// Create a request object for middleware processing.
#[wasm_bindgen]
pub fn create_request(method: &str, url: &str, body: JsValue) -> Result<JsValue, JsValue> {
    let obj = Object::new();
    Reflect::set(&obj, &"method".into(), &JsValue::from_str(method))?;
    Reflect::set(&obj, &"url".into(), &JsValue::from_str(url))?;
    Reflect::set(&obj, &"headers".into(), &Object::new())?;
    Reflect::set(&obj, &"body".into(), &body)?;
    Ok(obj.into())
}
