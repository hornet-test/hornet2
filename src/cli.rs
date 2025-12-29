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
        /// Path to OpenAPI file
        #[arg(short, long)]
        openapi: PathBuf,

        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,
    },

    /// Visualize workflow as a graph
    Visualize {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,

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

        /// Default project name (for compatibility API)
        #[arg(long)]
        default_project: Option<String>,
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
