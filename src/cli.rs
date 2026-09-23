use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Automa Core CLI - High-performance web automation engine & daemon
#[derive(Parser, Debug)]
#[command(
    name = "automa",
    author = "Tuquet Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Automa Core CLI & Web Automation Engine",
    long_about = "Automa Core is the native Rust engine powering web automation workflows, Chrome extension bridges, and local/cloud storage."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Flag for backward compatibility: export openapi spec
    #[arg(long)]
    pub export_openapi: Option<Option<PathBuf>>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
