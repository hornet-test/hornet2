//! In-memory trace storage backend.
//!
//! Primarily used for testing and development.

use crate::Result;
use crate::tracer::types::{HttpSpan, SpanDirection, TraceSession, TraceStatistics};
use parking_lot::RwLock;
use std::collections::HashMap;

/// In-memory storage backend
pub struct MemoryStore {
    sessions: RwLock<HashMap<String, TraceSession>>,
    spans: RwLock<HashMap<String, Vec<HttpSpan>>>,
}

impl MemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            spans: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self, session: &TraceSession) -> Result<()> {
        let mut sessions = self.sessions.write();
        sessions.insert(session.id.clone(), session.clone());

        let mut spans = self.spans.write();
        spans.insert(session.id.clone(), Vec::new());

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<TraceSession>> {
        let sessions = self.sessions.read();
        Ok(sessions.get(session_id).cloned())
    }

    pub async fn list_sessions(&self) -> Result<Vec<TraceSession>> {
        let sessions = self.sessions.read();
        let mut list: Vec<_> = sessions.values().cloned().collect();
        list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(list)
    }

    pub async fn update_session(&self, session: &TraceSession) -> Result<()> {
        let mut sessions = self.sessions.write();
        sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write();
        sessions.remove(session_id);

        let mut spans = self.spans.write();
        spans.remove(session_id);

        Ok(())
    }

    pub async fn store_span(&self, session_id: &str, span: &HttpSpan) -> Result<()> {
        let mut spans = self.spans.write();
        if let Some(session_spans) = spans.get_mut(session_id) {
            session_spans.push(span.clone());
        }
        Ok(())
    }

    pub async fn store_spans(&self, session_id: &str, new_spans: &[HttpSpan]) -> Result<()> {
        let mut spans = self.spans.write();
        if let Some(session_spans) = spans.get_mut(session_id) {
            session_spans.extend(new_spans.iter().cloned());
        }
        Ok(())
    }

    pub async fn get_spans(
        &self,
        session_id: &str,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<HttpSpan>> {
        let spans = self.spans.read();
        if let Some(session_spans) = spans.get(session_id) {
            let start = offset as usize;
            let end = std::cmp::min(start + limit as usize, session_spans.len());
            if start < session_spans.len() {
                return Ok(session_spans[start..end].to_vec());
            }
        }
        Ok(Vec::new())
    }

    pub async fn get_span_count(&self, session_id: &str) -> Result<u64> {
        let spans = self.spans.read();
        Ok(spans.get(session_id).map(|s| s.len() as u64).unwrap_or(0))
    }

    pub async fn get_spans_by_trace_id(
        &self,
        session_id: &str,
        trace_id: &str,
    ) -> Result<Vec<HttpSpan>> {
        let spans = self.spans.read();
        if let Some(session_spans) = spans.get(session_id) {
            return Ok(session_spans
                .iter()
                .filter(|s| s.trace_id == trace_id)
                .cloned()
                .collect());
        }
        Ok(Vec::new())
    }

    pub async fn get_endpoints(&self, session_id: &str) -> Result<Vec<(String, String, u64)>> {
        let spans = self.spans.read();
        if let Some(session_spans) = spans.get(session_id) {
            let mut endpoint_counts: HashMap<(String, String), u64> = HashMap::new();
            for span in session_spans {
                let key = (
                    span.method.clone(),
                    span.route.clone().unwrap_or_else(|| span.path.clone()),
                );
                *endpoint_counts.entry(key).or_insert(0) += 1;
            }
            return Ok(endpoint_counts
                .into_iter()
                .map(|((method, path), count)| (method, path, count))
                .collect());
        }
        Ok(Vec::new())
    }

    pub async fn get_dependencies(&self, session_id: &str) -> Result<Vec<(String, u16, u64)>> {
        let spans = self.spans.read();
        if let Some(session_spans) = spans.get(session_id) {
            let mut dep_counts: HashMap<(String, u16), u64> = HashMap::new();
            for span in session_spans {
                if span.direction == SpanDirection::Outgoing {
                    let key = (span.server_address.clone(), span.server_port);
                    *dep_counts.entry(key).or_insert(0) += 1;
                }
            }
            return Ok(dep_counts
                .into_iter()
                .map(|((host, port), count)| (host, port, count))
                .collect());
        }
        Ok(Vec::new())
    }

    pub async fn update_statistics(&self, session_id: &str, stats: &TraceStatistics) -> Result<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.statistics = stats.clone();
        }
        Ok(())
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_session() -> TraceSession {
        TraceSession {
            id: "test-session".to_string(),
            name: Some("Test Session".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            config: crate::tracer::types::TraceConfig::default(),
            statistics: TraceStatistics::default(),
        }
    }

    fn create_test_span(trace_id: &str, span_id: &str) -> HttpSpan {
        HttpSpan {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100.0,
            service_name: "test-service".to_string(),
            service_version: None,
            direction: SpanDirection::Incoming,
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            route: Some("/api/users".to_string()),
            query: None,
            scheme: "https".to_string(),
            status_code: 200,
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

    #[tokio::test]
    async fn test_session_crud() {
        let store = MemoryStore::new();
        let session = create_test_session();

        // Create
        store.create_session(&session).await.unwrap();

        // Read
        let retrieved = store.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);

        // List
        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete
        store.delete_session(&session.id).await.unwrap();
        let retrieved = store.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_span_storage() {
        let store = MemoryStore::new();
        let session = create_test_session();
        store.create_session(&session).await.unwrap();

        // Store spans
        let span1 = create_test_span("trace1", "span1");
        let span2 = create_test_span("trace1", "span2");
        store.store_span(&session.id, &span1).await.unwrap();
        store.store_span(&session.id, &span2).await.unwrap();

        // Get count
        let count = store.get_span_count(&session.id).await.unwrap();
        assert_eq!(count, 2);

        // Get spans
        let spans = store.get_spans(&session.id, 0, 10).await.unwrap();
        assert_eq!(spans.len(), 2);

        // Get by trace ID
        let trace_spans = store
            .get_spans_by_trace_id(&session.id, "trace1")
            .await
            .unwrap();
        assert_eq!(trace_spans.len(), 2);
    }

    #[tokio::test]
    async fn test_endpoints_and_dependencies() {
        let store = MemoryStore::new();
        let session = create_test_session();
        store.create_session(&session).await.unwrap();

        // Store incoming span
        let mut span1 = create_test_span("trace1", "span1");
        span1.direction = SpanDirection::Incoming;
        store.store_span(&session.id, &span1).await.unwrap();

        // Store outgoing span (dependency)
        let mut span2 = create_test_span("trace1", "span2");
        span2.direction = SpanDirection::Outgoing;
        span2.server_address = "external-api.com".to_string();
        span2.server_port = 443;
        store.store_span(&session.id, &span2).await.unwrap();

        // Get endpoints
        let endpoints = store.get_endpoints(&session.id).await.unwrap();
        assert_eq!(endpoints.len(), 1);

        // Get dependencies
        let deps = store.get_dependencies(&session.id).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "external-api.com");
        assert_eq!(deps[0].1, 443);
    }
}
