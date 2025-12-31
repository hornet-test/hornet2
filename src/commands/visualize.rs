use crate::{
    HornetError, Result,
    cli::OutputFormat,
    graph::{
        builder::build_flow_graph,
        exporter::{export_dot, export_json, export_mermaid},
        validator::validate_flow_graph,
    },
    loader::{self, SourceDescriptionResolver},
};
use colored::*;
use std::path::PathBuf;

pub fn execute_visualize(
    arazzo_path: &PathBuf,
    format: &OutputFormat,
    output_path: &Option<PathBuf>,
) -> Result<()> {
    // Load Arazzo
    println!("{}", "Loading Arazzo file...".bright_blue());
    println!("  Path: {}", arazzo_path.display());

    let arazzo = loader::load_arazzo(arazzo_path)?;
    println!("{}", "✓ Arazzo loaded successfully".green());
    println!();

    // Load OpenAPI sources from sourceDescriptions
    let resolver_helper = SourceDescriptionResolver::new(arazzo_path)?;
    let source_result = resolver_helper.load_sources(&arazzo.source_descriptions);

    // Report source loading errors (but continue)
    if !source_result.errors.is_empty() {
        println!("{}", "⚠️  Some OpenAPI sources failed to load:".yellow());
        for err in &source_result.errors {
            println!("  - Source '{}': {}", err.name, err.message);
        }
        println!();
    }

    // Build graph for the first workflow
    if arazzo.workflows.is_empty() {
        return Err(HornetError::ValidationError(
            "No workflows found in Arazzo file".to_string(),
        ));
    }

    let workflow = &arazzo.workflows[0];
    println!(
        "{}",
        format!("Building flow graph for workflow: {}", workflow.workflow_id).bright_blue()
    );

    let resolver = if source_result.resolver.get_all_specs().is_empty() {
        None
    } else {
        Some(&source_result.resolver)
    };

    let graph = build_flow_graph(workflow, resolver)?;
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
