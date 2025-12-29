use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use hornet2::server::{api, state::AppState};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tempfile::TempDir;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_crud_api_multi_project() {
    // Setup a temporary directory acting as root_dir
    let temp_dir = TempDir::new().unwrap();
    let root_path = temp_dir.path().to_path_buf();

    // Create a project directory "project1"
    let project_dir = root_path.join("project1");
    std::fs::create_dir(&project_dir).unwrap();

    // Create arazzo.yaml in project1
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
    std::fs::write(project_dir.join("arazzo.yaml"), yaml).unwrap();

    // Setup state
    let state = AppState {
        root_dir: root_path.clone(),
        projects: Arc::new(RwLock::new(hornet2::server::state::ProjectCache::new(
            Duration::from_secs(60),
        ))),
    };

    // Build app with new multi-project API structure
    // Replicating routes from src/server/mod.rs
    let app = Router::new()
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
        .with_state(state);

    // 1. GET /api/projects/project1/arazzo - Get full specification
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/project1/arazzo")
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

    // 2. GET /api/projects/project1/workflows - List all workflows
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/project1/workflows")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let workflows: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(workflows["workflows"][0]["workflow_id"], "test-flow");

    // 3. GET /api/projects/project1/workflows/test-flow - Get specific workflow
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/project1/workflows/test-flow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let workflow: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(workflow["workflowId"], "test-flow");

    // 4. POST /api/projects/project1/workflows - Create new workflow
    let new_workflow_req = serde_json::json!({
        "workflow": {
            "workflowId": "new-flow",
            "summary": "New Workflow",
            "steps": [
                {
                    "stepId": "step1",
                    "operationId": "someOp"
                }
            ]
        }
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/project1/workflows")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&new_workflow_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify it was written to file
    let content = std::fs::read_to_string(project_dir.join("arazzo.yaml")).unwrap();
    assert!(content.contains("new-flow"));

    // 5. PUT /api/projects/project1/workflows/test-flow - Update workflow
    let updated_workflow = serde_json::json!({
        "workflowId": "test-flow",
        "summary": "Updated Workflow",
        "steps": [
            {
                "stepId": "step1",
                "operationId": "getTest"
            }
        ]
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/projects/project1/workflows/test-flow")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&updated_workflow).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. DELETE /api/projects/project1/workflows/test-flow
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/projects/project1/workflows/test-flow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify it was removed from file
    let content = std::fs::read_to_string(project_dir.join("arazzo.yaml")).unwrap();
    assert!(!content.contains("test-flow"));
    assert!(content.contains("new-flow"));
}
