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

    #[test]
    fn test_resolve_source_descriptions_empty() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must define sourceDescriptions"));
    }

    #[test]
    fn test_resolve_source_descriptions_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![crate::models::arazzo::SourceDescription {
                name: "missing-api".to_string(),
                url: "./nonexistent.yaml".to_string(),
                source_type: Some("openapi".to_string()),
            }],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("OpenAPI file not found"));
        assert!(err_msg.contains("missing-api"));
    }

    #[test]
    fn test_resolve_source_descriptions_external_url() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![crate::models::arazzo::SourceDescription {
                name: "external-api".to_string(),
                url: "https://example.com/openapi.yaml".to_string(),
                source_type: Some("openapi".to_string()),
            }],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("External OpenAPI URLs are not yet supported"));
    }

    #[test]
    fn test_resolve_source_descriptions_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Create multiple OpenAPI files
        fs::write(
            project_dir.join("api1.yaml"),
            r#"openapi: 3.0.0
info:
  title: API 1
  version: 1.0.0
paths: {}
"#,
        )
        .unwrap();

        fs::write(
            project_dir.join("api2.yaml"),
            r#"openapi: 3.0.0
info:
  title: API 2
  version: 1.0.0
paths: {}
"#,
        )
        .unwrap();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![
                crate::models::arazzo::SourceDescription {
                    name: "first-api".to_string(),
                    url: "./api1.yaml".to_string(),
                    source_type: Some("openapi".to_string()),
                },
                crate::models::arazzo::SourceDescription {
                    name: "second-api".to_string(),
                    url: "./api2.yaml".to_string(),
                    source_type: Some("openapi".to_string()),
                },
            ],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_ok());
        let (paths, name_map) = result.unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(name_map.len(), 2);

        // Verify name mappings
        let api1_path = project_dir.join("api1.yaml");
        let api2_path = project_dir.join("api2.yaml");
        assert_eq!(name_map.get(&api1_path), Some(&"first-api".to_string()));
        assert_eq!(name_map.get(&api2_path), Some(&"second-api".to_string()));
    }

    #[test]
    fn test_resolve_source_descriptions_all_non_openapi() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![
                crate::models::arazzo::SourceDescription {
                    name: "graphql-api".to_string(),
                    url: "./schema.graphql".to_string(),
                    source_type: Some("graphql".to_string()),
                },
                crate::models::arazzo::SourceDescription {
                    name: "grpc-api".to_string(),
                    url: "./service.proto".to_string(),
                    source_type: Some("grpc".to_string()),
                },
            ],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No OpenAPI sources found"));
    }

    #[test]
    fn test_resolve_source_descriptions_mixed_types() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Create OpenAPI file only
        fs::write(
            project_dir.join("openapi.yaml"),
            r#"openapi: 3.0.0
info:
  title: REST API
  version: 1.0.0
paths: {}
"#,
        )
        .unwrap();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![
                crate::models::arazzo::SourceDescription {
                    name: "rest-api".to_string(),
                    url: "./openapi.yaml".to_string(),
                    source_type: Some("openapi".to_string()),
                },
                crate::models::arazzo::SourceDescription {
                    name: "graphql-api".to_string(),
                    url: "./schema.graphql".to_string(),
                    source_type: Some("graphql".to_string()),
                },
            ],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        // Should succeed with only the OpenAPI file
        assert!(result.is_ok());
        let (paths, name_map) = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(name_map.len(), 1);

        let openapi_path = project_dir.join("openapi.yaml");
        assert_eq!(name_map.get(&openapi_path), Some(&"rest-api".to_string()));
    }

    #[test]
    fn test_resolve_source_descriptions_default_type() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        fs::write(
            project_dir.join("openapi.yaml"),
            r#"openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths: {}
"#,
        )
        .unwrap();

        let arazzo_spec = ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                summary: None,
                description: None,
            },
            source_descriptions: vec![crate::models::arazzo::SourceDescription {
                name: "default-api".to_string(),
                url: "./openapi.yaml".to_string(),
                source_type: None, // No type specified - should default to "openapi"
            }],
            workflows: vec![],
            components: None,
        };

        let scanner = ProjectScanner::new(project_dir);
        let result = scanner.resolve_source_descriptions(&arazzo_spec, project_dir);

        assert!(result.is_ok());
        let (paths, name_map) = result.unwrap();
        assert_eq!(paths.len(), 1);

        let openapi_path = project_dir.join("openapi.yaml");
        assert_eq!(
            name_map.get(&openapi_path),
            Some(&"default-api".to_string())
        );
    }
}
