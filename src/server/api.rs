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
