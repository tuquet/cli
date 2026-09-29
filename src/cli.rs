use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
}

pub const MAIN_HELP_TEMPLATE: &str = "\
{about-with-newline}
{usage-heading} {usage}

\x1b[1;32mServices:\x1b[0m
  \x1b[1;36mautoma\x1b[0m          Browser automation engine & workflow runner
  \x1b[1;36mrunner\x1b[0m          Distributed daemon worker & cloud execution node (aliases: daemon, worker)
  \x1b[1;36mcloud\x1b[0m           Tuquet Cloud authentication & multi-tenant pairing (alias: auth)
  \x1b[1;36mbrowser\x1b[0m         Dedicated isolated browser runtime management

\x1b[1;32mOptions:\x1b[0m
  \x1b[1;36m-h, --help\x1b[0m          Print help (see more with '--help')
  \x1b[1;36m-V, --version\x1b[0m       Print version

\x1b[1;32mCompatibility & Probes:\x1b[0m
      \x1b[1;36m--probe\x1b[0m                 Active capability negotiation and handshake probe for Tuquet Runner (tqr)
      \x1b[1;36m--export-openapi\x1b[0m        Export OpenAPI v3 JSON specification to file

\x1b[1;32mExamples:\x1b[0m
  tuquet automa run ./workflow.json --headless
  tuquet automa workflow list
  tuquet runner start --port 8765
  tuquet cloud login --token <enrollment-token>
  tuquet browser install";

/// Tuquet - Unified CLI & Distributed Automation Engine
#[derive(Parser, Debug)]
#[command(
    name = "tuquet",
    author = "Tuquet Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Tuquet - Unified CLI & Distributed Automation Engine",
    long_about = "Master control CLI, cloud-connected runner daemon, and browser automation engine for the Tuquet Ecosystem.",
    styles = get_styles(),
    help_template = MAIN_HELP_TEMPLATE
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Active capability negotiation and handshake probe for Tuquet Runner (tqr)
    #[arg(long, help_heading = "Compatibility & Probes")]
    pub probe: bool,

    /// Flag for backward compatibility: export openapi spec
    #[arg(long, help_heading = "Compatibility & Probes")]
    pub export_openapi: Option<Option<PathBuf>>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ---------------------------------------------------------
    // Core Services
    // ---------------------------------------------------------
    /// Automa browser workflow automation engine
    #[command(name = "automa")]
    Automa {
        #[command(subcommand)]
        command: AutomaSubcommands,
    },

    /// Distributed runner daemon worker & cloud execution node
    #[command(name = "runner", aliases = ["daemon", "worker"])]
    Runner {
        #[command(subcommand)]
        command: RunnerSubcommands,
    },

    /// Tuquet Cloud authentication & multi-tenant pairing
    #[command(name = "cloud", alias = "auth")]
    Cloud {
        #[command(subcommand)]
        command: CloudSubcommands,
    },

    /// Dedicated isolated browser runtime management
    #[command(name = "browser")]
    Browser {
        #[command(subcommand)]
        command: BrowserCommands,
    },

    // ---------------------------------------------------------
    // Backward-Compatibility Root Shortcuts (Hidden from help)
    // ---------------------------------------------------------
    #[command(hide = true)]
    Run {
        #[arg(value_name = "WORKFLOW_FILE")]
        workflow_pos: Option<String>,
        #[arg(short, long)]
        workflow: Option<String>,
        #[arg(long)]
        workflow_json: Option<String>,
        #[arg(long)]
        headless: bool,
        #[arg(short, long)]
        browser: Option<String>,
        #[arg(long)]
        browser_id: Option<String>,
        #[arg(short = 'p', long = "var", alias = "param", value_name = "KEY=VALUE")]
        variables: Vec<String>,
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    #[command(hide = true, name = "workflow", alias = "wf")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },

    #[command(hide = true)]
    Inspect {
        #[arg(value_name = "WORKFLOW")]
        workflow: String,
    },

    #[command(hide = true, name = "server", aliases = ["start", "up"])]
    Server {
        #[arg(short = 'H', long)]
        host: Option<String>,
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(short, long)]
        data_dir: Option<PathBuf>,
        #[arg(short, long)]
        log_level: Option<String>,
    },

    #[command(hide = true)]
    SetupExt {
        #[arg(short, long, default_value = "chrome")]
        browser: String,
        #[arg(short, long)]
        extension_path: Option<PathBuf>,
    },

    #[command(hide = true)]
    ExportOpenapi {
        #[arg(short, long, default_value = "openapi.json")]
        output: PathBuf,
    },

    #[command(hide = true)]
    Status {
        #[arg(short, long)]
        url: Option<String>,
    },

    #[command(hide = true)]
    Login {
        #[arg(short, long)]
        url: Option<String>,
        #[arg(short, long)]
        token: Option<String>,
        #[arg(short, long)]
        name: Option<String>,
    },

    #[command(hide = true)]
    Logout,

    #[command(hide = true)]
    Whoami,

    #[command(hide = true)]
    Probe,
}

#[derive(Subcommand, Debug)]
pub enum AutomaSubcommands {
    /// Execute a workflow directly via browser worker
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

        /// Workflow variables in KEY=VALUE format
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

    /// Inspect and validate a workflow JSON file or stored workflow ID
    Inspect {
        /// Path to workflow JSON file (.workflow.json) or saved workflow ID/Name
        #[arg(value_name = "WORKFLOW")]
        workflow: String,
    },

    /// Launch Automa Web Studio in default system browser
    Studio,

    /// Probe Automa manifest capabilities
    Probe,
}

#[derive(Subcommand, Debug)]
pub enum RunnerSubcommands {
    /// Start the Tuquet runner daemon and cloud worker (aliases: server, up, worker)
    #[command(name = "start", aliases = ["server", "up", "worker"])]
    Start {
        /// HTTP server listening host IP (e.g. 127.0.0.1, 0.0.0.0)
        #[arg(short = 'H', long)]
        host: Option<String>,

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

    /// Inspect local Tuquet daemon status and health check endpoint
    Status {
        /// Daemon server base URL (defaults to http://{AUTOMA_HOST}:{AUTOMA_PORT} or http://127.0.0.1:8765)
        #[arg(short, long)]
        url: Option<String>,
    },

    /// Active capability negotiation probe returning manifest JSON for Tuquet Runner
    Probe,

    /// Export OpenAPI v3 JSON specification to file
    ExportOpenapi {
        /// Output JSON destination file path
        #[arg(short, long, default_value = "openapi.json")]
        output: PathBuf,
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
}

#[derive(Subcommand, Debug)]
pub enum CloudSubcommands {
    /// Authenticate and enroll this workstation with Tuquet Cloud
    Login {
        /// Tuquet Cloud endpoint URL (e.g. https://cloud.tuquet.com)
        #[arg(short, long)]
        url: Option<String>,

        /// Organization / Tenant enrollment token
        #[arg(short, long)]
        token: Option<String>,

        /// Custom workstation name (defaults to machine hostname)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Log out and disconnect this workstation from Tuquet Cloud
    Logout,

    /// Show current Tuquet Cloud authentication and enrollment status (alias: status)
    #[command(name = "whoami", alias = "status")]
    Whoami,
}

pub type AuthCommands = CloudSubcommands;

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

