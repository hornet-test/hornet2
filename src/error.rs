use thiserror::Error;

#[derive(Error, Debug)]
pub enum HornetError {
    #[error("Failed to load OpenAPI file: {0}")]
    OpenApiLoadError(String),

    #[error("Failed to load Arazzo file: {0}")]
    ArazzoLoadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Graph generation error: {0}")]
    GraphError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Runtime expression error: {0}")]
    RuntimeExprError(String),

    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    #[error("Operation not found: {0}")]
    OperationNotFound(String),
}

pub type Result<T> = std::result::Result<T, HornetError>;
