use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .usage(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::BrightBlack.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
}

pub const MAIN_HELP_TEMPLATE: &str = "\
\x1b[38;2;56;189;248m  ______          ____                  __ \x1b[0m
\x1b[38;2;96;165;250m /_  __/_  __    / __ \\__  __  ___     / /_\x1b[0m
\x1b[38;2;129;140;248m  / /  / / / /  / / / // / / // _ \\   / __/\x1b[0m   \x1b[1;38;2;56;189;248mv1.0.0\x1b[0m
\x1b[38;2;168;85;247m / /  / /_/ /  / /_/ // /_/ //  __/  / /_  \x1b[0m
\x1b[38;2;192;132;252m/_/   \\__,_/   \\___\\_\\\\__,_/ \\___/   \\__/\x1b[0m

  Autonomous Browser Automation & Distributed Mesh Runtime

\x1b[1;38;2;56;189;248mUsage:\x1b[0m \x1b[1mtuquet\x1b[0m [COMMAND] [OPTIONS]

\x1b[1;38;2;56;189;248mQuick Commands:\x1b[0m
  \x1b[1;32mstatus\x1b[0m          Inspect unified status across Cloud, Runner, and Browser
  \x1b[1;32mwhoami\x1b[0m          Check active cloud enrollment identity & device ID
  \x1b[1;32mlogin\x1b[0m           Authenticate and pair workstation with Tuquet Cloud

\x1b[1;38;2;56;189;248mSubsystems:\x1b[0m
  \x1b[1;36mautoma\x1b[0m          Browser automation engine & workflow runner
  \x1b[1;36mrunner\x1b[0m          Distributed daemon worker & cloud execution node (aliases: daemon, worker)
  \x1b[1;36mcloud\x1b[0m           Tuquet Cloud authentication & multi-tenant pairing (alias: auth)
  \x1b[1;36mbrowser\x1b[0m         Dedicated isolated browser runtime management

\x1b[1;38;2;56;189;248mInteractive:\x1b[0m
  \x1b[1;35mshell\x1b[0m           Launch interactive scoped shell session (alias: repl)

\x1b[1;38;2;56;189;248mOptions:\x1b[0m
  \x1b[38;2;148;163;184m-h, --help\x1b[0m      Print help overview
  \x1b[38;2;148;163;184m-V, --version\x1b[0m   Print version

\x1b[1;38;2;56;189;248mExamples:\x1b[0m
  tuquet                          Launch modern interactive REPL
  tuquet status                   Check full ecosystem dashboard
  tuquet whoami                   Display cloud device identity
  tuquet automa run ./wf.json     Execute workflow directly
  tuquet runner start --port 8765 Start local daemon worker
  tuquet browser status           Inspect dedicated browser engine";

/// Tuquet - Unified CLI Tool
#[derive(Parser, Debug)]
#[command(
    name = "tuquet",
    author = "Tuquet Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Tuquet - Unified CLI Tool",
    long_about = "Tuquet is a unified CLI tool for managing and orchestrating services in the Tuquet ecosystem.",
    styles = get_styles(),
    help_template = MAIN_HELP_TEMPLATE,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show unified status overview of Tuquet ecosystem
    #[command(name = "status")]
    Status,

    /// Show current Tuquet Cloud identity and enrollment status
    #[command(name = "whoami")]
    Whoami,

    /// Authenticate and pair workstation with Tuquet Cloud
    #[command(name = "login")]
    Login {
        /// Tuquet Cloud endpoint URL
        #[arg(short, long)]
        url: Option<String>,

        /// Organization / Tenant enrollment token
        #[arg(short, long)]
        token: Option<String>,

        /// Custom workstation name (defaults to machine hostname)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Automa browser workflow automation engine
    #[command(name = "automa")]
    Automa {
        #[command(subcommand)]
        command: Option<AutomaSubcommands>,
    },

    /// Distributed runner daemon worker & cloud execution node
    #[command(name = "runner", aliases = ["daemon", "worker"])]
    Runner {
        #[command(subcommand)]
        command: Option<RunnerSubcommands>,
    },

    /// Tuquet Cloud authentication & multi-tenant pairing
    #[command(name = "cloud", alias = "auth")]
    Cloud {
        #[command(subcommand)]
        command: Option<CloudSubcommands>,
    },

    /// Dedicated isolated browser runtime management
    #[command(name = "browser")]
    Browser {
        #[command(subcommand)]
        command: Option<BrowserCommands>,
    },

    /// Launch interactive scoped shell session
    #[command(name = "shell", alias = "repl")]
    Shell {
        /// Optional target service scope to enter (automa, runner, cloud, browser)
        #[arg(value_name = "SERVICE")]
        service: Option<String>,
    },
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

        /// Run in background as detached daemon process
        #[arg(short = 'd', long)]
        detach: bool,

        /// Path to custom data directory (stores SQLite DB and logs)
        #[arg(short, long)]
        data_dir: Option<PathBuf>,

        /// Log verbosity level (trace, debug, info, warn, error)
        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Gracefully stop the running Tuquet runner daemon
    Stop {
        /// Force terminate without waiting for active jobs
        #[arg(short, long)]
        force: bool,
    },

    /// Restart the local Tuquet runner daemon
    Restart {
        /// Run in background as detached daemon process
        #[arg(short = 'd', long)]
        detach: bool,
    },

    /// Inspect local Tuquet daemon status and health check endpoint
    Status {
        /// Daemon server base URL (defaults to http://{AUTOMA_HOST}:{AUTOMA_PORT} or http://127.0.0.1:8765)
        #[arg(short, long)]
        url: Option<String>,
    },

    /// View or tail runner daemon execution and telemetry logs
    Logs {
        /// Follow / stream log output continuously
        #[arg(short = 'f', long)]
        follow: bool,

        /// Number of tail lines to display
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: usize,
    },

    /// Active capability negotiation probe returning manifest JSON for Runner
    Probe,

    /// Developer utility: Export OpenAPI v3 JSON specification to file (Hidden)
    #[command(hide = true)]
    ExportOpenapi {
        /// Output JSON destination file path
        #[arg(short, long, default_value = "openapi.json")]
        output: PathBuf,
    },

    /// [Deprecated: Moved to 'tuquet browser ext']
    #[command(hide = true)]
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

    /// Manage and configure browser extensions (Automa MV3 and custom extensions)
    #[command(name = "ext", aliases = ["extension", "setup-ext"])]
    Ext {
        #[command(subcommand)]
        command: Option<ExtCommands>,

        /// Target browser to launch (chrome, edge, brave, auto)
        #[arg(short, long, default_value = "chrome")]
        browser: String,

        /// Custom path to unpacked extension directory
        #[arg(short, long)]
        extension_path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ExtCommands {
    /// List all registered browser extensions (alias: ls)
    #[command(name = "list", alias = "ls")]
    List,

    /// Browse and search available extensions from tuquet-scoop-bucket (aliases: search, available)
    #[command(name = "catalog", aliases = ["search", "available"])]
    Catalog {
        /// Optional keyword to filter extensions
        query: Option<String>,
    },

    /// Download and install an extension package from tuquet-scoop-bucket (alias: get)
    #[command(name = "install", alias = "get")]
    Install {
        /// Extension ID from catalog (e.g. 'automa', 'ublock', 'cookie-injector')
        id: String,

        /// Force re-download and overwrite existing installation
        #[arg(short, long)]
        force: bool,
    },

    /// Register a new browser extension from an unpacked directory
    #[command(name = "add")]
    Add {
        /// Path to unpacked extension directory containing manifest.json
        path: PathBuf,

        /// Optional custom ID for the extension (defaults to slugified manifest name)
        #[arg(short, long)]
        id: Option<String>,
    },

    /// Unregister a browser extension by ID (alias: rm)
    #[command(name = "remove", alias = "rm")]
    Remove {
        /// Extension ID to remove
        id: String,
    },

    /// Enable an extension for automated browser sessions
    #[command(name = "enable")]
    Enable {
        /// Extension ID to enable
        id: String,
    },

    /// Disable an extension
    #[command(name = "disable")]
    Disable {
        /// Extension ID to disable
        id: String,
    },

    /// Show detailed metadata and manifest for an extension
    #[command(name = "info")]
    Info {
        /// Extension ID to inspect (defaults to 'automa')
        #[arg(default_value = "automa")]
        id: String,
    },

    /// Print the absolute filesystem path of an extension (defaults to 'automa')
    #[command(name = "path")]
    Path {
        /// Extension ID (defaults to 'automa')
        #[arg(default_value = "automa")]
        id: String,
    },

    /// Launch browser with specified extension(s) or all enabled extensions
    #[command(name = "launch")]
    Launch {
        /// Comma-separated extension IDs to load (or 'all' for all enabled)
        #[arg(short, long)]
        ext: Option<String>,

        /// Target browser to launch (chrome, edge, brave)
        #[arg(short, long, default_value = "chrome")]
        browser: String,
    },
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

        /// Only list workflows from Vault directory (~/.tuquet/workflows)
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

