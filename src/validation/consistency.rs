use super::ErrorType;
use super::data_dependencies::DataDependencyValidator;
use super::operations::OperationValidator;
use super::parameters::ParameterValidator;
use super::schemas::SchemaValidator;
use crate::error::Result;
use crate::loader::OpenApiResolver;
use crate::models::arazzo::ArazzoSpec;

/// Main validator for Arazzo-OpenAPI consistency
pub struct ArazzoOpenApiValidator<'a> {
    arazzo: &'a ArazzoSpec,
    resolver: &'a OpenApiResolver,
}

impl<'a> ArazzoOpenApiValidator<'a> {
    /// Create a new consistency validator
    pub fn new(arazzo: &'a ArazzoSpec, resolver: &'a OpenApiResolver) -> Self {
        Self { arazzo, resolver }
    }

    /// Validate all consistency checks
    pub fn validate_all(&self) -> Result<ConsistencyValidationResult> {
        let mut result = ConsistencyValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        // Phase 1: Basic operation reference checks
        let op_validator = OperationValidator::new(self.arazzo, self.resolver);
        let (op_errors, op_warnings) = op_validator.validate()?;
        result.errors.extend(op_errors);
        result.warnings.extend(op_warnings);

        // If there are critical operation errors, skip further validation
        if !result.errors.is_empty() {
            result.is_valid = false;
            return Ok(result);
        }

        // Phase 2: Parameter validation
        let param_validator = ParameterValidator::new(self.arazzo, self.resolver);
        let (param_errors, param_warnings) = param_validator.validate()?;
        result.errors.extend(param_errors);
        result.warnings.extend(param_warnings);

        // Phase 3: Data dependency validation
        let dep_validator = DataDependencyValidator::new(self.arazzo, self.resolver);
        let (dep_errors, dep_warnings) = dep_validator.validate()?;
        result.errors.extend(dep_errors);
        result.warnings.extend(dep_warnings);

        // Phase 4: Schema validation (warnings only)
        let schema_validator = SchemaValidator::new(self.arazzo, self.resolver);
        let (schema_errors, schema_warnings) = schema_validator.validate()?;
        result.errors.extend(schema_errors);
        result.warnings.extend(schema_warnings);

        // Update is_valid flag
        result.is_valid = result.errors.is_empty();

        Ok(result)
    }
}

/// Result of consistency validation
#[derive(Debug, Clone)]
pub struct ConsistencyValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub workflow_id: Option<String>,
    pub step_id: Option<String>,
    pub source_name: Option<String>,
    pub error_type: ErrorType,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
}

impl ValidationError {
    pub fn new(error_type: ErrorType, message: impl Into<String>) -> Self {
        Self {
            workflow_id: None,
            step_id: None,
            source_name: None,
            error_type,
            message: message.into(),
            file_path: None,
            line_number: None,
        }
    }

    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    pub fn with_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_source(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }

    pub fn with_location(mut self, file_path: impl Into<String>, line_number: usize) -> Self {
        self.file_path = Some(file_path.into());
        self.line_number = Some(line_number);
        self
    }

    /// Format error message with location context
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        // File location
        if let (Some(path), Some(line)) = (&self.file_path, self.line_number) {
            parts.push(format!("{}:{}", path, line));
        }

        // Source name
        if let Some(source) = &self.source_name {
            parts.push(format!("[source: {}]", source));
        }

        // Workflow/step context
        match (&self.workflow_id, &self.step_id) {
            (Some(w), Some(s)) => parts.push(format!("[workflow: {}, step: {}]", w, s)),
            (Some(w), None) => parts.push(format!("[workflow: {}]", w)),
            (None, Some(s)) => parts.push(format!("[step: {}]", s)),
            (None, None) => {}
        }

        // Message
        parts.push(self.message.clone());

        parts.join(" ")
    }
}

/// Validation warning with context
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub workflow_id: Option<String>,
    pub step_id: Option<String>,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
}

impl ValidationWarning {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            workflow_id: None,
            step_id: None,
            message: message.into(),
            file_path: None,
            line_number: None,
        }
    }

    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    pub fn with_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_location(mut self, file_path: impl Into<String>, line_number: usize) -> Self {
        self.file_path = Some(file_path.into());
        self.line_number = Some(line_number);
        self
    }

    /// Format warning message with location context
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        // File location
        if let (Some(path), Some(line)) = (&self.file_path, self.line_number) {
            parts.push(format!("{}:{}", path, line));
        }

        // Workflow/step context
        match (&self.workflow_id, &self.step_id) {
            (Some(w), Some(s)) => parts.push(format!("[workflow: {}, step: {}]", w, s)),
            (Some(w), None) => parts.push(format!("[workflow: {}]", w)),
            (None, Some(s)) => parts.push(format!("[step: {}]", s)),
            (None, None) => {}
        }

        // Message
        parts.push(self.message.clone());

        parts.join(" ")
    }
}
