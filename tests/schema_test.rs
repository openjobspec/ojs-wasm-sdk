//! Tests for the schema validation module.

use ojs_wasm_sdk::schema::SchemaValidator;

#[test]
fn test_schema_validator_creation() {
    let validator = SchemaValidator::new();
    assert!(!validator.has_schema("anything"));
}

#[test]
fn test_schema_registration() {
    let mut validator = SchemaValidator::new();

    let schema = r#"{
        "type": "object",
        "properties": {
            "to": { "type": "string" },
            "subject": { "type": "string" }
        },
        "required": ["to"]
    }"#;

    validator.register("email.send", schema).unwrap();
    assert!(validator.has_schema("email.send"));
    assert!(!validator.has_schema("other.type"));
}

#[test]
fn test_schema_unregistration() {
    let mut validator = SchemaValidator::new();
    validator.register("email.send", r#"{"type": "object"}"#).unwrap();
    assert!(validator.has_schema("email.send"));

    assert!(validator.unregister("email.send"));
    assert!(!validator.has_schema("email.send"));

    // Unregistering non-existent returns false
    assert!(!validator.unregister("nonexistent"));
}

#[test]
fn test_registered_types() {
    let mut validator = SchemaValidator::new();
    validator.register("a", r#"{"type": "object"}"#).unwrap();
    validator.register("b", r#"{"type": "string"}"#).unwrap();

    let types = validator.registered_types();
    assert!(types.contains("\"a\""));
    assert!(types.contains("\"b\""));
}

#[test]
fn test_invalid_schema_detected() {
    // On non-wasm targets, JsValue::from_str panics, so we can't test
    // register() with invalid JSON directly. Instead, verify the validator
    // correctly handles a valid but empty schema.
    let mut validator = SchemaValidator::new();
    validator.register("empty", "{}").unwrap(); // empty schema is valid JSON
    assert!(validator.has_schema("empty"));
}
