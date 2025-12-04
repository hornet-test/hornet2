use crate::{
    graph::{builder::build_flow_graph, validator::validate_flow_graph},
    loader, Result,
};
use colored::*;
use std::path::Path;

pub fn execute_validate(openapi_path: &Path, arazzo_path: &Path) -> Result<()> {
    let mut has_errors = false;

    // Validate OpenAPI
    println!("{}", "Validating OpenAPI file...".bright_blue());
    println!("  Path: {}", openapi_path.display());

    let openapi = match loader::load_openapi(openapi_path.to_str().unwrap()) {
        Ok(spec) => {
            println!("{}", "✓ OpenAPI is valid".green());
            println!("  Title: {}", spec.info.title.bold());
            println!("  Version: {}", spec.info.version);
            println!("  OpenAPI Version: {}", spec.openapi);
            let path_count = spec.paths.as_ref().map(|p| p.len()).unwrap_or(0);
            println!("  Paths: {}", path_count);
            println!();
            Some(spec)
        }
        Err(e) => {
            println!("{}", "✗ OpenAPI validation failed".red().bold());
            println!("  {}", e.to_string().red());
            has_errors = true;
            println!();
            None
        }
    };

    // Validate Arazzo
    println!("{}", "Validating Arazzo file...".bright_blue());
    println!("  Path: {}", arazzo_path.display());

    let arazzo = match loader::load_arazzo(arazzo_path.to_str().unwrap()) {
        Ok(spec) => {
            println!("{}", "✓ Arazzo is valid".green());
            println!("  Title: {}", spec.info.title.bold());
            println!("  Version: {}", spec.info.version);
            println!("  Arazzo Version: {}", spec.arazzo);
            println!("  Workflows: {}", spec.workflows.len());
            println!();
            Some(spec)
        }
        Err(e) => {
            println!("{}", "✗ Arazzo validation failed".red().bold());
            println!("  {}", e.to_string().red());
            has_errors = true;
            println!();
            None
        }
    };

    // Validate workflows (graph structure)
    if let Some(arazzo) = arazzo {
        println!("{}", "Validating workflow graphs...".bright_blue());

        for workflow in &arazzo.workflows {
            println!("  Workflow: {}", workflow.workflow_id.cyan());

            match build_flow_graph(workflow, openapi.as_ref()) {
                Ok(graph) => match validate_flow_graph(&graph) {
                    Ok(validation) => {
                        if validation.is_ok() {
                            println!("    {}", "✓ Graph is valid".green());
                        } else {
                            if !validation.warnings.is_empty() {
                                println!("    {}", "⚠ Warnings:".yellow());
                                for warning in &validation.warnings {
                                    println!("      - {}", warning.yellow());
                                }
                            }

                            if !validation.errors.is_empty() {
                                println!("    {}", "✗ Errors:".red().bold());
                                for error in &validation.errors {
                                    println!("      - {}", error.red());
                                }
                                has_errors = true;
                            }
                        }
                    }
                    Err(e) => {
                        println!("    {}", "✗ Validation failed".red().bold());
                        println!("      {}", e.to_string().red());
                        has_errors = true;
                    }
                },
                Err(e) => {
                    println!("    {}", "✗ Failed to build graph".red().bold());
                    println!("      {}", e.to_string().red());
                    has_errors = true;
                }
            }
        }
        println!();
    }

    // Summary
    if has_errors {
        println!("{}", "✗ Validation completed with errors".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "✓ All validations passed successfully".green().bold());
        Ok(())
    }
}
