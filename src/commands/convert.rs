//! Convert command implementation
//!
//! Converts Arazzo workflows to various test script formats.

use crate::converters::{ConvertOptions, Converter, K6Converter};
use crate::error::Result;
use crate::loader::{arazzo::load_arazzo, openapi::load_openapi};
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Execute the convert command
pub fn execute_convert(
    arazzo_path: &Path,
    openapi_path: &Path,
    output_path: Option<&Path>,
    target: &str,
    workflow_id: Option<&str>,
    base_url: Option<&str>,
    vus: Option<u32>,
    duration: Option<&str>,
    iterations: Option<u32>,
) -> Result<()> {
    // Load Arazzo file
    let arazzo = load_arazzo(arazzo_path)?;
    println!(
        "{} Loaded Arazzo file: {}",
        "✓".green(),
        arazzo_path.display()
    );

    // Load OpenAPI file
    let openapi = load_openapi(openapi_path)?;
    println!(
        "{} Loaded OpenAPI file: {}",
        "✓".green(),
        openapi_path.display()
    );

    // Build convert options
    let options = ConvertOptions {
        base_url: base_url.map(|s| s.to_string()),
        vus,
        duration: duration.map(|s| s.to_string()),
        iterations,
    };

    // Generate script based on target
    let script = match target.to_lowercase().as_str() {
        "k6" => {
            let converter = K6Converter::new();

            if let Some(wf_id) = workflow_id {
                // Convert specific workflow
                let workflow = arazzo
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == wf_id)
                    .ok_or_else(|| {
                        crate::error::HornetError::ValidationError(format!(
                            "Workflow '{}' not found",
                            wf_id
                        ))
                    })?;

                converter.convert_workflow(workflow, &openapi, &options)?
            } else {
                // Convert all workflows
                converter.convert_spec(&arazzo, &openapi, &options)?
            }
        }
        _ => {
            return Err(crate::error::HornetError::ValidationError(format!(
                "Unsupported target format: {}. Supported: k6",
                target
            )));
        }
    };

    // Output result
    if let Some(path) = output_path {
        fs::write(path, &script)?;
        println!(
            "{} Generated {} script: {}",
            "✓".green(),
            target,
            path.display()
        );
    } else {
        println!("\n{}", script);
    }

    Ok(())
}

/// Execute the run command (convert and run)
pub fn execute_run(
    arazzo_path: &Path,
    openapi_path: &Path,
    engine: &str,
    workflow_id: Option<&str>,
    base_url: Option<&str>,
    vus: Option<u32>,
    duration: Option<&str>,
    iterations: Option<u32>,
) -> Result<()> {
    use crate::runner::{K6Runner, Runner};

    // First, generate the script
    println!("{} Generating test script...", "→".blue());

    // Load files
    let arazzo = load_arazzo(arazzo_path)?;
    let openapi = load_openapi(openapi_path)?;

    let options = ConvertOptions {
        base_url: base_url.map(|s| s.to_string()),
        vus,
        duration: duration.map(|s| s.to_string()),
        iterations,
    };

    match engine.to_lowercase().as_str() {
        "k6" => {
            let converter = K6Converter::new();
            let runner = K6Runner::new();

            // Check if k6 is available
            if !runner.is_available() {
                return Err(crate::error::HornetError::ValidationError(
                    "k6 is not installed or not in PATH. Please install k6 first: https://k6.io/docs/get-started/installation/"
                        .to_string(),
                ));
            }

            println!("{} k6 version: {}", "✓".green(), runner.version()?);

            let script = if let Some(wf_id) = workflow_id {
                let workflow = arazzo
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == wf_id)
                    .ok_or_else(|| {
                        crate::error::HornetError::ValidationError(format!(
                            "Workflow '{}' not found",
                            wf_id
                        ))
                    })?;
                converter.convert_workflow(workflow, &openapi, &options)?
            } else {
                // Use first workflow
                converter.convert_workflow(&arazzo.workflows[0], &openapi, &options)?
            };

            println!("{} Running tests with k6...\n", "→".blue());

            let result = runner.run_script_content(&script)?;

            // Print output
            if !result.stdout.is_empty() {
                println!("{}", result.stdout);
            }

            if !result.stderr.is_empty() {
                eprintln!("{}", result.stderr);
            }

            // Print summary
            if result.success {
                println!("\n{} Test run completed successfully!", "✓".green());
            } else {
                println!(
                    "\n{} Test run failed with exit code: {}",
                    "✗".red(),
                    result.exit_code
                );
            }

            if let Some(metrics) = result.metrics {
                println!("\n{}", "Metrics Summary:".bold());
                println!("  HTTP Requests: {}", metrics.http_reqs);
                println!("  Iterations: {}", metrics.iterations);
                println!("  Avg Response Time: {:.2}ms", metrics.avg_response_time_ms);
                println!(
                    "  Checks: {} passed, {} failed",
                    metrics.checks_passed, metrics.checks_failed
                );
            }

            if !result.success {
                return Err(crate::error::HornetError::ValidationError(format!(
                    "Test run failed with exit code: {}",
                    result.exit_code
                )));
            }
        }
        _ => {
            return Err(crate::error::HornetError::ValidationError(format!(
                "Unsupported engine: {}. Supported: k6",
                engine
            )));
        }
    }

    Ok(())
}
