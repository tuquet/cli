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
        #[arg(value_name = "WORKFLOW_FILE")]
        workflow_pos: Option<String>,

        /// Path to workflow JSON file (.workflow.json)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Raw JSON string of the workflow
        #[arg(long)]
        workflow_json: Option<String>,

        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,

        /// Target browser executable type (chrome, chromium, edge)
        #[arg(short, long)]
        browser: Option<String>,

        /// Target browser profile identifier
        #[arg(long)]
        browser_id: Option<String>,

        /// Workflow variables in KEY=VALUE format (can be specified multiple times, alias: -p, --param)
        #[arg(short = 'p', long = "var", alias = "param", value_name = "KEY=VALUE")]
        variables: Vec<String>,

        /// Optional workflow execution timeout in seconds
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    /// Manage stored workflows in local database and vault (alias: wf)
    #[command(name = "workflow", alias = "wf")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },

    /// Inspect and validate a workflow JSON file or stored workflow ID without launching a browser
    Inspect {
        /// Path to workflow JSON file (.workflow.json) or saved workflow ID/Name
        #[arg(value_name = "WORKFLOW")]
        workflow: String,
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

        /// Custom path to unpacked @automa/runner extension directory
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
        #[arg(short, long, default_value = "http://127.0.0.1:8765")]
        url: String,
    },

    /// Manage dedicated isolated browser runtime (install, status, clean, path)
    #[command(name = "browser")]
    Browser {
        #[command(subcommand)]
        command: BrowserCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum BrowserCommands {
    /// Download and install dedicated Open-Source Chromium runtime (BSD-3-Clause)
    #[command(name = "install")]
    Install {
        /// Force re-download even if already installed
        #[arg(short, long)]
        force: bool,

        /// Specific Chromium revision (defaults to pinned LTS revision)
        #[arg(short, long)]
        revision: Option<String>,
    },

    /// Display installation status, executable path, and disk usage of dedicated browser
    #[command(name = "status")]
    Status,

    /// Delete installed browser runtime to reclaim disk space
    #[command(name = "clean")]
    Clean,

    /// Print the absolute executable path of the browser (for scripting / integrations)
    #[command(name = "path")]
    Path,
}

#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// List all workflows saved in database and vault (alias: ls)
    #[command(name = "list", alias = "ls")]
    List {
        /// Filter workflows by keyword (name, ID, or description)
        #[arg(short, long)]
        search: Option<String>,

        /// Only list workflows from SQLite database
        #[arg(long)]
        db_only: bool,

        /// Only list workflows from Vault directory (~/.automa/workflows)
        #[arg(long)]
        vault_only: bool,
    },

    /// Import a workflow file (.json) into SQLite database and vault (alias: add)
    #[command(name = "import", alias = "add")]
    Import {
        /// Path to workflow JSON file (.workflow.json or .json)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Custom identifier for the workflow
        #[arg(long)]
        id: Option<String>,

        /// Custom display name for the workflow
        #[arg(short, long)]
        name: Option<String>,

        /// Custom description for the workflow
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Export a workflow from database or vault to a JSON file
    Export {
        /// Workflow ID or Name to export
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,

        /// Destination file path (default: <id>.workflow.json)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Inspect details, triggers, parameters, and block sequence of a stored workflow (alias: show)
    #[command(name = "info", alias = "show")]
    Info {
        /// Workflow ID or Name
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,
    },

    /// Delete a workflow from database and vault (alias: rm)
    #[command(name = "delete", alias = "rm")]
    Delete {
        /// Workflow ID to delete
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,

        /// Also delete from vault directory if present
        #[arg(long, default_value_t = true)]
        vault: bool,
    },
}

