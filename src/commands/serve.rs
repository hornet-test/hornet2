use crate::{server, Result};
use colored::*;
use std::path::Path;
use std::path::PathBuf;

pub async fn execute_serve(
    arazzo_path: &Path,
    openapi_path: &Option<PathBuf>,
    port: u16,
) -> Result<()> {
    println!("{}", "Starting web server...".bright_blue());
    println!("  Arazzo: {}", arazzo_path.display());

    if let Some(ref path) = openapi_path {
        println!("  OpenAPI: {}", path.display());
    }

    println!("  Port: {}", port);
    println!();

    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let arazzo_str = arazzo_path.to_str().unwrap().to_string();
    let openapi_str = openapi_path
        .as_ref()
        .map(|p| p.to_str().unwrap().to_string());

    server::start_server(addr, arazzo_str, openapi_str).await?;

    Ok(())
}
