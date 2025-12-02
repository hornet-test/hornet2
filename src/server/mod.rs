pub mod api;

use axum::{
    Router,
    routing::get,
    http::StatusCode,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

/// Start the web server
pub async fn start_server(
    addr: SocketAddr,
    arazzo_path: String,
    openapi_path: Option<String>,
) -> crate::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create shared state
    let state = api::AppState {
        arazzo_path,
        openapi_path,
    };

    // Build the router
    let app = Router::new()
        // API routes
        .route("/api/workflows", get(api::get_workflows))
        .route("/api/graph/{workflow_id}", get(api::get_graph))
        // Static files (HTML, CSS, JS)
        .route("/static/{*path}", get(serve_static))
        // Root route serves index.html
        .route("/", get(serve_index))
        .with_state(state)
        .layer(CorsLayer::permissive());

    tracing::info!("Starting server on http://{}", addr);
    tracing::info!("Open http://{} in your browser", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve static files (CSS, JS)
async fn serve_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let file_path = format!("ui/{}", path);

    match tokio::fs::read(&file_path).await {
        Ok(content) => {
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream();

            Ok(axum::response::Response::builder()
                .header("Content-Type", mime.as_ref())
                .body(axum::body::Body::from(content))
                .unwrap())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Serve index.html
async fn serve_index() -> Result<axum::response::Html<String>, StatusCode> {
    match tokio::fs::read_to_string("ui/index.html").await {
        Ok(content) => Ok(axum::response::Html(content)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
