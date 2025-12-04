use crate::{
    cli::OutputFormat,
    graph::{
        builder::build_flow_graph,
        exporter::{export_dot, export_json, export_mermaid},
        validator::validate_flow_graph,
    },
    loader, Result,
};
use colored::*;
use std::path::PathBuf;

pub fn execute_visualize(
    arazzo_path: &PathBuf,
    openapi_path: &Option<PathBuf>,
    format: &OutputFormat,
    output_path: &Option<PathBuf>,
) -> Result<()> {
    // Load Arazzo
    println!("{}", "Loading Arazzo file...".bright_blue());
    println!("  Path: {}", arazzo_path.display());

    let arazzo = loader::load_arazzo(arazzo_path.to_str().unwrap())?;
    println!("{}", "✓ Arazzo loaded successfully".green());
    println!();

    // Load OpenAPI (optional)
    let openapi = if let Some(path) = openapi_path {
        println!("{}", "Loading OpenAPI file...".bright_blue());
        println!("  Path: {}", path.display());

        let spec = loader::load_openapi(path.to_str().unwrap())?;
        println!("{}", "✓ OpenAPI loaded successfully".green());
        println!();
        Some(spec)
    } else {
        None
    };

    // Build graph for the first workflow
    if arazzo.workflows.is_empty() {
        eprintln!("{}", "✗ No workflows found in Arazzo file".red().bold());
        std::process::exit(1);
    }

    let workflow = &arazzo.workflows[0];
    println!(
        "{}",
        format!("Building flow graph for workflow: {}", workflow.workflow_id).bright_blue()
    );

    let graph = build_flow_graph(workflow, openapi.as_ref())?;
    println!("{}", "✓ Graph built successfully".green());
    println!();

    // Validate graph
    println!("{}", "Validating graph...".bright_blue());
    let validation = validate_flow_graph(&graph)?;

    if validation.is_ok() {
        println!("{}", "✓ Graph is valid".green());
    } else {
        if !validation.warnings.is_empty() {
            println!("{}", "⚠ Warnings:".yellow());
            for warning in &validation.warnings {
                println!("  - {}", warning.yellow());
            }
        }

        if !validation.errors.is_empty() {
            println!("{}", "✗ Errors:".red().bold());
            for error in &validation.errors {
                println!("  - {}", error.red());
            }
        }
    }
    println!();

    // Export graph
    let output = match format {
        OutputFormat::Dot => export_dot(&graph),
        OutputFormat::Json => {
            let json = export_json(&graph)?;
            serde_json::to_string_pretty(&json)?
        }
        OutputFormat::Mermaid => export_mermaid(&graph),
    };

    // Write output
    if let Some(path) = output_path {
        std::fs::write(path, &output)?;
        println!(
            "{}",
            format!("✓ Output written to: {}", path.display())
                .green()
                .bold()
        );
    } else {
        println!("{}", "# Output:".bright_cyan());
        println!();
        println!("{}", output);
    }

    Ok(())
}
