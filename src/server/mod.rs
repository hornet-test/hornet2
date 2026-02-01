pub mod api;
pub mod lsp;
pub mod state;
pub mod trace_api;

use axum::{Router, http::StatusCode, routing::get};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::LatencyUnit;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

use state::AppState;
pub use trace_api::TraceState;

/// Webサーバーを起動する（マルチプロジェクトモード）
pub async fn start_server(addr: SocketAddr, root_dir: PathBuf) -> crate::Result<()> {
    // .envファイルを読み込む（エラーは無視）
    dotenv::dotenv().ok();

    // トレーシングとOpenTelemetryを初期化
    let _telemetry_guard = crate::telemetry::init_telemetry()?;

    // 共有状態を作成
    let state = AppState::new(root_dir.clone())?;

    // トレース状態を作成（オプション）
    let trace_store_path = root_dir.join(".hornet2/traces");
    let trace_state = trace_api::TraceState::new(trace_store_path).ok();

    // ルーターを構築
    let mut app = Router::new()
        // Multi-Project API
        .route("/api/projects", get(api::list_projects))
        .route("/api/projects/{project_name}", get(api::get_project))
        .route(
            "/api/projects/{project_name}/arazzo",
            get(api::get_project_arazzo).put(api::update_project_arazzo),
        )
        .route(
            "/api/projects/{project_name}/workflows",
            get(api::get_project_workflows).post(api::create_project_workflow),
        )
        .route(
            "/api/projects/{project_name}/workflows/{workflow_id}",
            get(api::get_project_workflow)
                .put(api::update_project_workflow)
                .delete(api::delete_project_workflow),
        )
        .route(
            "/api/projects/{project_name}/graph/{workflow_id}",
            get(api::get_project_graph),
        )
        // Editor API
        .route(
            "/api/projects/{project_name}/operations",
            get(api::get_project_operations),
        )
        .route(
            "/api/projects/{project_name}/openapi",
            get(api::get_project_openapi),
        )
        .route("/api/validate", axum::routing::post(api::validate_arazzo))
        .route("/api/openapi.json", get(api::get_openapi_spec))
        .route("/api/arazzo.json", get(api::get_arazzo_spec))
        // LSP
        .route("/lsp", get(lsp::lsp_handler))
        // Static files (CSS, JS) - from dist folder
        .route("/assets/{*path}", get(serve_static))
        // ルートルートはindex.htmlを提供
        .route("/", get(serve_index))
        // SPAルーティングのフォールバック - 他のすべてのルートに対してindex.htmlを提供
        .fallback(serve_index)
        .with_state(state);

    // Add trace API if store was created successfully
    if let Some(ts) = trace_state {
        let trace_router = trace_api::trace_router(ts.clone());
        // Also add the OTLP receiver endpoint
        let otlp_router = ts.receiver.http_router();

        app = app
            .nest("/api/trace", trace_router)
            .nest("/otlp", otlp_router);

        tracing::info!("Trace API enabled at /api/trace");
        tracing::info!("OTLP receiver enabled at /otlp/v1/traces");
    }

    let app = app
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(tracing::Level::INFO)
                        .include_headers(false),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(CorsLayer::permissive());

    tracing::info!("Starting server on http://{}", addr);
    tracing::info!("Open http://{} in your browser", addr);

    // サーバーを起動（graceful shutdown付き）
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Graceful shutdownのためのシグナルハンドラ
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown");
        },
    }
}

/// ui/dist/assets/ から静的ファイル (CSS, JS, etc.) を提供する
async fn serve_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let file_path = format!("ui/dist/assets/{}", path);

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

/// ui/dist/ (本番ビルド) から index.html を提供する
async fn serve_index() -> Result<axum::response::Html<String>, StatusCode> {
    match tokio::fs::read_to_string("ui/dist/index.html").await {
        Ok(content) => Ok(axum::response::Html(content)),
        Err(_) => {
            // dist/index.html が存在しない場合、役立つメッセージを表示
            let dev_message = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Hornet2 - Development Mode</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffc107;
            border-radius: 4px;
            padding: 20px;
            margin: 20px 0;
        }
        .info {
            background: #d1ecf1;
            border: 1px solid #0dcaf0;
            border-radius: 4px;
            padding: 20px;
            margin: 20px 0;
        }
        code {
            background: #f5f5f5;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: "Courier New", monospace;
        }
        pre {
            background: #f5f5f5;
            padding: 15px;
            border-radius: 4px;
            overflow-x: auto;
        }
    </style>
</head>
<body>
    <h1>Hornet2 API Server</h1>

    <div class="warning">
        <h2>⚠️ UI Not Built</h2>
        <p>The UI has not been built yet. The API server is running, but no frontend files were found.</p>
    </div>

    <div class="info">
        <h2>🚀 Quick Start</h2>

        <h3>Development Mode (Recommended)</h3>
        <p>Run both the API server and UI dev server simultaneously:</p>
        <pre>make dev</pre>
        <p>Then open <a href="http://localhost:5173">http://localhost:5173</a> in your browser.</p>

        <h3>Production Mode</h3>
        <p>Build the UI first, then start the server:</p>
        <pre>cd ui && pnpm build
cargo run -- serve --arazzo tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml</pre>
        <p>Then open <a href="http://localhost:3000">http://localhost:3000</a> in your browser.</p>
    </div>

    <div class="info">
        <h2>📡 API Endpoints</h2>
        <p>The following API endpoints are available:</p>
        <h3>Multi-Project API</h3>
        <ul>
            <li><code>GET /api/projects</code> - List all projects</li>
            <li><code>GET /api/projects/{project_name}</code> - Get project details</li>
            <li><code>GET /api/projects/{project_name}/arazzo</code> - Get Arazzo specification</li>
            <li><code>PUT /api/projects/{project_name}/arazzo</code> - Update Arazzo specification</li>
            <li><code>GET /api/projects/{project_name}/workflows</code> - List all workflows</li>
            <li><code>POST /api/projects/{project_name}/workflows</code> - Create new workflow</li>
            <li><code>GET /api/projects/{project_name}/workflows/{id}</code> - Get specific workflow</li>
            <li><code>PUT /api/projects/{project_name}/workflows/{id}</code> - Update specific workflow</li>
            <li><code>DELETE /api/projects/{project_name}/workflows/{id}</code> - Delete specific workflow</li>
            <li><code>GET /api/projects/{project_name}/graph/{id}</code> - Get workflow graph visualization</li>
            <li><code>GET /api/openapi.json</code> - Get API specification (OpenAPI 3.0.3)</li>
            <li><code>GET /api/arazzo.json</code> - Get Arazzo workflow specification (Arazzo 1.0.0)</li>
        </ul>
    </div>
</body>
</html>
            "#;
            Ok(axum::response::Html(dev_message.to_string()))
        }
    }
}
