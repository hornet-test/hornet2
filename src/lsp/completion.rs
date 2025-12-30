use crate::lsp::document::{CompletionContext, DocumentManager};
use crate::lsp::workspace::WorkspaceManager;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, MarkupContent,
    MarkupKind, Position, Url,
};

/// Provide code completion for a position in a document
pub fn provide_completion(
    doc_manager: &Arc<DocumentManager>,
    workspace_manager: &Arc<WorkspaceManager>,
    uri: &Url,
    position: Position,
) -> Result<Option<CompletionResponse>> {
    // Get document
    let doc = match doc_manager.get(uri) {
        Some(d) => d,
        None => return Ok(None),
    };

    // Get completion context
    let context = match doc
        .position_map
        .get_completion_context(position, &doc.content)
    {
        Some(c) => c,
        None => return Ok(None),
    };

    match context {
        CompletionContext::OperationId => {
            // Get OpenAPI resolver
            let resolver = match workspace_manager.get_resolver_for_document(uri) {
                Some(r) => r,
                None => return Ok(None),
            };

            // Collect all operations from all specs
            let mut items = Vec::new();

            for spec in resolver.get_all_specs().values() {
                if let Some(paths) = &spec.paths {
                    for (path, path_item) in paths.iter() {
                        let operations = [
                            ("GET", &path_item.get),
                            ("POST", &path_item.post),
                            ("PUT", &path_item.put),
                            ("DELETE", &path_item.delete),
                            ("PATCH", &path_item.patch),
                            ("OPTIONS", &path_item.options),
                            ("HEAD", &path_item.head),
                            ("TRACE", &path_item.trace),
                        ];

                        for (method, op_option) in &operations {
                            if let Some(op) = op_option {
                                if let Some(op_id) = &op.operation_id {
                                    let detail = format!("{} {}", method, path);

                                    let documentation =
                                        op.summary.as_ref().or(op.description.as_ref()).map(
                                            |text| {
                                                Documentation::MarkupContent(MarkupContent {
                                                    kind: MarkupKind::Markdown,
                                                    value: text.clone(),
                                                })
                                            },
                                        );

                                    items.push(CompletionItem {
                                        label: op_id.clone(),
                                        kind: Some(CompletionItemKind::FUNCTION),
                                        detail: Some(detail),
                                        documentation,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Ok(Some(CompletionResponse::Array(items)))
        }
        CompletionContext::OperationPath => {
            // Get OpenAPI resolver
            let resolver = match workspace_manager.get_resolver_for_document(uri) {
                Some(r) => r,
                None => return Ok(None),
            };

            // Collect all operations from all specs
            let mut items = Vec::new();

            for spec in resolver.get_all_specs().values() {
                if let Some(paths) = &spec.paths {
                    for (path, path_item) in paths.iter() {
                        let operations = [
                            ("GET", &path_item.get),
                            ("POST", &path_item.post),
                            ("PUT", &path_item.put),
                            ("DELETE", &path_item.delete),
                            ("PATCH", &path_item.patch),
                            ("OPTIONS", &path_item.options),
                            ("HEAD", &path_item.head),
                            ("TRACE", &path_item.trace),
                        ];

                        for (method, op_option) in &operations {
                            if let Some(op) = op_option {
                                let op_id = op.operation_id.as_deref().unwrap_or("unknown");
                                let label = format!("{} {}", method, path);

                                items.push(CompletionItem {
                                    label,
                                    kind: Some(CompletionItemKind::REFERENCE),
                                    detail: Some(format!("operationId: {}", op_id)),
                                    insert_text: Some(path.clone()),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }

            Ok(Some(CompletionResponse::Array(items)))
        }
    }
}
