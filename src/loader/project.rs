use crate::error::{HornetError, Result};
use crate::loader;
use crate::models::arazzo::ArazzoSpec;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// プロジェクトメタデータ
#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    pub name: String,
    pub directory: PathBuf,
    pub arazzo_path: PathBuf,
    pub arazzo_spec: ArazzoSpec,
    pub openapi_paths: Vec<PathBuf>,
    /// Map from OpenAPI file path to source description name
    pub source_name_map: HashMap<PathBuf, String>,
}

/// プロジェクトスキャナー
pub struct ProjectScanner {
    root_dir: PathBuf,
}

impl ProjectScanner {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// すべてのプロジェクトをスキャン
    pub fn scan_projects(&self) -> Result<Vec<ProjectMetadata>> {
        let mut projects = Vec::new();

        // Check if the root directory itself contains arazzo.yaml (single-project mode)
        let root_arazzo = self.root_dir.join("arazzo.yaml");
        if root_arazzo.exists() {
            match self.load_project_metadata(&self.root_dir) {
                Ok(meta) => projects.push(meta),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load project at root {}: {}",
                        self.root_dir.display(),
                        e
                    );
                }
            }
        }

        // Scan subdirectories for additional projects (multi-project mode)
        for entry in fs::read_dir(&self.root_dir).map_err(HornetError::IoError)? {
            let entry = entry.map_err(HornetError::IoError)?;

            let path = entry.path();

            if path.is_dir() {
                let arazzo_path = path.join("arazzo.yaml");

                if arazzo_path.exists() {
                    match self.load_project_metadata(&path) {
                        Ok(meta) => projects.push(meta),
                        Err(e) => {
                            // ログ警告して続行
                            eprintln!(
                                "Warning: Failed to load project at {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(projects)
    }

    /// 単一プロジェクトのメタデータを読み込み
    pub fn load_project_metadata(&self, project_dir: &Path) -> Result<ProjectMetadata> {
        // Get project name from directory name, or use canonical path's file_name for root dir
        let project_name = if project_dir == self.root_dir {
            // For root directory, use the absolute path's last component
            project_dir
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "default".to_string())
        } else {
            project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| HornetError::InvalidPath("Invalid project directory name".into()))?
                .to_string()
        };

        let arazzo_path = project_dir.join("arazzo.yaml");

        if !arazzo_path.exists() {
            return Err(HornetError::ArazzoLoadError(format!(
                "arazzo.yaml not found in {}",
                project_dir.display()
            )));
        }

        let arazzo_spec = loader::load_arazzo(&arazzo_path)?;

        let (openapi_paths, source_name_map) =
            self.resolve_source_descriptions(&arazzo_spec, project_dir)?;

        Ok(ProjectMetadata {
            name: project_name,
            directory: project_dir.to_path_buf(),
            arazzo_path,
            arazzo_spec,
            openapi_paths,
            source_name_map,
        })
    }

    /// Resolve OpenAPI files from Arazzo sourceDescriptions
    fn resolve_source_descriptions(
        &self,
        arazzo_spec: &ArazzoSpec,
        project_dir: &Path,
    ) -> Result<(Vec<PathBuf>, HashMap<PathBuf, String>)> {
        let mut openapi_paths = Vec::new();
        let mut source_name_map = HashMap::new();

        // sourceDescriptions must be defined
        if arazzo_spec.source_descriptions.is_empty() {
            return Err(HornetError::ArazzoLoadError(
                "arazzo.yaml must define sourceDescriptions with OpenAPI files".into(),
            ));
        }

        // Process sourceDescriptions
        for source_desc in &arazzo_spec.source_descriptions {
            // Filter by type (only process "openapi" sources)
            let source_type = source_desc.source_type.as_deref().unwrap_or("openapi");

            if source_type != "openapi" {
                continue;
            }

            // Resolve URL (relative path or external URL)
            let path = if source_desc.url.starts_with("http://")
                || source_desc.url.starts_with("https://")
            {
                // External URL: Not supported yet
                return Err(HornetError::OpenApiLoadError(format!(
                    "External OpenAPI URLs are not yet supported: {}",
                    source_desc.url
                )));
            } else {
                // Relative path: resolve from project directory
                let relative = source_desc.url.trim_start_matches("./");
                project_dir.join(relative)
            };

            // Check if file exists
            if !path.exists() {
                return Err(HornetError::OpenApiLoadError(format!(
                    "OpenAPI file not found: {} (referenced in sourceDescriptions as '{}')",
                    path.display(),
                    source_desc.name
                )));
            }

            openapi_paths.push(path.clone());
            source_name_map.insert(path, source_desc.name.clone());
        }

        // Ensure at least one OpenAPI source was found
        if openapi_paths.is_empty() {
            return Err(HornetError::ArazzoLoadError(
                "No OpenAPI sources found in sourceDescriptions (all sources may be non-openapi type)".into(),
            ));
        }

        Ok((openapi_paths, source_name_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_projects() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // プロジェクト1
        let project1 = root.join("project1");
        fs::create_dir(&project1).unwrap();
        fs::write(
            project1.join("arazzo.yaml"),
            r#"arazzo: 1.0.0
info:
  title: Project 1
  version: 1.0.0
sourceDescriptions:
  - name: api1
    url: ./openapi.yaml
    type: openapi
workflows:
  - workflowId: test
    steps:
      - stepId: step1
        operationId: op1
"#,
        )
        .unwrap();
        fs::write(
            project1.join("openapi.yaml"),
            r#"openapi: 3.0.0
info:
  title: API 1
  version: 1.0.0
paths:
  /test:
    get:
      operationId: op1
      responses:
        '200':
          description: OK
"#,
        )
        .unwrap();

        // プロジェクト2
        let project2 = root.join("project2");
        fs::create_dir(&project2).unwrap();
        fs::write(
            project2.join("arazzo.yaml"),
            r#"arazzo: 1.0.0
info:
  title: Project 2
  version: 1.0.0
sourceDescriptions:
  - name: api2
    url: ./openapi.yaml
    type: openapi
workflows:
  - workflowId: test2
    steps:
      - stepId: step1
        operationId: op2
"#,
        )
        .unwrap();
        fs::write(
            project2.join("openapi.yaml"),
            r#"openapi: 3.0.0
info:
  title: API 2
  version: 1.0.0
paths:
  /test2:
    get:
      operationId: op2
      responses:
        '200':
          description: OK
"#,
        )
        .unwrap();

        // arazzo.yamlのないディレクトリ（無視される）
        let not_project = root.join("not_a_project");
        fs::create_dir(&not_project).unwrap();

        let scanner = ProjectScanner::new(root);
        let projects = scanner.scan_projects().unwrap();

        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.name == "project1"));
        assert!(projects.iter().any(|p| p.name == "project2"));
    }
}
