//! OTLP (OpenTelemetry Protocol) receiver for trace collection.
//!
//! Supports both gRPC and HTTP protocols for receiving traces from
//! OBI (OpenTelemetry eBPF Instrumentation) and other OTLP exporters.

use super::TracerConfig;
use super::span_classifier::SpanClassifier;
use super::store::TraceStore;
use super::types::{SpanDirection, TraceSession, TraceStatistics};
use crate::Result;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use chrono::Utc;
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use parking_lot::RwLock;
use prost::Message;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// OTLP Receiver state
pub struct OtlpReceiverState {
    /// Trace store
    store: Arc<TraceStore>,
    /// Span classifier
    classifier: Arc<SpanClassifier>,
    /// Active session ID
    active_session_id: RwLock<Option<String>>,
    /// Statistics accumulator
    stats: RwLock<TraceStatistics>,
    /// Channel for span notifications
    span_tx: Option<mpsc::Sender<SpanNotification>>,
}

/// Notification when new spans are received
#[derive(Debug, Clone)]
pub struct SpanNotification {
    pub session_id: String,
    pub span_count: usize,
}

/// OTLP Receiver
pub struct OtlpReceiver {
    state: Arc<OtlpReceiverState>,
    #[allow(dead_code)]
    config: TracerConfig,
}

impl OtlpReceiver {
    /// Create a new OTLP receiver
    pub fn new(store: Arc<TraceStore>, config: TracerConfig) -> Self {
        let classifier = Arc::new(
            SpanClassifier::new().with_body_capture(config.capture_bodies, config.max_body_size),
        );

        Self {
            state: Arc::new(OtlpReceiverState {
                store,
                classifier,
                active_session_id: RwLock::new(None),
                stats: RwLock::new(TraceStatistics::default()),
                span_tx: None,
            }),
            config,
        }
    }

    /// Set span notification channel
    pub fn with_notification_channel(mut self, tx: mpsc::Sender<SpanNotification>) -> Self {
        // We need to recreate the state with the channel
        let state = Arc::try_unwrap(self.state).unwrap_or_else(|arc| (*arc).clone_state());
        self.state = Arc::new(OtlpReceiverState {
            store: state.store,
            classifier: state.classifier,
            active_session_id: RwLock::new(state.active_session_id.read().clone()),
            stats: RwLock::new(state.stats.read().clone()),
            span_tx: Some(tx),
        });
        self
    }

    /// Start a new trace session
    pub async fn start_session(&self, name: Option<String>) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let session = TraceSession {
            id: session_id.clone(),
            name,
            started_at: Utc::now(),
            ended_at: None,
            config: super::types::TraceConfig::default(),
            statistics: TraceStatistics::default(),
        };

        self.state.store.create_session(&session).await?;

        {
            let mut active = self.state.active_session_id.write();
            *active = Some(session_id.clone());
        }

        {
            let mut stats = self.state.stats.write();
            *stats = TraceStatistics::default();
        }

        info!(session_id = %session_id, "Started trace session");
        Ok(session_id)
    }

    /// Stop the active trace session
    pub async fn stop_session(&self) -> Result<Option<String>> {
        let session_id = {
            let mut active = self.state.active_session_id.write();
            active.take()
        };

        if let Some(id) = &session_id {
            // Update session with end time and final stats
            if let Some(mut session) = self.state.store.get_session(id).await? {
                session.ended_at = Some(Utc::now());
                session.statistics = self.state.stats.read().clone();
                self.state.store.update_session(&session).await?;
            }
            info!(session_id = %id, "Stopped trace session");
        }

        Ok(session_id)
    }

    /// Get the active session ID
    pub fn active_session_id(&self) -> Option<String> {
        self.state.active_session_id.read().clone()
    }

    /// Get current statistics
    pub fn current_stats(&self) -> TraceStatistics {
        self.state.stats.read().clone()
    }

    /// Create Axum router for HTTP OTLP endpoint
    pub fn http_router(&self) -> Router {
        Router::new()
            .route("/v1/traces", post(handle_otlp_traces))
            .with_state(self.state.clone())
    }

    /// Process an OTLP export request
    pub async fn process_export_request(
        &self,
        request: ExportTraceServiceRequest,
    ) -> Result<ExportTraceServiceResponse> {
        let session_id = match self.state.active_session_id.read().clone() {
            Some(id) => id,
            None => {
                warn!("Received traces but no active session");
                return Ok(ExportTraceServiceResponse {
                    partial_success: None,
                });
            }
        };

        let mut spans_processed = 0;
        let mut http_spans = Vec::new();

        for resource_spans in &request.resource_spans {
            let resource_attrs = resource_spans
                .resource
                .as_ref()
                .map(|r| r.attributes.as_slice())
                .unwrap_or(&[]);

            for scope_spans in &resource_spans.scope_spans {
                for span in &scope_spans.spans {
                    if let Some(http_span) = self.state.classifier.classify(span, resource_attrs) {
                        http_spans.push(http_span);
                        spans_processed += 1;
                    }
                }
            }
        }

        if !http_spans.is_empty() {
            // Update statistics
            {
                let mut stats = self.state.stats.write();
                for span in &http_spans {
                    stats.total_spans += 1;
                    match span.direction {
                        SpanDirection::Incoming => stats.incoming_requests += 1,
                        SpanDirection::Outgoing => stats.outgoing_requests += 1,
                    }
                    *stats.status_codes.entry(span.status_code).or_insert(0) += 1;
                }
            }

            // Store spans
            self.state
                .store
                .store_spans(&session_id, &http_spans)
                .await?;

            // Send notification
            if let Some(tx) = &self.state.span_tx {
                let _ = tx
                    .send(SpanNotification {
                        session_id: session_id.clone(),
                        span_count: http_spans.len(),
                    })
                    .await;
            }

            debug!(
                session_id = %session_id,
                spans = spans_processed,
                "Processed OTLP trace export"
            );
        }

        Ok(ExportTraceServiceResponse {
            partial_success: None,
        })
    }
}

impl OtlpReceiverState {
    fn clone_state(&self) -> Self {
        Self {
            store: self.store.clone(),
            classifier: self.classifier.clone(),
            active_session_id: RwLock::new(self.active_session_id.read().clone()),
            stats: RwLock::new(self.stats.read().clone()),
            span_tx: None,
        }
    }
}

/// Axum handler for OTLP HTTP traces endpoint
async fn handle_otlp_traces(
    State(state): State<Arc<OtlpReceiverState>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Check content type
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let request = if content_type.contains("application/x-protobuf")
        || content_type.contains("application/protobuf")
    {
        // Protobuf encoding
        match ExportTraceServiceRequest::decode(body) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to decode protobuf: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error": "Failed to decode protobuf: {}"}}"#, e),
                );
            }
        }
    } else if content_type.contains("application/json") {
        // JSON encoding
        match serde_json::from_slice::<ExportTraceServiceRequest>(&body) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to decode JSON: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error": "Failed to decode JSON: {}"}}"#, e),
                );
            }
        }
    } else {
        // Default to protobuf
        match ExportTraceServiceRequest::decode(body) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to decode request: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error": "Failed to decode request: {}"}}"#, e),
                );
            }
        }
    };

    // Get active session
    let session_id = match state.active_session_id.read().clone() {
        Some(id) => id,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"error": "No active trace session"}"#.to_string(),
            );
        }
    };

    // Process spans
    let mut spans_processed = 0;
    let mut http_spans = Vec::new();

    for resource_spans in &request.resource_spans {
        let resource_attrs = resource_spans
            .resource
            .as_ref()
            .map(|r| r.attributes.as_slice())
            .unwrap_or(&[]);

        for scope_spans in &resource_spans.scope_spans {
            for span in &scope_spans.spans {
                if let Some(http_span) = state.classifier.classify(span, resource_attrs) {
                    http_spans.push(http_span);
                    spans_processed += 1;
                }
            }
        }
    }

    if !http_spans.is_empty() {
        // Update statistics
        {
            let mut stats = state.stats.write();
            for span in &http_spans {
                stats.total_spans += 1;
                match span.direction {
                    SpanDirection::Incoming => stats.incoming_requests += 1,
                    SpanDirection::Outgoing => stats.outgoing_requests += 1,
                }
                *stats.status_codes.entry(span.status_code).or_insert(0) += 1;
            }
        }

        // Store spans
        if let Err(e) = state.store.store_spans(&session_id, &http_spans).await {
            error!("Failed to store spans: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                format!(r#"{{"error": "Failed to store spans: {}"}}"#, e),
            );
        }

        // Send notification
        if let Some(tx) = &state.span_tx {
            let _ = tx
                .send(SpanNotification {
                    session_id: session_id.clone(),
                    span_count: http_spans.len(),
                })
                .await;
        }

        debug!(
            session_id = %session_id,
            spans = spans_processed,
            "Processed OTLP HTTP trace export"
        );
    }

    // Return success response
    let response = ExportTraceServiceResponse {
        partial_success: None,
    };

    let response_bytes = response.encode_to_vec();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-protobuf")],
        String::from_utf8_lossy(&response_bytes).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_receiver_session_lifecycle() {
        let store = Arc::new(TraceStore::new_memory());
        let config = TracerConfig::default();
        let receiver = OtlpReceiver::new(store, config);

        // Start session
        let session_id = receiver
            .start_session(Some("Test".to_string()))
            .await
            .unwrap();
        assert!(receiver.active_session_id().is_some());

        // Stop session
        let stopped_id = receiver.stop_session().await.unwrap();
        assert_eq!(stopped_id, Some(session_id));
        assert!(receiver.active_session_id().is_none());
    }
}
