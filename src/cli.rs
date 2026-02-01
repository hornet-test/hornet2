use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hornet2")]
#[command(version)]
#[command(about = "Document-driven API testing tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List workflows in Arazzo file
    List {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,
    },

    /// Validate Arazzo file (OpenAPI paths from sourceDescriptions)
    Validate {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,
    },

    /// Visualize workflow as a graph
    Visualize {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,

        /// Output format
        #[arg(short, long, default_value = "dot")]
        format: OutputFormat,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,
    },

    /// Start web server for visualization (multi-project mode)
    Serve {
        /// Root directory containing project folders
        #[arg(short, long)]
        root_dir: PathBuf,

        /// Port number
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Start in LSP mode (Language Server Protocol)
        #[arg(long)]
        lsp: bool,
    },

    /// Convert Arazzo workflow to test script
    Convert {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,

        /// Target format (k6)
        #[arg(short, long, default_value = "k6")]
        to: String,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,

        /// Specific workflow ID to convert (all if not specified)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Base URL override for API requests
        #[arg(long)]
        base_url: Option<String>,

        /// Number of virtual users (for load testing)
        #[arg(long)]
        vus: Option<u32>,

        /// Duration of the test (e.g., "30s", "5m")
        #[arg(long)]
        duration: Option<String>,

        /// Number of iterations (mutually exclusive with duration)
        #[arg(long)]
        iterations: Option<u32>,
    },

    /// Run tests using an external engine
    Run {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,

        /// Test engine to use (k6)
        #[arg(short, long, default_value = "k6")]
        engine: String,

        /// Specific workflow ID to run (first workflow if not specified)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Base URL override for API requests
        #[arg(long)]
        base_url: Option<String>,

        /// Number of virtual users (for load testing)
        #[arg(long)]
        vus: Option<u32>,

        /// Duration of the test (e.g., "30s", "5m")
        #[arg(long)]
        duration: Option<String>,

        /// Number of iterations (mutually exclusive with duration)
        #[arg(long)]
        iterations: Option<u32>,
    },

    /// Export Hornet2 API specification in OpenAPI format
    ExportOpenapi {
        /// Output format (json or yaml)
        #[arg(short, long, default_value = "yaml")]
        format: ExportFormat,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,
    },

    /// Export Hornet2 Arazzo specification
    ExportArazzo {
        /// Output format (json or yaml)
        #[arg(short, long, default_value = "yaml")]
        format: ExportFormat,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,
    },

    /// Trace HTTP traffic to generate OpenAPI specs
    Trace {
        #[command(subcommand)]
        command: TraceCommands,
    },
}

/// Trace subcommands
#[derive(Subcommand)]
pub enum TraceCommands {
    /// Start a new trace session
    Start {
        /// Session name
        #[arg(short, long)]
        name: Option<String>,

        /// OTLP HTTP receiver port
        #[arg(long, default_value = "4318")]
        otlp_port: u16,

        /// Storage path for trace data
        #[arg(long)]
        store_path: Option<PathBuf>,
    },

    /// Stop the active trace session
    Stop {
        /// Session ID to stop (defaults to active session)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// List all trace sessions
    List {
        /// Show only active sessions
        #[arg(long)]
        active: bool,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: TraceListFormat,
    },

    /// Show trace session details
    Show {
        /// Session ID
        session_id: String,

        /// Show spans
        #[arg(long)]
        spans: bool,

        /// Limit number of spans shown
        #[arg(long, default_value = "100")]
        limit: u64,
    },

    /// Export trace data as OpenAPI specification
    ExportOpenapi {
        /// Session ID
        session_id: String,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "yaml")]
        format: ExportFormat,

        /// Infer path parameters from observed paths
        #[arg(long, default_value = "true")]
        infer_params: bool,

        /// Infer schema from observed bodies
        #[arg(long, default_value = "true")]
        infer_schemas: bool,
    },

    /// Export trace data as Arazzo workflow
    ExportArazzo {
        /// Session ID
        session_id: String,

        /// Output file (stdout if not specified)
        #[arg(short = 'O', long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "yaml")]
        format: ExportFormat,

        /// Workflow name
        #[arg(long)]
        workflow_name: Option<String>,
    },

    /// Generate dependency API stubs
    ExportStubs {
        /// Session ID
        session_id: String,

        /// Output directory
        #[arg(short = 'O', long)]
        output_dir: PathBuf,

        /// Stub format (wiremock, prism, native)
        #[arg(long, default_value = "wiremock")]
        format: StubFormat,

        /// Host filter (only generate stubs for matching hosts)
        #[arg(long)]
        host: Option<String>,
    },

    /// Delete a trace session
    Delete {
        /// Session ID
        session_id: String,

        /// Force delete without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    /// GraphViz DOT format
    Dot,
    /// JSON format
    Json,
    /// Mermaid diagram format
    Mermaid,
}

#[derive(Clone, ValueEnum)]
pub enum ExportFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
}

#[derive(Clone, ValueEnum)]
pub enum TraceListFormat {
    /// Table format
    Table,
    /// JSON format
    Json,
}

#[derive(Clone, ValueEnum)]
pub enum StubFormat {
    /// WireMock JSON mappings
    Wiremock,
    /// Prism mock server (OpenAPI with examples)
    Prism,
    /// Hornet2 native stub format
    Native,
}
