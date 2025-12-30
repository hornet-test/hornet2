use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub service_name: String,
    pub sample_rate: f64,
}

impl TelemetryConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("OTEL_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:5080/api/default".to_string());

        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "hornet2".to_string());

        let mut headers = HashMap::new();
        if let Ok(auth_header) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
            // Parse "key=value,key2=value2" format or "key=value" format
            for pair in auth_header.split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    headers.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        Self {
            enabled,
            endpoint,
            headers,
            service_name,
            sample_rate: 1.0, // Sample all traces (100%)
        }
    }
}
