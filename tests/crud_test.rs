use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hornet2::server::api;
use std::io::Write;
use tempfile::NamedTempFile;
use tower::util::ServiceExt; // for oneshot

#[tokio::test]
async fn test_crud_api() {
    // Setup a temporary Arazzo file
    let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
sourceDescriptions: []
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        operationId: getTest
"#;
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_path_buf();
    let temp_path_str = temp_path.to_str().unwrap().to_string();

    // Setup state
    let state = api::AppState {
        arazzo_path: temp_path_str.clone(),
        openapi_path: None,
    };

    // Build app directly (bypassing start_server to avoid binding ports)
    let app = axum::Router::new()
        .route(
            "/api/spec",
            axum::routing::get(api::get_spec).put(api::update_spec),
        )
        .route("/api/workflows", axum::routing::get(api::get_workflows))
        .route(
            "/api/workflows/{workflow_id}",
            axum::routing::get(api::get_workflow)
                .put(api::update_workflow)
                .delete(api::delete_workflow),
        )
        .with_state(state);

    // 1. GET /api/spec
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/spec")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(spec["info"]["title"], "Test Workflow");

    // 2. GET /api/workflows/test-flow
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workflows/test-flow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 3. PUT /api/workflows/new-flow
    let new_workflow = serde_json::json!({
        "workflowId": "new-flow",
        "summary": "New Workflow",
        "steps": [
            {
                "stepId": "step1",
                "operationId": "someOp"
            }
        ]
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/workflows/new-flow")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&new_workflow).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it was written to file
    let content = std::fs::read_to_string(&temp_path).unwrap();
    assert!(content.contains("new-flow"));

    // 4. DELETE /api/workflows/test-flow
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/workflows/test-flow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it was removed from file
    let content = std::fs::read_to_string(&temp_path).unwrap();
    assert!(!content.contains("test-flow"));
    assert!(content.contains("new-flow"));
}
