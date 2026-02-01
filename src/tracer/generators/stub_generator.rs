//! API stub generator from trace data.
//!
//! Generates mock server stubs in various formats:
//! - WireMock JSON mappings
//! - Prism (OpenAPI with examples)
//! - Native hornet2 format

use crate::tracer::inference::OpenApiGenerator;
use crate::tracer::types::{BodyData, HttpSpan, SpanDirection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Supported stub output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubFormat {
    /// WireMock JSON mappings
    WireMock,
    /// Prism mock server (OpenAPI with examples)
    Prism,
    /// Hornet2 native stub format
    Native,
}

/// Generates API stubs from HTTP spans
pub struct StubGenerator {
    format: StubFormat,
}

/// Configuration for stub generation
#[derive(Debug, Clone)]
pub struct StubGeneratorConfig {
    /// Filter by host (only generate stubs for matching hosts)
    pub host_filter: Option<String>,
    /// Include request matchers
    pub include_request_matchers: bool,
    /// Include response headers
    pub include_headers: bool,
    /// Generate stubs for status codes >= 400
    pub include_error_responses: bool,
}

impl Default for StubGeneratorConfig {
    fn default() -> Self {
        Self {
            host_filter: None,
            include_request_matchers: true,
            include_headers: true,
            include_error_responses: true,
        }
    }
}

/// WireMock mapping structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMockMapping {
    /// Unique mapping ID
    pub id: String,
    /// Request matcher
    pub request: WireMockRequest,
    /// Response definition
    pub response: WireMockResponse,
    /// Mapping priority
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireMockRequest {
    pub method: String,
    pub url_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_parameters: Option<HashMap<String, WireMockMatcher>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, WireMockMatcher>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_patterns: Option<Vec<WireMockBodyPattern>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireMockMatcher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireMockBodyPattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_to_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches_json_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireMockResponse {
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_body: Option<Value>,
}

/// Native hornet2 stub format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeStub {
    /// Stub version
    pub version: String,
    /// Target service info
    pub service: ServiceInfo,
    /// Stub endpoints
    pub endpoints: Vec<NativeEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub host: String,
    pub port: u16,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeEndpoint {
    pub method: String,
    pub path: String,
    pub responses: Vec<NativeResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeResponse {
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// When to use this response (request matcher)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<NativeRequestMatcher>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeRequestMatcher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_contains: Option<String>,
}

/// Generated stubs result
pub struct GeneratedStubs {
    /// Host this stub is for
    pub host: String,
    /// Port
    pub port: u16,
    /// Stubs in the specified format
    pub stubs: StubOutput,
}

pub enum StubOutput {
    WireMock(Vec<WireMockMapping>),
    Prism(Value), // OpenAPI with examples
    Native(NativeStub),
}

impl StubGenerator {
    /// Create a new stub generator
    pub fn new(format: StubFormat) -> Self {
        Self { format }
    }

    /// Generate stubs from spans
    pub fn generate(
        &self,
        spans: &[HttpSpan],
        config: &StubGeneratorConfig,
    ) -> Vec<GeneratedStubs> {
        // Filter to outgoing spans only (these are the dependency calls)
        let outgoing_spans: Vec<&HttpSpan> = spans
            .iter()
            .filter(|s| s.direction == SpanDirection::Outgoing)
            .filter(|s| {
                config
                    .host_filter
                    .as_ref()
                    .map(|h| s.server_address.contains(h))
                    .unwrap_or(true)
            })
            .filter(|s| config.include_error_responses || s.status_code < 400)
            .collect();

        // Group by host:port
        let mut by_host: HashMap<(String, u16), Vec<&HttpSpan>> = HashMap::new();
        for span in outgoing_spans {
            by_host
                .entry((span.server_address.clone(), span.server_port))
                .or_default()
                .push(span);
        }

        by_host
            .into_iter()
            .map(|((host, port), spans)| {
                let stubs = match self.format {
                    StubFormat::WireMock => {
                        StubOutput::WireMock(self.generate_wiremock(&spans, config))
                    }
                    StubFormat::Prism => StubOutput::Prism(self.generate_prism(&spans, config)),
                    StubFormat::Native => {
                        StubOutput::Native(self.generate_native(&host, port, &spans, config))
                    }
                };

                GeneratedStubs { host, port, stubs }
            })
            .collect()
    }

    fn generate_wiremock(
        &self,
        spans: &[&HttpSpan],
        config: &StubGeneratorConfig,
    ) -> Vec<WireMockMapping> {
        let mut mappings = Vec::new();

        for (i, span) in spans.iter().enumerate() {
            let id = format!(
                "{}-{}-{}",
                span.method.to_lowercase(),
                span.path
                    .replace('/', "-")
                    .trim_start_matches('-'),
                i
            );

            // Build request matcher
            let query_parameters = if config.include_request_matchers {
                span.query.as_ref().map(|q| {
                    Self::parse_query(q)
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                WireMockMatcher {
                                    equal_to: Some(v),
                                    matches: None,
                                    contains: None,
                                },
                            )
                        })
                        .collect()
                })
            } else {
                None
            };

            let body_patterns = if config.include_request_matchers {
                span.request_body.as_ref().and_then(|b| {
                    if let BodyData::Json(json) = &b.data {
                        Some(vec![WireMockBodyPattern {
                            equal_to_json: Some(json.clone()),
                            matches_json_path: None,
                        }])
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            let request = WireMockRequest {
                method: span.method.clone(),
                url_path: span.path.clone(),
                query_parameters,
                headers: None,
                body_patterns,
            };

            // Build response
            let (body, json_body) = span
                .response_body
                .as_ref()
                .map(|b| match &b.data {
                    BodyData::Json(json) => (None, Some(json.clone())),
                    BodyData::Text(text) => (Some(text.clone()), None),
                    _ => (None, None),
                })
                .unwrap_or((None, None));

            let headers = if config.include_headers {
                let mut h = HashMap::new();
                h.insert(
                    "Content-Type".to_string(),
                    span.response_body
                        .as_ref()
                        .and_then(|b| b.content_type.clone())
                        .unwrap_or_else(|| "application/json".to_string()),
                );
                Some(h)
            } else {
                None
            };

            let response = WireMockResponse {
                status: span.status_code,
                headers,
                body,
                json_body,
            };

            mappings.push(WireMockMapping {
                id,
                request,
                response,
                priority: None,
            });
        }

        mappings
    }

    fn generate_prism(&self, spans: &[&HttpSpan], _config: &StubGeneratorConfig) -> Value {
        // Prism uses OpenAPI with examples
        // Convert spans to HttpSpan references for the generator
        let spans_owned: Vec<HttpSpan> = spans.iter().map(|s| (*s).clone()).collect();

        let generator = OpenApiGenerator::new()
            .with_title("Dependency API Stub")
            .with_version("1.0.0");

        let config = crate::tracer::inference::openapi_generator::GeneratorConfig {
            infer_params: true,
            infer_schemas: true,
            include_examples: true, // Important for Prism
            direction_filter: Some(SpanDirection::Outgoing),
        };

        generator.generate(&spans_owned, &config)
    }

    fn generate_native(
        &self,
        host: &str,
        port: u16,
        spans: &[&HttpSpan],
        config: &StubGeneratorConfig,
    ) -> NativeStub {
        // Group by method + path
        let mut by_endpoint: HashMap<(String, String), Vec<&HttpSpan>> = HashMap::new();
        for span in spans {
            by_endpoint
                .entry((span.method.clone(), span.path.clone()))
                .or_default()
                .push(span);
        }

        let endpoints: Vec<NativeEndpoint> = by_endpoint
            .into_iter()
            .map(|((method, path), endpoint_spans)| {
                let responses: Vec<NativeResponse> = endpoint_spans
                    .into_iter()
                    .map(|span| {
                        let body = span.response_body.as_ref().and_then(|b| {
                            match &b.data {
                                BodyData::Json(json) => Some(json.clone()),
                                BodyData::Text(text) => Some(Value::String(text.clone())),
                                _ => None,
                            }
                        });

                        let headers = if config.include_headers && !span.response_headers.is_empty()
                        {
                            Some(span.response_headers.clone())
                        } else {
                            None
                        };

                        let when = if config.include_request_matchers {
                            let query = span.query.as_ref().map(|q| {
                                Self::parse_query(q).into_iter().collect()
                            });

                            if query.is_some() {
                                Some(NativeRequestMatcher {
                                    query,
                                    headers: None,
                                    body_contains: None,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        NativeResponse {
                            status: span.status_code,
                            content_type: span
                                .response_body
                                .as_ref()
                                .and_then(|b| b.content_type.clone()),
                            body,
                            headers,
                            when,
                        }
                    })
                    .collect();

                NativeEndpoint {
                    method,
                    path,
                    responses,
                }
            })
            .collect();

        NativeStub {
            version: "1.0.0".to_string(),
            service: ServiceInfo {
                host: host.to_string(),
                port,
                name: None,
            },
            endpoints,
        }
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

    /// Write stubs to files in the specified directory
    pub fn write_to_directory(
        &self,
        stubs: &[GeneratedStubs],
        dir: &Path,
    ) -> std::io::Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(dir)?;

        let mut files = Vec::new();

        for stub in stubs {
            let filename = format!("{}_{}", stub.host.replace('.', "_"), stub.port);

            match &stub.stubs {
                StubOutput::WireMock(mappings) => {
                    let path = dir.join(format!("{}_mappings.json", filename));
                    let content = serde_json::to_string_pretty(mappings)?;
                    std::fs::write(&path, content)?;
                    files.push(path);
                }
                StubOutput::Prism(openapi) => {
                    let path = dir.join(format!("{}_openapi.yaml", filename));
                    let content = serde_yaml::to_string(openapi).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                    })?;
                    std::fs::write(&path, content)?;
                    files.push(path);
                }
                StubOutput::Native(native) => {
                    let path = dir.join(format!("{}_stub.yaml", filename));
                    let content = serde_yaml::to_string(native).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                    })?;
                    std::fs::write(&path, content)?;
                    files.push(path);
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::types::CapturedBody;
    use chrono::Utc;

    fn create_outgoing_span(method: &str, path: &str, status: u16) -> HttpSpan {
        HttpSpan {
            trace_id: "trace1".to_string(),
            span_id: "span1".to_string(),
            parent_span_id: None,
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100.0,
            service_name: "test".to_string(),
            service_version: None,
            direction: SpanDirection::Outgoing,
            method: method.to_string(),
            path: path.to_string(),
            route: None,
            query: None,
            scheme: "https".to_string(),
            status_code: status,
            server_address: "external-api.com".to_string(),
            server_port: 443,
            client_address: None,
            request_body: None,
            response_body: Some(CapturedBody {
                content_type: Some("application/json".to_string()),
                size: 50,
                data: BodyData::Json(serde_json::json!({"id": 1, "name": "Test"})),
            }),
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_generate_wiremock_stubs() {
        let generator = StubGenerator::new(StubFormat::WireMock);

        let spans = vec![
            create_outgoing_span("GET", "/users/1", 200),
            create_outgoing_span("POST", "/users", 201),
        ];

        let config = StubGeneratorConfig::default();
        let result = generator.generate(&spans, &config);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].host, "external-api.com");

        if let StubOutput::WireMock(mappings) = &result[0].stubs {
            assert_eq!(mappings.len(), 2);
            assert_eq!(mappings[0].request.method, "GET");
            assert_eq!(mappings[0].response.status, 200);
        } else {
            panic!("Expected WireMock output");
        }
    }

    #[test]
    fn test_generate_native_stubs() {
        let generator = StubGenerator::new(StubFormat::Native);

        let spans = vec![create_outgoing_span("GET", "/users/1", 200)];

        let config = StubGeneratorConfig::default();
        let result = generator.generate(&spans, &config);

        assert_eq!(result.len(), 1);

        if let StubOutput::Native(native) = &result[0].stubs {
            assert_eq!(native.service.host, "external-api.com");
            assert_eq!(native.endpoints.len(), 1);
            assert_eq!(native.endpoints[0].responses.len(), 1);
        } else {
            panic!("Expected Native output");
        }
    }

    #[test]
    fn test_host_filter() {
        let generator = StubGenerator::new(StubFormat::WireMock);

        let mut span1 = create_outgoing_span("GET", "/users", 200);
        span1.server_address = "api-a.com".to_string();

        let mut span2 = create_outgoing_span("GET", "/items", 200);
        span2.server_address = "api-b.com".to_string();

        let config = StubGeneratorConfig {
            host_filter: Some("api-a".to_string()),
            ..Default::default()
        };

        let result = generator.generate(&[span1, span2], &config);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].host, "api-a.com");
    }

    #[test]
    fn test_filter_incoming_spans() {
        let generator = StubGenerator::new(StubFormat::WireMock);

        let mut incoming = create_outgoing_span("GET", "/users", 200);
        incoming.direction = SpanDirection::Incoming;

        let outgoing = create_outgoing_span("GET", "/external", 200);

        let config = StubGeneratorConfig::default();
        let result = generator.generate(&[incoming, outgoing], &config);

        // Only outgoing spans should generate stubs
        assert_eq!(result.len(), 1);

        if let StubOutput::WireMock(mappings) = &result[0].stubs {
            assert_eq!(mappings.len(), 1);
            assert_eq!(mappings[0].request.url_path, "/external");
        }
    }
}
