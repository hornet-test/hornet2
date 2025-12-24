//! Test runners for executing generated scripts
//!
//! This module provides implementations for running test scripts
//! using various engines (k6, etc.)

pub mod k6;

pub use k6::K6Runner;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Whether the run was successful
    pub success: bool,
    /// Exit code of the runner
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Metrics from the run (if available)
    pub metrics: Option<RunMetrics>,
}

/// Metrics from a test run
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunMetrics {
    /// Number of HTTP requests made
    pub http_reqs: u64,
    /// Number of iterations completed
    pub iterations: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Number of failed checks
    pub checks_failed: u64,
    /// Number of passed checks
    pub checks_passed: u64,
    /// Virtual users used
    pub vus: u32,
    /// Duration of the run
    pub duration: String,
}

/// Trait for test runners
pub trait Runner {
    /// Check if the runner is available (installed)
    fn is_available(&self) -> bool;

    /// Get the version of the runner
    fn version(&self) -> Result<String>;

    /// Run a script file
    fn run_script<P: AsRef<Path>>(&self, script_path: P) -> Result<RunResult>;

    /// Run script content directly (without writing to a file)
    fn run_script_content(&self, content: &str) -> Result<RunResult>;
}
