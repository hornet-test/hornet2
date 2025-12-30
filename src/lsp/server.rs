use crate::error::Result;
use crate::lsp::diagnostic::{validation_error_to_diagnostic, validation_warning_to_diagnostic};
use crate::lsp::document::{DocumentManager, ValidationRequest};
use crate::lsp::workspace::WorkspaceManager;
use crate::validation::ArazzoOpenApiValidator;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::{
    CompletionOptions, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer};

/// Arazzo Language Server
pub struct ArazzoLanguageServer {
    client: Client,
    document_manager: Arc<DocumentManager>,
    workspace_manager: Arc<WorkspaceManager>,
}

impl ArazzoLanguageServer {
    pub fn new(client: Client, root_dir: PathBuf) -> Self {
        // Create validation channel
        let (validation_tx, validation_rx) = mpsc::channel::<ValidationRequest>(100);

        let document_manager = Arc::new(DocumentManager::new(validation_tx));
        let workspace_manager = Arc::new(WorkspaceManager::new(root_dir));

        // Spawn validation worker
        let client_clone = client.clone();
        let doc_manager_clone = document_manager.clone();
        let workspace_manager_clone = workspace_manager.clone();

        tokio::spawn(async move {
            validation_worker(
                validation_rx,
                client_clone,
                doc_manager_clone,
                workspace_manager_clone,
            )
            .await;
        });

        Self {
            client,
            document_manager,
            workspace_manager,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ArazzoLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> RpcResult<InitializeResult> {
        // Initialize workspace
        if let Err(e) = self.workspace_manager.initialize() {
            eprintln!("Failed to initialize workspace: {}", e);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                definition_provider: Some(tower_lsp::lsp_types::OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), " ".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                "Hornet2 LSP server initialized",
            )
            .await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let content = params.text_document.text;

        self.document_manager.update(uri, version, content);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next() {
            self.document_manager.update(uri, version, change.text);
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Refresh workspace on save (in case OpenAPI files changed)
        if params.text_document.uri.path().contains("openapi") {
            self.workspace_manager
                .invalidate_cache_for_document(&params.text_document.uri);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.document_manager.remove(&params.text_document.uri);
    }

    async fn hover(
        &self,
        params: tower_lsp::lsp_types::HoverParams,
    ) -> RpcResult<Option<tower_lsp::lsp_types::Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        crate::lsp::hover::provide_hover(
            &self.document_manager,
            &self.workspace_manager,
            &uri,
            position,
        )
    }

    async fn goto_definition(
        &self,
        params: tower_lsp::lsp_types::GotoDefinitionParams,
    ) -> RpcResult<Option<tower_lsp::lsp_types::GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        crate::lsp::definition::provide_definition(
            &self.document_manager,
            &self.workspace_manager,
            &uri,
            position,
        )
    }

    async fn completion(
        &self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> RpcResult<Option<tower_lsp::lsp_types::CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        crate::lsp::completion::provide_completion(
            &self.document_manager,
            &self.workspace_manager,
            &uri,
            position,
        )
    }
}

/// Background worker that validates documents
async fn validation_worker(
    mut rx: mpsc::Receiver<ValidationRequest>,
    client: Client,
    doc_manager: Arc<DocumentManager>,
    workspace_manager: Arc<WorkspaceManager>,
) {
    while let Some(req) = rx.recv().await {
        if let Err(e) =
            validate_and_publish(&req.uri, &client, &doc_manager, &workspace_manager).await
        {
            eprintln!("Validation error for {}: {}", req.uri, e);
        }
    }
}

/// Validate a document and publish diagnostics
async fn validate_and_publish(
    uri: &Url,
    client: &Client,
    doc_manager: &Arc<DocumentManager>,
    workspace_manager: &Arc<WorkspaceManager>,
) -> Result<()> {
    // Get document
    let doc = match doc_manager.get(uri) {
        Some(d) => d,
        None => return Ok(()),
    };

    // Only validate Arazzo files
    if !uri.path().contains("arazzo") {
        return Ok(());
    }

    // Find project
    let project = match workspace_manager.find_project_for_file(uri) {
        Some(p) => p,
        None => {
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::WARNING,
                    format!("No project found for {}", uri),
                )
                .await;
            return Ok(());
        }
    };

    // Get OpenAPI spec
    let openapi = match workspace_manager.get_openapi_for_project(&project) {
        Ok(o) => o,
        Err(e) => {
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::ERROR,
                    format!("Failed to load OpenAPI: {}", e),
                )
                .await;
            return Ok(());
        }
    };

    // Run validation
    let validator = ArazzoOpenApiValidator::new(&project.arazzo_spec, &openapi);
    let result = match validator.validate_all() {
        Ok(result) => result,
        Err(e) => {
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::ERROR,
                    format!("Validation failed: {}", e),
                )
                .await;
            return Ok(());
        }
    };

    let errors = result.errors;
    let warnings = result.warnings;

    // Convert to diagnostics
    let mut diagnostics = Vec::new();

    for error in &errors {
        let diagnostic = validation_error_to_diagnostic(error, &doc.position_map);
        diagnostics.push(diagnostic);
    }

    for warning in &warnings {
        let diagnostic = validation_warning_to_diagnostic(warning, &doc.position_map);
        diagnostics.push(diagnostic);
    }

    // Publish diagnostics
    client
        .publish_diagnostics(uri.clone(), diagnostics, Some(doc.version))
        .await;

    Ok(())
}
