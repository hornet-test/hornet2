//! Trace command implementations.
//!
//! Commands for HTTP traffic tracing, OpenAPI inference, and stub generation.

use crate::Result;
use crate::cli::{ExportFormat, StubFormat, TraceListFormat};
use crate::tracer::{
    OtlpReceiver, SpanDirection, TraceStore, TracerConfig,
    generators::{
        ArazzoGenerator, StubGenerator, arazzo_generator::ArazzoGeneratorConfig,
        stub_generator::StubGeneratorConfig,
    },
    inference::{OpenApiGenerator, openapi_generator::GeneratorConfig},
};
use colored::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Arguments for the trace start command
pub struct TraceStartArgs {
    pub name: Option<String>,
    pub otlp_port: u16,
    pub store_path: Option<PathBuf>,
}

/// Arguments for the trace stop command
pub struct TraceStopArgs {
    pub session: Option<String>,
}

/// Arguments for the trace list command
pub struct TraceListArgs {
    pub active: bool,
    pub format: TraceListFormat,
}

/// Arguments for the trace show command
pub struct TraceShowArgs {
    pub session_id: String,
    pub spans: bool,
    pub limit: u64,
}

/// Arguments for the trace export-openapi command
pub struct TraceExportOpenapiArgs {
    pub session_id: String,
    pub output: Option<PathBuf>,
    pub format: ExportFormat,
    pub infer_params: bool,
    pub infer_schemas: bool,
}

/// Arguments for the trace export-arazzo command
pub struct TraceExportArazzoArgs {
    pub session_id: String,
    pub output: Option<PathBuf>,
    pub format: ExportFormat,
    pub workflow_name: Option<String>,
}

/// Arguments for the trace export-stubs command
pub struct TraceExportStubsArgs {
    pub session_id: String,
    pub output_dir: PathBuf,
    pub format: StubFormat,
    pub host: Option<String>,
}

/// Arguments for the trace delete command
pub struct TraceDeleteArgs {
    pub session_id: String,
    pub force: bool,
}

/// Execute trace start command
pub async fn execute_trace_start(args: TraceStartArgs) -> Result<()> {
    let store_path = args
        .store_path
        .unwrap_or_else(|| PathBuf::from(".hornet2/traces"));

    println!("{}", "Starting trace session...".bright_blue());
    println!("  OTLP HTTP port: {}", args.otlp_port);
    println!("  Store path: {}", store_path.display());

    // Create store
    let store_file = store_path.join("traces.db");
    let store = Arc::new(TraceStore::new_sqlite(store_file)?);

    // Create receiver and config
    let mut config = TracerConfig::from_env();
    config.http_port = args.otlp_port;

    let receiver = OtlpReceiver::new(store.clone(), config);

    // Start session
    let session_id = receiver.start_session(args.name.clone()).await?;

    println!();
    println!("{}", "✓ Trace session started".green());
    println!("  Session ID: {}", session_id.bold());
    if let Some(name) = args.name {
        println!("  Name: {}", name);
    }
    println!();
    println!("{}", "OTLP Endpoint:".bright_cyan());
    println!("  HTTP: http://localhost:{}/v1/traces", args.otlp_port);
    println!();
    println!("Configure your application with:");
    println!(
        "  {}",
        format!(
            "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:{}",
            args.otlp_port
        )
        .yellow()
    );
    println!();

    // Create HTTP server for OTLP
    let router = receiver.http_router();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.otlp_port)).await?;

    println!("{}", "Press Ctrl+C to stop tracing...".bright_black());

    // Set up graceful shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = shutdown_tx.send(());
    });

    // Run server until shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        })
        .await?;

    // Stop session
    let _ = receiver.stop_session().await?;

    let stats = receiver.current_stats();
    println!();
    println!("{}", "✓ Trace session stopped".green());
    println!("  Session ID: {}", session_id);
    println!("  Total spans: {}", stats.total_spans);
    println!("  Incoming requests: {}", stats.incoming_requests);
    println!("  Outgoing requests: {}", stats.outgoing_requests);

    Ok(())
}

/// Execute trace stop command
pub async fn execute_trace_stop(args: TraceStopArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = Arc::new(TraceStore::new_sqlite(store_path)?);

    if let Some(session_id) = args.session {
        // Stop specific session
        if let Some(mut session) = store.get_session(&session_id).await? {
            session.ended_at = Some(chrono::Utc::now());
            store.update_session(&session).await?;
            println!("{}", format!("✓ Stopped session: {}", session_id).green());
        } else {
            println!("{}", format!("Session not found: {}", session_id).yellow());
        }
    } else {
        println!(
            "{}",
            "No session specified. Use --session to specify session ID.".yellow()
        );
    }

    Ok(())
}

/// Execute trace list command
pub async fn execute_trace_list(args: TraceListArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;
    let sessions = store.list_sessions().await?;

    let sessions: Vec<_> = if args.active {
        sessions
            .into_iter()
            .filter(|s| s.ended_at.is_none())
            .collect()
    } else {
        sessions
    };

    match args.format {
        TraceListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&sessions)?);
        }
        TraceListFormat::Table => {
            if sessions.is_empty() {
                println!("{}", "No trace sessions found".yellow());
                return Ok(());
            }

            println!("{}", format!("Trace Sessions ({}):", sessions.len()).bold());
            println!();

            for session in sessions {
                let status = if session.ended_at.is_some() {
                    "completed".bright_black()
                } else {
                    "active".green()
                };

                println!("  {} [{}]", session.id.cyan(), status);

                if let Some(name) = &session.name {
                    println!("    Name: {}", name);
                }

                println!(
                    "    Started: {}",
                    session.started_at.format("%Y-%m-%d %H:%M:%S")
                );

                if let Some(ended) = session.ended_at {
                    println!("    Ended: {}", ended.format("%Y-%m-%d %H:%M:%S"));
                }

                println!("    Spans: {}", session.statistics.total_spans);
                println!(
                    "    Requests: {} incoming, {} outgoing",
                    session.statistics.incoming_requests, session.statistics.outgoing_requests
                );
                println!();
            }
        }
    }

    Ok(())
}

/// Execute trace show command
pub async fn execute_trace_show(args: TraceShowArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;

    let session = store.get_session(&args.session_id).await?;

    if let Some(session) = session {
        println!("{}", "Trace Session".bold());
        println!("  ID: {}", session.id.cyan());

        if let Some(name) = &session.name {
            println!("  Name: {}", name);
        }

        let status = if session.ended_at.is_some() {
            "completed".bright_black()
        } else {
            "active".green()
        };
        println!("  Status: {}", status);
        println!(
            "  Started: {}",
            session.started_at.format("%Y-%m-%d %H:%M:%S")
        );

        if let Some(ended) = session.ended_at {
            println!("  Ended: {}", ended.format("%Y-%m-%d %H:%M:%S"));
        }

        println!();
        println!("{}", "Statistics".bold());
        println!("  Total spans: {}", session.statistics.total_spans);
        println!(
            "  Incoming requests: {}",
            session.statistics.incoming_requests
        );
        println!(
            "  Outgoing requests: {}",
            session.statistics.outgoing_requests
        );
        println!(
            "  Unique endpoints: {}",
            session.statistics.unique_endpoints
        );
        println!(
            "  Unique dependencies: {}",
            session.statistics.unique_dependencies
        );

        if !session.statistics.status_codes.is_empty() {
            println!();
            println!("{}", "Status Codes".bold());
            for (code, count) in &session.statistics.status_codes {
                let code_color = if *code < 300 {
                    format!("{}", code).green()
                } else if *code < 400 {
                    format!("{}", code).yellow()
                } else {
                    format!("{}", code).red()
                };
                println!("  {}: {}", code_color, count);
            }
        }

        // Show endpoints
        let endpoints = store.get_endpoints(&args.session_id).await?;
        if !endpoints.is_empty() {
            println!();
            println!("{}", "Endpoints".bold());
            for (method, path, count) in endpoints.iter().take(20) {
                let method_color = match method.as_str() {
                    "GET" => method.green(),
                    "POST" => method.bright_yellow(),
                    "PUT" => method.blue(),
                    "DELETE" => method.red(),
                    _ => method.normal(),
                };
                println!("  {} {} ({})", method_color, path, count);
            }
            if endpoints.len() > 20 {
                println!("  ... and {} more", endpoints.len() - 20);
            }
        }

        // Show dependencies
        let dependencies = store.get_dependencies(&args.session_id).await?;
        if !dependencies.is_empty() {
            println!();
            println!("{}", "Dependencies (Outgoing Calls)".bold());
            for (host, port, count) in &dependencies {
                println!("  {}:{} ({})", host.cyan(), port, count);
            }
        }

        // Show spans if requested
        if args.spans {
            let spans = store.get_spans(&args.session_id, 0, args.limit).await?;
            if !spans.is_empty() {
                println!();
                println!(
                    "{}",
                    format!("Spans (showing {} of total)", spans.len()).bold()
                );
                for span in spans {
                    let direction = match span.direction {
                        crate::tracer::types::SpanDirection::Incoming => "→".green(),
                        crate::tracer::types::SpanDirection::Outgoing => "←".blue(),
                    };
                    println!(
                        "  {} {} {} {} [{}ms]",
                        direction,
                        span.method.bright_yellow(),
                        span.path,
                        span.status_code,
                        format!("{:.1}", span.duration_ms).bright_black()
                    );
                }
            }
        }
    } else {
        println!(
            "{}",
            format!("Session not found: {}", args.session_id).yellow()
        );
    }

    Ok(())
}

/// Execute trace export-openapi command
pub async fn execute_trace_export_openapi(args: TraceExportOpenapiArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;

    let session = store.get_session(&args.session_id).await?;

    if session.is_none() {
        println!(
            "{}",
            format!("Session not found: {}", args.session_id).yellow()
        );
        return Ok(());
    }

    println!("{}", "Generating OpenAPI specification...".bright_blue());
    println!("  Session: {}", args.session_id);
    println!("  Infer params: {}", args.infer_params);
    println!("  Infer schemas: {}", args.infer_schemas);

    // Get all spans
    let spans = store.get_spans(&args.session_id, 0, u64::MAX).await?;

    if spans.is_empty() {
        println!("{}", "No spans found in session".yellow());
        return Ok(());
    }

    println!("  Processing {} spans...", spans.len());

    // Generate OpenAPI spec
    let generator = OpenApiGenerator::new()
        .with_title("Generated API")
        .with_version("1.0.0");

    let config = GeneratorConfig {
        infer_params: args.infer_params,
        infer_schemas: args.infer_schemas,
        include_examples: true,
        direction_filter: Some(SpanDirection::Incoming),
    };

    let spec = generator.generate(&spans, &config);

    // Output
    let output = match args.format {
        ExportFormat::Yaml => serde_yaml::to_string(&spec)
            .map_err(|e| crate::HornetError::InferenceError(e.to_string()))?,
        ExportFormat::Json => serde_json::to_string_pretty(&spec)?,
    };

    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &output)?;
        println!();
        println!(
            "{}",
            format!(
                "✓ OpenAPI specification written to {}",
                output_path.display()
            )
            .green()
        );
    } else {
        println!();
        println!("{}", output);
    }

    Ok(())
}

/// Execute trace export-arazzo command
pub async fn execute_trace_export_arazzo(args: TraceExportArazzoArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;

    let session = store.get_session(&args.session_id).await?;

    if session.is_none() {
        println!(
            "{}",
            format!("Session not found: {}", args.session_id).yellow()
        );
        return Ok(());
    }

    let workflow_name = args
        .workflow_name
        .clone()
        .unwrap_or_else(|| "generated-workflow".to_string());

    println!("{}", "Generating Arazzo workflow...".bright_blue());
    println!("  Session: {}", args.session_id);
    println!("  Workflow name: {}", workflow_name);

    // Get all spans
    let spans = store.get_spans(&args.session_id, 0, u64::MAX).await?;

    if spans.is_empty() {
        println!("{}", "No spans found in session".yellow());
        return Ok(());
    }

    println!("  Processing {} spans...", spans.len());

    // Generate Arazzo spec
    let generator = ArazzoGenerator::new()
        .with_workflow_name(&workflow_name)
        .with_description("Auto-generated from trace data");

    let config = ArazzoGeneratorConfig {
        include_success_criteria: true,
        include_request_bodies: true,
        include_outputs: true,
        group_by_trace: false,
        direction_filter: Some(SpanDirection::Incoming),
    };

    let arazzo = generator.generate(&spans, &config);

    // Output
    let output = match args.format {
        ExportFormat::Yaml => serde_yaml::to_string(&arazzo)
            .map_err(|e| crate::HornetError::InferenceError(e.to_string()))?,
        ExportFormat::Json => serde_json::to_string_pretty(&arazzo)?,
    };

    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &output)?;
        println!();
        println!(
            "{}",
            format!("✓ Arazzo workflow written to {}", output_path.display()).green()
        );
    } else {
        println!();
        println!("{}", output);
    }

    Ok(())
}

/// Execute trace export-stubs command
pub async fn execute_trace_export_stubs(args: TraceExportStubsArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;

    let session = store.get_session(&args.session_id).await?;

    if session.is_none() {
        println!(
            "{}",
            format!("Session not found: {}", args.session_id).yellow()
        );
        return Ok(());
    }

    let stub_format = match args.format {
        StubFormat::Wiremock => crate::tracer::StubFormat::WireMock,
        StubFormat::Prism => crate::tracer::StubFormat::Prism,
        StubFormat::Native => crate::tracer::StubFormat::Native,
    };

    println!("{}", "Generating dependency stubs...".bright_blue());
    println!("  Session: {}", args.session_id);
    println!("  Output directory: {}", args.output_dir.display());
    println!(
        "  Format: {}",
        match args.format {
            StubFormat::Wiremock => "WireMock",
            StubFormat::Prism => "Prism",
            StubFormat::Native => "Native",
        }
    );
    if let Some(host) = &args.host {
        println!("  Host filter: {}", host);
    }

    // Get all spans
    let spans = store.get_spans(&args.session_id, 0, u64::MAX).await?;

    if spans.is_empty() {
        println!("{}", "No spans found in session".yellow());
        return Ok(());
    }

    // Get dependencies summary
    let dependencies = store.get_dependencies(&args.session_id).await?;

    if dependencies.is_empty() {
        println!("{}", "No outgoing dependencies found".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "Dependencies found:".bright_cyan());
    for (host, port, count) in &dependencies {
        println!("  {}:{} ({} calls)", host, port, count);
    }

    // Generate stubs
    let generator = StubGenerator::new(stub_format);
    let config = StubGeneratorConfig {
        host_filter: args.host.clone(),
        include_request_matchers: true,
        include_headers: true,
        include_error_responses: true,
    };

    let stubs = generator.generate(&spans, &config);

    if stubs.is_empty() {
        println!(
            "{}",
            "No stubs generated (no matching outgoing spans)".yellow()
        );
        return Ok(());
    }

    // Write to files
    let files = generator.write_to_directory(&stubs, &args.output_dir)?;

    println!();
    println!("{}", "✓ Stubs generated:".green());
    for file in files {
        println!("  {}", file.display());
    }

    Ok(())
}

/// Execute trace delete command
pub async fn execute_trace_delete(args: TraceDeleteArgs) -> Result<()> {
    let store_path = PathBuf::from(".hornet2/traces/traces.db");

    if !store_path.exists() {
        println!("{}", "No trace store found".yellow());
        return Ok(());
    }

    let store = TraceStore::new_sqlite(store_path)?;

    let session = store.get_session(&args.session_id).await?;

    if let Some(session) = session {
        if !args.force {
            println!("{}", "Session to delete:".bright_yellow());
            println!("  ID: {}", session.id);
            if let Some(name) = &session.name {
                println!("  Name: {}", name);
            }
            println!("  Spans: {}", session.statistics.total_spans);
            println!();
            println!("{}", "Use --force to confirm deletion".bright_black());
            return Ok(());
        }

        store.delete_session(&args.session_id).await?;
        println!(
            "{}",
            format!("✓ Deleted session: {}", args.session_id).green()
        );
    } else {
        println!(
            "{}",
            format!("Session not found: {}", args.session_id).yellow()
        );
    }

    Ok(())
}
