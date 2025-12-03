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

    /// Start web server for visualization
    Serve {
        /// Path to Arazzo file
        #[arg(short, long)]
        arazzo: PathBuf,

        /// Path to OpenAPI file (optional)
        #[arg(short, long)]
        openapi: Option<PathBuf>,

        /// Port number
        #[arg(short, long, default_value = "3000")]
        port: u16,
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
