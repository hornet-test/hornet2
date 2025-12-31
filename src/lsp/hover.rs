use crate::lsp::document::{DocumentManager, IdentifierKind};
use crate::lsp::workspace::WorkspaceManager;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    Hover, HoverContents, MarkedString, MarkupContent, MarkupKind, Position, Url,
};

/// Provide hover information for a position in a document
pub fn provide_hover(
    doc_manager: &Arc<DocumentManager>,
    workspace_manager: &Arc<WorkspaceManager>,
    uri: &Url,
    position: Position,
) -> Result<Option<Hover>> {
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
            // Get OpenAPI resolver
            let resolver = match workspace_manager.get_resolver_for_document(uri) {
                Some(r) => r,
                None => return Ok(None),
            };

            // Find operation details
            if let Some((op_ref, operation)) = resolver.find_operation_with_details(op_id) {
                let method = op_ref.method.to_uppercase();
                let path = &op_ref.path;
                let summary = operation.summary.as_deref().unwrap_or("");
                let description = operation.description.as_deref().unwrap_or("");

                let markdown = if !description.is_empty() {
                    format!(
                        "**Operation**: `{}`\n\n**Method**: {}\n\n**Path**: `{}`\n\n**Summary**: {}\n\n{}",
                        op_id, method, path, summary, description
                    )
                } else if !summary.is_empty() {
                    format!(
                        "**Operation**: `{}`\n\n**Method**: {}\n\n**Path**: `{}`\n\n{}",
                        op_id, method, path, summary
                    )
                } else {
                    format!(
                        "**Operation**: `{}`\n\n**Method**: {}\n\n**Path**: `{}`",
                        op_id, method, path
                    )
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: Some(identifier.range),
                }));
            }

            // Operation not found
            Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(format!(
                    "Operation '{}' not found in OpenAPI specification",
                    op_id
                ))),
                range: Some(identifier.range),
            }))
        }
        IdentifierKind::WorkflowId(workflow_id) => {
            // Find project and workflow details
            if let Some(project) = workspace_manager.find_project_for_file(uri)
                && let Some(workflow) = project
                    .arazzo_spec
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == *workflow_id)
            {
                let mut markdown = format!("**Workflow**: `{}`\n\n", workflow_id);

                if let Some(ref description) = workflow.description {
                    markdown.push_str(&format!("{}\n\n", description));
                }

                markdown.push_str(&format!("**Steps**: {}", workflow.steps.len()));

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: Some(identifier.range),
                }));
            }

            Ok(None)
        }
        IdentifierKind::StepId(step_id) => {
            // Find step details in current workflow
            if let Some(project) = workspace_manager.find_project_for_file(uri) {
                for workflow in &project.arazzo_spec.workflows {
                    if let Some(step) = workflow.steps.iter().find(|s| s.step_id == *step_id) {
                        let mut markdown = format!("**Step**: `{}`\n\n", step_id);

                        if let Some(ref description) = step.description {
                            markdown.push_str(&format!("{}\n\n", description));
                        }

                        if let Some(ref op_id) = step.operation_id {
                            markdown.push_str(&format!("**Operation**: `{}`", op_id));
                        } else if let Some(ref op_path) = step.operation_path {
                            markdown.push_str(&format!("**Operation Path**: `{}`", op_path));
                        }

                        return Ok(Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: markdown,
                            }),
                            range: Some(identifier.range),
                        }));
                    }
                }
            }

            Ok(None)
        }
    }
}
