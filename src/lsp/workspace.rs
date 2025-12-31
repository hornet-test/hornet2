use crate::error::Result;
use crate::loader::openapi::load_openapi;
use crate::loader::openapi_resolver::OpenApiResolver;
use crate::loader::project::{ProjectMetadata, ProjectScanner};
use dashmap::DashMap;
use oas3::OpenApiV3Spec;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::lsp_types::Url;

/// Manages workspace folders and their projects
pub struct WorkspaceManager {
    root_path: PathBuf,
    projects: DashMap<String, ProjectInfo>,
    openapi_cache: DashMap<PathBuf, Arc<OpenApiV3Spec>>,
    resolver_cache: DashMap<PathBuf, Arc<OpenApiResolver>>,
}

impl WorkspaceManager {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            projects: DashMap::new(),
            openapi_cache: DashMap::new(),
            resolver_cache: DashMap::new(),
        }
    }

    /// Initialize workspace by scanning for projects
    pub fn initialize(&self) -> Result<()> {
        let scanner = ProjectScanner::new(&self.root_path);
        let projects = scanner.scan_projects()?;

        for project in projects {
            let project_info = ProjectInfo {
                metadata: Arc::new(project),
            };

            self.projects
                .insert(project_info.metadata.name.clone(), project_info);
        }

        Ok(())
    }

    /// Find project that contains the given file
    pub fn find_project_for_file(&self, uri: &Url) -> Option<Arc<ProjectMetadata>> {
        let file_path = uri.to_file_path().ok()?;

        for entry in self.projects.iter() {
            let project = &entry.value().metadata;
            if file_path.starts_with(&project.directory) {
                return Some(project.clone());
            }
        }

        None
    }

    /// Get OpenAPI spec for a project
    pub fn get_openapi_for_project(&self, project: &ProjectMetadata) -> Result<Arc<OpenApiV3Spec>> {
        // Try to find the first OpenAPI file in the project
        let openapi_path = project.openapi_paths.first().ok_or_else(|| {
            crate::error::HornetError::OpenApiLoadError(
                "No OpenAPI file found in project".to_string(),
            )
        })?;

        // Check cache
        if let Some(cached) = self.openapi_cache.get(openapi_path) {
            return Ok(cached.clone());
        }

        // Load and cache
        let openapi = load_openapi(openapi_path)?;
        let openapi_arc = Arc::new(openapi);
        self.openapi_cache
            .insert(openapi_path.clone(), openapi_arc.clone());

        Ok(openapi_arc)
    }

    /// Get OpenAPI spec for a document URI
    pub fn get_openapi_for_document(&self, uri: &Url) -> Option<(PathBuf, Arc<OpenApiV3Spec>)> {
        let project = self.find_project_for_file(uri)?;
        let openapi_path = project.openapi_paths.first()?.clone();
        let openapi = self.get_openapi_for_project(&project).ok()?;

        Some((openapi_path, openapi))
    }

    /// Get OpenAPI resolver for a document URI
    pub fn get_resolver_for_document(&self, uri: &Url) -> Option<Arc<OpenApiResolver>> {
        let project = self.find_project_for_file(uri)?;
        let project_dir = &project.directory;

        // Check cache based on project directory
        if let Some(cached) = self.resolver_cache.get(project_dir) {
            return Some(cached.clone());
        }

        // Create resolver with project directory
        let mut resolver = OpenApiResolver::new(&project.directory);

        // Load specs from OpenAPI paths
        if let Err(e) = resolver.load_specs(&project.openapi_paths) {
            eprintln!("Failed to load OpenAPI specs: {}", e);
            return None;
        }

        let resolver_arc = Arc::new(resolver);

        self.resolver_cache
            .insert(project_dir.clone(), resolver_arc.clone());

        Some(resolver_arc)
    }

    /// Invalidate caches for a specific document (e.g., when it changes)
    pub fn invalidate_cache_for_document(&self, uri: &Url) {
        if let Some(project) = self.find_project_for_file(uri) {
            for openapi_path in &project.openapi_paths {
                self.openapi_cache.remove(openapi_path);
                self.resolver_cache.remove(openapi_path);
            }
        }
    }

    /// Refresh project list (rescan the workspace)
    pub fn refresh_projects(&self) -> Result<()> {
        self.projects.clear();
        self.openapi_cache.clear();
        self.resolver_cache.clear();
        self.initialize()
    }
}

/// Information about a project in the workspace
#[derive(Debug, Clone)]
struct ProjectInfo {
    metadata: Arc<ProjectMetadata>,
}
