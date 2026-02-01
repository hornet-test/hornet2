//! Core data types for trace collection and processing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trace session containing collected spans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSession {
    /// Unique session identifier
    pub id: String,
    /// Optional human-readable name
    pub name: Option<String>,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time (None if still active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Session configuration
    pub config: TraceConfig,
    /// Aggregated statistics
    pub statistics: TraceStatistics,
}

/// Trace collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    /// Target service names to collect (empty = all)
    #[serde(default)]
    pub target_services: Vec<String>,
    /// Target ports to monitor (empty = all)
    #[serde(default)]
    pub target_ports: Vec<u16>,
    /// Host patterns treated as external dependencies
    #[serde(default)]
    pub external_hosts: Vec<String>,
    /// Whether to capture request/response bodies
    #[serde(default = "default_capture_bodies")]
    pub capture_bodies: bool,
    /// Maximum body size to capture (bytes)
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

fn default_capture_bodies() -> bool {
    true
}

fn default_max_body_size() -> usize {
    65536 // 64KB
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            target_services: Vec::new(),
            target_ports: Vec::new(),
            external_hosts: Vec::new(),
            capture_bodies: default_capture_bodies(),
            max_body_size: default_max_body_size(),
        }
    }
}

/// Aggregated trace statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceStatistics {
    /// Total number of spans collected
    pub total_spans: u64,
    /// Number of incoming (server) requests
    pub incoming_requests: u64,
    /// Number of outgoing (client) requests
    pub outgoing_requests: u64,
    /// Number of unique endpoints detected
    pub unique_endpoints: u64,
    /// Number of unique external dependencies
    pub unique_dependencies: u64,
    /// Status code distribution
    #[serde(default)]
    pub status_codes: HashMap<u16, u64>,
}

/// Normalized HTTP span from OpenTelemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSpan {
    // === Identifiers ===
    /// OpenTelemetry trace ID (hex string)
    pub trace_id: String,
    /// OpenTelemetry span ID (hex string)
    pub span_id: String,
    /// Parent span ID (if any)
    pub parent_span_id: Option<String>,

    // === Timing ===
    /// Span start time
    pub start_time: DateTime<Utc>,
    /// Span end time
    pub end_time: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: f64,

    // === Service Info ===
    /// Service name from resource attributes
    pub service_name: String,
    /// Service version (if available)
    pub service_version: Option<String>,

    // === HTTP Info (Semantic Conventions) ===
    /// Span direction (incoming/outgoing)
    pub direction: SpanDirection,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path (e.g., /api/users/123)
    pub path: String,
    /// Route template if available (e.g., /api/users/{id})
    pub route: Option<String>,
    /// Query string (without leading ?)
    pub query: Option<String>,
    /// URL scheme (http/https)
    pub scheme: String,
    /// HTTP status code
    pub status_code: u16,

    // === Host Info ===
    /// Server address (hostname or IP)
    pub server_address: String,
    /// Server port
    pub server_port: u16,
    /// Client address (if available)
    pub client_address: Option<String>,

    // === Body Info ===
    /// Captured request body
    pub request_body: Option<CapturedBody>,
    /// Captured response body
    pub response_body: Option<CapturedBody>,

    // === Headers ===
    /// Request headers (filtered for relevance)
    #[serde(default)]
    pub request_headers: HashMap<String, String>,
    /// Response headers (filtered for relevance)
    #[serde(default)]
    pub response_headers: HashMap<String, String>,

    // === Additional Attributes ===
    /// Other span attributes
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Direction of the HTTP span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpanDirection {
    /// Incoming request (server receiving)
    Incoming,
    /// Outgoing request (client sending, dependency call)
    Outgoing,
}

impl SpanDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanDirection::Incoming => "incoming",
            SpanDirection::Outgoing => "outgoing",
        }
    }
}

/// Captured HTTP body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedBody {
    /// Content-Type header value
    pub content_type: Option<String>,
    /// Original body size in bytes
    pub size: usize,
    /// Body data
    pub data: BodyData,
}

/// Body data representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum BodyData {
    /// Parsed JSON body
    Json(serde_json::Value),
    /// Plain text body
    Text(String),
    /// Binary data (base64 encoded)
    Binary(String),
    /// Body was too large to capture
    TooLarge,
    /// Body capture was disabled or not available
    NotCaptured,
}

impl BodyData {
    /// Check if body data is available
    pub fn is_available(&self) -> bool {
        matches!(
            self,
            BodyData::Json(_) | BodyData::Text(_) | BodyData::Binary(_)
        )
    }
}

/// Endpoint key for grouping spans
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointKey {
    /// HTTP method
    pub method: String,
    /// Path or route template
    pub path: String,
    /// Target host (for outgoing requests)
    pub host: Option<String>,
}

impl EndpointKey {
    pub fn from_span(span: &HttpSpan) -> Self {
        Self {
            method: span.method.clone(),
            path: span.route.clone().unwrap_or_else(|| span.path.clone()),
            host: if span.direction == SpanDirection::Outgoing {
                Some(format!("{}:{}", span.server_address, span.server_port))
            } else {
                None
            },
        }
    }
}

/// Dependency information (external API being called)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Endpoints called on this dependency
    pub endpoints: Vec<DependencyEndpoint>,
    /// Total number of calls
    pub total_calls: u64,
}

/// Endpoint on a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEndpoint {
    /// HTTP method
    pub method: String,
    /// Path template
    pub path: String,
    /// Number of observations
    pub call_count: u64,
    /// Observed status codes
    pub status_codes: HashMap<u16, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_direction_serialization() {
        let incoming = SpanDirection::Incoming;
        let outgoing = SpanDirection::Outgoing;

        assert_eq!(serde_json::to_string(&incoming).unwrap(), "\"incoming\"");
        assert_eq!(serde_json::to_string(&outgoing).unwrap(), "\"outgoing\"");
    }

    #[test]
    fn test_body_data_availability() {
        assert!(BodyData::Json(serde_json::json!({})).is_available());
        assert!(BodyData::Text("hello".to_string()).is_available());
        assert!(!BodyData::TooLarge.is_available());
        assert!(!BodyData::NotCaptured.is_available());
    }

    #[test]
    fn test_endpoint_key_from_span() {
        let span = HttpSpan {
            trace_id: "abc".to_string(),
            span_id: "def".to_string(),
            parent_span_id: None,
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100.0,
            service_name: "test".to_string(),
            service_version: None,
            direction: SpanDirection::Outgoing,
            method: "GET".to_string(),
            path: "/users/123".to_string(),
            route: Some("/users/{id}".to_string()),
            query: None,
            scheme: "https".to_string(),
            status_code: 200,
            server_address: "api.example.com".to_string(),
            server_port: 443,
            client_address: None,
            request_body: None,
            response_body: None,
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            attributes: HashMap::new(),
        };

        let key = EndpointKey::from_span(&span);
        assert_eq!(key.method, "GET");
        assert_eq!(key.path, "/users/{id}");
        assert_eq!(key.host, Some("api.example.com:443".to_string()));
    }
}
