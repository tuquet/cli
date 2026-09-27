use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Automa Core CLI - High-performance web automation engine & bridge daemon
#[derive(Parser, Debug)]
#[command(
    name = "automa",
    author = "Tuquet Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Tuquet Automa Core - Native Browser Bridge & Launcher Daemon",
    long_about = "Lightweight native process launcher, WebSocket event hub, and SQLite storage bridge for Automa WebExtension."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Active capability negotiation and handshake probe for Tuquet Runner (tqr)
    #[arg(long)]
    pub probe: bool,

    /// Flag for backward compatibility: export openapi spec
    #[arg(long)]
    pub export_openapi: Option<Option<PathBuf>>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Active discovery probe returning AutomaManifest JSON for Tuquet Runner
    Probe,

    /// Execute a workflow directly via browser worker and stream logs to stdout
    Run {
        /// Path to workflow JSON file (.workflow.json)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Raw JSON string of the workflow
        #[arg(long)]
        workflow_json: Option<String>,

        /// Run browser in headless mode
        #[arg(long, default_value_t = true)]
        headless: bool,

        /// Target browser profile identifier
        #[arg(short, long)]
        browser_id: Option<String>,
    },

    /// Start the Automa Core HTTP/WebSocket daemon server
    Server {
        /// HTTP server listening port
        #[arg(short, long)]
        port: Option<u16>,

        /// Path to custom data directory (stores SQLite DB and logs)
        #[arg(short, long)]
        data_dir: Option<PathBuf>,

        /// Log verbosity level (trace, debug, info, warn, error)
        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Developer utility to launch browser with unpacked extension loaded
    SetupExt {
        /// Target browser to launch (chrome, edge, brave, auto)
        #[arg(short, long, default_value = "chrome")]
        browser: String,

        /// Custom path to unpacked @automa/webe extension directory
        #[arg(short, long)]
        extension_path: Option<PathBuf>,
    },

    /// Export OpenAPI v3 JSON specification to file
    ExportOpenapi {
        /// Output JSON destination file path
        #[arg(short, long, default_value = "openapi.json")]
        output: PathBuf,
    },

    /// Inspect local Automa Core daemon status and health check endpoint
    Status {
        /// Daemon server base URL
        #[arg(short, long, default_value = "http://127.0.0.1:3000")]
        url: String,
    },
}
