use crate::{Result, server};
use colored::*;
use std::path::Path;

pub async fn execute_serve(root_dir: &Path, port: u16, lsp_mode: bool) -> Result<()> {
    if lsp_mode {
        // LSP mode
        eprintln!("Starting hornet2 LSP server...");
        eprintln!("  Root directory: {}", root_dir.display());
        eprintln!();

        use crate::lsp::ArazzoLanguageServer;
        use tower_lsp::{LspService, Server};

        let (service, socket) =
            LspService::new(|client| ArazzoLanguageServer::new(client, root_dir.to_path_buf()));

        Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
            .serve(service)
            .await;

        Ok(())
    } else {
        // HTTP server mode
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
}
