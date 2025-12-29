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
            root_dir,
            port,
            default_project,
        } => {
            commands::execute_serve(&root_dir, port, default_project).await?;
        }
        Commands::Convert {
            arazzo,
            openapi,
            to,
            output,
            workflow,
            base_url,
            vus,
            duration,
            iterations,
        } => {
            commands::execute_convert(commands::ConvertCommandArgs {
                arazzo_path: &arazzo,
                openapi_path: &openapi,
                output_path: output.as_deref(),
                target: &to,
                workflow_id: workflow.as_deref(),
                base_url: base_url.as_deref(),
                vus,
                duration: duration.as_deref(),
                iterations,
            })?;
        }
        Commands::Run {
            arazzo,
            openapi,
            engine,
            workflow,
            base_url,
            vus,
            duration,
            iterations,
        } => {
            commands::execute_run(commands::RunCommandArgs {
                arazzo_path: &arazzo,
                openapi_path: &openapi,
                engine: &engine,
                workflow_id: workflow.as_deref(),
                base_url: base_url.as_deref(),
                vus,
                duration: duration.as_deref(),
                iterations,
            })?;
        }
    }

    Ok(())
}
