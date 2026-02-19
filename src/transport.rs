use crate::error::{ErrorResponse, OjsWasmError, Result};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn build_request(method: &str, url: &str, body: Option<String>) -> Result<Request> {
    let opts = RequestInit::new();
    opts.set_method(method);

    let headers = Headers::new().map_err(OjsWasmError::from)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(OjsWasmError::from)?;
    opts.set_headers(&headers);

    if let Some(b) = body {
        let js_body = wasm_bindgen::JsValue::from_str(&b);
        opts.set_body(&js_body);
    }

    Request::new_with_str_and_init(url, &opts).map_err(OjsWasmError::from)
}

async fn execute(request: Request) -> Result<String> {
    let resp_value = JsFuture::from(window().fetch_with_request(&request))
        .await
        .map_err(OjsWasmError::from)?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| OjsWasmError::Transport("response is not a Response".into()))?;

    let text = JsFuture::from(
        resp.text().map_err(OjsWasmError::from)?,
    )
    .await
    .map_err(OjsWasmError::from)?;

    let body = text
        .as_string()
        .unwrap_or_default();

    if !resp.ok() {
        if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            return Err(OjsWasmError::Server(crate::error::ServerError {
                code: err_resp.error.code,
                message: err_resp.error.message,
                retryable: err_resp.error.retryable,
            }));
        }
        return Err(OjsWasmError::Transport(format!(
            "HTTP {}: {}",
            resp.status(),
            body
        )));
    }

    Ok(body)
}

/// Send a POST request with a JSON body.
pub async fn post(url: &str, body: &str) -> Result<String> {
    let request = build_request("POST", url, Some(body.to_string()))?;
    execute(request).await
}

/// Send a GET request.
pub async fn get(url: &str) -> Result<String> {
    let request = build_request("GET", url, None)?;
    execute(request).await
}

/// Send a DELETE request.
pub async fn delete(url: &str) -> Result<String> {
    let request = build_request("DELETE", url, None)?;
    execute(request).await
}
