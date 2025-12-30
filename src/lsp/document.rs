use dashmap::DashMap;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tower_lsp::lsp_types::{Position, Range, Url};

/// Request to validate a document
#[derive(Debug, Clone)]
pub struct ValidationRequest {
    pub uri: Url,
}

/// Manages open documents and their state
pub struct DocumentManager {
    documents: DashMap<Url, Document>,
    validation_tx: mpsc::Sender<ValidationRequest>,
}

impl DocumentManager {
    pub fn new(validation_tx: mpsc::Sender<ValidationRequest>) -> Self {
        Self {
            documents: DashMap::new(),
            validation_tx,
        }
    }

    /// Add or update a document
    pub fn update(&self, uri: Url, version: i32, content: String) {
        let position_map = PositionMap::build(&content);

        let document = Document {
            uri: uri.clone(),
            version,
            content,
            position_map,
        };

        self.documents.insert(uri.clone(), document);

        // Schedule validation
        self.schedule_validation(uri);
    }

    /// Get a document by URI
    pub fn get(&self, uri: &Url) -> Option<Arc<Document>> {
        self.documents.get(uri).map(|doc| Arc::new(doc.clone()))
    }

    /// Remove a document
    pub fn remove(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// Schedule validation with debouncing
    fn schedule_validation(&self, uri: Url) {
        let tx = self.validation_tx.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(300)).await;
            let _ = tx.send(ValidationRequest { uri }).await;
        });
    }
}

/// A tracked document
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    pub position_map: PositionMap,
}

/// Maps identifiers and references to their positions in the document
#[derive(Debug, Clone)]
pub struct PositionMap {
    /// workflowId → Range
    workflows: HashMap<String, Range>,
    /// stepId → Range (workflow-qualified: "workflow_id.step_id")
    steps: HashMap<String, Range>,
    /// operationId → Range (step-qualified: "workflow_id.step_id.operationId")
    #[allow(dead_code)]
    operation_ids: HashMap<String, Range>,
    /// All identifiers with their ranges and kinds
    identifiers: Vec<IdentifierInfo>,
}

impl PositionMap {
    /// Build position map from document content
    pub fn build(content: &str) -> Self {
        let mut workflows = HashMap::new();
        let mut steps = HashMap::new();
        let mut operation_ids = HashMap::new();
        let mut identifiers = Vec::new();

        let workflow_re = Regex::new(r"^\s*workflowId:\s*([a-zA-Z0-9_-]+)").unwrap();
        let step_re = Regex::new(r"^\s*-\s*stepId:\s*([a-zA-Z0-9_-]+)").unwrap();
        let operation_id_re = Regex::new(r"^\s*operationId:\s*([a-zA-Z0-9_-]+)").unwrap();

        let mut current_workflow: Option<String> = None;
        let mut current_step: Option<String> = None;

        for (line_idx, line) in content.lines().enumerate() {
            let line_number = line_idx as u32;

            // Check for workflowId
            if let Some(cap) = workflow_re.captures(line) {
                let workflow_id = cap[1].to_string();
                let col = line.find("workflowId:").unwrap() as u32;
                let range = Range::new(
                    Position::new(line_number, col),
                    Position::new(
                        line_number,
                        col + "workflowId:".len() as u32 + workflow_id.len() as u32,
                    ),
                );

                workflows.insert(workflow_id.clone(), range);
                identifiers.push(IdentifierInfo {
                    range,
                    kind: IdentifierKind::WorkflowId(workflow_id.clone()),
                });

                current_workflow = Some(workflow_id);
                current_step = None;
            }

            // Check for stepId
            if let Some(cap) = step_re.captures(line) {
                let step_id = cap[1].to_string();
                let col = line.find("stepId:").unwrap() as u32;
                let range = Range::new(
                    Position::new(line_number, col),
                    Position::new(
                        line_number,
                        col + "stepId:".len() as u32 + step_id.len() as u32,
                    ),
                );

                if let Some(ref wf_id) = current_workflow {
                    let qualified_step_id = format!("{}.{}", wf_id, step_id);
                    steps.insert(qualified_step_id, range);
                }

                identifiers.push(IdentifierInfo {
                    range,
                    kind: IdentifierKind::StepId(step_id.clone()),
                });

                current_step = Some(step_id);
            }

            // Check for operationId
            if let Some(cap) = operation_id_re.captures(line) {
                let op_id = cap[1].to_string();
                let col = line.find("operationId:").unwrap() as u32;
                let range = Range::new(
                    Position::new(line_number, col),
                    Position::new(
                        line_number,
                        col + "operationId:".len() as u32 + op_id.len() as u32,
                    ),
                );

                if let (Some(wf_id), Some(step_id)) = (&current_workflow, &current_step) {
                    let qualified_op_id = format!("{}.{}.{}", wf_id, step_id, op_id);
                    operation_ids.insert(qualified_op_id, range);
                }

                identifiers.push(IdentifierInfo {
                    range,
                    kind: IdentifierKind::OperationId(op_id.clone()),
                });
            }
        }

        Self {
            workflows,
            steps,
            operation_ids,
            identifiers,
        }
    }

    /// Get range for a workflow ID
    pub fn get_workflow_range(&self, workflow_id: &str) -> Option<Range> {
        self.workflows.get(workflow_id).copied()
    }

    /// Get range for a step ID (workflow-qualified)
    pub fn get_step_range(&self, workflow_id: &str, step_id: &str) -> Option<Range> {
        let qualified = format!("{}.{}", workflow_id, step_id);
        self.steps.get(&qualified).copied()
    }

    /// Find identifier at a given position
    pub fn find_identifier_at(&self, position: Position) -> Option<&IdentifierInfo> {
        self.identifiers
            .iter()
            .find(|id| Self::contains_position(id.range, position))
    }

    /// Check if a range contains a position
    fn contains_position(range: Range, position: Position) -> bool {
        let start_match = position.line > range.start.line
            || (position.line == range.start.line && position.character >= range.start.character);
        let end_match = position.line < range.end.line
            || (position.line == range.end.line && position.character <= range.end.character);

        start_match && end_match
    }

    /// Get completion context at a given position
    pub fn get_completion_context(
        &self,
        position: Position,
        content: &str,
    ) -> Option<CompletionContext> {
        let lines: Vec<&str> = content.lines().collect();
        let line = lines.get(position.line as usize)?;

        // Check if we're on an operationId line
        if line.contains("operationId:") {
            return Some(CompletionContext::OperationId);
        }

        // Check if we're on an operationPath line
        if line.contains("operationPath:") {
            return Some(CompletionContext::OperationPath);
        }

        None
    }
}

/// Information about an identifier in the document
#[derive(Debug, Clone)]
pub struct IdentifierInfo {
    pub range: Range,
    pub kind: IdentifierKind,
}

/// Kind of identifier
#[derive(Debug, Clone)]
pub enum IdentifierKind {
    WorkflowId(String),
    StepId(String),
    OperationId(String),
}

/// Context for code completion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    OperationId,
    OperationPath,
}
