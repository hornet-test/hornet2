use hornet2::{
    graph::{builder::build_flow_graph, exporter::{export_dot, export_json}, validator::validate_flow_graph},
    loader, Result,
};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("hornet2 - Document-driven API testing tool");
        println!();
        println!("Usage:");
        println!("  hornet2 validate-openapi <path-to-openapi.yaml>");
        println!("  hornet2 validate-arazzo <path-to-arazzo.yaml>");
        println!("  hornet2 visualize <path-to-arazzo.yaml> [--openapi <path-to-openapi.yaml>] [--format dot|json]");
        println!();
        println!("Examples:");
        println!("  hornet2 validate-openapi tests/fixtures/openapi.yaml");
        println!("  hornet2 validate-arazzo tests/fixtures/arazzo.yaml");
        println!("  hornet2 visualize tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml --format dot");
        return Ok(());
    }

    match args[1].as_str() {
        "validate-openapi" => {
            if args.len() < 3 {
                eprintln!("Error: Missing path to OpenAPI file");
                std::process::exit(1);
            }
            let path = &args[2];
            println!("Loading OpenAPI from: {}", path);

            match loader::load_openapi(path) {
                Ok(spec) => {
                    println!("✓ OpenAPI loaded successfully");
                    println!("  Title: {}", spec.info.title);
                    println!("  Version: {}", spec.info.version);
                    println!("  OpenAPI Version: {}", spec.openapi);
                    let path_count = spec.paths.as_ref().map(|p| p.len()).unwrap_or(0);
                    println!("  Paths: {}", path_count);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Failed to load OpenAPI: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "validate-arazzo" => {
            if args.len() < 3 {
                eprintln!("Error: Missing path to Arazzo file");
                std::process::exit(1);
            }
            let path = &args[2];
            println!("Loading Arazzo from: {}", path);

            match loader::load_arazzo(path) {
                Ok(spec) => {
                    println!("✓ Arazzo loaded successfully");
                    println!("  Title: {}", spec.info.title);
                    println!("  Version: {}", spec.info.version);
                    println!("  Arazzo Version: {}", spec.arazzo);
                    println!("  Workflows: {}", spec.workflows.len());
                    for workflow in &spec.workflows {
                        println!("    - {} ({} steps)", workflow.workflow_id, workflow.steps.len());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Failed to load Arazzo: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "visualize" => {
            if args.len() < 3 {
                eprintln!("Error: Missing path to Arazzo file");
                std::process::exit(1);
            }
            let arazzo_path = &args[2];

            // Parse optional arguments
            let mut openapi_path = None;
            let mut format = "dot".to_string();

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--openapi" => {
                        if i + 1 < args.len() {
                            openapi_path = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --openapi requires a path");
                            std::process::exit(1);
                        }
                    }
                    "--format" => {
                        if i + 1 < args.len() {
                            format = args[i + 1].clone();
                            i += 2;
                        } else {
                            eprintln!("Error: --format requires a value (dot or json)");
                            std::process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!("Unknown option: {}", args[i]);
                        std::process::exit(1);
                    }
                }
            }

            println!("Loading Arazzo from: {}", arazzo_path);
            let arazzo = match loader::load_arazzo(arazzo_path) {
                Ok(spec) => spec,
                Err(e) => {
                    eprintln!("✗ Failed to load Arazzo: {}", e);
                    std::process::exit(1);
                }
            };

            let openapi = if let Some(ref path) = openapi_path {
                println!("Loading OpenAPI from: {}", path);
                match loader::load_openapi(path) {
                    Ok(spec) => Some(spec),
                    Err(e) => {
                        eprintln!("✗ Failed to load OpenAPI: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            // Build graph for the first workflow
            if arazzo.workflows.is_empty() {
                eprintln!("✗ No workflows found in Arazzo file");
                std::process::exit(1);
            }

            let workflow = &arazzo.workflows[0];
            println!(
                "Building flow graph for workflow: {}",
                workflow.workflow_id
            );

            let graph = match build_flow_graph(workflow, openapi.as_ref()) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("✗ Failed to build graph: {}", e);
                    std::process::exit(1);
                }
            };

            // Validate graph
            match validate_flow_graph(&graph) {
                Ok(validation) => {
                    if validation.is_ok() {
                        println!("✓ Graph is valid");
                    } else {
                        println!("⚠ Graph validation warnings:");
                        for warning in &validation.warnings {
                            println!("  - {}", warning);
                        }
                        if !validation.errors.is_empty() {
                            println!("✗ Graph validation errors:");
                            for error in &validation.errors {
                                println!("  - {}", error);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to validate graph: {}", e);
                    std::process::exit(1);
                }
            }

            // Export graph
            match format.as_str() {
                "dot" => {
                    let dot = export_dot(&graph);
                    println!("\n# DOT format output:\n");
                    println!("{}", dot);
                }
                "json" => {
                    match export_json(&graph) {
                        Ok(json) => {
                            println!("\n# JSON format output:\n");
                            println!("{}", serde_json::to_string_pretty(&json).unwrap());
                        }
                        Err(e) => {
                            eprintln!("✗ Failed to export JSON: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!("Unknown format: {}. Use 'dot' or 'json'", format);
                    std::process::exit(1);
                }
            }

            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Run without arguments to see usage");
            std::process::exit(1);
        }
    }
}
