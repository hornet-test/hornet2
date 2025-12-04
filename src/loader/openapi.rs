use crate::error::{HornetError, Result};
use oas3::OpenApiV3Spec;
use std::fs;
use std::path::Path;

/// Load an OpenAPI specification from a file
pub fn load_openapi<P: AsRef<Path>>(path: P) -> Result<OpenApiV3Spec> {
    let path = path.as_ref();

    // Read the file
    let content = fs::read_to_string(path).map_err(|e| {
        HornetError::OpenApiLoadError(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    // Parse YAML
    let spec: OpenApiV3Spec = serde_yaml::from_str(&content).map_err(|e| {
        HornetError::OpenApiLoadError(format!("Failed to parse OpenAPI YAML: {}", e))
    })?;

    // Basic validation
    validate_openapi(&spec)?;

    Ok(spec)
}

/// Validate the OpenAPI specification
fn validate_openapi(spec: &OpenApiV3Spec) -> Result<()> {
    // Check version
    if !spec.openapi.starts_with("3.0") && !spec.openapi.starts_with("3.1") {
        return Err(HornetError::ValidationError(format!(
            "Unsupported OpenAPI version: {}. Only 3.0.x and 3.1.x are supported.",
            spec.openapi
        )));
    }

    // Check that there are paths defined
    if spec.paths.as_ref().is_none_or(|p| p.is_empty()) {
        return Err(HornetError::ValidationError(
            "OpenAPI spec must have at least one path".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_openapi() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      operationId: getTest
      responses:
        '200':
          description: OK
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_openapi(file.path());
        assert!(result.is_ok());

        let spec = result.unwrap();
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.openapi, "3.0.0");
    }

    #[test]
    fn test_load_invalid_version() {
        let yaml = r#"
openapi: 2.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_openapi(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_no_paths() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths: {}
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = load_openapi(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_openapi("/nonexistent/file.yaml");
        assert!(result.is_err());
    }
}
