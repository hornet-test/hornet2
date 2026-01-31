//! Trace API endpoints for the web server.
//!
//! Provides REST endpoints for managing trace sessions and viewing trace data.

use crate::tracer::{OtlpReceiver, TraceStore, TracerConfig};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Trace-specific application state
#[derive(Clone)]
pub struct TraceState {
    /// Trace store
    pub store: Arc<TraceStore>,
    /// OTLP receiver
    pub receiver: Arc<OtlpReceiver>,
    /// Active session ID
    pub active_session_id: Arc<RwLock<Option<String>>>,
}

impl TraceState {
    /// Create a new trace state
    pub fn new(store_path: PathBuf) -> crate::Result<Self> {
        let store_file = store_path.join("traces.db");

        // Ensure directory exists
        if let Some(parent) = store_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let store = Arc::new(TraceStore::new_sqlite(store_file)?);
        let config = TracerConfig::from_env();
        let receiver = Arc::new(OtlpReceiver::new(store.clone(), config));

        Ok(Self {
            store,
            receiver,
            active_session_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a new trace state with in-memory store (for testing)
    pub fn new_memory() -> Self {
        let store = Arc::new(TraceStore::new_memory());
        let config = TracerConfig::default();
        let receiver = Arc::new(OtlpReceiver::new(store.clone(), config));

        Self {
            store,
            receiver,
            active_session_id: Arc::new(RwLock::new(None)),
        }
    }
}

/// Request body for starting a trace session
#[derive(Debug, Deserialize)]
pub struct StartSessionRequest {
    /// Optional session name
    pub name: Option<String>,
}

/// Response for session operations
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub name: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub total_spans: u64,
    pub incoming_requests: u64,
    pub outgoing_requests: u64,
}

/// Response for listing sessions
#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionResponse>,
}

/// Query parameters for listing spans
#[derive(Debug, Deserialize)]
pub struct ListSpansQuery {
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    100
}

/// Start a new trace session
#[tracing::instrument(skip(state))]
pub async fn start_session(
    State(state): State<TraceState>,
    Json(req): Json<StartSessionRequest>,
) -> impl IntoResponse {
    match state.receiver.start_session(req.name).await {
        Ok(session_id) => {
            // Store the active session ID
            {
                let mut active = state.active_session_id.write();
                *active = Some(session_id.clone());
            }

            tracing::info!(session_id = %session_id, "Started trace session");

            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "session_id": session_id,
                    "status": "active",
                    "message": "Trace session started"
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to start session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to start session: {}", e)
                })),
            )
        }
    }
}

/// Stop the active trace session
#[tracing::instrument(skip(state))]
pub async fn stop_session(State(state): State<TraceState>) -> impl IntoResponse {
    let session_id = state.active_session_id.read().clone();

    match session_id {
        Some(id) => {
            match state.receiver.stop_session().await {
                Ok(_) => {
                    // Clear the active session ID
                    {
                        let mut active = state.active_session_id.write();
                        *active = None;
                    }

                    tracing::info!(session_id = %id, "Stopped trace session");

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "session_id": id,
                            "status": "stopped",
                            "message": "Trace session stopped"
                        })),
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to stop session: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to stop session: {}", e)
                        })),
                    )
                }
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No active trace session"
            })),
        ),
    }
}

/// Get the active session status
#[tracing::instrument(skip(state))]
pub async fn get_active_session(State(state): State<TraceState>) -> impl IntoResponse {
    let session_id = state.active_session_id.read().clone();

    match session_id {
        Some(id) => {
            let stats = state.receiver.current_stats();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "session_id": id,
                    "status": "active",
                    "stats": {
                        "total_spans": stats.total_spans,
                        "incoming_requests": stats.incoming_requests,
                        "outgoing_requests": stats.outgoing_requests,
                        "status_codes": stats.status_codes
                    }
                })),
            )
        }
        None => (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": null,
                "status": "inactive"
            })),
        ),
    }
}

/// List all trace sessions
#[tracing::instrument(skip(state))]
pub async fn list_sessions(State(state): State<TraceState>) -> impl IntoResponse {
    match state.store.list_sessions().await {
        Ok(sessions) => {
            let responses: Vec<SessionResponse> = sessions
                .into_iter()
                .map(|s| SessionResponse {
                    id: s.id,
                    name: s.name,
                    started_at: s.started_at.to_rfc3339(),
                    ended_at: s.ended_at.map(|t| t.to_rfc3339()),
                    total_spans: s.statistics.total_spans,
                    incoming_requests: s.statistics.incoming_requests,
                    outgoing_requests: s.statistics.outgoing_requests,
                })
                .collect();

            (StatusCode::OK, Json(ListSessionsResponse { sessions: responses }))
        }
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ListSessionsResponse { sessions: vec![] }),
            )
        }
    }
}

/// Get a specific session
#[tracing::instrument(skip(state))]
pub async fn get_session(
    State(state): State<TraceState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.store.get_session(&session_id).await {
        Ok(Some(session)) => {
            let response = SessionResponse {
                id: session.id,
                name: session.name,
                started_at: session.started_at.to_rfc3339(),
                ended_at: session.ended_at.map(|t| t.to_rfc3339()),
                total_spans: session.statistics.total_spans,
                incoming_requests: session.statistics.incoming_requests,
                outgoing_requests: session.statistics.outgoing_requests,
            };
            (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Session not found: {}", session_id)
            })),
        ),
        Err(e) => {
            tracing::error!("Failed to get session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to get session: {}", e)
                })),
            )
        }
    }
}

/// Delete a session
#[tracing::instrument(skip(state))]
pub async fn delete_session(
    State(state): State<TraceState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.store.delete_session(&session_id).await {
        Ok(()) => {
            tracing::info!(session_id = %session_id, "Deleted trace session");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": format!("Session {} deleted", session_id)
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to delete session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to delete session: {}", e)
                })),
            )
        }
    }
}

/// List spans for a session
#[tracing::instrument(skip(state))]
pub async fn list_spans(
    State(state): State<TraceState>,
    Path(session_id): Path<String>,
    Query(query): Query<ListSpansQuery>,
) -> impl IntoResponse {
    match state
        .store
        .get_spans(&session_id, query.offset, query.limit)
        .await
    {
        Ok(spans) => (StatusCode::OK, Json(serde_json::to_value(spans).unwrap())),
        Err(e) => {
            tracing::error!("Failed to list spans: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to list spans: {}", e)
                })),
            )
        }
    }
}

/// Get endpoints for a session
#[tracing::instrument(skip(state))]
pub async fn get_endpoints(
    State(state): State<TraceState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.store.get_endpoints(&session_id).await {
        Ok(endpoints) => {
            let result: Vec<serde_json::Value> = endpoints
                .into_iter()
                .map(|(method, path, count)| {
                    serde_json::json!({
                        "method": method,
                        "path": path,
                        "count": count
                    })
                })
                .collect();
            (StatusCode::OK, Json(result))
        }
        Err(e) => {
            tracing::error!("Failed to get endpoints: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(vec![serde_json::json!({
                    "error": format!("Failed to get endpoints: {}", e)
                })]),
            )
        }
    }
}

/// Get dependencies for a session
#[tracing::instrument(skip(state))]
pub async fn get_dependencies(
    State(state): State<TraceState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.store.get_dependencies(&session_id).await {
        Ok(deps) => {
            let result: Vec<serde_json::Value> = deps
                .into_iter()
                .map(|(host, port, count)| {
                    serde_json::json!({
                        "host": host,
                        "port": port,
                        "count": count
                    })
                })
                .collect();
            (StatusCode::OK, Json(result))
        }
        Err(e) => {
            tracing::error!("Failed to get dependencies: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(vec![serde_json::json!({
                    "error": format!("Failed to get dependencies: {}", e)
                })]),
            )
        }
    }
}

/// Build trace API router
pub fn trace_router(state: TraceState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        // Session management
        .route("/sessions", get(list_sessions).post(start_session))
        .route("/sessions/active", get(get_active_session).delete(stop_session))
        .route(
            "/sessions/{session_id}",
            get(get_session).delete(delete_session),
        )
        // Span data
        .route("/sessions/{session_id}/spans", get(list_spans))
        .route("/sessions/{session_id}/endpoints", get(get_endpoints))
        .route("/sessions/{session_id}/dependencies", get(get_dependencies))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let state = TraceState::new_memory();
        let router = trace_router(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_active_session_none() {
        let state = TraceState::new_memory();
        let router = trace_router(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/sessions/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
