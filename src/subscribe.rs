//! Server-Sent Events (SSE) subscription for real-time job events in the browser.
//!
//! Uses the browser's native `EventSource` API via `web-sys` for efficient
//! real-time event streaming without polling.
//!
//! # Example
//!
//! ```js
//! import { SSESubscription } from '@openjobspec/wasm';
//!
//! const sub = new SSESubscription("http://localhost:8080", "queue:default");
//!
//! sub.on_event((event) => {
//!   console.log(`Event: ${event.type} — ${event.data}`);
//! });
//!
//! // Later: close the connection
//! sub.close();
//! ```

use js_sys::Function;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Closure type used for `MessageEvent` listeners registered on the stream.
type MessageListener = Closure<dyn FnMut(web_sys::MessageEvent)>;

/// SSE subscription for real-time OJS job events.
///
/// Wraps the browser's `EventSource` API to receive server-sent events
/// from the OJS event stream endpoint.
#[wasm_bindgen]
pub struct SSESubscription {
    event_source: web_sys::EventSource,
    _on_message: Option<MessageListener>,
    _on_error: Option<Closure<dyn FnMut(web_sys::Event)>>,
    _on_open: Option<Closure<dyn FnMut(web_sys::Event)>>,
    // Retained named-event listeners. Held here (rather than leaked via
    // `Closure::forget`) so they are freed when the subscription is dropped,
    // together with the `EventSource` they are attached to.
    _named_listeners: RefCell<Vec<MessageListener>>,
}

#[wasm_bindgen]
impl SSESubscription {
    /// Create a new SSE subscription.
    ///
    /// `url` is the OJS server base URL (e.g., "http://localhost:8080").
    /// `channel` is the SSE channel (e.g., `job:<id>`, `queue:<name>`).
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str, channel: &str) -> Result<SSESubscription, JsValue> {
        let stream_url = format!(
            "{}/ojs/v1/events/stream?channel={}",
            url.trim_end_matches('/'),
            js_sys::encode_uri_component(channel)
        );

        let event_source = web_sys::EventSource::new(&stream_url)?;

        Ok(SSESubscription {
            event_source,
            _on_message: None,
            _on_error: None,
            _on_open: None,
            _named_listeners: RefCell::new(Vec::new()),
        })
    }

    /// Register a callback for incoming events.
    ///
    /// The callback receives a JS object with `{ type, data, id }` fields.
    pub fn on_event(&mut self, callback: Function) {
        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &event.type_().into());
            let _ = js_sys::Reflect::set(&obj, &"data".into(), &event.data());
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &event.last_event_id().into());
            let _ = callback.call1(&JsValue::NULL, &obj);
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.event_source
            .set_onmessage(Some(closure.as_ref().unchecked_ref()));
        self._on_message = Some(closure);
    }

    /// Register a callback for specific event types.
    ///
    /// Use this to listen for named events like "job.state_changed".
    pub fn on_named_event(&self, event_type: &str, callback: Function) -> Result<(), JsValue> {
        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &event.type_().into());
            let _ = js_sys::Reflect::set(&obj, &"data".into(), &event.data());
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &event.last_event_id().into());
            let _ = callback.call1(&JsValue::NULL, &obj);
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.event_source
            .add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;

        // Retain the closure for the lifetime of the subscription instead of
        // leaking it with `Closure::forget`. When the subscription is dropped,
        // the closure and its `EventSource` are torn down together.
        self._named_listeners.borrow_mut().push(closure);
        Ok(())
    }

    /// Register a callback for connection errors.
    pub fn on_error(&mut self, callback: Function) {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let _ = callback.call0(&JsValue::NULL);
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.event_source
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        self._on_error = Some(closure);
    }

    /// Register a callback for when the connection opens.
    pub fn on_open(&mut self, callback: Function) {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let _ = callback.call0(&JsValue::NULL);
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.event_source
            .set_onopen(Some(closure.as_ref().unchecked_ref()));
        self._on_open = Some(closure);
    }

    /// Get the current connection state.
    ///
    /// Returns: 0 = CONNECTING, 1 = OPEN, 2 = CLOSED
    pub fn ready_state(&self) -> u16 {
        self.event_source.ready_state()
    }

    /// Close the SSE connection.
    pub fn close(&self) {
        self.event_source.close();
    }
}

/// Subscribe to events for a specific job.
///
/// Convenience function that creates an SSE subscription for `job:<id>`.
#[wasm_bindgen]
pub fn subscribe_job(url: &str, job_id: &str) -> Result<SSESubscription, JsValue> {
    SSESubscription::new(url, &format!("job:{}", job_id))
}

/// Subscribe to events for all jobs in a queue.
///
/// Convenience function that creates an SSE subscription for `queue:<name>`.
#[wasm_bindgen]
pub fn subscribe_queue(url: &str, queue_name: &str) -> Result<SSESubscription, JsValue> {
    SSESubscription::new(url, &format!("queue:{}", queue_name))
}
