use crate::{server, Result};
use colored::*;
use std::path::Path;

pub async fn execute_serve(
    root_dir: &Path,
    port: u16,
    default_project: Option<String>,
) -> Result<()> {
    println!("{}", "Starting web server (multi-project mode)...".bright_blue());
    println!("  Root directory: {}", root_dir.display());
    println!("  Port: {}", port);

    if let Some(ref proj) = default_project {
        println!("  Default project: {}", proj);
    }

    println!();

    let addr = format!("127.0.0.1:{}", port).parse().unwrap();

    server::start_server(addr, root_dir.to_path_buf(), default_project).await?;

    Ok(())
}
