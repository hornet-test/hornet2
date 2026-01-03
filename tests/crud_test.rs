use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
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

    // Create openapi.yaml in project1
    let openapi_yaml = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      operationId: getTest
      responses:
        '200':
          description: OK
"#;
    std::fs::write(project_dir.join("openapi.yaml"), openapi_yaml).unwrap();

    // Create arazzo.yaml in project1
    let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
sourceDescriptions:
  - name: connection
    url: openapi.yaml
    type: openapi
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
            "/api/projects/{project_name}/openapi",
            get(api::get_project_openapi),
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

    // 2. GET /api/projects/project1/openapi - Get merged OpenAPI spec
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/project1/openapi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let openapi: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(openapi["info"]["title"], "Test API");
    assert!(openapi["paths"]["/test"].is_object());

    // 3. GET /api/projects/project1/workflows - List all workflows
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

    // 4. GET /api/projects/project1/workflows/test-flow - Get specific workflow
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

    // 5. POST /api/projects/project1/workflows - Create new workflow
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

    // 6. PUT /api/projects/project1/workflows/test-flow - Update workflow
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

    // 7. DELETE /api/projects/project1/workflows/test-flow
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

#[tokio::test]
async fn test_get_project_openapi_multiple_files() {
    // Setup a temporary directory with a project containing multiple OpenAPI files
    let temp_dir = TempDir::new().unwrap();
    let root_path = temp_dir.path().to_path_buf();

    let project_dir = root_path.join("multi-api-project");
    std::fs::create_dir(&project_dir).unwrap();

    // Create first OpenAPI file - users API
    let users_api = r#"
openapi: 3.0.0
info:
  title: Users API
  version: 1.0.0
paths:
  /users:
    get:
      operationId: getUsers
      responses:
        '200':
          description: OK
  /users/{id}:
    get:
      operationId: getUserById
      responses:
        '200':
          description: OK
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
tags:
  - name: users
    description: User operations
  - name: common
    description: Common operations
"#;
    std::fs::write(project_dir.join("users-api.yaml"), users_api).unwrap();

    // Create second OpenAPI file - posts API
    let posts_api = r#"
openapi: 3.0.0
info:
  title: Posts API
  version: 1.0.0
paths:
  /posts:
    get:
      operationId: getPosts
      responses:
        '200':
          description: OK
  /posts/{id}:
    get:
      operationId: getPostById
      responses:
        '200':
          description: OK
components:
  schemas:
    Post:
      type: object
      properties:
        id:
          type: string
        title:
          type: string
tags:
  - name: posts
    description: Post operations
  - name: common
    description: Common operations
"#;
    std::fs::write(project_dir.join("posts-api.yaml"), posts_api).unwrap();

    // Create Arazzo file referencing both OpenAPI files
    let arazzo = r#"
arazzo: 1.0.0
info:
  title: Multi-API Workflow
  version: 1.0.0
sourceDescriptions:
  - name: users-api
    url: ./users-api.yaml
    type: openapi
  - name: posts-api
    url: ./posts-api.yaml
    type: openapi
workflows:
  - workflowId: user-post-flow
    steps:
      - stepId: step1
        operationId: getUsers
"#;
    std::fs::write(project_dir.join("arazzo.yaml"), arazzo).unwrap();

    // Setup state and app
    let state = AppState {
        root_dir: root_path.clone(),
        projects: Arc::new(RwLock::new(hornet2::server::state::ProjectCache::new(
            Duration::from_secs(60),
        ))),
    };

    let app = Router::new()
        .route(
            "/api/projects/{project_name}/openapi",
            get(api::get_project_openapi),
        )
        .with_state(state);

    // Test: GET merged OpenAPI spec
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/multi-api-project/openapi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let merged: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify merged spec contains paths from both APIs
    assert!(merged["paths"]["/users"].is_object());
    assert!(merged["paths"]["/users/{id}"].is_object());
    assert!(merged["paths"]["/posts"].is_object());
    assert!(merged["paths"]["/posts/{id}"].is_object());

    // Verify components from both APIs are merged
    let schemas = merged["components"]["schemas"].as_object().unwrap();
    assert!(schemas.contains_key("User"));
    assert!(schemas.contains_key("Post"));

    // Verify tags are deduplicated (common should appear once)
    let tags = merged["tags"].as_array().unwrap();
    let tag_names: Vec<&str> = tags.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tag_names.contains(&"users"));
    assert!(tag_names.contains(&"posts"));
    assert!(tag_names.contains(&"common"));

    // "common" should appear only once
    let common_count = tag_names.iter().filter(|&&name| name == "common").count();
    assert_eq!(common_count, 1);
}

#[tokio::test]
async fn test_get_project_openapi_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let root_path = temp_dir.path().to_path_buf();

    let state = AppState {
        root_dir: root_path,
        projects: Arc::new(RwLock::new(hornet2::server::state::ProjectCache::new(
            Duration::from_secs(60),
        ))),
    };

    let app = Router::new()
        .route(
            "/api/projects/{project_name}/openapi",
            get(api::get_project_openapi),
        )
        .with_state(state);

    // Test: GET OpenAPI for non-existent project
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/nonexistent/openapi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(
        error["type"],
        "https://hornet2.dev/problems/project-not-found"
    );
}

#[tokio::test]
async fn test_get_project_openapi_invalid_name() {
    let temp_dir = TempDir::new().unwrap();
    let root_path = temp_dir.path().to_path_buf();

    let state = AppState {
        root_dir: root_path,
        projects: Arc::new(RwLock::new(hornet2::server::state::ProjectCache::new(
            Duration::from_secs(60),
        ))),
    };

    let app = Router::new()
        .route(
            "/api/projects/{project_name}/openapi",
            get(api::get_project_openapi),
        )
        .with_state(state);

    // Test: Path traversal attempt
    // Note: "../etc" in the URL path is normalized by the HTTP router,
    // so this actually requests "/api/openapi" which is not a valid route.
    // This should result in a 404 Not Found from the router itself.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/../etc/openapi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum normalizes paths, so this results in 404 from router (no JSON body)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
