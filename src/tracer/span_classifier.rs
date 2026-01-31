//! Span classification logic for determining incoming vs outgoing requests.

use super::semantic_conventions::{self as semconv, span_kind, AttributeExtractor};
use super::types::{BodyData, CapturedBody, HttpSpan, SpanDirection};
use chrono::{DateTime, TimeZone, Utc};
use opentelemetry_proto::tonic::trace::v1::Span;
use std::collections::HashMap;

/// Classifies and converts OTLP spans to HttpSpan
pub struct SpanClassifier {
    /// Hosts considered as internal (not dependencies)
    internal_hosts: Vec<String>,
    /// Capture body configuration
    capture_bodies: bool,
    /// Maximum body size
    max_body_size: usize,
}

impl SpanClassifier {
    /// Create a new span classifier
    pub fn new() -> Self {
        Self {
            internal_hosts: Vec::new(),
            capture_bodies: true,
            max_body_size: 65536,
        }
    }

    /// Set internal hosts (requests to these are not classified as dependencies)
    pub fn with_internal_hosts(mut self, hosts: Vec<String>) -> Self {
        self.internal_hosts = hosts;
        self
    }

    /// Set body capture configuration
    pub fn with_body_capture(mut self, capture: bool, max_size: usize) -> Self {
        self.capture_bodies = capture;
        self.max_body_size = max_size;
        self
    }

    /// Classify and convert an OTLP span to HttpSpan
    pub fn classify(
        &self,
        span: &Span,
        resource_attrs: &[opentelemetry_proto::tonic::common::v1::KeyValue],
    ) -> Option<HttpSpan> {
        // Only process HTTP spans (must have http.request.method)
        let method =
            AttributeExtractor::get_string(&span.attributes, semconv::http::HTTP_REQUEST_METHOD)?;

        // Determine direction from span kind
        let direction = match span.kind {
            k if k == span_kind::SERVER => SpanDirection::Incoming,
            k if k == span_kind::CLIENT => SpanDirection::Outgoing,
            _ => return None, // Not an HTTP request span
        };

        // Extract timing
        let start_time = self.nanos_to_datetime(span.start_time_unix_nano);
        let end_time = self.nanos_to_datetime(span.end_time_unix_nano);
        let duration_ms =
            (span.end_time_unix_nano - span.start_time_unix_nano) as f64 / 1_000_000.0;

        // Extract service info from resource attributes
        let service_name = AttributeExtractor::get_string(resource_attrs, semconv::service::SERVICE_NAME)
            .unwrap_or_else(|| "unknown".to_string());
        let service_version =
            AttributeExtractor::get_string(resource_attrs, semconv::service::SERVICE_VERSION);

        // Extract HTTP info
        let path = AttributeExtractor::get_string(&span.attributes, semconv::url::URL_PATH)
            .unwrap_or_else(|| "/".to_string());
        let route = AttributeExtractor::get_string(&span.attributes, semconv::http::HTTP_ROUTE);
        let query = AttributeExtractor::get_string(&span.attributes, semconv::url::URL_QUERY);
        let scheme = AttributeExtractor::get_string(&span.attributes, semconv::url::URL_SCHEME)
            .unwrap_or_else(|| "http".to_string());
        let status_code =
            AttributeExtractor::get_int(&span.attributes, semconv::http::HTTP_RESPONSE_STATUS_CODE)
                .unwrap_or(0) as u16;

        // Extract host info
        let server_address =
            AttributeExtractor::get_string(&span.attributes, semconv::server::SERVER_ADDRESS)
                .unwrap_or_else(|| "localhost".to_string());
        let server_port =
            AttributeExtractor::get_int(&span.attributes, semconv::server::SERVER_PORT)
                .unwrap_or(if scheme == "https" { 443 } else { 80 }) as u16;
        let client_address =
            AttributeExtractor::get_string(&span.attributes, semconv::client::CLIENT_ADDRESS);

        // Extract body info if available (OBI typically doesn't capture bodies)
        let request_body = self.extract_body(&span.attributes, "request");
        let response_body = self.extract_body(&span.attributes, "response");

        // Extract additional attributes
        let attributes = self.extract_additional_attributes(&span.attributes);

        Some(HttpSpan {
            trace_id: hex::encode(&span.trace_id),
            span_id: hex::encode(&span.span_id),
            parent_span_id: if span.parent_span_id.is_empty() {
                None
            } else {
                Some(hex::encode(&span.parent_span_id))
            },
            start_time,
            end_time,
            duration_ms,
            service_name,
            service_version,
            direction,
            method,
            path,
            route,
            query,
            scheme,
            status_code,
            server_address,
            server_port,
            client_address,
            request_body,
            response_body,
            request_headers: HashMap::new(), // Populated separately if available
            response_headers: HashMap::new(),
            attributes,
        })
    }

    /// Check if a span represents an external dependency call
    pub fn is_external_dependency(&self, span: &HttpSpan) -> bool {
        if span.direction != SpanDirection::Outgoing {
            return false;
        }

        let host = &span.server_address;

        // Check if host is in internal list
        !self.internal_hosts.iter().any(|internal| {
            host.contains(internal) || internal.contains(host)
        })
    }

    /// Convert nanoseconds timestamp to DateTime
    fn nanos_to_datetime(&self, nanos: u64) -> DateTime<Utc> {
        let secs = (nanos / 1_000_000_000) as i64;
        let nsecs = (nanos % 1_000_000_000) as u32;
        Utc.timestamp_opt(secs, nsecs).unwrap()
    }

    /// Extract body from span attributes (if captured by instrumentation)
    fn extract_body(
        &self,
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
        prefix: &str,
    ) -> Option<CapturedBody> {
        if !self.capture_bodies {
            return None;
        }

        // Check for body size attribute first
        let size_key = format!("http.{}.body.size", prefix);
        let size = AttributeExtractor::get_int(attributes, &size_key).unwrap_or(0) as usize;

        if size == 0 {
            return None;
        }

        // Look for body content (custom attribute, not standard)
        let body_key = format!("http.{}.body", prefix);
        let content_type_key = format!("http.{}.header.content-type", prefix);

        let content_type = AttributeExtractor::get_string(attributes, &content_type_key);

        // Try to get body content
        if let Some(body_str) = AttributeExtractor::get_string(attributes, &body_key) {
            if body_str.len() > self.max_body_size {
                return Some(CapturedBody {
                    content_type,
                    size,
                    data: BodyData::TooLarge,
                });
            }

            // Try to parse as JSON
            if let Ok(json) = serde_json::from_str(&body_str) {
                return Some(CapturedBody {
                    content_type,
                    size,
                    data: BodyData::Json(json),
                });
            }

            return Some(CapturedBody {
                content_type,
                size,
                data: BodyData::Text(body_str),
            });
        }

        // Body size is known but content not captured
        Some(CapturedBody {
            content_type,
            size,
            data: BodyData::NotCaptured,
        })
    }

    /// Extract additional attributes not covered by semantic conventions
    fn extract_additional_attributes(
        &self,
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
    ) -> HashMap<String, serde_json::Value> {
        let known_keys = [
            semconv::http::HTTP_REQUEST_METHOD,
            semconv::http::HTTP_RESPONSE_STATUS_CODE,
            semconv::http::HTTP_ROUTE,
            semconv::url::URL_PATH,
            semconv::url::URL_QUERY,
            semconv::url::URL_SCHEME,
            semconv::server::SERVER_ADDRESS,
            semconv::server::SERVER_PORT,
            semconv::client::CLIENT_ADDRESS,
        ];

        attributes
            .iter()
            .filter(|kv| !known_keys.contains(&kv.key.as_str()))
            .filter_map(|kv| {
                kv.value
                    .as_ref()
                    .map(|v| (kv.key.clone(), AttributeExtractor::any_value_to_json(v)))
            })
            .collect()
    }
}

impl Default for SpanClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classifier_creation() {
        let classifier = SpanClassifier::new()
            .with_internal_hosts(vec!["localhost".to_string()])
            .with_body_capture(true, 1024);

        assert!(classifier.capture_bodies);
        assert_eq!(classifier.max_body_size, 1024);
        assert_eq!(classifier.internal_hosts, vec!["localhost"]);
    }

    #[test]
    fn test_is_external_dependency() {
        let classifier = SpanClassifier::new()
            .with_internal_hosts(vec!["localhost".to_string(), "internal.svc".to_string()]);

        let mut span = HttpSpan {
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
            path: "/api".to_string(),
            route: None,
            query: None,
            scheme: "https".to_string(),
            status_code: 200,
            server_address: "external-api.com".to_string(),
            server_port: 443,
            client_address: None,
            request_body: None,
            response_body: None,
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            attributes: HashMap::new(),
        };

        assert!(classifier.is_external_dependency(&span));

        span.server_address = "localhost".to_string();
        assert!(!classifier.is_external_dependency(&span));

        span.server_address = "internal.svc.cluster.local".to_string();
        assert!(!classifier.is_external_dependency(&span));

        span.direction = SpanDirection::Incoming;
        span.server_address = "external-api.com".to_string();
        assert!(!classifier.is_external_dependency(&span));
    }
}
