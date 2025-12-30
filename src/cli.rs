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

    /// Validate OpenAPI and Arazzo files
    Validate {
        /// Root directory containing openapi.yaml and arazzo.yaml (single-project mode)
        #[arg(short, long, conflicts_with_all = ["openapi", "arazzo"])]
        root_dir: Option<PathBuf>,

        /// Path to OpenAPI file
        #[arg(short, long, requires = "arazzo")]
        openapi: Option<PathBuf>,

        /// Path to Arazzo file
        #[arg(short, long, requires = "openapi")]
        arazzo: Option<PathBuf>,
    },

    /// Visualize workflow as a graph
    Visualize {
        /// Root directory containing openapi.yaml and arazzo.yaml (single-project mode)
        #[arg(short, long, conflicts_with_all = ["openapi", "arazzo"])]
        root_dir: Option<PathBuf>,

        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: Option<PathBuf>,

        /// Path to OpenAPI file (optional)
        #[arg(short, long)]
        openapi: Option<PathBuf>,

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

        /// Path to OpenAPI file
        #[arg(short, long)]
        openapi: PathBuf,

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

        /// Path to OpenAPI file
        #[arg(short, long)]
        openapi: PathBuf,

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
