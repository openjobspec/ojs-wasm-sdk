use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use ojs_wasm_sdk::OJSClient;

#[wasm_bindgen_test]
fn test_client_creation() {
    let client = OJSClient::new("http://localhost:8080");
    // Client should be created without panicking.
    drop(client);
}

#[wasm_bindgen_test]
fn test_client_trailing_slash() {
    let client = OJSClient::new("http://localhost:8080/");
    drop(client);
}
