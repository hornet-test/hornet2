use crate::{Result, loader};
use colored::*;
use std::path::Path;

pub fn execute_list(arazzo_path: &Path) -> Result<()> {
    println!("{}", "Loading Arazzo file...".bright_blue());
    println!("  Path: {}", arazzo_path.display());

    let arazzo = loader::load_arazzo(arazzo_path.to_str().unwrap())?;

    println!("\n{}", "✓ Arazzo loaded successfully".green());
    println!("  Title: {}", arazzo.info.title.bold());
    println!("  Version: {}", arazzo.info.version);
    println!("  Arazzo Version: {}", arazzo.arazzo);
    println!();

    if arazzo.workflows.is_empty() {
        println!("{}", "No workflows found".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("Workflows ({}):", arazzo.workflows.len()).bold()
    );
    for (idx, workflow) in arazzo.workflows.iter().enumerate() {
        println!();
        println!(
            "  {}. {} {}",
            idx + 1,
            "Workflow:".bright_cyan(),
            workflow.workflow_id.bold()
        );

        if let Some(ref summary) = workflow.summary {
            println!("     Summary: {}", summary);
        }

        if let Some(ref description) = workflow.description {
            println!("     Description: {}", description);
        }

        println!("     Steps: {}", workflow.steps.len());

        for (step_idx, step) in workflow.steps.iter().enumerate() {
            println!("       {}. {}", step_idx + 1, step.step_id.cyan());

            if let Some(ref description) = step.description {
                println!("          Description: {}", description);
            }

            if let Some(ref operation_id) = step.operation_id {
                println!("          Operation: {}", operation_id.bright_yellow());
            } else if let Some(ref operation_path) = step.operation_path {
                println!(
                    "          Operation Path: {}",
                    operation_path.bright_yellow()
                );
            } else if let Some(ref workflow_id) = step.workflow_id {
                println!("          Workflow: {}", workflow_id.bright_magenta());
            }
        }
    }

    Ok(())
}
