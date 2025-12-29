use crate::error::{HornetError, Result};
use colored::*;
use std::fs;
use std::path::Path;

/// Execute the export-arazzo command
pub fn execute_export_arazzo(format: &str, output: Option<&Path>) -> Result<()> {
    println!(
        "{}",
        "Exporting Hornet2 Arazzo specification...".bright_blue()
    );

    // Embed the Arazzo YAML at compile time
    const ARAZZO_YAML: &str = include_str!("../../arazzo.yaml");

    // Convert format based on user preference
    let content = match format {
        "json" => {
            println!("  Format: {}", "JSON".bold());
            let spec: serde_json::Value = serde_yaml::from_str(ARAZZO_YAML).map_err(|e| {
                HornetError::ValidationError(format!("Failed to parse Arazzo YAML: {}", e))
            })?;
            serde_json::to_string_pretty(&spec).map_err(|e| {
                HornetError::ValidationError(format!("Failed to serialize to JSON: {}", e))
            })?
        }
        "yaml" => {
            println!("  Format: {}", "YAML".bold());
            ARAZZO_YAML.to_string()
        }
        _ => {
            return Err(HornetError::ValidationError(format!(
                "Unsupported format: {}",
                format
            )))
        }
    };

    // Write to output
    match output {
        Some(path) => {
            println!("  Output: {}", path.display());
            fs::write(path, content).map_err(|e| {
                HornetError::ArazzoLoadError(format!("Failed to write file: {}", e))
            })?;
            println!(
                "\n{}",
                "✓ Arazzo specification exported successfully".green()
            );
        }
        None => {
            // Write to stdout (no colored output)
            println!("\n{}", content);
        }
    }

    Ok(())
}
