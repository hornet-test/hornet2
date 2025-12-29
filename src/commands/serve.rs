use crate::{server, Result};
use colored::*;
use std::path::Path;

pub async fn execute_serve(root_dir: &Path, port: u16) -> Result<()> {
    println!(
        "{}",
        "Starting web server (multi-project mode)...".bright_blue()
    );
    println!("  Root directory: {}", root_dir.display());
    println!("  Port: {}", port);

    println!();

    let addr = format!("127.0.0.1:{}", port).parse().unwrap();

    server::start_server(addr, root_dir.to_path_buf()).await?;

    Ok(())
}
