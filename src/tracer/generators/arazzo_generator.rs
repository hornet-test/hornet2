//! Arazzo workflow generator from trace data.
//!
//! Generates Arazzo workflow specifications from observed HTTP traffic.

use crate::models::arazzo::{
    ArazzoSpec, Info, Parameter, RequestBody, SourceDescription, Step, SuccessCriteria, Workflow,
};
use crate::tracer::inference::OpenApiGenerator;
use crate::tracer::types::{BodyData, HttpSpan, SpanDirection};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Generates Arazzo workflows from HTTP spans
pub struct ArazzoGenerator {
    /// Workflow name
    workflow_name: String,
    /// Workflow description
    workflow_description: Option<String>,
    /// Source description name for OpenAPI reference
    source_name: String,
}

/// Configuration for Arazzo generation
#[derive(Debug, Clone)]
pub struct ArazzoGeneratorConfig {
    /// Include success criteria based on observed status codes
    pub include_success_criteria: bool,
    /// Include request body examples
    pub include_request_bodies: bool,
    /// Include output mappings for response data
    pub include_outputs: bool,
    /// Group traces by trace ID into separate workflows
    pub group_by_trace: bool,
    /// Filter for direction
    pub direction_filter: Option<SpanDirection>,
}

impl Default for ArazzoGeneratorConfig {
    fn default() -> Self {
        Self {
            include_success_criteria: true,
            include_request_bodies: true,
            include_outputs: true,
            group_by_trace: false,
            direction_filter: None,
        }
    }
}

impl ArazzoGenerator {
    /// Create a new Arazzo generator
    pub fn new() -> Self {
        Self {
            workflow_name: "generated-workflow".to_string(),
            workflow_description: None,
            source_name: "api".to_string(),
        }
    }

    /// Set the workflow name
    pub fn with_workflow_name(mut self, name: impl Into<String>) -> Self {
        self.workflow_name = name.into();
        self
    }

    /// Set the workflow description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.workflow_description = Some(desc.into());
        self
    }

    /// Set the source description name
    pub fn with_source_name(mut self, name: impl Into<String>) -> Self {
        self.source_name = name.into();
        self
    }

    /// Generate Arazzo specification from spans
    pub fn generate(&self, spans: &[HttpSpan], config: &ArazzoGeneratorConfig) -> ArazzoSpec {
        // Filter spans
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

        if config.group_by_trace {
            self.generate_grouped(&spans, config)
        } else {
            self.generate_single(&spans, config)
        }
    }

    /// Generate a single workflow from all spans
    fn generate_single(&self, spans: &[&HttpSpan], config: &ArazzoGeneratorConfig) -> ArazzoSpec {
        // Sort spans by time
        let mut sorted_spans: Vec<&&HttpSpan> = spans.iter().collect();
        sorted_spans.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        // Generate steps
        let steps = self.generate_steps(&sorted_spans, config);

        let workflow = Workflow {
            workflow_id: self.workflow_name.clone(),
            summary: Some(format!("Generated workflow with {} steps", steps.len())),
            description: self.workflow_description.clone(),
            inputs: None,
            steps,
            success_criteria: None,
            outputs: None,
            extensions: IndexMap::new(),
        };

        ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: Info {
                title: format!("{} Workflow", self.workflow_name),
                version: "1.0.0".to_string(),
                summary: Some("Auto-generated from trace data".to_string()),
                description: self.workflow_description.clone(),
            },
            source_descriptions: vec![SourceDescription {
                name: self.source_name.clone(),
                url: "openapi.yaml".to_string(),
                source_type: Some("openapi".to_string()),
            }],
            workflows: vec![workflow],
            components: None,
        }
    }

    /// Generate grouped workflows by trace ID
    fn generate_grouped(&self, spans: &[&HttpSpan], config: &ArazzoGeneratorConfig) -> ArazzoSpec {
        // Group by trace ID
        let mut trace_groups: HashMap<&str, Vec<&&HttpSpan>> = HashMap::new();
        for span in spans {
            trace_groups.entry(&span.trace_id).or_default().push(span);
        }

        let mut workflows = Vec::new();

        for (i, (trace_id, mut trace_spans)) in trace_groups.into_iter().enumerate() {
            // Sort by time
            trace_spans.sort_by(|a, b| a.start_time.cmp(&b.start_time));

            let steps = self.generate_steps(&trace_spans, config);
            let workflow_id = format!("{}-{}", self.workflow_name, i + 1);

            workflows.push(Workflow {
                workflow_id: workflow_id.clone(),
                summary: Some(format!("Trace {} with {} steps", trace_id, steps.len())),
                description: None,
                inputs: None,
                steps,
                success_criteria: None,
                outputs: None,
                extensions: IndexMap::new(),
            });
        }

        ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: Info {
                title: format!("{} Workflows", self.workflow_name),
                version: "1.0.0".to_string(),
                summary: Some(format!(
                    "Auto-generated {} workflows from trace data",
                    workflows.len()
                )),
                description: self.workflow_description.clone(),
            },
            source_descriptions: vec![SourceDescription {
                name: self.source_name.clone(),
                url: "openapi.yaml".to_string(),
                source_type: Some("openapi".to_string()),
            }],
            workflows,
            components: None,
        }
    }

    fn generate_steps(&self, spans: &[&&HttpSpan], config: &ArazzoGeneratorConfig) -> Vec<Step> {
        let mut steps = Vec::new();
        let mut step_counter: HashMap<String, usize> = HashMap::new();

        for span in spans {
            // Generate operation path
            let operation_path = format!(
                "{}#/{}/{}",
                self.source_name,
                span.method.to_lowercase(),
                span.path.trim_start_matches('/')
            );

            // Generate unique step ID
            let base_id = Self::generate_step_id(&span.method, &span.path);
            let count = step_counter.entry(base_id.clone()).or_insert(0);
            *count += 1;

            let step_id = if *count == 1 {
                base_id
            } else {
                format!("{}-{}", base_id, count)
            };

            // Generate request body
            let request_body = if config.include_request_bodies {
                span.request_body.as_ref().and_then(|body| {
                    if let BodyData::Json(json) = &body.data {
                        Some(RequestBody {
                            content_type: body.content_type.clone(),
                            payload: json.clone(),
                        })
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            // Generate success criteria
            let success_criteria = if config.include_success_criteria {
                Some(vec![SuccessCriteria {
                    context: "$statusCode".to_string(),
                    condition: "$eq".to_string(),
                    value: Some(serde_json::json!(span.status_code)),
                    criteria_type: Some("simple".to_string()),
                }])
            } else {
                None
            };

            // Generate outputs
            let outputs = if config.include_outputs && span.response_body.is_some() {
                let mut output_map = serde_json::Map::new();
                if let Some(body) = &span.response_body {
                    if let BodyData::Json(json) = &body.data {
                        // Extract top-level fields as outputs
                        if let Some(obj) = json.as_object() {
                            for key in obj.keys().take(5) {
                                output_map.insert(
                                    key.clone(),
                                    serde_json::json!(format!("$response.body.{}", key)),
                                );
                            }
                        }
                    }
                }
                if output_map.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(output_map))
                }
            } else {
                None
            };

            // Generate parameters from query string
            let parameters: Vec<Parameter> = span
                .query
                .as_ref()
                .map(|query| {
                    Self::parse_query(query)
                        .into_iter()
                        .map(|(name, value)| Parameter {
                            name,
                            location: "query".to_string(),
                            value: serde_json::Value::String(value),
                        })
                        .collect()
                })
                .unwrap_or_default();

            steps.push(Step {
                step_id,
                description: Some(format!("{} {}", span.method, span.path)),
                operation_id: None,
                operation_path: Some(operation_path),
                workflow_id: None,
                parameters,
                request_body,
                success_criteria,
                on_success: None,
                on_failure: None,
                outputs,
            });
        }

        steps
    }

    fn generate_step_id(method: &str, path: &str) -> String {
        // Convert path to step ID
        // e.g., "GET /users/{id}" -> "get-users-id"

        let clean_path = path
            .trim_start_matches('/')
            .replace('/', "-")
            .replace('{', "")
            .replace('}', "")
            .to_lowercase();

        format!("{}-{}", method.to_lowercase(), clean_path)
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

    /// Generate both Arazzo spec and corresponding OpenAPI spec
    pub fn generate_with_openapi(
        &self,
        spans: &[HttpSpan],
        config: &ArazzoGeneratorConfig,
    ) -> (ArazzoSpec, serde_json::Value) {
        let arazzo = self.generate(spans, config);

        // Generate OpenAPI for the same spans
        let openapi_generator = OpenApiGenerator::new()
            .with_title(format!("{} API", self.workflow_name))
            .with_version("1.0.0");

        let openapi_config = crate::tracer::inference::openapi_generator::GeneratorConfig {
            infer_params: true,
            infer_schemas: true,
            include_examples: true,
            direction_filter: config.direction_filter.clone(),
        };

        let openapi = openapi_generator.generate(spans, &openapi_config);

        (arazzo, openapi)
    }
}

impl Default for ArazzoGenerator {
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
            span_id: format!("span-{}", path),
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
    fn test_generate_basic_workflow() {
        let generator = ArazzoGenerator::new().with_workflow_name("test-workflow");

        let spans = vec![
            create_test_span("POST", "/users", 201),
            create_test_span("GET", "/users/1", 200),
        ];

        let config = ArazzoGeneratorConfig::default();
        let arazzo = generator.generate(&spans, &config);

        assert_eq!(arazzo.arazzo, "1.0.0");
        assert_eq!(arazzo.workflows.len(), 1);
        assert_eq!(arazzo.workflows[0].steps.len(), 2);
    }

    #[test]
    fn test_generate_step_id() {
        assert_eq!(
            ArazzoGenerator::generate_step_id("GET", "/users"),
            "get-users"
        );
        assert_eq!(
            ArazzoGenerator::generate_step_id("GET", "/users/{id}"),
            "get-users-id"
        );
        assert_eq!(
            ArazzoGenerator::generate_step_id("POST", "/users/{userId}/posts"),
            "post-users-userid-posts"
        );
    }

    #[test]
    fn test_generate_with_request_body() {
        let generator = ArazzoGenerator::new();

        let mut span = create_test_span("POST", "/users", 201);
        span.request_body = Some(CapturedBody {
            content_type: Some("application/json".to_string()),
            size: 50,
            data: BodyData::Json(serde_json::json!({
                "name": "John"
            })),
        });

        let config = ArazzoGeneratorConfig::default();
        let arazzo = generator.generate(&[span], &config);

        assert!(arazzo.workflows[0].steps[0].request_body.is_some());
    }

    #[test]
    fn test_generate_grouped_by_trace() {
        let generator = ArazzoGenerator::new();

        let mut span1 = create_test_span("POST", "/users", 201);
        span1.trace_id = "trace-a".to_string();

        let mut span2 = create_test_span("GET", "/users/1", 200);
        span2.trace_id = "trace-a".to_string();

        let mut span3 = create_test_span("GET", "/health", 200);
        span3.trace_id = "trace-b".to_string();

        let config = ArazzoGeneratorConfig {
            group_by_trace: true,
            ..Default::default()
        };

        let arazzo = generator.generate(&[span1, span2, span3], &config);

        assert_eq!(arazzo.workflows.len(), 2);
    }
}
