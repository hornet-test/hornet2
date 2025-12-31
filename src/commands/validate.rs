use crate::{
    Result,
    graph::{builder::build_flow_graph, validator::validate_flow_graph},
    loader::{self, SourceDescriptionResolver},
};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_validate(arazzo_path: &PathBuf) -> Result<()> {
    let mut has_errors = false;

    // Validate Arazzo file
    println!("{}", "Validating Arazzo file...".bright_blue());
    println!("  Path: {}", arazzo_path.display());

    let arazzo = match loader::load_arazzo(arazzo_path) {
        Ok(spec) => {
            println!("{}", "✓ Arazzo structure is valid".green());
            println!("  Title: {}", spec.info.title.bold());
            println!("  Version: {}", spec.info.version);
            println!("  Arazzo Version: {}", spec.arazzo);
            println!("  Workflows: {}", spec.workflows.len());
            println!("  Source Descriptions: {}", spec.source_descriptions.len());
            println!();
            spec
        }
        Err(e) => {
            println!("{}", "✗ Arazzo validation failed".red().bold());
            println!("  {}", e.to_string().red());
            println!();
            return Err(e);
        }
    };

    // Load OpenAPI sources from sourceDescriptions
    println!(
        "{}",
        "Loading OpenAPI sources from sourceDescriptions...".bright_blue()
    );
    let resolver_helper = SourceDescriptionResolver::new(arazzo_path)?;
    let source_result = resolver_helper.load_sources(&arazzo.source_descriptions);

    // Report source loading errors
    if !source_result.errors.is_empty() {
        println!("{}", "⚠️  Source Loading Errors:".yellow().bold());
        for err in &source_result.errors {
            println!(
                "  - Source '{}' ({}): {}",
                err.name.yellow(),
                err.url,
                err.message.red()
            );
            has_errors = true;
        }
        println!();
    }

    // Validate each OpenAPI source
    if !source_result.resolver.get_all_specs().is_empty() {
        println!("{}", "Validating OpenAPI sources...".bright_blue());
        for (name, spec) in source_result.resolver.get_all_specs() {
            println!("  Source '{}': {}", name.cyan(), "✓ Valid".green());
            println!("    Title: {}", spec.info.title.bold());
            println!("    Version: {}", spec.info.version);
            let path_count = spec.paths.as_ref().map(|p| p.len()).unwrap_or(0);
            println!("    Paths: {}", path_count);
        }
        println!();
    }

    // Validate Arazzo-OpenAPI consistency
    if !source_result.resolver.get_all_specs().is_empty() {
        println!(
            "{}",
            "Validating Arazzo-OpenAPI consistency...".bright_blue()
        );

        use crate::validation::ArazzoOpenApiValidator;

        let validator = ArazzoOpenApiValidator::new(&arazzo, &source_result.resolver);

        match validator.validate_all() {
            Ok(result) => {
                // Annotate errors and warnings with line numbers
                let result = annotate_with_line_numbers(result, arazzo_path);

                // Display errors
                if !result.errors.is_empty() {
                    println!("  {}", "✗ Consistency Errors:".red().bold());
                    for error in &result.errors {
                        println!("    - {}", error.format().red());
                    }
                    has_errors = true;
                }

                // Display warnings
                if !result.warnings.is_empty() {
                    println!("  {}", "⚠ Consistency Warnings:".yellow());
                    for warning in &result.warnings {
                        println!("    - {}", warning.format().yellow());
                    }
                }

                if result.errors.is_empty() && result.warnings.is_empty() {
                    println!("  {}", "✓ Consistency checks passed".green());
                }
            }
            Err(e) => {
                println!("  {}", "✗ Consistency validation failed".red().bold());
                println!("    {}", e.to_string().red());
                has_errors = true;
            }
        }
        println!();
    }

    // Validate workflows (graph structure)
    println!("{}", "Validating workflow graphs...".bright_blue());

    for workflow in &arazzo.workflows {
        println!("  Workflow: {}", workflow.workflow_id.cyan());

        let resolver = if source_result.resolver.get_all_specs().is_empty() {
            None
        } else {
            Some(&source_result.resolver)
        };

        match build_flow_graph(workflow, resolver) {
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

    // Summary
    if has_errors {
        println!("{}", "✗ Validation completed with errors".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "✓ All validations passed successfully".green().bold());
        Ok(())
    }
}

/// Build a map of workflow IDs and step IDs to their line numbers in the YAML file
fn build_line_map(file_path: &Path) -> Result<HashMap<String, usize>> {
    let content = fs::read_to_string(file_path)?;
    let mut map = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1; // 1-indexed
        let trimmed = line.trim();

        // Match workflowId: value
        if let Some(pos) = trimmed.find("workflowId:")
            && let Some(id) = trimmed[pos + 11..].trim().split('#').next()
        {
            let id = id.trim();
            if !id.is_empty() {
                map.insert(format!("workflow:{}", id), line_number);
            }
        }

        // Match stepId: value
        if let Some(pos) = trimmed.find("stepId:")
            && let Some(id) = trimmed[pos + 7..].trim().split('#').next()
        {
            let id = id.trim();
            if !id.is_empty() {
                map.insert(format!("step:{}", id), line_number);
            }
        }
    }

    Ok(map)
}

/// Annotate errors and warnings with line numbers from the Arazzo file
fn annotate_with_line_numbers(
    mut result: crate::validation::ConsistencyValidationResult,
    arazzo_path: &Path,
) -> crate::validation::ConsistencyValidationResult {
    let line_map = match build_line_map(arazzo_path) {
        Ok(map) => map,
        Err(_) => return result, // If we can't read the file, just return as-is
    };

    let file_name = arazzo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arazzo.yaml");

    // Annotate errors
    for error in &mut result.errors {
        if let Some(step_id) = &error.step_id {
            if let Some(&line_num) = line_map.get(&format!("step:{}", step_id)) {
                error.file_path = Some(file_name.to_string());
                error.line_number = Some(line_num);
            }
        } else if let Some(workflow_id) = &error.workflow_id
            && let Some(&line_num) = line_map.get(&format!("workflow:{}", workflow_id))
        {
            error.file_path = Some(file_name.to_string());
            error.line_number = Some(line_num);
        }
    }

    // Annotate warnings
    for warning in &mut result.warnings {
        if let Some(step_id) = &warning.step_id {
            if let Some(&line_num) = line_map.get(&format!("step:{}", step_id)) {
                warning.file_path = Some(file_name.to_string());
                warning.line_number = Some(line_num);
            }
        } else if let Some(workflow_id) = &warning.workflow_id
            && let Some(&line_num) = line_map.get(&format!("workflow:{}", workflow_id))
        {
            warning.file_path = Some(file_name.to_string());
            warning.line_number = Some(line_num);
        }
    }

    result
}
