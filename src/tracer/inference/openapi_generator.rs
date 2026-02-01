//! OpenAPI specification generator from trace data.
//!
//! Generates OpenAPI 3.0 specifications from observed HTTP traffic.

use super::path_analyzer::{ParameterType, PathAnalysisResult, PathAnalyzer};
use super::schema_inferrer::SchemaInferrer;
use crate::tracer::types::{BodyData, HttpSpan, SpanDirection};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Generates OpenAPI specifications from HTTP spans
pub struct OpenApiGenerator {
    path_analyzer: PathAnalyzer,
    schema_inferrer: SchemaInferrer,
    /// Service title
    title: String,
    /// Service version
    version: String,
    /// Base URL for servers
    base_url: Option<String>,
}

/// Configuration for OpenAPI generation
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Whether to infer path parameters
    pub infer_params: bool,
    /// Whether to infer schemas from bodies
    pub infer_schemas: bool,
    /// Whether to include examples
    pub include_examples: bool,
    /// Filter for direction (incoming = own API, outgoing = dependencies)
    pub direction_filter: Option<SpanDirection>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            infer_params: true,
            infer_schemas: true,
            include_examples: true,
            direction_filter: None,
        }
    }
}

/// Grouped endpoint data
struct EndpointData {
    method: String,
    route: String,
    parameters: Vec<ParameterInfo>,
    request_bodies: Vec<Value>,
    responses: HashMap<u16, Vec<Value>>,
    spans: Vec<HttpSpan>,
}

struct ParameterInfo {
    name: String,
    location: String, // "path" or "query"
    param_type: ParameterType,
    required: bool,
    examples: Vec<String>,
}

impl OpenApiGenerator {
    /// Create a new OpenAPI generator
    pub fn new() -> Self {
        Self {
            path_analyzer: PathAnalyzer::new(),
            schema_inferrer: SchemaInferrer::new(),
            title: "Generated API".to_string(),
            version: "1.0.0".to_string(),
            base_url: None,
        }
    }

    /// Set the API title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the API version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Generate OpenAPI specification from spans
    pub fn generate(&self, spans: &[HttpSpan], config: &GeneratorConfig) -> Value {
        // Filter spans by direction if specified
        let spans: Vec<&HttpSpan> = spans
            .iter()
            .filter(|s| {
                config
                    .direction_filter
                    .as_ref()
                    .map(|d| &s.direction == d)
                    .unwrap_or(true)
            })
            .collect();

        // Group spans by endpoint
        let endpoints = self.group_by_endpoint(&spans, config);

        // Build OpenAPI spec
        self.build_openapi_spec(&endpoints, config)
    }

    fn group_by_endpoint<'a>(
        &self,
        spans: &[&'a HttpSpan],
        config: &GeneratorConfig,
    ) -> Vec<EndpointData> {
        // First, analyze paths to detect parameters
        let paths: Vec<(String, String)> = spans
            .iter()
            .map(|s| (s.method.clone(), s.path.clone()))
            .collect();

        let path_results = if config.infer_params {
            self.path_analyzer.analyze(&paths)
        } else {
            // Use paths as-is
            paths
                .iter()
                .map(|(method, path)| PathAnalysisResult {
                    route: format!("{} {}", method, path),
                    parameters: vec![],
                    occurrence_count: 1,
                })
                .collect()
        };

        // Create a mapping from method+path to route pattern
        let route_mapping = self.create_route_mapping(spans, &path_results);

        // Group spans by route
        let mut grouped: HashMap<String, EndpointData> = HashMap::new();

        for span in spans {
            let route_key = format!("{} {}", span.method, span.path);
            let route_pattern = route_mapping
                .get(&route_key)
                .cloned()
                .unwrap_or_else(|| route_key.clone());

            let entry = grouped.entry(route_pattern.clone()).or_insert_with(|| {
                // Find the matching path analysis result
                let analysis = path_results
                    .iter()
                    .find(|r| self.matches_route(&r.route, &span.method, &span.path))
                    .cloned();

                let (method, route) = route_pattern
                    .split_once(' ')
                    .map(|(m, r)| (m.to_string(), r.to_string()))
                    .unwrap_or((span.method.clone(), span.path.clone()));

                let parameters = analysis
                    .map(|a| {
                        a.parameters
                            .into_iter()
                            .map(|p| ParameterInfo {
                                name: p.name,
                                location: "path".to_string(),
                                param_type: p.param_type,
                                required: true,
                                examples: p.example_values,
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                EndpointData {
                    method,
                    route,
                    parameters,
                    request_bodies: vec![],
                    responses: HashMap::new(),
                    spans: vec![],
                }
            });

            // Add query parameters
            if let Some(query) = &span.query {
                for (name, value) in Self::parse_query(query) {
                    if !entry.parameters.iter().any(|p| p.name == name) {
                        entry.parameters.push(ParameterInfo {
                            name: name.clone(),
                            location: "query".to_string(),
                            param_type: ParameterType::String,
                            required: false,
                            examples: vec![value],
                        });
                    }
                }
            }

            // Add request body
            if let Some(body) = &span.request_body {
                if let BodyData::Json(json) = &body.data {
                    entry.request_bodies.push(json.clone());
                }
            }

            // Add response body
            if let Some(body) = &span.response_body {
                if let BodyData::Json(json) = &body.data {
                    entry
                        .responses
                        .entry(span.status_code)
                        .or_default()
                        .push(json.clone());
                }
            }

            entry.spans.push((*span).clone());
        }

        grouped.into_values().collect()
    }

    fn create_route_mapping(
        &self,
        spans: &[&HttpSpan],
        results: &[PathAnalysisResult],
    ) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        for span in spans {
            let key = format!("{} {}", span.method, span.path);

            // Find matching route
            for result in results {
                if self.matches_route(&result.route, &span.method, &span.path) {
                    mapping.insert(key.clone(), result.route.clone());
                    break;
                }
            }
        }

        mapping
    }

    fn matches_route(&self, route: &str, method: &str, path: &str) -> bool {
        let route_parts: Vec<&str> = route.split(' ').collect();
        if route_parts.len() != 2 {
            return false;
        }

        if route_parts[0] != method {
            return false;
        }

        let route_segments: Vec<&str> = route_parts[1].split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if route_segments.len() != path_segments.len() {
            return false;
        }

        route_segments
            .iter()
            .zip(path_segments.iter())
            .all(|(r, p)| r.starts_with('{') && r.ends_with('}') || r == p)
    }

    fn parse_query(query: &str) -> Vec<(String, String)> {
        query
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((key.to_string(), value.to_string()))
            })
            .collect()
    }

    fn build_openapi_spec(&self, endpoints: &[EndpointData], config: &GeneratorConfig) -> Value {
        let mut paths: Map<String, Value> = Map::new();
        let mut schemas: Map<String, Value> = Map::new();

        for endpoint in endpoints {
            let path_item = paths
                .entry(endpoint.route.clone())
                .or_insert_with(|| Value::Object(Map::new()));

            let operation = self.build_operation(endpoint, config, &mut schemas);

            if let Value::Object(pi) = path_item {
                pi.insert(endpoint.method.to_lowercase(), operation);
            }
        }

        let mut spec = serde_json::json!({
            "openapi": "3.0.3",
            "info": {
                "title": self.title,
                "version": self.version
            },
            "paths": paths
        });

        // Add servers if base URL is set
        if let Some(ref base_url) = self.base_url {
            spec["servers"] = serde_json::json!([{"url": base_url}]);
        }

        // Add component schemas if any
        if !schemas.is_empty() {
            spec["components"] = serde_json::json!({
                "schemas": schemas
            });
        }

        spec
    }

    fn build_operation(
        &self,
        endpoint: &EndpointData,
        config: &GeneratorConfig,
        _schemas: &mut Map<String, Value>,
    ) -> Value {
        let mut operation = Map::new();

        // Generate operation ID
        let operation_id = self.generate_operation_id(&endpoint.method, &endpoint.route);
        operation.insert("operationId".to_string(), Value::String(operation_id));

        // Add parameters
        if !endpoint.parameters.is_empty() {
            let params: Vec<Value> = endpoint
                .parameters
                .iter()
                .map(|p| self.build_parameter(p, config))
                .collect();
            operation.insert("parameters".to_string(), Value::Array(params));
        }

        // Add request body
        if !endpoint.request_bodies.is_empty() && config.infer_schemas {
            let schema = self.schema_inferrer.infer(&endpoint.request_bodies);
            let openapi_schema = self.schema_inferrer.to_openapi_schema(&schema.schema);

            let mut content = Map::new();
            let mut json_content = Map::new();
            json_content.insert("schema".to_string(), openapi_schema);

            if config.include_examples && !schema.examples.is_empty() {
                json_content.insert("example".to_string(), schema.examples[0].clone());
            }

            content.insert(
                "application/json".to_string(),
                Value::Object(json_content),
            );

            operation.insert(
                "requestBody".to_string(),
                serde_json::json!({
                    "content": content
                }),
            );
        }

        // Add responses
        let mut responses = Map::new();

        for (status_code, bodies) in &endpoint.responses {
            let mut response = Map::new();
            response.insert(
                "description".to_string(),
                Value::String(Self::status_description(*status_code)),
            );

            if !bodies.is_empty() && config.infer_schemas {
                let schema = self.schema_inferrer.infer(bodies);
                let openapi_schema = self.schema_inferrer.to_openapi_schema(&schema.schema);

                let mut json_content = Map::new();
                json_content.insert("schema".to_string(), openapi_schema);

                if config.include_examples && !schema.examples.is_empty() {
                    json_content.insert("example".to_string(), schema.examples[0].clone());
                }

                response.insert(
                    "content".to_string(),
                    serde_json::json!({
                        "application/json": json_content
                    }),
                );
            }

            responses.insert(status_code.to_string(), Value::Object(response));
        }

        // Ensure at least a default response
        if responses.is_empty() {
            responses.insert(
                "200".to_string(),
                serde_json::json!({
                    "description": "Successful response"
                }),
            );
        }

        operation.insert("responses".to_string(), Value::Object(responses));

        Value::Object(operation)
    }

    fn build_parameter(&self, param: &ParameterInfo, config: &GeneratorConfig) -> Value {
        let mut p = serde_json::json!({
            "name": param.name,
            "in": param.location,
            "required": param.required,
            "schema": self.param_type_schema(&param.param_type)
        });

        if config.include_examples && !param.examples.is_empty() {
            p["example"] = Value::String(param.examples[0].clone());
        }

        p
    }

    fn param_type_schema(&self, param_type: &ParameterType) -> Value {
        match param_type {
            ParameterType::Uuid => serde_json::json!({
                "type": "string",
                "format": "uuid"
            }),
            ParameterType::Integer => serde_json::json!({
                "type": "integer"
            }),
            ParameterType::Slug => serde_json::json!({
                "type": "string",
                "pattern": "^[a-zA-Z0-9-_]+$"
            }),
            ParameterType::String => serde_json::json!({
                "type": "string"
            }),
        }
    }

    fn generate_operation_id(&self, method: &str, route: &str) -> String {
        // Convert route to operation ID
        // e.g., "GET /users/{id}" -> "getUsersById"

        let route_parts: Vec<&str> = route
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut parts = vec![method.to_lowercase()];

        for part in route_parts {
            if part.starts_with('{') && part.ends_with('}') {
                // Parameter - add "By" + capitalized name
                let name = &part[1..part.len() - 1];
                parts.push("By".to_string());
                parts.push(Self::capitalize(name));
            } else {
                parts.push(Self::capitalize(part));
            }
        }

        parts.join("")
    }

    fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }

    fn status_description(code: u16) -> String {
        match code {
            200 => "Successful response",
            201 => "Created",
            204 => "No content",
            400 => "Bad request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not found",
            500 => "Internal server error",
            _ => "Response",
        }
        .to_string()
    }
}

impl Default for OpenApiGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::types::CapturedBody;
    use chrono::Utc;

    fn create_test_span(method: &str, path: &str, status: u16) -> HttpSpan {
        HttpSpan {
            trace_id: "trace1".to_string(),
            span_id: "span1".to_string(),
            parent_span_id: None,
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100.0,
            service_name: "test".to_string(),
            service_version: None,
            direction: SpanDirection::Incoming,
            method: method.to_string(),
            path: path.to_string(),
            route: None,
            query: None,
            scheme: "https".to_string(),
            status_code: status,
            server_address: "localhost".to_string(),
            server_port: 8080,
            client_address: None,
            request_body: None,
            response_body: None,
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_generate_basic_spec() {
        let generator = OpenApiGenerator::new()
            .with_title("Test API")
            .with_version("1.0.0");

        let spans = vec![
            create_test_span("GET", "/users", 200),
            create_test_span("GET", "/users/1", 200),
            create_test_span("GET", "/users/2", 200),
        ];

        let config = GeneratorConfig::default();
        let spec = generator.generate(&spans, &config);

        assert_eq!(spec["openapi"], "3.0.3");
        assert_eq!(spec["info"]["title"], "Test API");
        assert!(spec["paths"].as_object().is_some());
    }

    #[test]
    fn test_generate_with_request_body() {
        let generator = OpenApiGenerator::new();

        let mut span = create_test_span("POST", "/users", 201);
        span.request_body = Some(CapturedBody {
            content_type: Some("application/json".to_string()),
            size: 50,
            data: BodyData::Json(serde_json::json!({
                "name": "John",
                "email": "john@example.com"
            })),
        });
        span.response_body = Some(CapturedBody {
            content_type: Some("application/json".to_string()),
            size: 100,
            data: BodyData::Json(serde_json::json!({
                "id": 1,
                "name": "John",
                "email": "john@example.com"
            })),
        });

        let config = GeneratorConfig::default();
        let spec = generator.generate(&[span], &config);

        let post_op = &spec["paths"]["/users"]["post"];
        assert!(post_op["requestBody"].is_object());
        assert!(post_op["responses"]["201"].is_object());
    }

    #[test]
    fn test_operation_id_generation() {
        let generator = OpenApiGenerator::new();

        assert_eq!(
            generator.generate_operation_id("GET", "/users"),
            "getUsers"
        );
        assert_eq!(
            generator.generate_operation_id("GET", "/users/{id}"),
            "getUsersById"
        );
        assert_eq!(
            generator.generate_operation_id("POST", "/users/{userId}/posts"),
            "postUsersByUserIdPosts"
        );
    }
}
