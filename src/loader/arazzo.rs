use crate::error::{HornetError, Result};
use crate::models::arazzo::ArazzoSpec;
use std::fs;
use std::path::Path;

/// ファイルからArazzo仕様をロードする
pub fn load_arazzo<P: AsRef<Path>>(path: P) -> Result<ArazzoSpec> {
    let path = path.as_ref();

    // ファイルを読み込む
    let content = fs::read_to_string(path).map_err(|e| {
        HornetError::ArazzoLoadError(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    // YAMLをパースする
    let spec: ArazzoSpec = serde_yaml::from_str(&content)
        .map_err(|e| HornetError::ArazzoLoadError(format!("Failed to parse Arazzo YAML: {}", e)))?;

    // 検証する
    spec.validate()?;

    Ok(spec)
}

/// Arazzo仕様をファイルに保存する
pub fn save_arazzo<P: AsRef<Path>>(path: P, spec: &ArazzoSpec) -> Result<()> {
    let path = path.as_ref();

    // 保存前に検証する
    spec.validate()?;

    // YAMLにシリアライズする
    let yaml = serde_yaml::to_string(spec).map_err(|e| {
        HornetError::ArazzoLoadError(format!("Failed to serialize Arazzo to YAML: {}", e))
    })?;

    // ファイルに書き込む
    fs::write(path, yaml).map_err(|e| {
        HornetError::ArazzoLoadError(format!("Failed to write file {}: {}", path.display(), e))
    })?;

    Ok(())
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

    #[test]
    fn test_save_arazzo() {
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
        // First load it
        let mut input_file = NamedTempFile::new().unwrap();
        input_file.write_all(yaml.as_bytes()).unwrap();
        let spec = load_arazzo(input_file.path()).unwrap();

        // Then save it to a new file
        let output_file = NamedTempFile::new().unwrap();
        let output_path = output_file.path().to_path_buf(); // Keep path before file drops

        let result = save_arazzo(&output_path, &spec);
        assert!(result.is_ok());

        // Read it back and verify
        let saved_spec = load_arazzo(&output_path).unwrap();
        assert_eq!(saved_spec.info.title, "Test Workflow");
        assert_eq!(saved_spec.workflows.len(), 1);
        assert_eq!(saved_spec.workflows[0].workflow_id, "test-flow");
    }

    #[test]
    fn test_save_preserves_field_order() {
        use indexmap::IndexMap;

        // Create spec with extensions in specific order
        let mut extensions = IndexMap::new();
        // Add extensions in specific order (alphabetically reversed)
        extensions.insert("x-zebra".to_string(), serde_json::json!("last"));
        extensions.insert("x-middle".to_string(), serde_json::json!("middle"));
        extensions.insert("x-alpha".to_string(), serde_json::json!("first"));

        let workflow = crate::models::arazzo::Workflow {
            workflow_id: "test".to_string(),
            summary: Some("Test workflow".to_string()),
            description: None,
            inputs: None,
            steps: vec![crate::models::arazzo::Step {
                step_id: "step1".to_string(),
                description: None,
                operation_id: Some("testOp".to_string()),
                operation_path: None,
                workflow_id: None,
                parameters: vec![],
                request_body: None,
                success_criteria: None,
                on_success: None,
                on_failure: None,
                outputs: None,
            }],
            success_criteria: None,
            outputs: None,
            extensions,
        };

        let spec = crate::models::arazzo::ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Order Test".to_string(),
                summary: None,
                description: None,
                version: "1.0.0".to_string(),
            },
            source_descriptions: vec![],
            workflows: vec![workflow],
            components: None,
        };

        // Save and check order
        let output_file = NamedTempFile::new().unwrap();
        let output_path = output_file.path().to_path_buf();
        save_arazzo(&output_path, &spec).unwrap();

        // Verify order in YAML (should be insertion order, not alphabetical)
        let yaml = std::fs::read_to_string(&output_path).unwrap();
        let zebra_pos = yaml.find("x-zebra").unwrap();
        let middle_pos = yaml.find("x-middle").unwrap();
        let alpha_pos = yaml.find("x-alpha").unwrap();

        assert!(
            zebra_pos < middle_pos,
            "x-zebra should come before x-middle"
        );
        assert!(
            middle_pos < alpha_pos,
            "x-middle should come before x-alpha"
        );

        // Verify round-trip preserves order
        let loaded = load_arazzo(&output_path).unwrap();
        let keys: Vec<_> = loaded.workflows[0].extensions.keys().cloned().collect();
        assert_eq!(keys, vec!["x-zebra", "x-middle", "x-alpha"]);
    }

    #[test]
    fn test_structural_field_order() {
        // Verify top-level fields appear in expected order
        let spec = crate::models::arazzo::ArazzoSpec {
            arazzo: "1.0.0".to_string(),
            info: crate::models::arazzo::Info {
                title: "Test".to_string(),
                summary: None,
                description: None,
                version: "1.0.0".to_string(),
            },
            source_descriptions: vec![],
            workflows: vec![],
            components: None,
        };

        let output_file = NamedTempFile::new().unwrap();
        let output_path = output_file.path().to_path_buf();
        save_arazzo(&output_path, &spec).unwrap();

        let yaml = std::fs::read_to_string(&output_path).unwrap();

        // Verify struct fields appear in definition order
        let arazzo_pos = yaml.find("arazzo:").unwrap();
        let info_pos = yaml.find("info:").unwrap();
        let workflows_pos = yaml.find("workflows:").unwrap();

        assert!(arazzo_pos < info_pos);
        assert!(info_pos < workflows_pos);
    }
}
