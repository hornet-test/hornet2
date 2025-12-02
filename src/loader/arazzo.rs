use crate::error::{HornetError, Result};
use crate::models::arazzo::ArazzoSpec;
use std::fs;
use std::path::Path;

/// Load an Arazzo specification from a file
pub fn load_arazzo<P: AsRef<Path>>(path: P) -> Result<ArazzoSpec> {
    let path = path.as_ref();

    // Read the file
    let content = fs::read_to_string(path).map_err(|e| {
        HornetError::ArazzoLoadError(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    // Parse YAML
    let spec: ArazzoSpec = serde_yaml::from_str(&content).map_err(|e| {
        HornetError::ArazzoLoadError(format!("Failed to parse Arazzo YAML: {}", e))
    })?;

    // Validate
    spec.validate()?;

    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_arazzo() {
        let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
sourceDescriptions:
  - name: api
    url: openapi.yaml
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        operationId: getTest
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_arazzo(file.path());
        assert!(result.is_ok());

        let spec = result.unwrap();
        assert_eq!(spec.info.title, "Test Workflow");
        assert_eq!(spec.workflows.len(), 1);
        assert_eq!(spec.workflows[0].workflow_id, "test-flow");
    }

    #[test]
    fn test_load_invalid_version() {
        let yaml = r#"
arazzo: 2.0.0
info:
  title: Test Workflow
  version: 1.0.0
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        operationId: getTest
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_arazzo(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_duplicate_workflow_ids() {
        let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        operationId: getTest
  - workflowId: test-flow
    steps:
      - stepId: step2
        operationId: getOther
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_arazzo(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_duplicate_step_ids() {
        let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        operationId: getTest
      - stepId: step1
        operationId: getOther
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_arazzo(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_step_without_operation() {
        let yaml = r#"
arazzo: 1.0.0
info:
  title: Test Workflow
  version: 1.0.0
workflows:
  - workflowId: test-flow
    steps:
      - stepId: step1
        description: A step without operation reference
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_arazzo(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_arazzo("/nonexistent/file.yaml");
        assert!(result.is_err());
    }
}
