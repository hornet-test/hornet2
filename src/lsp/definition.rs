use crate::lsp::document::{DocumentManager, IdentifierKind};
use crate::lsp::workspace::WorkspaceManager;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

/// Provide go-to-definition for a position in a document
pub fn provide_definition(
    doc_manager: &Arc<DocumentManager>,
    workspace_manager: &Arc<WorkspaceManager>,
    uri: &Url,
    position: Position,
) -> Result<Option<GotoDefinitionResponse>> {
    // Get document
    let doc = match doc_manager.get(uri) {
        Some(d) => d,
        None => return Ok(None),
    };

    // Find identifier at position
    let identifier = match doc.position_map.find_identifier_at(position) {
        Some(id) => id,
        None => return Ok(None),
    };

    match &identifier.kind {
        IdentifierKind::OperationId(op_id) => {
            // Get OpenAPI file path and spec
            let (openapi_path, _openapi) = match workspace_manager.get_openapi_for_document(uri) {
                Some(data) => data,
                None => return Ok(None),
            };

            // Convert path to URI
            let openapi_uri = match Url::from_file_path(&openapi_path) {
                Ok(u) => u,
                Err(_) => return Ok(None),
            };

            // Find operationId position in OpenAPI file
            let op_position = find_operation_id_in_file(&openapi_path, op_id)?;

            if let Some(range) = op_position {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: openapi_uri,
                    range,
                })));
            }

            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Find the position of an operationId in an OpenAPI file
fn find_operation_id_in_file(
    file_path: &std::path::Path,
    operation_id: &str,
) -> Result<Option<Range>> {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    // Search for operationId line
    for (line_idx, line) in content.lines().enumerate() {
        if line.contains("operationId:") && line.contains(operation_id) {
            let line_number = line_idx as u32;

            // Find column position
            let col_start = line.find(operation_id).unwrap_or(0) as u32;
            let col_end = col_start + operation_id.len() as u32;

            return Ok(Some(Range::new(
                Position::new(line_number, col_start),
                Position::new(line_number, col_end),
            )));
        }
    }

    Ok(None)
}
