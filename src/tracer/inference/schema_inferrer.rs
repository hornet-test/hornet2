//! JSON Schema inference from observed request/response bodies.
//!
//! Infers JSON Schema from multiple observed JSON values.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Infers JSON Schema from observed values
pub struct SchemaInferrer {
    /// Merge arrays with single type
    merge_array_types: bool,
    /// Maximum examples to keep per property
    max_examples: usize,
}

/// Inferred schema representation
#[derive(Debug, Clone)]
pub struct InferredSchema {
    /// The JSON Schema object
    pub schema: Value,
    /// Example values
    pub examples: Vec<Value>,
    /// Number of samples used
    pub sample_count: usize,
}

impl SchemaInferrer {
    /// Create a new schema inferrer
    pub fn new() -> Self {
        Self {
            merge_array_types: true,
            max_examples: 3,
        }
    }

    /// Infer schema from multiple JSON values
    pub fn infer(&self, values: &[Value]) -> InferredSchema {
        if values.is_empty() {
            return InferredSchema {
                schema: Value::Object(Map::new()),
                examples: vec![],
                sample_count: 0,
            };
        }

        let mut merged_schema = self.infer_single(&values[0]);

        for value in values.iter().skip(1) {
            let schema = self.infer_single(value);
            merged_schema = self.merge_schemas(&merged_schema, &schema);
        }

        let examples: Vec<Value> = values.iter().take(self.max_examples).cloned().collect();

        InferredSchema {
            schema: merged_schema,
            examples,
            sample_count: values.len(),
        }
    }

    /// Infer schema from a single value
    fn infer_single(&self, value: &Value) -> Value {
        match value {
            Value::Null => self.null_schema(),
            Value::Bool(_) => self.boolean_schema(),
            Value::Number(n) => {
                if n.is_i64() {
                    self.integer_schema()
                } else {
                    self.number_schema()
                }
            }
            Value::String(s) => self.string_schema_with_format(s),
            Value::Array(arr) => self.array_schema(arr),
            Value::Object(obj) => self.object_schema(obj),
        }
    }

    fn null_schema(&self) -> Value {
        serde_json::json!({
            "type": "null"
        })
    }

    fn boolean_schema(&self) -> Value {
        serde_json::json!({
            "type": "boolean"
        })
    }

    fn integer_schema(&self) -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }

    fn number_schema(&self) -> Value {
        serde_json::json!({
            "type": "number"
        })
    }

    fn string_schema_with_format(&self, value: &str) -> Value {
        let format = self.detect_string_format(value);
        let mut schema = serde_json::json!({
            "type": "string"
        });

        if let Some(fmt) = format {
            schema["format"] = Value::String(fmt);
        }

        schema
    }

    fn detect_string_format(&self, value: &str) -> Option<String> {
        // UUID
        if value.len() == 36 && self.is_uuid(value) {
            return Some("uuid".to_string());
        }

        // Email
        if value.contains('@') && value.contains('.') {
            return Some("email".to_string());
        }

        // Date-time (ISO 8601)
        if value.len() >= 10
            && (value.contains('T') || value.contains(' '))
            && value.chars().take(4).all(|c| c.is_ascii_digit())
            && value.chars().nth(4) == Some('-')
        {
            return Some("date-time".to_string());
        }

        // Date only
        if value.len() == 10
            && value.chars().take(4).all(|c| c.is_ascii_digit())
            && value.chars().nth(4) == Some('-')
        {
            return Some("date".to_string());
        }

        // URI
        if value.starts_with("http://") || value.starts_with("https://") {
            return Some("uri".to_string());
        }

        None
    }

    fn is_uuid(&self, s: &str) -> bool {
        s.len() == 36
            && s.chars().enumerate().all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            })
    }

    fn array_schema(&self, arr: &[Value]) -> Value {
        if arr.is_empty() {
            return serde_json::json!({
                "type": "array",
                "items": {}
            });
        }

        if self.merge_array_types {
            // Infer schema from all items and merge
            let item_schemas: Vec<Value> = arr.iter().map(|v| self.infer_single(v)).collect();

            let merged_items = item_schemas
                .iter()
                .skip(1)
                .fold(item_schemas[0].clone(), |acc, schema| {
                    self.merge_schemas(&acc, schema)
                });

            serde_json::json!({
                "type": "array",
                "items": merged_items
            })
        } else {
            // Use first item's schema
            serde_json::json!({
                "type": "array",
                "items": self.infer_single(&arr[0])
            })
        }
    }

    fn object_schema(&self, obj: &Map<String, Value>) -> Value {
        let mut properties = Map::new();
        let mut required: Vec<String> = Vec::new();

        for (key, value) in obj {
            properties.insert(key.clone(), self.infer_single(value));
            required.push(key.clone());
        }

        required.sort();

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    /// Merge two schemas into a unified schema
    fn merge_schemas(&self, a: &Value, b: &Value) -> Value {
        let type_a = a.get("type").and_then(|v| v.as_str());
        let type_b = b.get("type").and_then(|v| v.as_str());

        match (type_a, type_b) {
            (Some(ta), Some(tb)) if ta == tb => {
                match ta {
                    "object" => self.merge_object_schemas(a, b),
                    "array" => self.merge_array_schemas(a, b),
                    _ => a.clone(), // Same primitive type, keep first
                }
            }
            (Some(_), Some(_)) => {
                // Different types - create oneOf
                serde_json::json!({
                    "oneOf": [a, b]
                })
            }
            _ => a.clone(),
        }
    }

    fn merge_object_schemas(&self, a: &Value, b: &Value) -> Value {
        let props_a = a.get("properties").and_then(|v| v.as_object());
        let props_b = b.get("properties").and_then(|v| v.as_object());

        let req_a: HashSet<String> = a
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let req_b: HashSet<String> = b
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let mut merged_props = Map::new();
        let mut all_keys: HashSet<String> = HashSet::new();

        if let Some(pa) = props_a {
            for (k, v) in pa {
                merged_props.insert(k.clone(), v.clone());
                all_keys.insert(k.clone());
            }
        }

        if let Some(pb) = props_b {
            for (k, v) in pb {
                all_keys.insert(k.clone());
                if let Some(existing) = merged_props.get(k) {
                    merged_props.insert(k.clone(), self.merge_schemas(existing, v));
                } else {
                    merged_props.insert(k.clone(), v.clone());
                }
            }
        }

        // Only required if present in both
        let required: Vec<String> = req_a
            .intersection(&req_b)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .collect();

        let mut result = serde_json::json!({
            "type": "object",
            "properties": merged_props
        });

        if !required.is_empty() {
            let mut sorted_required = required;
            sorted_required.sort();
            result["required"] =
                Value::Array(sorted_required.into_iter().map(Value::String).collect());
        }

        result
    }

    fn merge_array_schemas(&self, a: &Value, b: &Value) -> Value {
        let items_a = a.get("items");
        let items_b = b.get("items");

        let merged_items = match (items_a, items_b) {
            (Some(ia), Some(ib)) => self.merge_schemas(ia, ib),
            (Some(ia), None) => ia.clone(),
            (None, Some(ib)) => ib.clone(),
            (None, None) => Value::Object(Map::new()),
        };

        serde_json::json!({
            "type": "array",
            "items": merged_items
        })
    }

    /// Convert inferred schema to OpenAPI 3.0 schema format
    pub fn to_openapi_schema(&self, schema: &Value) -> Value {
        // OpenAPI 3.0 uses mostly the same schema format as JSON Schema
        // Just need to handle some edge cases
        self.convert_to_openapi(schema)
    }

    fn convert_to_openapi(&self, schema: &Value) -> Value {
        match schema {
            Value::Object(obj) => {
                let mut result = Map::new();

                for (key, value) in obj {
                    match key.as_str() {
                        "properties" => {
                            if let Value::Object(props) = value {
                                let mut converted = Map::new();
                                for (k, v) in props {
                                    converted.insert(k.clone(), self.convert_to_openapi(v));
                                }
                                result.insert(key.clone(), Value::Object(converted));
                            }
                        }
                        "items" => {
                            result.insert(key.clone(), self.convert_to_openapi(value));
                        }
                        "oneOf" => {
                            if let Value::Array(arr) = value {
                                let converted: Vec<Value> =
                                    arr.iter().map(|v| self.convert_to_openapi(v)).collect();
                                result.insert(key.clone(), Value::Array(converted));
                            }
                        }
                        _ => {
                            result.insert(key.clone(), value.clone());
                        }
                    }
                }

                Value::Object(result)
            }
            _ => schema.clone(),
        }
    }
}

impl Default for SchemaInferrer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_primitive_types() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[serde_json::json!(42)]);
        assert_eq!(schema.schema["type"], "integer");

        let schema = inferrer.infer(&[serde_json::json!(3.14)]);
        assert_eq!(schema.schema["type"], "number");

        let schema = inferrer.infer(&[serde_json::json!(true)]);
        assert_eq!(schema.schema["type"], "boolean");

        let schema = inferrer.infer(&[serde_json::json!("hello")]);
        assert_eq!(schema.schema["type"], "string");
    }

    #[test]
    fn test_infer_string_formats() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[serde_json::json!("550e8400-e29b-41d4-a716-446655440000")]);
        assert_eq!(schema.schema["type"], "string");
        assert_eq!(schema.schema["format"], "uuid");

        let schema = inferrer.infer(&[serde_json::json!("user@example.com")]);
        assert_eq!(schema.schema["type"], "string");
        assert_eq!(schema.schema["format"], "email");

        let schema = inferrer.infer(&[serde_json::json!("2024-01-15")]);
        assert_eq!(schema.schema["type"], "string");
        assert_eq!(schema.schema["format"], "date");

        let schema = inferrer.infer(&[serde_json::json!("https://example.com")]);
        assert_eq!(schema.schema["type"], "string");
        assert_eq!(schema.schema["format"], "uri");
    }

    #[test]
    fn test_infer_object() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[serde_json::json!({
            "name": "John",
            "age": 30
        })]);

        assert_eq!(schema.schema["type"], "object");
        assert_eq!(schema.schema["properties"]["name"]["type"], "string");
        assert_eq!(schema.schema["properties"]["age"]["type"], "integer");
    }

    #[test]
    fn test_infer_array() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[serde_json::json!([1, 2, 3])]);

        assert_eq!(schema.schema["type"], "array");
        assert_eq!(schema.schema["items"]["type"], "integer");
    }

    #[test]
    fn test_merge_objects() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[
            serde_json::json!({"name": "John", "age": 30}),
            serde_json::json!({"name": "Jane", "email": "jane@example.com"}),
        ]);

        assert_eq!(schema.schema["type"], "object");
        // name is in both, so should be required
        // age and email are only in one, so not required
        let required = schema.schema["required"].as_array().unwrap();
        assert!(required.contains(&Value::String("name".to_string())));
    }

    #[test]
    fn test_to_openapi_schema() {
        let inferrer = SchemaInferrer::new();

        let schema = inferrer.infer(&[serde_json::json!({
            "users": [{"id": 1, "name": "John"}]
        })]);

        let openapi_schema = inferrer.to_openapi_schema(&schema.schema);
        assert_eq!(openapi_schema["type"], "object");
        assert_eq!(openapi_schema["properties"]["users"]["type"], "array");
    }
}
