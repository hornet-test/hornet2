use crate::{
    graph::{builder::build_flow_graph, validator::validate_flow_graph},
    loader, HornetError, Result,
};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_validate(
    root_dir: &Option<PathBuf>,
    openapi_path: &Option<PathBuf>,
    arazzo_path: &Option<PathBuf>,
) -> Result<()> {
    // Determine file paths
    let (openapi_file, arazzo_file) = if let Some(root) = root_dir {
        // Single-project mode
        let openapi = root.join("openapi.yaml");
        let arazzo = root.join("arazzo.yaml");

        if !openapi.exists() {
            return Err(HornetError::InvalidPath(format!(
                "openapi.yaml not found in {}",
                root.display()
            )));
        }
        if !arazzo.exists() {
            return Err(HornetError::InvalidPath(format!(
                "arazzo.yaml not found in {}",
                root.display()
            )));
        }

        (openapi, arazzo)
    } else if let (Some(openapi), Some(arazzo)) = (openapi_path, arazzo_path) {
        // Individual file mode
        (openapi.clone(), arazzo.clone())
    } else {
        return Err(HornetError::ValidationError(
            "Either --root-dir or both --openapi and --arazzo must be specified".to_string(),
        ));
    };

    validate_files(&openapi_file, &arazzo_file)
}

fn validate_files(openapi_path: &Path, arazzo_path: &Path) -> Result<()> {
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

    // Validate Arazzo-OpenAPI consistency
    if let (Some(ref arazzo), Some(ref openapi)) = (&arazzo, &openapi) {
        println!(
            "{}",
            "Validating Arazzo-OpenAPI consistency...".bright_blue()
        );

        use crate::validation::ArazzoOpenApiValidator;

        let validator = ArazzoOpenApiValidator::new(arazzo, openapi);

        match validator.validate_all() {
            Ok(result) => {
                // Annotate errors and warnings with line numbers
                let result = annotate_with_line_numbers(result, arazzo_path);

                // Display errors
                if !result.errors.is_empty() {
                    println!("    {}", "✗ Consistency Errors:".red().bold());
                    for error in &result.errors {
                        println!("      - {}", error.format().red());
                    }
                    has_errors = true;
                }

                // Display warnings
                if !result.warnings.is_empty() {
                    println!("    {}", "⚠ Consistency Warnings:".yellow());
                    for warning in &result.warnings {
                        println!("      - {}", warning.format().yellow());
                    }
                }

                if result.errors.is_empty() && result.warnings.is_empty() {
                    println!("    {}", "✓ Consistency checks passed".green());
                }
            }
            Err(e) => {
                println!("    {}", "✗ Consistency validation failed".red().bold());
                println!("      {}", e.to_string().red());
                has_errors = true;
            }
        }
        println!();
    }

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

/// Build a map of workflow IDs and step IDs to their line numbers in the YAML file
fn build_line_map(file_path: &Path) -> Result<HashMap<String, usize>> {
    let content = fs::read_to_string(file_path)?;
    let mut map = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1; // 1-indexed
        let trimmed = line.trim();

        // Match workflowId: value
        if let Some(pos) = trimmed.find("workflowId:") {
            if let Some(id) = trimmed[pos + 11..].trim().split('#').next() {
                let id = id.trim();
                if !id.is_empty() {
                    map.insert(format!("workflow:{}", id), line_number);
                }
            }
        }

        // Match stepId: value
        if let Some(pos) = trimmed.find("stepId:") {
            if let Some(id) = trimmed[pos + 7..].trim().split('#').next() {
                let id = id.trim();
                if !id.is_empty() {
                    map.insert(format!("step:{}", id), line_number);
                }
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
        } else if let Some(workflow_id) = &error.workflow_id {
            if let Some(&line_num) = line_map.get(&format!("workflow:{}", workflow_id)) {
                error.file_path = Some(file_name.to_string());
                error.line_number = Some(line_num);
            }
        }
    }

    // Annotate warnings
    for warning in &mut result.warnings {
        if let Some(step_id) = &warning.step_id {
            if let Some(&line_num) = line_map.get(&format!("step:{}", step_id)) {
                warning.file_path = Some(file_name.to_string());
                warning.line_number = Some(line_num);
            }
        } else if let Some(workflow_id) = &warning.workflow_id {
            if let Some(&line_num) = line_map.get(&format!("workflow:{}", workflow_id)) {
                warning.file_path = Some(file_name.to_string());
                warning.line_number = Some(line_num);
            }
        }
    }

    result
}
