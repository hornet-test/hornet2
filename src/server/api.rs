use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    graph::{builder::build_flow_graph, exporter::export_json},
    loader,
    models::arazzo::{ArazzoSpec, Workflow},
};

/// 共有アプリケーション状態
#[derive(Clone)]
pub struct AppState {
    pub arazzo_path: String,
    pub openapi_path: Option<String>,
}

/// /api/workflows のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowsResponse {
    pub workflows: Vec<WorkflowInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub workflow_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub steps: usize,
}

/// GET /api/spec - 完全なArazzo仕様を取得
pub async fn get_spec(
    State(state): State<AppState>,
) -> Result<Json<ArazzoSpec>, (StatusCode, String)> {
    let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    Ok(Json(arazzo))
}

/// PUT /api/spec - 完全なArazzo仕様を更新
pub async fn update_spec(
    State(state): State<AppState>,
    Json(spec): Json<ArazzoSpec>,
) -> Result<Json<ArazzoSpec>, (StatusCode, String)> {
    loader::save_arazzo(&state.arazzo_path, &spec).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save Arazzo: {}", e),
        )
    })?;

    Ok(Json(spec))
}

/// GET /api/workflows - すべてのワークフローをリスト
pub async fn get_workflows(
    State(state): State<AppState>,
) -> Result<Json<WorkflowsResponse>, (StatusCode, String)> {
    // Arazzoファイルをロード
    let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    let workflows: Vec<WorkflowInfo> = arazzo
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

/// GET /api/workflows/:workflow_id - 特定のワークフローを取得
pub async fn get_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    let workflow = arazzo
        .workflows
        .into_iter()
        .find(|w| w.workflow_id == workflow_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Workflow '{}' not found", workflow_id),
            )
        })?;

    Ok(Json(workflow))
}

/// PUT /api/workflows/:workflow_id - ワークフローを更新または作成
pub async fn update_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Json(workflow): Json<Workflow>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    if workflow.workflow_id != workflow_id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Workflow ID mismatch: path={}, body={}",
                workflow_id, workflow.workflow_id
            ),
        ));
    }

    let mut arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    if let Some(pos) = arazzo
        .workflows
        .iter()
        .position(|w| w.workflow_id == workflow_id)
    {
        // 既存のワークフローを更新
        arazzo.workflows[pos] = workflow.clone();
    } else {
        // 新しいワークフローを追加
        arazzo.workflows.push(workflow.clone());
    }

    loader::save_arazzo(&state.arazzo_path, &arazzo).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save Arazzo: {}", e),
        )
    })?;

    Ok(Json(workflow))
}

/// DELETE /api/workflows/:workflow_id - ワークフローを削除
pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    let initial_len = arazzo.workflows.len();
    arazzo.workflows.retain(|w| w.workflow_id != workflow_id);

    if arazzo.workflows.len() == initial_len {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Workflow '{}' not found", workflow_id),
        ));
    }

    loader::save_arazzo(&state.arazzo_path, &arazzo).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save Arazzo: {}", e),
        )
    })?;

    Ok(StatusCode::OK)
}

/// GET /api/graph/:workflow_id - 特定のワークフローのグラフを取得
pub async fn get_graph(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Arazzoファイルをロード
    let arazzo = loader::load_arazzo(&state.arazzo_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load Arazzo: {}", e),
        )
    })?;

    // OpenAPIファイルが提供されている場合はロード
    let openapi = if let Some(ref path) = state.openapi_path {
        Some(loader::load_openapi(path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load OpenAPI: {}", e),
            )
        })?)
    } else {
        None
    };

    // ワークフローを検索
    let workflow = arazzo
        .workflows
        .iter()
        .find(|w| w.workflow_id == workflow_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Workflow '{}' not found", workflow_id),
            )
        })?;

    // グラフを構築
    let graph = build_flow_graph(workflow, openapi.as_ref()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build graph: {}", e),
        )
    })?;

    // JSONにエクスポート
    let json = export_json(&graph).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to export graph: {}", e),
        )
    })?;

    Ok(Json(json))
}

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
pub async fn get_operations(
    State(state): State<AppState>,
) -> Result<Json<OperationsResponse>, (StatusCode, String)> {
    // Load OpenAPI file
    let openapi = if let Some(ref path) = state.openapi_path {
        loader::load_openapi(path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load OpenAPI: {}", e),
            )
        })?
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            "No OpenAPI file configured".to_string(),
        ));
    };

    let mut operations = Vec::new();

    // Extract operations from paths
    if let Some(paths) = &openapi.paths {
        for (path_str, path_item) in paths.iter() {
            // Helper function to extract operation info
            let extract_op = |method: &str, op: &oas3::spec::Operation| {
                let operation_id = op
                    .operation_id
                    .clone()
                    .unwrap_or_else(|| format!("{}_{}", method, path_str.replace('/', "_")));

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
                            // Expected format: "#/components/parameters/Name"
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
                        // Try to resolve reference from components
                        // Expected format: "#/components/requestBodies/Name"
                        if let Some(components) = &openapi.components {
                            if let Some(name) = ref_path.strip_prefix("#/components/requestBodies/")
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
                            // Filter out "default" and keep only numeric codes
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

                    // Find first 2xx response
                    for (code, response) in responses.iter() {
                        if code.starts_with('2') && code.chars().all(|c| c.is_ascii_digit()) {
                            success_response = Some((code.clone(), response));
                            break;
                        }
                    }

                    success_response.and_then(|(code, response)| {
                        // Helper to extract properties from schema
                        let _extract_props_from_schema = |_schema: &oas3::spec::Schema| -> Option<ResponseSchemaInfo> {
                            // Try to match Object variant if Schema is an enum
                            // Note: oas3::spec::Schema might be an enum or have specific structure
                            // Since we don't know exact definition, valid guess for oas3 crate:
                            // Schema::Object(ObjectSchema)
                            // or access properties via check

                            // Trying pattern match assuming it is an enum or has properties field we missed?
                            // Error said "no field properties".
                            // If it's 0.20, maybe it is `Schema { kind: SchemaKind }`?
                            // Let's try matching generic Object variant often found in such crates.

                            // To be safe and debug, let's just return None for now if we can't access properties,
                            // OR try to guess.
                            // But I want to fix it.

                            // Let's assume Schema delegates to ObjectSchema
                            // Does Schema implement Deref?

                            // Plan B: use debug formatting to extract? No.

                            // Let's try:
                            // if let Ok(obj) = schema.try_into() ... no.

                            // Checking common oas3 usage:
                            // Schema is struct with `schema_kind: SchemaKind`.
                            // SchemaKind::Type(Type::Object(ObjectSchema))?

                            // Given I can't check docs, I will stub it to return None for Schema type,
                            // and rely on ObjectSchema paths which seem to work (since extract_props_from_obj_schema compiles?)
                            // Wait, does extract_props_from_obj_schema compile?
                            // Step 187 only complained about line 294 (extract_props_from_schema).
                            // It did NOT complain about line 310 (extract_props_from_obj_schema).
                            // So ObjectSchema HAS properties.

                            // So if I can get ObjectSchema from Schema, I win.
                            // I'll try matching Schema::Object(o).
                            None                        };

                        // Helper to extract properties from ObjectSchema
                        let extract_props_from_obj_schema = |schema: &oas3::spec::ObjectSchema| -> Option<ResponseSchemaInfo> {
                            // Extract top-level property names
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
                                // Extract schema from first content type (usually application/json)
                                resp.content.values().next().and_then(|media_type| {
                                    match &media_type.schema {
                                        Some(oas3::spec::ObjectOrReference::Object(schema)) => {
                                            extract_props_from_obj_schema(schema)
                                        }
                                        Some(oas3::spec::ObjectOrReference::Ref { ref_path, .. }) => {
                                            // Resolve schema reference
                                            // Expected: "#/components/schemas/Name"
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
                                // Resolve response reference
                                // Expected: "#/components/responses/Name"
                                if let Some(components) = &openapi.components {
                                    if let Some(name) = ref_path.strip_prefix("#/components/responses/") {
                                        if let Some(oas3::spec::ObjectOrReference::Object(resp)) = components.responses.get(name) {
                                             return resp.content.values().next().and_then(|media_type| {
                                                match &media_type.schema {
                                                    Some(oas3::spec::ObjectOrReference::Object(schema)) => {
                                                        extract_props_from_obj_schema(schema)
                                                    }
                                                    Some(oas3::spec::ObjectOrReference::Ref { ref_path, .. }) => {
                                                        // Resolve nested schema reference
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

            // Extract operations for each HTTP method
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

    Ok(Json(OperationsResponse { operations }))
}

/// Request body for /api/editor/validate
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub yaml: String,
}

/// Response for /api/editor/validate
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

/// POST /api/editor/validate - Validate Arazzo YAML
pub async fn validate_arazzo(
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, (StatusCode, String)> {
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

/// Extract line and column from serde_yaml error message
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
