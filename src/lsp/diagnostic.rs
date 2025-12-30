use crate::lsp::document::PositionMap;
use crate::validation::{ValidationError, ValidationWarning};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// Convert ValidationError to LSP Diagnostic
pub fn validation_error_to_diagnostic(
    error: &ValidationError,
    position_map: &PositionMap,
) -> Diagnostic {
    let range = error_to_range(error, position_map);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(format!("{:?}", error.error_type))),
        source: Some("hornet2".to_string()),
        message: error.message.clone(),
        ..Default::default()
    }
}

/// Convert ValidationWarning to LSP Diagnostic
pub fn validation_warning_to_diagnostic(
    warning: &ValidationWarning,
    position_map: &PositionMap,
) -> Diagnostic {
    let range = warning_to_range(warning, position_map);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        source: Some("hornet2".to_string()),
        message: warning.message.clone(),
        ..Default::default()
    }
}

/// Determine the range for a validation error
fn error_to_range(error: &ValidationError, position_map: &PositionMap) -> Range {
    // If we have explicit line number information, use it
    if let Some(line_number) = error.line_number {
        let line = (line_number - 1) as u32; // Convert 1-indexed to 0-indexed
        return Range::new(Position::new(line, 0), Position::new(line, 0));
    }

    // Try to find range from position map
    if let (Some(workflow_id), Some(step_id)) = (&error.workflow_id, &error.step_id) {
        if let Some(range) = position_map.get_step_range(workflow_id, step_id) {
            return range;
        }
    }

    if let Some(workflow_id) = &error.workflow_id {
        if let Some(range) = position_map.get_workflow_range(workflow_id) {
            return range;
        }
    }

    // Fallback: first line
    Range::new(Position::new(0, 0), Position::new(0, 0))
}

/// Determine the range for a validation warning
fn warning_to_range(warning: &ValidationWarning, position_map: &PositionMap) -> Range {
    // If we have explicit line number information, use it
    if let Some(line_number) = warning.line_number {
        let line = (line_number - 1) as u32; // Convert 1-indexed to 0-indexed
        return Range::new(Position::new(line, 0), Position::new(line, 0));
    }

    // Try to find range from position map
    if let (Some(workflow_id), Some(step_id)) = (&warning.workflow_id, &warning.step_id) {
        if let Some(range) = position_map.get_step_range(workflow_id, step_id) {
            return range;
        }
    }

    if let Some(workflow_id) = &warning.workflow_id {
        if let Some(range) = position_map.get_workflow_range(workflow_id) {
            return range;
        }
    }

    // Fallback: first line
    Range::new(Position::new(0, 0), Position::new(0, 0))
}
