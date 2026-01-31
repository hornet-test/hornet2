//! OpenTelemetry Semantic Conventions for HTTP spans.
//!
//! Based on OpenTelemetry Semantic Conventions v1.29.0
//! https://opentelemetry.io/docs/specs/semconv/http/http-spans/

/// HTTP Request attributes (Stable)
pub mod http {
    /// HTTP request method (GET, POST, etc.)
    pub const HTTP_REQUEST_METHOD: &str = "http.request.method";

    /// Original HTTP method for method override scenarios
    pub const HTTP_REQUEST_METHOD_ORIGINAL: &str = "http.request.method_original";

    /// HTTP response status code
    pub const HTTP_RESPONSE_STATUS_CODE: &str = "http.response.status_code";

    /// Size of the request body in bytes
    pub const HTTP_REQUEST_BODY_SIZE: &str = "http.request.body.size";

    /// Size of the response body in bytes
    pub const HTTP_RESPONSE_BODY_SIZE: &str = "http.response.body.size";

    /// The matched route template (e.g., /users/{id})
    pub const HTTP_ROUTE: &str = "http.route";
}

/// URL attributes (Stable)
pub mod url {
    /// Full URL (scheme://host:port/path?query#fragment)
    pub const URL_FULL: &str = "url.full";

    /// URL path component
    pub const URL_PATH: &str = "url.path";

    /// URL query string (without leading ?)
    pub const URL_QUERY: &str = "url.query";

    /// URL scheme (http, https)
    pub const URL_SCHEME: &str = "url.scheme";
}

/// Server attributes (Stable)
pub mod server {
    /// Server address (hostname or IP)
    pub const SERVER_ADDRESS: &str = "server.address";

    /// Server port
    pub const SERVER_PORT: &str = "server.port";
}

/// Client attributes (Stable)
pub mod client {
    /// Client address (peer address)
    pub const CLIENT_ADDRESS: &str = "client.address";

    /// Client port
    pub const CLIENT_PORT: &str = "client.port";
}

/// Network attributes (Stable)
pub mod network {
    /// Network protocol name (http, https)
    pub const NETWORK_PROTOCOL_NAME: &str = "network.protocol.name";

    /// Network protocol version (1.0, 1.1, 2, 3)
    pub const NETWORK_PROTOCOL_VERSION: &str = "network.protocol.version";

    /// Network transport (tcp, udp)
    pub const NETWORK_TRANSPORT: &str = "network.transport";
}

/// User agent attributes
pub mod user_agent {
    /// User agent original string
    pub const USER_AGENT_ORIGINAL: &str = "user_agent.original";
}

/// Error attributes
pub mod error {
    /// Error type/category
    pub const ERROR_TYPE: &str = "error.type";
}

/// Service resource attributes
pub mod service {
    /// Service name
    pub const SERVICE_NAME: &str = "service.name";

    /// Service version
    pub const SERVICE_VERSION: &str = "service.version";

    /// Service namespace
    pub const SERVICE_NAMESPACE: &str = "service.namespace";
}

/// SpanKind values
pub mod span_kind {
    /// Server span (incoming request handler)
    pub const SERVER: i32 = 2;

    /// Client span (outgoing request)
    pub const CLIENT: i32 = 3;

    /// Producer span (message producer)
    pub const PRODUCER: i32 = 4;

    /// Consumer span (message consumer)
    pub const CONSUMER: i32 = 5;
}

/// Attribute value extractor helpers
pub struct AttributeExtractor;

impl AttributeExtractor {
    /// Extract string attribute from OTLP KeyValue
    pub fn get_string(
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
        key: &str,
    ) -> Option<String> {
        attributes.iter().find(|kv| kv.key == key).and_then(|kv| {
            kv.value.as_ref().and_then(|v| {
                v.value.as_ref().and_then(|val| match val {
                    opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s) => {
                        Some(s.clone())
                    }
                    _ => None,
                })
            })
        })
    }

    /// Extract integer attribute from OTLP KeyValue
    pub fn get_int(
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
        key: &str,
    ) -> Option<i64> {
        attributes.iter().find(|kv| kv.key == key).and_then(|kv| {
            kv.value.as_ref().and_then(|v| {
                v.value.as_ref().and_then(|val| match val {
                    opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(i) => {
                        Some(*i)
                    }
                    _ => None,
                })
            })
        })
    }

    /// Extract boolean attribute from OTLP KeyValue
    pub fn get_bool(
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
        key: &str,
    ) -> Option<bool> {
        attributes.iter().find(|kv| kv.key == key).and_then(|kv| {
            kv.value.as_ref().and_then(|v| {
                v.value.as_ref().and_then(|val| match val {
                    opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(b) => {
                        Some(*b)
                    }
                    _ => None,
                })
            })
        })
    }

    /// Extract double attribute from OTLP KeyValue
    pub fn get_double(
        attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
        key: &str,
    ) -> Option<f64> {
        attributes.iter().find(|kv| kv.key == key).and_then(|kv| {
            kv.value.as_ref().and_then(|v| {
                v.value.as_ref().and_then(|val| match val {
                    opentelemetry_proto::tonic::common::v1::any_value::Value::DoubleValue(d) => {
                        Some(*d)
                    }
                    _ => None,
                })
            })
        })
    }

    /// Convert OTLP AnyValue to serde_json::Value
    pub fn any_value_to_json(
        value: &opentelemetry_proto::tonic::common::v1::AnyValue,
    ) -> serde_json::Value {
        match value.value.as_ref() {
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s)) => {
                serde_json::Value::String(s.clone())
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(i)) => {
                serde_json::Value::Number((*i).into())
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::DoubleValue(d)) => {
                serde_json::Number::from_f64(*d)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(b)) => {
                serde_json::Value::Bool(*b)
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::ArrayValue(arr)) => {
                serde_json::Value::Array(
                    arr.values
                        .iter()
                        .map(Self::any_value_to_json)
                        .collect(),
                )
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::KvlistValue(kv)) => {
                let map: serde_json::Map<String, serde_json::Value> = kv
                    .values
                    .iter()
                    .filter_map(|kv| {
                        kv.value
                            .as_ref()
                            .map(|v| (kv.key.clone(), Self::any_value_to_json(v)))
                    })
                    .collect();
                serde_json::Value::Object(map)
            }
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::BytesValue(b)) => {
                use base64::Engine;
                serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
            }
            None => serde_json::Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_convention_keys() {
        assert_eq!(http::HTTP_REQUEST_METHOD, "http.request.method");
        assert_eq!(http::HTTP_RESPONSE_STATUS_CODE, "http.response.status_code");
        assert_eq!(url::URL_PATH, "url.path");
        assert_eq!(server::SERVER_ADDRESS, "server.address");
    }
}
