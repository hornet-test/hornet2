use hornet2::{loader, Result};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("hornet2 - Document-driven API testing tool");
        println!();
        println!("Usage:");
        println!("  hornet2 validate-openapi <path-to-openapi.yaml>");
        println!("  hornet2 validate-arazzo <path-to-arazzo.yaml>");
        println!();
        println!("Examples:");
        println!("  hornet2 validate-openapi tests/fixtures/openapi.yaml");
        println!("  hornet2 validate-arazzo tests/fixtures/arazzo.yaml");
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
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Run without arguments to see usage");
            std::process::exit(1);
        }
    }
}
