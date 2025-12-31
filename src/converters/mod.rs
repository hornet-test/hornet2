//! Converters for generating test scripts from Arazzo workflows
//!
//! This module provides traits and implementations for converting
//! Arazzo workflows to various test script formats.

pub mod k6;

pub use k6::K6Converter;

use crate::error::Result;
use crate::loader::OpenApiResolver;
use crate::models::arazzo::{ArazzoSpec, Workflow};

/// Configuration options for converters
#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    /// Base URL to use for API requests (overrides OpenAPI server URL)
    pub base_url: Option<String>,
    /// Number of virtual users for load testing
    pub vus: Option<u32>,
    /// Duration for load testing (e.g., "30s", "5m")
    pub duration: Option<String>,
    /// Number of iterations (mutually exclusive with duration)
    pub iterations: Option<u32>,
}

/// Trait for converting Arazzo workflows to test scripts
pub trait Converter {
    /// The output type of the conversion (usually String for script content)
    type Output;

    /// Convert an entire Arazzo specification to test scripts
    fn convert_spec(
        &self,
        arazzo: &ArazzoSpec,
        resolver: &OpenApiResolver,
        options: &ConvertOptions,
    ) -> Result<Self::Output>;

    /// Convert a single workflow to a test script
    fn convert_workflow(
        &self,
        workflow: &Workflow,
        resolver: &OpenApiResolver,
        options: &ConvertOptions,
    ) -> Result<Self::Output>;
}
