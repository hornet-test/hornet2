//! SQLite-based trace storage backend.

use crate::tracer::types::{HttpSpan, SpanDirection, TraceSession, TraceStatistics};
use crate::{HornetError, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::Arc;

/// SQLite storage backend
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl SqliteStore {
    /// Create a new SQLite store
    pub fn new(path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path).map_err(|e| {
            HornetError::StorageError(format!("Failed to open SQLite database: {}", e))
        })?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        store.initialize_schema()?;
        Ok(store)
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                config_json TEXT NOT NULL,
                statistics_json TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);

            CREATE TABLE IF NOT EXISTS spans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                span_id TEXT NOT NULL,
                parent_span_id TEXT,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                duration_ms REAL NOT NULL,
                service_name TEXT NOT NULL,
                service_version TEXT,
                direction TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                route TEXT,
                query TEXT,
                scheme TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                server_address TEXT NOT NULL,
                server_port INTEGER NOT NULL,
                client_address TEXT,
                request_body_json TEXT,
                response_body_json TEXT,
                request_headers_json TEXT,
                response_headers_json TEXT,
                attributes_json TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_spans_session_id ON spans(session_id);
            CREATE INDEX IF NOT EXISTS idx_spans_trace_id ON spans(trace_id);
            CREATE INDEX IF NOT EXISTS idx_spans_direction ON spans(direction);
            CREATE INDEX IF NOT EXISTS idx_spans_method_path ON spans(method, path);
            CREATE INDEX IF NOT EXISTS idx_spans_server ON spans(server_address, server_port);
            "#,
        )
        .map_err(|e| HornetError::StorageError(format!("Failed to initialize schema: {}", e)))?;

        Ok(())
    }

    pub async fn create_session(&self, session: &TraceSession) -> Result<()> {
        let conn = self.conn.lock();

        let config_json = serde_json::to_string(&session.config)
            .map_err(|e| HornetError::StorageError(format!("Failed to serialize config: {}", e)))?;
        let stats_json = serde_json::to_string(&session.statistics).map_err(|e| {
            HornetError::StorageError(format!("Failed to serialize statistics: {}", e))
        })?;

        conn.execute(
            "INSERT INTO sessions (id, name, started_at, ended_at, config_json, statistics_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.name,
                session.started_at.to_rfc3339(),
                session.ended_at.map(|t| t.to_rfc3339()),
                config_json,
                stats_json,
            ],
        )
        .map_err(|e| HornetError::StorageError(format!("Failed to create session: {}", e)))?;

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<TraceSession>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT id, name, started_at, ended_at, config_json, statistics_json
                 FROM sessions WHERE id = ?1",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let result = stmt
            .query_row(params![session_id], |row| {
                Ok(SessionRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    config_json: row.get(4)?,
                    statistics_json: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| HornetError::StorageError(format!("Failed to query session: {}", e)))?;

        match result {
            Some(row) => Ok(Some(row.into_session()?)),
            None => Ok(None),
        }
    }

    pub async fn list_sessions(&self) -> Result<Vec<TraceSession>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT id, name, started_at, ended_at, config_json, statistics_json
                 FROM sessions ORDER BY started_at DESC",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(SessionRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    config_json: row.get(4)?,
                    statistics_json: row.get(5)?,
                })
            })
            .map_err(|e| HornetError::StorageError(format!("Failed to query sessions: {}", e)))?;

        let mut sessions = Vec::new();
        for row in rows {
            let row =
                row.map_err(|e| HornetError::StorageError(format!("Failed to read row: {}", e)))?;
            sessions.push(row.into_session()?);
        }

        Ok(sessions)
    }

    pub async fn update_session(&self, session: &TraceSession) -> Result<()> {
        let conn = self.conn.lock();

        let stats_json = serde_json::to_string(&session.statistics).map_err(|e| {
            HornetError::StorageError(format!("Failed to serialize statistics: {}", e))
        })?;

        conn.execute(
            "UPDATE sessions SET name = ?1, ended_at = ?2, statistics_json = ?3 WHERE id = ?4",
            params![
                session.name,
                session.ended_at.map(|t| t.to_rfc3339()),
                stats_json,
                session.id,
            ],
        )
        .map_err(|e| HornetError::StorageError(format!("Failed to update session: {}", e)))?;

        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock();

        // Delete spans first (foreign key)
        conn.execute("DELETE FROM spans WHERE session_id = ?1", params![session_id])
            .map_err(|e| HornetError::StorageError(format!("Failed to delete spans: {}", e)))?;

        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .map_err(|e| HornetError::StorageError(format!("Failed to delete session: {}", e)))?;

        Ok(())
    }

    pub async fn store_span(&self, session_id: &str, span: &HttpSpan) -> Result<()> {
        let conn = self.conn.lock();
        insert_span(&conn, session_id, span)
    }

    pub async fn store_spans(&self, session_id: &str, spans: &[HttpSpan]) -> Result<()> {
        let conn = self.conn.lock();

        for span in spans {
            insert_span(&conn, session_id, span)?;
        }

        Ok(())
    }

    pub async fn get_spans(
        &self,
        session_id: &str,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<HttpSpan>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT trace_id, span_id, parent_span_id, start_time, end_time, duration_ms,
                        service_name, service_version, direction, method, path, route, query,
                        scheme, status_code, server_address, server_port, client_address,
                        request_body_json, response_body_json, request_headers_json,
                        response_headers_json, attributes_json
                 FROM spans WHERE session_id = ?1 ORDER BY start_time LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![session_id, limit, offset], row_to_span)
            .map_err(|e| HornetError::StorageError(format!("Failed to query spans: {}", e)))?;

        let mut spans = Vec::new();
        for row in rows {
            let span =
                row.map_err(|e| HornetError::StorageError(format!("Failed to read span: {}", e)))?;
            spans.push(span?);
        }

        Ok(spans)
    }

    pub async fn get_span_count(&self, session_id: &str) -> Result<u64> {
        let conn = self.conn.lock();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM spans WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to count spans: {}", e)))?;

        Ok(count as u64)
    }

    pub async fn get_spans_by_trace_id(
        &self,
        session_id: &str,
        trace_id: &str,
    ) -> Result<Vec<HttpSpan>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT trace_id, span_id, parent_span_id, start_time, end_time, duration_ms,
                        service_name, service_version, direction, method, path, route, query,
                        scheme, status_code, server_address, server_port, client_address,
                        request_body_json, response_body_json, request_headers_json,
                        response_headers_json, attributes_json
                 FROM spans WHERE session_id = ?1 AND trace_id = ?2 ORDER BY start_time",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![session_id, trace_id], row_to_span)
            .map_err(|e| HornetError::StorageError(format!("Failed to query spans: {}", e)))?;

        let mut spans = Vec::new();
        for row in rows {
            let span =
                row.map_err(|e| HornetError::StorageError(format!("Failed to read span: {}", e)))?;
            spans.push(span?);
        }

        Ok(spans)
    }

    pub async fn get_endpoints(&self, session_id: &str) -> Result<Vec<(String, String, u64)>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT method, COALESCE(route, path) as endpoint, COUNT(*) as count
                 FROM spans WHERE session_id = ?1
                 GROUP BY method, endpoint ORDER BY count DESC",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u64,
                ))
            })
            .map_err(|e| HornetError::StorageError(format!("Failed to query endpoints: {}", e)))?;

        let mut endpoints = Vec::new();
        for row in rows {
            endpoints.push(row.map_err(|e| {
                HornetError::StorageError(format!("Failed to read endpoint: {}", e))
            })?);
        }

        Ok(endpoints)
    }

    pub async fn get_dependencies(&self, session_id: &str) -> Result<Vec<(String, u16, u64)>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare(
                "SELECT server_address, server_port, COUNT(*) as count
                 FROM spans WHERE session_id = ?1 AND direction = 'outgoing'
                 GROUP BY server_address, server_port ORDER BY count DESC",
            )
            .map_err(|e| HornetError::StorageError(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u16,
                    row.get::<_, i64>(2)? as u64,
                ))
            })
            .map_err(|e| {
                HornetError::StorageError(format!("Failed to query dependencies: {}", e))
            })?;

        let mut deps = Vec::new();
        for row in rows {
            deps.push(row.map_err(|e| {
                HornetError::StorageError(format!("Failed to read dependency: {}", e))
            })?);
        }

        Ok(deps)
    }

    pub async fn update_statistics(&self, session_id: &str, stats: &TraceStatistics) -> Result<()> {
        let conn = self.conn.lock();

        let stats_json = serde_json::to_string(stats).map_err(|e| {
            HornetError::StorageError(format!("Failed to serialize statistics: {}", e))
        })?;

        conn.execute(
            "UPDATE sessions SET statistics_json = ?1 WHERE id = ?2",
            params![stats_json, session_id],
        )
        .map_err(|e| HornetError::StorageError(format!("Failed to update statistics: {}", e)))?;

        Ok(())
    }
}

// Helper structs and functions

struct SessionRow {
    id: String,
    name: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    config_json: String,
    statistics_json: String,
}

impl SessionRow {
    fn into_session(self) -> Result<TraceSession> {
        use chrono::DateTime;

        let started_at = DateTime::parse_from_rfc3339(&self.started_at)
            .map_err(|e| HornetError::StorageError(format!("Invalid started_at: {}", e)))?
            .with_timezone(&chrono::Utc);

        let ended_at = self
            .ended_at
            .map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| HornetError::StorageError(format!("Invalid ended_at: {}", e)))
            })
            .transpose()?;

        let config = serde_json::from_str(&self.config_json)
            .map_err(|e| HornetError::StorageError(format!("Invalid config: {}", e)))?;

        let statistics = serde_json::from_str(&self.statistics_json)
            .map_err(|e| HornetError::StorageError(format!("Invalid statistics: {}", e)))?;

        Ok(TraceSession {
            id: self.id,
            name: self.name,
            started_at,
            ended_at,
            config,
            statistics,
        })
    }
}

fn insert_span(conn: &Connection, session_id: &str, span: &HttpSpan) -> Result<()> {
    let request_body_json = span
        .request_body
        .as_ref()
        .map(|b| serde_json::to_string(b).ok())
        .flatten();
    let response_body_json = span
        .response_body
        .as_ref()
        .map(|b| serde_json::to_string(b).ok())
        .flatten();
    let request_headers_json =
        serde_json::to_string(&span.request_headers).unwrap_or_else(|_| "{}".to_string());
    let response_headers_json =
        serde_json::to_string(&span.response_headers).unwrap_or_else(|_| "{}".to_string());
    let attributes_json =
        serde_json::to_string(&span.attributes).unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        "INSERT INTO spans (session_id, trace_id, span_id, parent_span_id, start_time, end_time,
         duration_ms, service_name, service_version, direction, method, path, route, query,
         scheme, status_code, server_address, server_port, client_address, request_body_json,
         response_body_json, request_headers_json, response_headers_json, attributes_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                 ?19, ?20, ?21, ?22, ?23, ?24)",
        params![
            session_id,
            span.trace_id,
            span.span_id,
            span.parent_span_id,
            span.start_time.to_rfc3339(),
            span.end_time.to_rfc3339(),
            span.duration_ms,
            span.service_name,
            span.service_version,
            span.direction.as_str(),
            span.method,
            span.path,
            span.route,
            span.query,
            span.scheme,
            span.status_code,
            span.server_address,
            span.server_port,
            span.client_address,
            request_body_json,
            response_body_json,
            request_headers_json,
            response_headers_json,
            attributes_json,
        ],
    )
    .map_err(|e| HornetError::StorageError(format!("Failed to insert span: {}", e)))?;

    Ok(())
}

fn row_to_span(row: &rusqlite::Row) -> rusqlite::Result<Result<HttpSpan>> {
    use chrono::DateTime;
    use std::collections::HashMap;

    let trace_id: String = row.get(0)?;
    let span_id: String = row.get(1)?;
    let parent_span_id: Option<String> = row.get(2)?;
    let start_time_str: String = row.get(3)?;
    let end_time_str: String = row.get(4)?;
    let duration_ms: f64 = row.get(5)?;
    let service_name: String = row.get(6)?;
    let service_version: Option<String> = row.get(7)?;
    let direction_str: String = row.get(8)?;
    let method: String = row.get(9)?;
    let path: String = row.get(10)?;
    let route: Option<String> = row.get(11)?;
    let query: Option<String> = row.get(12)?;
    let scheme: String = row.get(13)?;
    let status_code: i64 = row.get(14)?;
    let server_address: String = row.get(15)?;
    let server_port: i64 = row.get(16)?;
    let client_address: Option<String> = row.get(17)?;
    let request_body_json: Option<String> = row.get(18)?;
    let response_body_json: Option<String> = row.get(19)?;
    let request_headers_json: Option<String> = row.get(20)?;
    let response_headers_json: Option<String> = row.get(21)?;
    let attributes_json: Option<String> = row.get(22)?;

    let start_time = DateTime::parse_from_rfc3339(&start_time_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let end_time = DateTime::parse_from_rfc3339(&end_time_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let direction = match direction_str.as_str() {
        "incoming" => SpanDirection::Incoming,
        "outgoing" => SpanDirection::Outgoing,
        _ => SpanDirection::Incoming,
    };

    let request_body = request_body_json.and_then(|s| serde_json::from_str(&s).ok());
    let response_body = response_body_json.and_then(|s| serde_json::from_str(&s).ok());
    let request_headers: HashMap<String, String> = request_headers_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let response_headers: HashMap<String, String> = response_headers_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let attributes: HashMap<String, serde_json::Value> = attributes_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(Ok(HttpSpan {
        trace_id,
        span_id,
        parent_span_id,
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
        status_code: status_code as u16,
        server_address,
        server_port: server_port as u16,
        client_address,
        request_body,
        response_body,
        request_headers,
        response_headers,
        attributes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

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

    fn create_test_span() -> HttpSpan {
        HttpSpan {
            trace_id: "trace1".to_string(),
            span_id: "span1".to_string(),
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
            request_headers: std::collections::HashMap::new(),
            response_headers: std::collections::HashMap::new(),
            attributes: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    pub async fn test_sqlite_session_crud() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(db_path).unwrap();

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
    pub async fn test_sqlite_span_storage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(db_path).unwrap();

        let session = create_test_session();
        store.create_session(&session).await.unwrap();

        let span = create_test_span();
        store.store_span(&session.id, &span).await.unwrap();

        let count = store.get_span_count(&session.id).await.unwrap();
        assert_eq!(count, 1);

        let spans = store.get_spans(&session.id, 0, 10).await.unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].trace_id, "trace1");
    }
}
