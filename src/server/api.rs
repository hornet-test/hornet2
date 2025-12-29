use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    graph::{builder::build_flow_graph, exporter::export_json},
    loader,
    models::arazzo::{ArazzoSpec, Workflow},
    server::state::AppState,
};

/// RFC 9457 Problem Details for HTTP APIs
/// https://datatracker.ietf.org/doc/html/rfc9457
#[derive(Debug, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type
    #[serde(rename = "type")]
    pub type_: String,
    /// A short, human-readable summary of the problem type
    pub title: String,
    /// The HTTP status code
    pub status: u16,
    /// A human-readable explanation specific to this occurrence
    pub detail: String,
    /// A URI reference that identifies the specific occurrence of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemDetails {
    fn new(status: StatusCode, type_suffix: &str, title: &str, detail: String) -> Self {
        Self {
            type_: format!("https://hornet2.dev/problems/{}", type_suffix),
            title: title.to_string(),
            status: status.as_u16(),
            detail,
            instance: None,
        }
    }

    #[allow(dead_code)]
    fn with_instance(mut self, instance: String) -> Self {
        self.instance = Some(instance);
        self
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(self);
        (
            status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}

/// エラーレスポンスを作成するヘルパー
type ApiResult<T> = Result<T, ProblemDetails>;

/// GET /api/arazzo/workflows のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowsResponse {
    pub workflows: Vec<WorkflowInfo>,
}

/// ワークフロー情報（一覧表示用）
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub workflow_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub steps: usize,
}

/// POST /api/arazzo/workflows のリクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkflowRequest {
    pub workflow: Workflow,
}

// ============================================================================
// Multi-Project API Response Types
// ============================================================================

/// GET /api/projects のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectsResponse {
    pub projects: Vec<ProjectInfo>,
}

/// プロジェクト情報（一覧表示用）
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub workflow_count: usize,
    pub arazzo_path: String,
    pub openapi_files: Vec<String>,
}

/// GET /api/projects/{project_name} のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub workflow_count: usize,
    pub openapi_files: Vec<String>,
}

// ============================================================================
// Legacy Single-File API Endpoints (compatibility)
// ============================================================================

/// GET /api/arazzo - 完全なArazzo仕様を取得
// pub async fn get_spec(State(state): State<AppState>) -> ApiResult<Json<ArazzoSpec>> {
//     let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     Ok(Json(arazzo))
// }

/// PUT /api/arazzo - 完全なArazzo仕様を更新
// pub async fn update_spec(
//     State(state): State<AppState>,
//     Json(spec): Json<ArazzoSpec>,
// ) -> ApiResult<Json<ArazzoSpec>> {
//     loader::save_arazzo(&state.arazzo_path, &spec).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "save-failed",
//             "Failed to Save Resource",
//             format!("Failed to save Arazzo specification: {}", e),
//         )
//     })?;

//     Ok(Json(spec))
// }

/// GET /api/arazzo/workflows - すべてのワークフローをリスト
// pub async fn get_workflows(State(state): State<AppState>) -> ApiResult<Json<WorkflowsResponse>> {
//     let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     let workflows: Vec<WorkflowInfo> = arazzo
//         .workflows
//         .iter()
//         .map(|w| WorkflowInfo {
//             workflow_id: w.workflow_id.clone(),
//             title: w.summary.clone(),
//             description: w.description.clone(),
//             steps: w.steps.len(),
//         })
//         .collect();

//     Ok(Json(WorkflowsResponse { workflows }))
// }

/// POST /api/arazzo/workflows - 新しいワークフローを作成
// pub async fn create_workflow(
//     State(state): State<AppState>,
//     Json(req): Json<CreateWorkflowRequest>,
// ) -> ApiResult<(StatusCode, Json<Workflow>)> {
//     let mut arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     // Check if workflow with same ID already exists
//     if arazzo
//         .workflows
//         .iter()
//         .any(|w| w.workflow_id == req.workflow.workflow_id)
//     {
//         return Err(ProblemDetails::new(
//             StatusCode::CONFLICT,
//             "workflow-exists",
//             "Resource Already Exists",
//             format!(
//                 "Workflow with ID '{}' already exists",
//                 req.workflow.workflow_id
//             ),
//         ));
//     }

//     // Add new workflow
//     arazzo.workflows.push(req.workflow.clone());

//     // Save updated spec
//     loader::save_arazzo(&state.arazzo_path, &arazzo).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "save-failed",
//             "Failed to Save Resource",
//             format!("Failed to save Arazzo specification: {}", e),
//         )
//     })?;

//     Ok((StatusCode::CREATED, Json(req.workflow)))
// }

/// GET /api/arazzo/workflows/{workflow_id} - 特定のワークフローを取得
// pub async fn get_workflow(
//     State(state): State<AppState>,
//     Path(workflow_id): Path<String>,
// ) -> ApiResult<Json<Workflow>> {
//     let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     let workflow = arazzo
//         .workflows
//         .into_iter()
//         .find(|w| w.workflow_id == workflow_id)
//         .ok_or_else(|| {
//             ProblemDetails::new(
//                 StatusCode::NOT_FOUND,
//                 "workflow-not-found",
//                 "Resource Not Found",
//                 format!("Workflow '{}' not found", workflow_id),
//             )
//         })?;

//     Ok(Json(workflow))
// }

/// PUT /api/arazzo/workflows/{workflow_id} - ワークフローを更新
// pub async fn update_workflow(
//     State(state): State<AppState>,
//     Path(workflow_id): Path<String>,
//     Json(workflow): Json<Workflow>,
// ) -> ApiResult<Json<Workflow>> {
//     // Validate workflow ID matches path parameter
//     if workflow.workflow_id != workflow_id {
//         return Err(ProblemDetails::new(
//             StatusCode::BAD_REQUEST,
//             "workflow-id-mismatch",
//             "Invalid Request",
//             format!(
//                 "Workflow ID in path ({}) does not match workflow ID in body ({})",
//                 workflow_id, workflow.workflow_id
//             ),
//         ));
//     }

//     let mut arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     // Find and update existing workflow
//     let pos = arazzo
//         .workflows
//         .iter()
//         .position(|w| w.workflow_id == workflow_id)
//         .ok_or_else(|| {
//             ProblemDetails::new(
//                 StatusCode::NOT_FOUND,
//                 "workflow-not-found",
//                 "Resource Not Found",
//                 format!("Workflow '{}' not found", workflow_id),
//             )
//         })?;

//     arazzo.workflows[pos] = workflow.clone();

//     loader::save_arazzo(&state.arazzo_path, &arazzo).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "save-failed",
//             "Failed to Save Resource",
//             format!("Failed to save Arazzo specification: {}", e),
//         )
//     })?;

//     Ok(Json(workflow))
// }

/// DELETE /api/arazzo/workflows/{workflow_id} - ワークフローを削除
// pub async fn delete_workflow(
//     State(state): State<AppState>,
//     Path(workflow_id): Path<String>,
// ) -> ApiResult<StatusCode> {
//     let mut arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     let initial_len = arazzo.workflows.len();
//     arazzo.workflows.retain(|w| w.workflow_id != workflow_id);

//     if arazzo.workflows.len() == initial_len {
//         return Err(ProblemDetails::new(
//             StatusCode::NOT_FOUND,
//             "workflow-not-found",
//             "Resource Not Found",
//             format!("Workflow '{}' not found", workflow_id),
//         ));
//     }

//     loader::save_arazzo(&state.arazzo_path, &arazzo).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "save-failed",
//             "Failed to Save Resource",
//             format!("Failed to save Arazzo specification: {}", e),
//         )
//     })?;

//     Ok(StatusCode::NO_CONTENT)
// }

/// GET /api/arazzo/graph/{workflow_id} - 特定のワークフローのグラフを取得
// pub async fn get_graph(
//     State(state): State<AppState>,
//     Path(workflow_id): Path<String>,
// ) -> ApiResult<Json<serde_json::Value>> {
//     let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "load-failed",
//             "Failed to Load Resource",
//             format!("Failed to load Arazzo specification: {}", e),
//         )
//     })?;

//     // Load OpenAPI file if provided
//     let openapi = if let Some(ref path) = state.openapi_path {
//         Some(loader::load_openapi(path).map_err(|e| {
//             ProblemDetails::new(
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "load-failed",
//                 "Failed to Load Resource",
//                 format!("Failed to load OpenAPI specification: {}", e),
//             )
//         })?)
//     } else {
//         None
//     };

//     // Find workflow
//     let workflow = arazzo
//         .workflows
//         .iter()
//         .find(|w| w.workflow_id == workflow_id)
//         .ok_or_else(|| {
//             ProblemDetails::new(
//                 StatusCode::NOT_FOUND,
//                 "workflow-not-found",
//                 "Resource Not Found",
//                 format!("Workflow '{}' not found", workflow_id),
//             )
//         })?;

//     // Build graph
//     let graph = build_flow_graph(workflow, openapi.as_ref()).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "graph-build-failed",
//             "Graph Build Failed",
//             format!("Failed to build workflow graph: {}", e),
//         )
//     })?;

//     // Export to JSON
//     let json = export_json(&graph).map_err(|e| {
//         ProblemDetails::new(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "graph-export-failed",
//             "Graph Export Failed",
//             format!("Failed to export graph to JSON: {}", e),
//         )
//     })?;

//     Ok(Json(json))
// }

// ============================================================================
// Editor API Endpoints
// ============================================================================

/// Response for /api/editor/operations
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationsResponse {
    pub operations: Vec<OperationInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationInfo {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub request_body: Option<RequestBodyInfo>,
    pub tags: Vec<String>,
    pub response_codes: Vec<String>, // HTTP response status codes (e.g. "200", "201", "404")
    pub response_schema: Option<ResponseSchemaInfo>, // Response schema for 2xx responses
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseSchemaInfo {
    pub status_code: String,
    pub properties: Vec<String>, // Top-level property names in response body
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub location: String, // "query", "header", "path", "cookie"
    pub required: bool,
    pub schema_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestBodyInfo {
    pub required: bool,
    pub content_types: Vec<String>,
    pub description: Option<String>,
}

/// GET /api/editor/operations - Get all OpenAPI operations
/// GET /api/projects/{project_name}/operations
pub async fn get_project_operations(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> ApiResult<Json<OperationsResponse>> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let mut operations = Vec::new();
    let specs = project.openapi_resolver.get_all_specs();

    // Iterate over all specs and extract operations
    for (spec_name, openapi) in specs {
        if let Some(paths) = &openapi.paths {
            for (path_str, path_item) in paths.iter() {
                // Helper function to extract operation info
                let extract_op = |method: &str, op: &oas3::spec::Operation| {
                    let operation_id = op.operation_id.clone().unwrap_or_else(|| {
                        format!("{}_{}_{}", spec_name, method, path_str.replace('/', "_"))
                    });

                    // Helper to convert parameter object to ParameterInfo
                    let to_param_info = |p: &oas3::spec::Parameter| ParameterInfo {
                        name: p.name.clone(),
                        location: format!("{:?}", p.location).to_lowercase(),
                        required: p.required.unwrap_or(false),
                        schema_type: p.schema.as_ref().and_then(|s| match s {
                            oas3::spec::ObjectOrReference::Object(schema) => {
                                schema.schema_type.as_ref().map(|t| format!("{:?}", t))
                            }
                            _ => None,
                        }),
                        description: p.description.clone(),
                    };

                    // Extract parameters
                    let mut params = Vec::new();
                    for param in &op.parameters {
                        match param {
                            oas3::spec::ObjectOrReference::Object(p) => {
                                params.push(to_param_info(p));
                            }
                            oas3::spec::ObjectOrReference::Ref { ref_path, .. } => {
                                // Try to resolve reference from components
                                if let Some(components) = &openapi.components {
                                    if let Some(name) =
                                        ref_path.strip_prefix("#/components/parameters/")
                                    {
                                        if let Some(oas3::spec::ObjectOrReference::Object(p)) =
                                            components.parameters.get(name)
                                        {
                                            params.push(to_param_info(p));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Helper to convert request body object to RequestBodyInfo
                    let to_body_info = |body: &oas3::spec::RequestBody| RequestBodyInfo {
                        required: body.required.unwrap_or(false),
                        content_types: body.content.keys().cloned().collect(),
                        description: body.description.clone(),
                    };

                    // Extract request body info
                    let request_body = op.request_body.as_ref().and_then(|rb| match rb {
                        oas3::spec::ObjectOrReference::Object(body) => Some(to_body_info(body)),
                        oas3::spec::ObjectOrReference::Ref { ref_path, .. } => {
                            if let Some(components) = &openapi.components {
                                if let Some(name) =
                                    ref_path.strip_prefix("#/components/requestBodies/")
                                {
                                    if let Some(oas3::spec::ObjectOrReference::Object(body)) =
                                        components.request_bodies.get(name)
                                    {
                                        return Some(to_body_info(body));
                                    }
                                }
                            }
                            None
                        }
                    });

                    // Extract response codes
                    let response_codes: Vec<String> = if let Some(responses) = &op.responses {
                        responses
                            .keys()
                            .filter_map(|code| {
                                if code.chars().all(|c| c.is_ascii_digit()) {
                                    Some(code.clone())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    // Extract response schema from first 2xx response
                    let response_schema = if let Some(responses) = &op.responses {
                        let mut success_response = None;
                        for (code, response) in responses.iter() {
                            if code.starts_with('2') && code.chars().all(|c| c.is_ascii_digit()) {
                                success_response = Some((code.clone(), response));
                                break;
                            }
                        }

                        success_response.and_then(|(code, response)| {
                            let extract_props_from_obj_schema = |schema: &oas3::spec::ObjectSchema| -> Option<ResponseSchemaInfo> {
                                let properties: Vec<String> =
                                    schema.properties.keys().cloned().collect();

                                if !properties.is_empty() {
                                    Some(ResponseSchemaInfo {
                                        status_code: code.clone(),
                                        properties,
                                    })
                                } else {
                                    None
                                }
                            };

                            match response {
                                oas3::spec::ObjectOrReference::Object(resp) => {
                                    resp.content.values().next().and_then(|media_type| {
                                        match &media_type.schema {
                                            Some(oas3::spec::ObjectOrReference::Object(schema)) => {
                                                extract_props_from_obj_schema(schema)
                                            }
                                            Some(oas3::spec::ObjectOrReference::Ref { ref_path, .. }) => {
                                                openapi.components.as_ref()
                                                    .and_then(|components| {
                                                        ref_path.strip_prefix("#/components/schemas/")
                                                            .and_then(|name| components.schemas.get(name))
                                                    })
                                                    .and_then(|resolved_schema_ref| {
                                                        if let oas3::spec::ObjectOrReference::Object(schema) = resolved_schema_ref {
                                                            extract_props_from_obj_schema(schema)
                                                        } else {
                                                            None
                                                        }
                                                    })
                                            }
                                            _ => None,
                                        }
                                    })
                                }
                                oas3::spec::ObjectOrReference::Ref { ref_path, .. } => {
                                    if let Some(components) = &openapi.components {
                                        if let Some(name) = ref_path.strip_prefix("#/components/responses/") {
                                            if let Some(oas3::spec::ObjectOrReference::Object(resp)) = components.responses.get(name) {
                                                    return resp.content.values().next().and_then(|media_type| {
                                                    match &media_type.schema {
                                                        Some(oas3::spec::ObjectOrReference::Object(schema)) => {
                                                            extract_props_from_obj_schema(schema)
                                                        }
                                                        Some(oas3::spec::ObjectOrReference::Ref { ref_path, .. }) => {
                                                            if let Some(components) = &openapi.components {
                                                                if let Some(name) = ref_path.strip_prefix("#/components/schemas/") {
                                                                    if let Some(oas3::spec::ObjectOrReference::Object(schema)) = components.schemas.get(name) {
                                                                        return extract_props_from_obj_schema(schema);
                                                                    }
                                                                }
                                                            }
                                                            None
                                                        }
                                                        _ => None,
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    None
                                }
                            }
                        })
                    } else {
                        None
                    };

                    OperationInfo {
                        operation_id,
                        method: method.to_uppercase(),
                        path: path_str.clone(),
                        summary: op.summary.clone(),
                        description: op.description.clone(),
                        parameters: params,
                        request_body,
                        tags: op.tags.clone(),
                        response_codes,
                        response_schema,
                    }
                };

                if let Some(op) = &path_item.get {
                    operations.push(extract_op("get", op));
                }
                if let Some(op) = &path_item.post {
                    operations.push(extract_op("post", op));
                }
                if let Some(op) = &path_item.put {
                    operations.push(extract_op("put", op));
                }
                if let Some(op) = &path_item.delete {
                    operations.push(extract_op("delete", op));
                }
                if let Some(op) = &path_item.patch {
                    operations.push(extract_op("patch", op));
                }
                if let Some(op) = &path_item.options {
                    operations.push(extract_op("options", op));
                }
                if let Some(op) = &path_item.head {
                    operations.push(extract_op("head", op));
                }
                if let Some(op) = &path_item.trace {
                    operations.push(extract_op("trace", op));
                }
            }
        }
    }

    Ok(Json(OperationsResponse { operations }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub yaml: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub errors: Vec<ValidationErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrorInfo {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// POST /api/validate - Validate Arazzo YAML
pub async fn validate_arazzo(
    Json(payload): Json<ValidateRequest>,
) -> ApiResult<Json<ValidateResponse>> {
    // Try to parse the YAML
    let parse_result: Result<ArazzoSpec, serde_yaml::Error> = serde_yaml::from_str(&payload.yaml);

    match parse_result {
        Ok(spec) => {
            // Basic validation - check required fields
            let mut errors = Vec::new();

            if spec.workflows.is_empty() {
                errors.push(ValidationErrorInfo {
                    message: "At least one workflow is required".to_string(),
                    line: None,
                    column: None,
                });
            }

            for workflow in spec.workflows.iter() {
                if workflow.steps.is_empty() {
                    errors.push(ValidationErrorInfo {
                        message: format!(
                            "Workflow '{}' must have at least one step",
                            workflow.workflow_id
                        ),
                        line: None,
                        column: None,
                    });
                }

                // Check for duplicate step IDs
                let mut step_ids = std::collections::HashSet::new();
                for step in &workflow.steps {
                    if !step_ids.insert(&step.step_id) {
                        errors.push(ValidationErrorInfo {
                            message: format!(
                                "Duplicate step ID '{}' in workflow '{}'",
                                step.step_id, workflow.workflow_id
                            ),
                            line: None,
                            column: None,
                        });
                    }
                }
            }

            Ok(Json(ValidateResponse {
                valid: errors.is_empty(),
                errors,
            }))
        }
        Err(e) => {
            // YAML parse error
            let error_msg = e.to_string();
            let (line, column) = extract_location_from_error(&error_msg);

            Ok(Json(ValidateResponse {
                valid: false,
                errors: vec![ValidationErrorInfo {
                    message: error_msg,
                    line,
                    column,
                }],
            }))
        }
    }
}

fn extract_location_from_error(error: &str) -> (Option<usize>, Option<usize>) {
    // serde_yaml errors often include "at line X column Y"
    let line_re = regex::Regex::new(r"line (\d+)").ok();
    let col_re = regex::Regex::new(r"column (\d+)").ok();

    let line = line_re.and_then(|re| {
        re.captures(error)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok())
    });

    let column = col_re.and_then(|re| {
        re.captures(error)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok())
    });

    (line, column)
}

// ============================================================================
// Multi-Project API Endpoints
// ============================================================================

/// GET /api/projects - すべてのプロジェクトをリスト
pub async fn list_projects(State(state): State<AppState>) -> ApiResult<Json<ProjectsResponse>> {
    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let projects = cache.list_projects(&state.root_dir).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "scan-failed",
            "Project Scan Failed",
            format!("Failed to scan projects: {}", e),
        )
    })?;

    let project_infos: Vec<ProjectInfo> = projects
        .iter()
        .map(|p| ProjectInfo {
            name: p.name.clone(),
            title: p.arazzo_spec.info.title.clone(),
            description: p.arazzo_spec.info.description.clone(),
            workflow_count: p.arazzo_spec.workflows.len(),
            arazzo_path: format!("{}/arazzo.yaml", p.name),
            openapi_files: p
                .openapi_resolver
                .list_files()
                .iter()
                .map(|name| format!("{}/{}.yaml", p.name, name))
                .collect(),
        })
        .collect();

    Ok(Json(ProjectsResponse {
        projects: project_infos,
    }))
}

/// GET /api/projects/{project_name} - プロジェクト詳細を取得
pub async fn get_project(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> ApiResult<Json<ProjectDetail>> {
    // セキュリティ: パストラバーサル攻撃を防ぐ
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let project_detail = ProjectDetail {
        name: project.name.clone(),
        title: project.arazzo_spec.info.title.clone(),
        description: project.arazzo_spec.info.description.clone(),
        workflow_count: project.arazzo_spec.workflows.len(),
        openapi_files: project
            .openapi_resolver
            .list_files()
            .iter()
            .map(|name| format!("{}.yaml", name))
            .collect(),
    };

    Ok(Json(project_detail))
}

/// GET /api/projects/{project_name}/arazzo - Arazzo仕様を取得
pub async fn get_project_arazzo(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> ApiResult<Json<ArazzoSpec>> {
    // セキュリティチェック
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    Ok(Json(project.arazzo_spec.clone()))
}

/// PUT /api/projects/{project_name}/arazzo - Arazzo仕様を更新
pub async fn update_project_arazzo(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
    Json(spec): Json<ArazzoSpec>,
) -> ApiResult<Json<ArazzoSpec>> {
    // セキュリティチェック
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    loader::save_arazzo(&project.arazzo_path, &spec).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save-failed",
            "Failed to Save Resource",
            format!("Failed to save Arazzo specification: {}", e),
        )
    })?;

    // キャッシュを再読み込み
    cache
        .reload_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cache-reload-failed",
                "Cache Reload Failed",
                format!("Failed to reload project cache: {}", e),
            )
        })?;

    Ok(Json(spec))
}
/// GET /api/projects/{project_name}/workflows - ワークフロー一覧を取得
pub async fn get_project_workflows(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> ApiResult<Json<WorkflowsResponse>> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let workflows: Vec<WorkflowInfo> = project
        .arazzo_spec
        .workflows
        .iter()
        .map(|w| WorkflowInfo {
            workflow_id: w.workflow_id.clone(),
            title: w.summary.clone(),
            description: w.description.clone(),
            steps: w.steps.len(),
        })
        .collect();

    Ok(Json(WorkflowsResponse { workflows }))
}

/// POST /api/projects/{project_name}/workflows - ワークフローを作成
pub async fn create_project_workflow(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
    Json(req): Json<CreateWorkflowRequest>,
) -> ApiResult<(StatusCode, Json<Workflow>)> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let mut arazzo = loader::load_arazzo(&project.arazzo_path).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "load-failed",
            "Failed to Load Resource",
            format!("Failed to load Arazzo specification: {}", e),
        )
    })?;

    // 重複チェック
    if arazzo
        .workflows
        .iter()
        .any(|w| w.workflow_id == req.workflow.workflow_id)
    {
        return Err(ProblemDetails::new(
            StatusCode::CONFLICT,
            "workflow-exists",
            "Resource Already Exists",
            format!(
                "Workflow with ID '{}' already exists",
                req.workflow.workflow_id
            ),
        ));
    }

    // ワークフロー追加（既存ファイルに追記）
    arazzo.workflows.push(req.workflow.clone());

    // 保存
    loader::save_arazzo(&project.arazzo_path, &arazzo).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save-failed",
            "Failed to Save Resource",
            format!("Failed to save Arazzo specification: {}", e),
        )
    })?;

    // キャッシュを再読み込み
    cache
        .reload_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cache-reload-failed",
                "Cache Reload Failed",
                format!("Failed to reload project cache: {}", e),
            )
        })?;

    Ok((StatusCode::CREATED, Json(req.workflow)))
}

/// GET /api/projects/{project_name}/workflows/{workflow_id} - 特定のワークフローを取得
pub async fn get_project_workflow(
    State(state): State<AppState>,
    Path((project_name, workflow_id)): Path<(String, String)>,
) -> ApiResult<Json<Workflow>> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let workflow = project
        .arazzo_spec
        .workflows
        .iter()
        .find(|w| w.workflow_id == workflow_id)
        .ok_or_else(|| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "workflow-not-found",
                "Resource Not Found",
                format!("Workflow '{}' not found", workflow_id),
            )
        })?;

    Ok(Json(workflow.clone()))
}

/// PUT /api/projects/{project_name}/workflows/{workflow_id} - ワークフローを更新
pub async fn update_project_workflow(
    State(state): State<AppState>,
    Path((project_name, workflow_id)): Path<(String, String)>,
    Json(workflow): Json<Workflow>,
) -> ApiResult<Json<Workflow>> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    // Validate workflow ID matches path parameter
    if workflow.workflow_id != workflow_id {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "workflow-id-mismatch",
            "Invalid Request",
            format!(
                "Workflow ID in path ({}) does not match workflow ID in body ({})",
                workflow_id, workflow.workflow_id
            ),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let mut arazzo = loader::load_arazzo(&project.arazzo_path).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "load-failed",
            "Failed to Load Resource",
            format!("Failed to load Arazzo specification: {}", e),
        )
    })?;

    // Find and update existing workflow
    let pos = arazzo
        .workflows
        .iter()
        .position(|w| w.workflow_id == workflow_id)
        .ok_or_else(|| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "workflow-not-found",
                "Resource Not Found",
                format!("Workflow '{}' not found", workflow_id),
            )
        })?;

    arazzo.workflows[pos] = workflow.clone();

    loader::save_arazzo(&project.arazzo_path, &arazzo).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save-failed",
            "Failed to Save Resource",
            format!("Failed to save Arazzo specification: {}", e),
        )
    })?;

    // キャッシュを再読み込み
    cache
        .reload_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cache-reload-failed",
                "Cache Reload Failed",
                format!("Failed to reload project cache: {}", e),
            )
        })?;

    Ok(Json(workflow))
}

/// DELETE /api/projects/{project_name}/workflows/{workflow_id} - ワークフローを削除
pub async fn delete_project_workflow(
    State(state): State<AppState>,
    Path((project_name, workflow_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    let mut arazzo = loader::load_arazzo(&project.arazzo_path).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "load-failed",
            "Failed to Load Resource",
            format!("Failed to load Arazzo specification: {}", e),
        )
    })?;

    let initial_len = arazzo.workflows.len();
    arazzo.workflows.retain(|w| w.workflow_id != workflow_id);

    if arazzo.workflows.len() == initial_len {
        return Err(ProblemDetails::new(
            StatusCode::NOT_FOUND,
            "workflow-not-found",
            "Resource Not Found",
            format!("Workflow '{}' not found", workflow_id),
        ));
    }

    loader::save_arazzo(&project.arazzo_path, &arazzo).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save-failed",
            "Failed to Save Resource",
            format!("Failed to save Arazzo specification: {}", e),
        )
    })?;

    // キャッシュを再読み込み
    cache
        .reload_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cache-reload-failed",
                "Cache Reload Failed",
                format!("Failed to reload project cache: {}", e),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/projects/{project_name}/graph/{workflow_id} - ワークフローグラフを取得
pub async fn get_project_graph(
    State(state): State<AppState>,
    Path((project_name, workflow_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if project_name.contains("..") || project_name.contains('/') || project_name.contains('\\') {
        return Err(ProblemDetails::new(
            StatusCode::BAD_REQUEST,
            "invalid-project-name",
            "Invalid Request",
            "Project name contains invalid characters".into(),
        ));
    }

    let mut cache = state.projects.write().map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cache-lock-failed",
            "Internal Error",
            "Failed to acquire project cache lock".into(),
        )
    })?;

    let project = cache
        .get_project(&project_name, &state.root_dir)
        .map_err(|e| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "project-not-found",
                "Project Not Found",
                format!("Project '{}' not found: {}", project_name, e),
            )
        })?;

    // Find workflow
    let workflow = project
        .arazzo_spec
        .workflows
        .iter()
        .find(|w| w.workflow_id == workflow_id)
        .ok_or_else(|| {
            ProblemDetails::new(
                StatusCode::NOT_FOUND,
                "workflow-not-found",
                "Resource Not Found",
                format!("Workflow '{}' not found", workflow_id),
            )
        })?;

    // Get first OpenAPI spec if available
    let openapi = project.openapi_resolver.get_all_specs().values().next();

    // Build graph
    let graph = build_flow_graph(workflow, openapi).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "graph-build-failed",
            "Graph Build Failed",
            format!("Failed to build workflow graph: {}", e),
        )
    })?;

    // Export to JSON
    let json = export_json(&graph).map_err(|e| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "graph-export-failed",
            "Graph Export Failed",
            format!("Failed to export graph to JSON: {}", e),
        )
    })?;

    Ok(Json(json))
}
