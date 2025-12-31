use crate::error::{HornetError, Result};
use crate::loader::OpenApiResolver;
use crate::models::arazzo::SourceDescription;
use std::path::{Path, PathBuf};

/// Resolves and loads OpenAPI specifications from Arazzo sourceDescriptions
#[derive(Debug)]
pub struct SourceDescriptionResolver {
    arazzo_dir: PathBuf,
}

impl SourceDescriptionResolver {
    /// Create a new resolver for the given Arazzo file path
    pub fn new(arazzo_path: &Path) -> Result<Self> {
        let arazzo_dir = arazzo_path
            .parent()
            .ok_or_else(|| {
                HornetError::InvalidPath(format!("Invalid Arazzo path: {:?}", arazzo_path))
            })?
            .to_path_buf();

        Ok(Self { arazzo_dir })
    }

    /// Load all OpenAPI specifications from sourceDescriptions
    pub fn load_sources(&self, source_descriptions: &[SourceDescription]) -> SourceLoadResult {
        let mut resolver = OpenApiResolver::new(&self.arazzo_dir);
        let mut errors = Vec::new();

        for source_desc in source_descriptions {
            // Skip non-OpenAPI sources
            if let Some(source_type) = &source_desc.source_type
                && source_type != "openapi"
            {
                continue;
            }

            // Resolve the URL to an absolute path
            match self.resolve_url(&source_desc.url) {
                Ok(path) => {
                    // Try to load the OpenAPI spec
                    if let Err(e) = resolver.load_spec(&source_desc.name, &path) {
                        errors.push(SourceLoadError {
                            name: source_desc.name.clone(),
                            url: source_desc.url.clone(),
                            message: e.to_string(),
                        });
                    }
                }
                Err(e) => {
                    errors.push(SourceLoadError {
                        name: source_desc.name.clone(),
                        url: source_desc.url.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        SourceLoadResult { resolver, errors }
    }

    /// Resolve a sourceDescription URL to an absolute path
    fn resolve_url(&self, url: &str) -> Result<PathBuf> {
        // Check for HTTP(S) URLs (not yet supported)
        if url.starts_with("http://") || url.starts_with("https://") {
            return Err(HornetError::InvalidPath(format!(
                "HTTP(S) URLs are not yet supported: {}",
                url
            )));
        }

        // Treat as file path (relative or absolute)
        let path = PathBuf::from(url);

        // If it's an absolute path, use as-is
        if path.is_absolute() {
            return Ok(path);
        }

        // Otherwise, resolve relative to Arazzo file directory
        let resolved = self.arazzo_dir.join(&path);

        // Canonicalize to resolve `.` and `..`
        // Note: This will fail if the file doesn't exist, but that's okay
        // because we want to return the resolved path even if it doesn't exist yet
        // (the error will be caught when trying to load the file)
        match resolved.canonicalize() {
            Ok(canonical) => Ok(canonical),
            Err(_) => {
                // If canonicalize fails, return the joined path as-is
                // This allows for better error messages when the file doesn't exist
                Ok(resolved)
            }
        }
    }
}

/// Result of loading sourceDescriptions
#[derive(Debug)]
pub struct SourceLoadResult {
    /// OpenAPI resolver with successfully loaded specs
    pub resolver: OpenApiResolver,
    /// Errors encountered while loading sources
    pub errors: Vec<SourceLoadError>,
}

/// Error encountered while loading a source
#[derive(Debug, Clone)]
pub struct SourceLoadError {
    /// Source name from sourceDescription
    pub name: String,
    /// Source URL from sourceDescription
    pub url: String,
    /// Error message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_new_resolver() {
        let arazzo_path = PathBuf::from("/tmp/test/arazzo.yaml");
        let resolver = SourceDescriptionResolver::new(&arazzo_path).unwrap();
        assert_eq!(resolver.arazzo_dir, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let arazzo_path = PathBuf::from("/tmp/test/arazzo.yaml");
        let resolver = SourceDescriptionResolver::new(&arazzo_path).unwrap();

        let url = "./openapi.yaml";
        let resolved = resolver.resolve_url(url).unwrap();
        // The exact path depends on whether the file exists, but it should contain the URL
        assert!(resolved.to_string_lossy().contains("openapi.yaml"));
    }

    #[test]
    fn test_resolve_http_url_fails() {
        let arazzo_path = PathBuf::from("/tmp/test/arazzo.yaml");
        let resolver = SourceDescriptionResolver::new(&arazzo_path).unwrap();

        let url = "https://example.com/openapi.yaml";
        let result = resolver.resolve_url(url);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HTTP(S) URLs are not yet supported")
        );
    }

    #[test]
    fn test_resolve_absolute_path() {
        let arazzo_path = PathBuf::from("/tmp/test/arazzo.yaml");
        let resolver = SourceDescriptionResolver::new(&arazzo_path).unwrap();

        let url = "/absolute/path/openapi.yaml";
        let resolved = resolver.resolve_url(url).unwrap();
        assert_eq!(resolved, PathBuf::from("/absolute/path/openapi.yaml"));
    }

    #[test]
    fn test_load_sources_with_missing_file() {
        let temp_dir = env::temp_dir().join("hornet2_test_source_resolver");
        fs::create_dir_all(&temp_dir).unwrap();

        let arazzo_path = temp_dir.join("arazzo.yaml");
        fs::write(&arazzo_path, "arazzo: 1.0.0").unwrap();

        let resolver = SourceDescriptionResolver::new(&arazzo_path).unwrap();

        let source_desc = SourceDescription {
            name: "testAPI".to_string(),
            url: "./missing.yaml".to_string(),
            source_type: Some("openapi".to_string()),
        };

        let result = resolver.load_sources(&[source_desc]);

        // Should have one error for the missing file
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].name, "testAPI");
        assert_eq!(result.errors[0].url, "./missing.yaml");

        // Cleanup
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
