//! Tracer configuration module.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Tracer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerConfig {
    /// OTLP gRPC server port
    pub grpc_port: u16,

    /// OTLP HTTP server port
    pub http_port: u16,

    /// Storage path for trace data
    pub store_path: PathBuf,

    /// Session timeout (auto-close inactive sessions)
    pub session_timeout: Duration,

    /// Maximum spans to keep in memory before flushing
    pub max_memory_spans: usize,

    /// Enable body capture
    pub capture_bodies: bool,

    /// Maximum body size to capture
    pub max_body_size: usize,

    /// Headers to capture (case-insensitive patterns)
    pub capture_headers: Vec<String>,

    /// Headers to redact (sensitive data)
    pub redact_headers: Vec<String>,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            grpc_port: 4317,
            http_port: 4318,
            store_path: PathBuf::from(".hornet2/traces"),
            session_timeout: Duration::from_secs(1800), // 30 minutes
            max_memory_spans: 10000,
            capture_bodies: true,
            max_body_size: 65536, // 64KB
            capture_headers: vec![
                "content-type".to_string(),
                "accept".to_string(),
                "user-agent".to_string(),
                "x-request-id".to_string(),
                "x-correlation-id".to_string(),
            ],
            redact_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
                "x-api-key".to_string(),
            ],
        }
    }
}

impl TracerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(port) = std::env::var("HORNET2_OTLP_GRPC_PORT") {
            if let Ok(p) = port.parse() {
                config.grpc_port = p;
            }
        }

        if let Ok(port) = std::env::var("HORNET2_OTLP_HTTP_PORT") {
            if let Ok(p) = port.parse() {
                config.http_port = p;
            }
        }

        if let Ok(path) = std::env::var("HORNET2_TRACE_STORE_PATH") {
            config.store_path = PathBuf::from(path);
        }

        if let Ok(timeout) = std::env::var("HORNET2_SESSION_TIMEOUT_SECS") {
            if let Ok(t) = timeout.parse() {
                config.session_timeout = Duration::from_secs(t);
            }
        }

        if let Ok(max_spans) = std::env::var("HORNET2_MAX_MEMORY_SPANS") {
            if let Ok(m) = max_spans.parse() {
                config.max_memory_spans = m;
            }
        }

        if let Ok(capture) = std::env::var("HORNET2_CAPTURE_BODIES") {
            config.capture_bodies = capture.to_lowercase() == "true" || capture == "1";
        }

        if let Ok(max_size) = std::env::var("HORNET2_MAX_BODY_SIZE") {
            if let Ok(s) = max_size.parse() {
                config.max_body_size = s;
            }
        }

        config
    }

    /// Check if a header should be captured
    pub fn should_capture_header(&self, header_name: &str) -> bool {
        let lower = header_name.to_lowercase();
        self.capture_headers
            .iter()
            .any(|h| lower.contains(&h.to_lowercase()))
    }

    /// Check if a header should be redacted
    pub fn should_redact_header(&self, header_name: &str) -> bool {
        let lower = header_name.to_lowercase();
        self.redact_headers
            .iter()
            .any(|h| lower.contains(&h.to_lowercase()))
    }

    /// Get redacted value for sensitive headers
    pub fn redact_value(&self, header_name: &str, _value: &str) -> String {
        format!("[REDACTED:{}]", header_name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracerConfig::default();
        assert_eq!(config.grpc_port, 4317);
        assert_eq!(config.http_port, 4318);
        assert!(config.capture_bodies);
    }

    #[test]
    fn test_header_capture() {
        let config = TracerConfig::default();
        assert!(config.should_capture_header("Content-Type"));
        assert!(config.should_capture_header("X-Request-ID"));
        assert!(!config.should_capture_header("X-Custom-Header"));
    }

    #[test]
    fn test_header_redaction() {
        let config = TracerConfig::default();
        assert!(config.should_redact_header("Authorization"));
        assert!(config.should_redact_header("Cookie"));
        assert!(!config.should_redact_header("Content-Type"));
    }
}
