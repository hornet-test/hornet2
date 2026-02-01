//! Trace storage module.
//!
//! Provides persistent storage for trace sessions and spans.

mod memory;
mod sqlite;

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

use super::types::{HttpSpan, TraceSession, TraceStatistics};
use crate::Result;

/// Trace store backend enum
enum StoreBackend {
    Memory(MemoryStore),
    Sqlite(SqliteStore),
}

/// Main trace store
pub struct TraceStore {
    backend: StoreBackend,
}

impl TraceStore {
    /// Create a new trace store with SQLite backend
    pub fn new_sqlite(path: std::path::PathBuf) -> Result<Self> {
        let backend = SqliteStore::new(path)?;
        Ok(Self {
            backend: StoreBackend::Sqlite(backend),
        })
    }

    /// Create a new trace store with in-memory backend (for testing)
    pub fn new_memory() -> Self {
        Self {
            backend: StoreBackend::Memory(MemoryStore::new()),
        }
    }

    /// Create a new trace session
    pub async fn create_session(&self, session: &TraceSession) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.create_session(session).await,
            StoreBackend::Sqlite(store) => store.create_session(session).await,
        }
    }

    /// Get a trace session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<TraceSession>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_session(session_id).await,
            StoreBackend::Sqlite(store) => store.get_session(session_id).await,
        }
    }

    /// List all trace sessions
    pub async fn list_sessions(&self) -> Result<Vec<TraceSession>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.list_sessions().await,
            StoreBackend::Sqlite(store) => store.list_sessions().await,
        }
    }

    /// Update session
    pub async fn update_session(&self, session: &TraceSession) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.update_session(session).await,
            StoreBackend::Sqlite(store) => store.update_session(session).await,
        }
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.delete_session(session_id).await,
            StoreBackend::Sqlite(store) => store.delete_session(session_id).await,
        }
    }

    /// Store a span
    pub async fn store_span(&self, session_id: &str, span: &HttpSpan) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.store_span(session_id, span).await,
            StoreBackend::Sqlite(store) => store.store_span(session_id, span).await,
        }
    }

    /// Store multiple spans
    pub async fn store_spans(&self, session_id: &str, spans: &[HttpSpan]) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.store_spans(session_id, spans).await,
            StoreBackend::Sqlite(store) => store.store_spans(session_id, spans).await,
        }
    }

    /// Get spans with pagination
    pub async fn get_spans(
        &self,
        session_id: &str,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<HttpSpan>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_spans(session_id, offset, limit).await,
            StoreBackend::Sqlite(store) => store.get_spans(session_id, offset, limit).await,
        }
    }

    /// Get span count
    pub async fn get_span_count(&self, session_id: &str) -> Result<u64> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_span_count(session_id).await,
            StoreBackend::Sqlite(store) => store.get_span_count(session_id).await,
        }
    }

    /// Get spans by trace ID
    pub async fn get_spans_by_trace_id(
        &self,
        session_id: &str,
        trace_id: &str,
    ) -> Result<Vec<HttpSpan>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_spans_by_trace_id(session_id, trace_id).await,
            StoreBackend::Sqlite(store) => store.get_spans_by_trace_id(session_id, trace_id).await,
        }
    }

    /// Get unique endpoints
    pub async fn get_endpoints(&self, session_id: &str) -> Result<Vec<(String, String, u64)>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_endpoints(session_id).await,
            StoreBackend::Sqlite(store) => store.get_endpoints(session_id).await,
        }
    }

    /// Get dependencies
    pub async fn get_dependencies(&self, session_id: &str) -> Result<Vec<(String, u16, u64)>> {
        match &self.backend {
            StoreBackend::Memory(store) => store.get_dependencies(session_id).await,
            StoreBackend::Sqlite(store) => store.get_dependencies(session_id).await,
        }
    }

    /// Update statistics
    pub async fn update_statistics(&self, session_id: &str, stats: &TraceStatistics) -> Result<()> {
        match &self.backend {
            StoreBackend::Memory(store) => store.update_statistics(session_id, stats).await,
            StoreBackend::Sqlite(store) => store.update_statistics(session_id, stats).await,
        }
    }
}
