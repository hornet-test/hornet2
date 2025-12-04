use clap::Parser;
use hornet2::{
    cli::{Cli, Commands},
    commands, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { arazzo } => {
            commands::execute_list(&arazzo)?;
        }
        Commands::Validate { openapi, arazzo } => {
            commands::execute_validate(&openapi, &arazzo)?;
        }
        Commands::Visualize {
            arazzo,
            openapi,
            format,
            output,
        } => {
            commands::execute_visualize(&arazzo, &openapi, &format, &output)?;
        }
        Commands::Serve {
            arazzo,
            openapi,
            port,
        } => {
            commands::execute_serve(&arazzo, &openapi, port).await?;
        }
    }

    Ok(())
}
