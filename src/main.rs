use clap::Parser;
use hornet2::{
    Result,
    cli::{Cli, Commands, ExportFormat, TraceCommands},
    commands,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { arazzo } => {
            commands::execute_list(&arazzo)?;
        }
        Commands::Validate { arazzo } => {
            commands::execute_validate(&arazzo)?;
        }
        Commands::Visualize {
            arazzo,
            format,
            output,
        } => {
            commands::execute_visualize(&arazzo, &format, &output)?;
        }
        Commands::Serve {
            root_dir,
            port,
            lsp,
        } => {
            commands::execute_serve(&root_dir, port, lsp).await?;
        }
        Commands::Convert {
            arazzo,
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
            engine,
            workflow,
            base_url,
            vus,
            duration,
            iterations,
        } => {
            commands::execute_run(commands::RunCommandArgs {
                arazzo_path: &arazzo,
                engine: &engine,
                workflow_id: workflow.as_deref(),
                base_url: base_url.as_deref(),
                vus,
                duration: duration.as_deref(),
                iterations,
            })?;
        }
        Commands::ExportOpenapi { format, output } => {
            commands::execute_export_openapi(
                match format {
                    ExportFormat::Yaml => "yaml",
                    ExportFormat::Json => "json",
                },
                output.as_deref(),
            )?;
        }
        Commands::ExportArazzo { format, output } => {
            commands::execute_export_arazzo(
                match format {
                    ExportFormat::Yaml => "yaml",
                    ExportFormat::Json => "json",
                },
                output.as_deref(),
            )?;
        }
        Commands::Trace { command } => {
            match command {
                TraceCommands::Start {
                    name,
                    otlp_port,
                    store_path,
                } => {
                    commands::execute_trace_start(commands::TraceStartArgs {
                        name,
                        otlp_port,
                        store_path,
                    })
                    .await?;
                }
                TraceCommands::Stop { session } => {
                    commands::execute_trace_stop(commands::TraceStopArgs { session }).await?;
                }
                TraceCommands::List { active, format } => {
                    commands::execute_trace_list(commands::TraceListArgs { active, format })
                        .await?;
                }
                TraceCommands::Show {
                    session_id,
                    spans,
                    limit,
                } => {
                    commands::execute_trace_show(commands::TraceShowArgs {
                        session_id,
                        spans,
                        limit,
                    })
                    .await?;
                }
                TraceCommands::ExportOpenapi {
                    session_id,
                    output,
                    format,
                    infer_params,
                    infer_schemas,
                } => {
                    commands::execute_trace_export_openapi(commands::TraceExportOpenapiArgs {
                        session_id,
                        output,
                        format,
                        infer_params,
                        infer_schemas,
                    })
                    .await?;
                }
                TraceCommands::ExportArazzo {
                    session_id,
                    output,
                    format,
                    workflow_name,
                } => {
                    commands::execute_trace_export_arazzo(commands::TraceExportArazzoArgs {
                        session_id,
                        output,
                        format,
                        workflow_name,
                    })
                    .await?;
                }
                TraceCommands::ExportStubs {
                    session_id,
                    output_dir,
                    format,
                    host,
                } => {
                    commands::execute_trace_export_stubs(commands::TraceExportStubsArgs {
                        session_id,
                        output_dir,
                        format,
                        host,
                    })
                    .await?;
                }
                TraceCommands::Delete { session_id, force } => {
                    commands::execute_trace_delete(commands::TraceDeleteArgs { session_id, force })
                        .await?;
                }
            }
        }
    }

    Ok(())
}
