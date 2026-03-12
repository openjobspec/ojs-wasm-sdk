//! Schema validation for OJS job definitions in the browser.
//!
//! Provides client-side validation of job types and arguments against
//! registered schemas, without requiring a server round-trip.
//!
//! # Example
//!
//! ```js
//! import { SchemaValidator } from '@openjobspec/wasm';
//!
//! const validator = new SchemaValidator();
//! validator.register("email.send", JSON.stringify({
//!   type: "object",
//!   properties: { to: { type: "string" } },
//!   required: ["to"]
//! }));
//!
//! const result = validator.validate("email.send", JSON.stringify({ to: "user@example.com" }));
//! console.log(result.valid); // true
//! ```

use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Client-side schema validator for job arguments.
#[wasm_bindgen]
pub struct SchemaValidator {
    schemas: HashMap<String, serde_json::Value>,
}

#[wasm_bindgen]
impl SchemaValidator {
    /// Create a new schema validator.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register a JSON Schema for a job type.
    ///
    /// `schema_json` should be a valid JSON Schema string.
    pub fn register(&mut self, job_type: &str, schema_json: &str) -> Result<(), JsValue> {
        let schema: serde_json::Value = serde_json::from_str(schema_json)
            .map_err(|e| JsValue::from_str(&format!("invalid schema JSON: {}", e)))?;
        self.schemas.insert(job_type.to_string(), schema);
        Ok(())
    }

    /// Unregister the schema for a job type.
    pub fn unregister(&mut self, job_type: &str) -> bool {
        self.schemas.remove(job_type).is_some()
    }

    /// Check if a schema is registered for the given job type.
    pub fn has_schema(&self, job_type: &str) -> bool {
        self.schemas.contains_key(job_type)
    }

    /// List all registered job types as a JSON array string.
    pub fn registered_types(&self) -> String {
        let types: Vec<&str> = self.schemas.keys().map(|s| s.as_str()).collect();
        serde_json::to_string(&types).unwrap_or_else(|_| "[]".to_string())
    }

    /// Validate job arguments against the registered schema.
    ///
    /// Returns a JS object with `{ valid: boolean, errors: string[] }`.
    /// If no schema is registered for the job type, returns `{ valid: true, errors: [] }`.
    pub fn validate(&self, job_type: &str, args_json: &str) -> Result<JsValue, JsValue> {
        let schema = match self.schemas.get(job_type) {
            Some(s) => s,
            None => {
                return make_result(true, vec![]);
            }
        };

        let args: serde_json::Value = serde_json::from_str(args_json)
            .map_err(|e| JsValue::from_str(&format!("invalid args JSON: {}", e)))?;

        let errors = validate_value(&args, schema, "");
        make_result(errors.is_empty(), errors)
    }
}

fn make_result(valid: bool, errors: Vec<String>) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"valid".into(), &JsValue::from_bool(valid))?;

    let err_arr = js_sys::Array::new();
    for e in &errors {
        err_arr.push(&JsValue::from_str(e));
    }
    js_sys::Reflect::set(&obj, &"errors".into(), &err_arr)?;

    Ok(obj.into())
}

/// Basic JSON Schema validation (type checking, required fields, enum).
fn validate_value(value: &serde_json::Value, schema: &serde_json::Value, path: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
        let actual_type = json_type_name(value);
        if actual_type != expected_type {
            errors.push(format!(
                "{}: expected type '{}', got '{}'",
                if path.is_empty() { "$" } else { path },
                expected_type,
                actual_type
            ));
            return errors;
        }
    }

    if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
        if !enum_values.contains(value) {
            errors.push(format!(
                "{}: value not in enum {:?}",
                if path.is_empty() { "$" } else { path },
                enum_values
            ));
        }
    }

    if let (Some(obj), Some(props)) = (
        value.as_object(),
        schema.get("properties").and_then(|p| p.as_object()),
    ) {
        for (key, prop_schema) in props {
            let field_path = if path.is_empty() {
                format!("$.{}", key)
            } else {
                format!("{}.{}", path, key)
            };
            if let Some(field_value) = obj.get(key) {
                errors.extend(validate_value(field_value, prop_schema, &field_path));
            }
        }

        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(field_name) = req.as_str() {
                    if !obj.contains_key(field_name) {
                        errors.push(format!(
                            "{}: missing required field '{}'",
                            if path.is_empty() { "$" } else { path },
                            field_name
                        ));
                    }
                }
            }
        }
    }

    if let (Some(arr), Some(item_schema)) = (value.as_array(), schema.get("items")) {
        for (i, item) in arr.iter().enumerate() {
            let item_path = format!("{}[{}]", if path.is_empty() { "$" } else { path }, i);
            errors.extend(validate_value(item, item_schema, &item_path));
        }
    }

    errors
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_validation() {
        let schema = serde_json::json!({ "type": "object" });
        assert!(validate_value(&serde_json::json!({}), &schema, "").is_empty());
        assert!(!validate_value(&serde_json::json!("string"), &schema, "").is_empty());
    }

    #[test]
    fn test_required_fields() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "to": { "type": "string" },
                "subject": { "type": "string" }
            },
            "required": ["to"]
        });

        let valid = serde_json::json!({ "to": "user@example.com" });
        assert!(validate_value(&valid, &schema, "").is_empty());

        let invalid = serde_json::json!({ "subject": "Hello" });
        let errors = validate_value(&invalid, &schema, "");
        assert!(!errors.is_empty());
        assert!(errors[0].contains("to"));
    }

    #[test]
    fn test_nested_validation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    }
                }
            }
        });

        let valid = serde_json::json!({ "address": { "city": "NYC" } });
        assert!(validate_value(&valid, &schema, "").is_empty());

        let invalid = serde_json::json!({ "address": { "city": 123 } });
        assert!(!validate_value(&invalid, &schema, "").is_empty());
    }

    #[test]
    fn test_enum_validation() {
        let schema = serde_json::json!({ "enum": ["a", "b", "c"] });
        assert!(validate_value(&serde_json::json!("a"), &schema, "").is_empty());
        assert!(!validate_value(&serde_json::json!("d"), &schema, "").is_empty());
    }

    #[test]
    fn test_array_items() {
        let schema = serde_json::json!({
            "type": "array",
            "items": { "type": "string" }
        });

        let valid = serde_json::json!(["a", "b"]);
        assert!(validate_value(&valid, &schema, "").is_empty());

        let invalid = serde_json::json!(["a", 1]);
        assert!(!validate_value(&invalid, &schema, "").is_empty());
    }
}
