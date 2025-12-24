pub mod api;

use axum::{http::StatusCode, routing::get, Router};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

/// Webサーバーを起動する
pub async fn start_server(
    addr: SocketAddr,
    arazzo_path: String,
    openapi_path: Option<String>,
) -> crate::Result<()> {
    // トレーシングを初期化
    tracing_subscriber::fmt::init();

    // 共有状態を作成
    let state = api::AppState {
        arazzo_path,
        openapi_path,
    };

    // ルーターを構築
    let app = Router::new()
        // APIルート
        .route("/api/spec", get(api::get_spec).put(api::update_spec))
        .route("/api/workflows", get(api::get_workflows))
        .route(
            "/api/workflows/{workflow_id}",
            get(api::get_workflow)
                .put(api::update_workflow)
                .delete(api::delete_workflow),
        )
        .route("/api/graph/{workflow_id}", get(api::get_graph))
        // 静的ファイル (CSS, JS) - distフォルダから
        .route("/assets/{*path}", get(serve_static))
        // ルートルートはindex.htmlを提供
        .route("/", get(serve_index))
        // SPAルーティングのフォールバック - 他のすべてのルートに対してindex.htmlを提供
        .fallback(serve_index)
        .with_state(state)
        .layer(CorsLayer::permissive());

    tracing::info!("Starting server on http://{}", addr);
    tracing::info!("Open http://{} in your browser", addr);

    // サーバーを起動
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
        <ul>
            <li><code>GET/PUT /api/spec</code> - Get/Update full spec</li>
            <li><code>GET /api/workflows</code> - List all workflows</li>
            <li><code>GET/PUT/DELETE /api/workflows/{workflow_id}</code> - Manage workflows</li>
            <li><code>GET /api/graph/{workflow_id}</code> - Get workflow graph</li>
        </ul>
    </div>
</body>
</html>
            "#;
            Ok(axum::response::Html(dev_message.to_string()))
        }
    }
}
