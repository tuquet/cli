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
\x1b[38;2;56;189;248m   _____  ____   ______  ______ ______  ______  ____  \x1b[0m
\x1b[38;2;96;165;250m  / ___/ / __ \\ / ____/ / ____//_  __/ / ____/ / __ \\ \x1b[0m
\x1b[38;2;129;140;248m  \\__ \\  / /_/ // __/   / /     / /    / __/   / /_/ /\x1b[0m   \x1b[1;38;2;56;189;248mv1.0.0\x1b[0m
\x1b[38;2;168;85;247m ___/ /  / ____// /___  / /___  / /    / /___  / _, _/\x1b[0m
\x1b[38;2;192;132;252m/____/   /_/    /_____/ \\____/  /_/    /_____/ /_/ |_|\x1b[0m

  Autonomous Browser Automation & Distributed Mesh Runtime

\x1b[1;38;2;56;189;248mUsage:\x1b[0m \x1b[1mspecter\x1b[0m [COMMAND] [OPTIONS]

\x1b[1;38;2;56;189;248mQuick Commands:\x1b[0m
  \x1b[1;32mstatus\x1b[0m          Inspect unified status across Cloud, Runner, and Browser
  \x1b[1;32mdoctor\x1b[0m          Check all ecosystem dependencies, system tools & health
  \x1b[1;32mbootstrap\x1b[0m       One-command workstation onboarding & ecosystem bootstrap
  \x1b[1;32mconfig\x1b[0m          Inspect or edit configuration across all microservice pillars
  \x1b[1;32mwhoami\x1b[0m          Check active cloud enrollment identity & device ID
  \x1b[1;32mlogin\x1b[0m           Authenticate and pair workstation with Tuquet Cloud
  \x1b[1;32mupgrade\x1b[0m         Check and upgrade Specter CLI to the latest release
  \x1b[1;32mschema\x1b[0m          Inspect CLI command manifests & microservice schemas


\x1b[1;38;2;56;189;248mSubsystems:\x1b[0m
  \x1b[1;36mautoma\x1b[0m          Browser automation engine & workflow runner
  \x1b[1;36mrunner\x1b[0m          Distributed daemon worker & cloud execution node
  \x1b[1;36mcloud\x1b[0m           Tuquet Cloud authentication & multi-tenant pairing
  \x1b[1;36mbrowser\x1b[0m         Dedicated isolated browser runtime management
  \x1b[1;36mfaker\x1b[0m           Synthetic persona & test data generator (CCCD, addresses)
  \x1b[1;36mbridge\x1b[0m          Network bridge, multi-VPS mesh & proxy manager

\x1b[1;38;2;56;189;248mIntegration:\x1b[0m
  \x1b[1;35mmcp\x1b[0m             Run Model Context Protocol (MCP) stdio server for AI agents

\x1b[1;38;2;56;189;248mInteractive:\x1b[0m
  \x1b[1;35mshell\x1b[0m           Launch interactive scoped shell session (SPECTER SHELL)

\x1b[1;38;2;56;189;248mOptions:\x1b[0m
  \x1b[38;2;148;163;184m-h, --help\x1b[0m      Print help overview
  \x1b[38;2;148;163;184m-V, --version\x1b[0m   Print version

\x1b[1;38;2;56;189;248mExamples:\x1b[0m
  specter                          Launch interactive SPECTER SHELL (REPL)
  specter doctor                   Check dependencies and system health
  specter status                   Check full ecosystem dashboard
  specter bridge status            Inspect multi-VPS tunnels & proxy health
  specter whoami                   Display cloud device identity
  specter faker generate -n 5      Generate 5 compliant personas in a table
  specter automa run ./wf.json     Execute workflow directly
  specter runner start --port 8765 Start local daemon worker
  specter browser status           Inspect dedicated browser engine
  specter mcp                      Start native MCP stdio server";

/// Specter - Autonomous Browser Automation & Distributed Mesh Runtime
#[derive(Parser, Debug)]
#[command(
    name = "specter",
    author = "Tuquet Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Specter - Autonomous Browser Automation & Distributed Mesh Runtime",
    long_about = "Specter is an autonomous browser automation & distributed mesh runtime engine.",
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
    Status {
        /// Output status in raw JSON format for machine parsing
        #[arg(short, long)]
        json: bool,
    },

    /// Check system dependencies, required tools & environment health
    #[command(name = "doctor")]
    Doctor {
        /// Attempt automatic fix / installation of missing dependencies where supported
        #[arg(short, long)]
        fix: bool,
    },

    /// One-command workstation onboarding & ecosystem bootstrap (SSOT directories, DB, MCP, browser)
    #[command(name = "bootstrap", aliases = ["setup", "init"])]
    Bootstrap {
        /// Force re-download and re-provisioning of runtimes even if already present
        #[arg(short, long)]
        force: bool,
    },

    /// Inspect or edit unified service configurations across Tuquet microservice pillars
    #[command(name = "config")]
    Config {
        /// Optional target service (bridge, automa, runner, browser, cloud, system, faker)
        #[arg(value_name = "SERVICE")]
        service: Option<String>,

        /// Open configuration file in default editor
        #[arg(short, long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short, long)]
        show: bool,
    },

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
    #[command(name = "runner")]
    Runner {
        #[command(subcommand)]
        command: Option<RunnerSubcommands>,
    },

    /// Tuquet Cloud authentication & multi-tenant pairing
    #[command(name = "cloud")]
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

    /// Manage isolated browser profiles with deterministic antidetect fingerprints
    #[command(name = "profile", aliases = ["profiles"])]
    Profile {
        #[command(subcommand)]
        command: Option<ProfileCommands>,
    },

    /// Launch browser profile with direct CDP DevTools bridge
    #[command(name = "launch")]
    Launch {
        /// Target profile ID or name (default: "default")
        #[arg(value_name = "PROFILE", default_value = "default")]
        profile: String,

        /// Chrome DevTools Protocol (CDP) port for Playwright / Puppeteer automation
        #[arg(short, long, default_value_t = 9222)]
        port: u16,

        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,

        /// Initial URL to navigate to
        #[arg(short, long)]
        url: Option<String>,

        /// Run in background without keeping terminal attached
        #[arg(short, long)]
        detach: bool,

        /// Override proxy server (e.g. socks5://127.0.0.1:1080)
        #[arg(long)]
        proxy: Option<String>,

        /// Automation mode: 'driver' (CDP DevTools bridge on port) or 'extension' (Zero-port ultra-stealth)
        #[arg(short, long, default_value = "driver")]
        mode: String,

        /// Shortcut for --mode extension (disables remote debugging port completely)
        #[arg(long)]
        no_cdp: bool,

        /// Bypass pre-flight proxy healthcheck and launch immediately
        #[arg(long)]
        skip_proxy_check: bool,
    },

    /// Enterprise synthetic persona & identity generator (CCCD, addresses, credentials)
    #[command(name = "faker", aliases = ["user", "persona"])]
    Faker {
        #[command(subcommand)]
        command: Option<FakerSubcommands>,
    },

    /// Launch interactive scoped shell session
    #[command(name = "shell")]
    Shell {
        /// Optional target service scope to enter (automa, runner, cloud, browser, bridge, faker)
        #[arg(value_name = "SERVICE")]
        service: Option<String>,
    },

    /// Run Model Context Protocol (MCP) JSON-RPC 2.0 stdio server for AI agents
    #[command(name = "mcp")]
    Mcp,

    /// Check and upgrade Tuquet CLI to the latest release
    #[command(name = "upgrade", aliases = ["update"])]
    Upgrade,

    /// Probe proxy connectivity, latency, and public egress IP address
    #[command(name = "proxy")]
    Proxy {
        #[command(subcommand)]
        command: Option<ProxyCommands>,
    },

    /// Network bridge, multi-VPS tunnel mesh & proxy manager
    #[command(name = "bridge", aliases = ["tunnel", "vps"])]
    Bridge {
        #[command(subcommand)]
        command: Option<BridgeSubcommands>,
    },

    /// Inspect and export CLI command manifests and microservice JSON schemas
    #[command(name = "schema", aliases = ["manifest"])]
    Schema {
        #[command(subcommand)]
        command: Option<SchemaSubcommands>,

        /// Output full schema manifest in raw JSON format
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SchemaSubcommands {
    /// List or export all commands declared in the schema manifest
    #[command(name = "commands", aliases = ["cmds"])]
    Commands {
        /// Filter commands by microservice pillar domain (system, automa, browser, bridge, faker, runner, cloud)
        #[arg(short, long)]
        pillar: Option<String>,

        /// Output catalog in raw JSON format
        #[arg(short, long)]
        json: bool,
    },

    /// Export the JSON Schema definition for a microservice configuration file
    #[command(name = "config")]
    Config {
        /// Service configuration schema to output (bridge, browser, automa, runner, faker, system)
        #[arg(value_name = "SERVICE")]
        service: Option<String>,
    },

    /// Export AI Agent Model Context Protocol (MCP) tool schemas generated from manifest
    #[command(name = "mcp")]
    Mcp,
}

#[derive(Subcommand, Debug, Clone)]
pub enum BridgeSubcommands {
    /// Show health check dashboard of all configured VPSs, ports, and workloads
    #[command(name = "status")]
    Status,

    /// Start a bridge connection for configured servers (default: all enabled)
    #[command(name = "start")]
    Start {
        /// Target server identifier or pattern (e.g. my-vps, vps-*, or omit for all enabled)
        #[arg(value_name = "SERVER")]
        server: Option<String>,

        /// Filter servers by tag (e.g. mmo, git, clean-ip)
        #[arg(short = 't', long)]
        tag: Option<String>,

        /// Also start the pure Rust embedded HTTP-to-SOCKS5 bridge (port 8118)
        #[arg(long)]
        http: bool,

        /// Also start local SSH port forwarding (e.g. port 2222) for direct SSH access
        #[arg(long)]
        ssh: bool,

        /// Run supervisor in foreground (keeps running until Ctrl+C, auto-heals dropped tunnels)
        #[arg(short = 'f', long)]
        foreground: bool,
    },

    /// Stop running bridge tunnels and proxy daemons
    #[command(name = "stop")]
    Stop {
        /// Server identifier to stop (or 'all' to terminate all bridge processes)
        #[arg(value_name = "SERVER")]
        server: Option<String>,
    },

    /// Enable a server in bridge configuration (~/.specter/bridge/bridge.json)
    #[command(name = "enable")]
    Enable {
        /// Server identifier to enable (e.g. my-vps, may-b)
        #[arg(value_name = "SERVER")]
        server: String,
    },

    /// Disable a server in bridge configuration (~/.specter/bridge/bridge.json)
    #[command(name = "disable")]
    Disable {
        /// Server identifier to disable (e.g. my-vps, may-b)
        #[arg(value_name = "SERVER")]
        server: String,
    },

    /// Validate ~/.specter/bridge/bridge.json configuration schema, ports, and workloads
    #[command(name = "check")]
    Check,

    /// Display, inspect, or edit the canonical bridge configuration file
    #[command(name = "config")]
    Config {
        /// Open ~/.specter/bridge/bridge.json in your default editor
        #[arg(short, long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short, long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum FakerSubcommands {
    /// Generate synthetic identity profiles with validated CCCD and addresses
    #[command(name = "generate", aliases = ["gen", "g"])]
    Generate {
        /// Number of profiles to generate
        #[arg(short = 'n', long, default_value = "1")]
        count: u32,

        /// Filter by gender: male, female, or all
        #[arg(short = 'g', long)]
        gender: Option<String>,

        /// Filter by nationality: VN, US, JP, or all
        #[arg(long, default_value = "VN")]
        nat: Option<String>,

        /// Avatar style: real (portrait photo) or svg (vector avatar)
        #[arg(long, default_value = "real")]
        avatar: Option<String>,

        /// Custom email domain (e.g. flowup.io.vn), overrides faker.json
        #[arg(short = 'd', long)]
        domain: Option<String>,

        /// Output format: table, card, json, or csv
        #[arg(short = 'f', long, default_value = "table")]
        format: String,

        /// Optional file path to export output to
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Inspect a single detailed persona card with full credentials
    #[command(name = "card", aliases = ["show", "inspect"])]
    Card {
        /// Filter by gender: male, female, or all
        #[arg(short = 'g', long)]
        gender: Option<String>,

        /// Filter by nationality: VN, US, JP, or all
        #[arg(long, default_value = "VN")]
        nat: Option<String>,

        /// Avatar style: real or svg
        #[arg(long, default_value = "real")]
        avatar: Option<String>,

        /// Custom email domain (e.g. flowup.io.vn), overrides faker.json
        #[arg(short = 'd', long)]
        domain: Option<String>,
    },

    /// Manage synthetic persona configuration (~/.specter/faker/faker.json)
    #[command(name = "config")]
    Config {
        /// Open faker.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,

        /// Set default email domain (e.g. flowup.io.vn)
        #[arg(short = 'd', long)]
        domain: Option<String>,
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

        /// Target cloud browser profile ID or name (auto-acquires lease, unpacks snapshot, verifies proxy, executes, packs delta, and releases lease)
        #[arg(long)]
        cloud_profile: Option<String>,

        /// Workflow variables in KEY=VALUE format
        #[arg(short = 'p', long = "var", value_name = "KEY=VALUE")]
        variables: Vec<String>,

        /// Optional workflow execution timeout in seconds
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    /// Manage stored workflows in local database and vault
    #[command(name = "workflow")]
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

    /// Display, inspect, or edit Automa configuration (~/.specter/automa/automa.json)
    #[command(name = "config")]
    Config {
        /// Open automa.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum RunnerSubcommands {
    /// Start the runner daemon and cloud worker
    #[command(name = "start")]
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
        #[arg(short = 'D', long)]
        data_dir: Option<PathBuf>,

        /// Log verbosity level (trace, debug, info, warn, error)
        #[arg(short, long)]
        log_level: Option<String>,

        /// Enable Autonomous Cloud Fleet Mesh worker mode
        #[arg(short = 'c', long)]
        cloud: bool,
    },

    /// Run as an autonomous cloud fleet worker polling and executing jobs
    #[command(name = "worker")]
    Worker {
        /// Target cloud profile ID or name to execute
        #[arg(long)]
        cloud_profile: Option<String>,

        /// Path to workflow JSON file to execute on cloud profile
        #[arg(short, long)]
        workflow: Option<String>,

        /// Headless browser execution
        #[arg(long)]
        headless: bool,

        /// Poll interval in seconds (default: 15)
        #[arg(short, long, default_value_t = 15)]
        interval: u64,

        /// Run once and exit
        #[arg(long)]
        once: bool,
    },

    /// Gracefully stop the running runner daemon
    Stop {
        /// Force terminate without waiting for active jobs
        #[arg(short, long)]
        force: bool,
    },

    /// Restart the local runner daemon
    Restart {
        /// Run in background as detached daemon process
        #[arg(short = 'd', long)]
        detach: bool,
    },

    /// Inspect local runner daemon status and health check endpoint
    Status {
        /// Daemon server base URL (defaults to http://{AUTOMA_HOST}:{AUTOMA_PORT} or http://127.0.0.1:8765)
        #[arg(short, long)]
        url: Option<String>,

        /// Output status in raw JSON format for machine parsing
        #[arg(short, long)]
        json: bool,
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

    /// Display, inspect, or edit Runner daemon configuration (~/.specter/automa/runner.json)
    #[command(name = "config")]
    Config {
        /// Open runner.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
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

    /// Show current Tuquet Cloud authentication and enrollment status
    #[command(name = "whoami")]
    Whoami,

    /// Display, inspect, or edit System & Cloud configuration (~/.specter/system/system.json)
    #[command(name = "config")]
    Config {
        /// Open system.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
    },
}

pub type AuthCommands = CloudSubcommands;

#[derive(Subcommand, Debug)]
pub enum BrowserCommands {
    /// Download and install C++ Antidetect Chromium version from curated manifest
    #[command(name = "install")]
    Install {
        /// Force re-download even if already installed
        #[arg(short, long)]
        force: bool,

        /// Target version or channel (e.g. '148', 'v148', '144', 'lts', 'latest')
        #[arg(value_name = "VERSION")]
        version: Option<String>,
    },

    /// Display installation status, executable path, and disk usage of dedicated browser
    #[command(name = "status")]
    Status,

    /// Search and list available engine releases from the curated manifest
    #[command(name = "search", aliases = ["releases"])]
    Search {
        /// Force refreshing remote manifest from GitHub CDN
        #[arg(short, long)]
        remote: bool,
    },

    /// List all locally installed browser runtimes on this machine
    #[command(name = "list", aliases = ["ls"])]
    List,

    /// Switch the active C++ Antidetect Chromium version (e.g. '148', 'v148', '144', 'lts')
    #[command(name = "use")]
    Use {
        /// Version identifier or alias (e.g. '148', 'v148', '144', '148.0.7778.215', 'lts')
        #[arg(value_name = "VERSION")]
        version: String,
    },

    /// Delete installed browser runtime to reclaim disk space
    #[command(name = "clean")]
    Clean,

    /// Print the absolute executable path of the browser (for scripting / integrations)
    #[command(name = "path")]
    Path,

    /// Manage and configure browser extensions (Automa MV3 and custom extensions)
    #[command(name = "ext")]
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

    /// Launch an antidetect browser profile with direct CDP DevTools bridge
    #[command(name = "launch", aliases = ["start", "open"])]
    Launch {
        /// Target profile ID or name (default: "default")
        #[arg(value_name = "PROFILE", default_value = "default")]
        profile: String,

        /// Chrome DevTools Protocol (CDP) port for Playwright / Puppeteer automation
        #[arg(short, long, default_value_t = 9222)]
        port: u16,

        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,

        /// Initial URL to navigate to
        #[arg(short, long)]
        url: Option<String>,

        /// Run in background without keeping terminal attached
        #[arg(short, long)]
        detach: bool,

        /// Override proxy server (e.g. socks5://127.0.0.1:1080)
        #[arg(long)]
        proxy: Option<String>,

        /// Automation mode: 'driver' (CDP DevTools bridge on port) or 'extension' (Zero-port ultra-stealth)
        #[arg(short, long, default_value = "driver")]
        mode: String,

        /// Shortcut for --mode extension (disables remote debugging port completely)
        #[arg(long)]
        no_cdp: bool,

        /// Bypass pre-flight proxy healthcheck and launch immediately
        #[arg(long)]
        skip_proxy_check: bool,
    },

    /// Live visual verification & stealth presentation (Cloudflare Turnstile, Bézier mouse, smooth scroll)
    #[command(name = "verify", aliases = ["demo", "inspect-live", "live"])]
    Verify {
        /// Target URL to test (defaults to Cloudflare Turnstile managed challenge test)
        #[arg(short, long)]
        url: Option<String>,

        /// Run in headless mode (default: false for visual presentation)
        #[arg(long)]
        headless: bool,

        /// Timeout in seconds for challenge resolution
        #[arg(short, long, default_value_t = 30)]
        timeout: u64,
    },

    /// Manage isolated browser profiles with deterministic antidetect fingerprints
    #[command(name = "profile", aliases = ["profiles"])]
    Profile {
        #[command(subcommand)]
        command: Option<ProfileCommands>,
    },

    /// Display, inspect, or edit dedicated browser configuration (~/.specter/browser/browser.json)
    #[command(name = "config")]
    Config {
        /// Open browser.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProfileCommands {
    /// List all local browser profiles
    #[command(name = "list", aliases = ["ls"])]
    List,

    /// Create a new browser profile with deterministic hardware specs
    #[command(name = "create", aliases = ["new", "add"])]
    Create {
        /// Profile name (e.g. 'Facebook-Ad-VN', 'TikTok-US')
        name: String,

        /// Deterministic PRNG seed (random u32 if omitted)
        #[arg(short, long)]
        seed: Option<u32>,

        /// OS Platform (windows, macos, linux)
        #[arg(long, default_value = "windows")]
        os: String,

        /// CPU cores to spoof
        #[arg(long)]
        cores: Option<u32>,

        /// RAM in GB to spoof
        #[arg(long)]
        ram: Option<u32>,

        /// Proxy URL (e.g. socks5://127.0.0.1:1080)
        #[arg(long)]
        proxy: Option<String>,

        /// Timezone (e.g. Asia/Ho_Chi_Minh, America/New_York)
        #[arg(long, default_value = "Asia/Ho_Chi_Minh")]
        timezone: String,

        /// Locale / Language (e.g. vi-VN, en-US)
        #[arg(long, default_value = "vi-VN")]
        locale: String,
    },

    /// Inspect detailed hardware fingerprint specifications of a profile
    #[command(name = "inspect", aliases = ["show", "info"])]
    Inspect {
        /// Profile ID or name
        id: String,
    },

    /// Probe and test the proxy configured for a specific profile
    #[command(name = "test-proxy", aliases = ["check-proxy", "probe-proxy"])]
    TestProxy {
        /// Profile ID or name
        id: String,

        /// Probe timeout in seconds (default: 5)
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,

        /// Output diagnostic result in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Delete a browser profile and its sandbox data
    #[command(name = "delete", aliases = ["remove", "rm"])]
    Delete {
        /// Profile ID or name
        id: String,

        /// Force deletion without confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Pack a profile into a lightweight .tar.zst archive with cache filtering
    #[command(name = "pack", aliases = ["compress", "archive"])]
    Pack {
        /// Profile ID or name
        id: String,

        /// Custom output archive path (default: <profile_dir>.tar.zst)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Zstandard compression level (1-19, default: 3)
        #[arg(short, long, default_value_t = 3)]
        level: i32,

        /// Output result in raw JSON format for scripting and pipelines
        #[arg(long)]
        json: bool,
    },

    /// Unpack and restore a profile from a .tar.zst archive into SSOT storage
    #[command(name = "unpack", aliases = ["restore", "extract"])]
    Unpack {
        /// Path to .tar.zst profile archive
        archive: PathBuf,

        /// Optional expected SHA-256 integrity hash
        #[arg(long)]
        hash: Option<String>,

        /// Output result in raw JSON format for scripting and pipelines
        #[arg(long)]
        json: bool,
    },

    /// Manage cloud-synchronized browser profiles and distributed lease locks
    #[command(name = "cloud")]
    Cloud {
        #[command(subcommand)]
        command: Option<ProfileCloudSubcommands>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProfileCloudSubcommands {
    /// List all central cloud browser profiles in current tenant
    #[command(name = "list", aliases = ["ls"])]
    List {
        /// Output result in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Acquire exclusive distributed lease lock on a cloud browser profile
    #[command(name = "acquire", aliases = ["lock", "checkout"])]
    Acquire {
        /// Cloud Browser Profile ID (UUID) or name
        id: String,

        /// Output result in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Release distributed lease lock and synchronize session delta back to cloud
    #[command(name = "release", aliases = ["unlock", "checkin"])]
    Release {
        /// Cloud Browser Profile ID (UUID) or name
        id: String,

        /// Output result in raw JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProxyCommands {
    /// Probe connectivity, latency, and public egress IP of a proxy
    #[command(name = "probe", aliases = ["test", "check"])]
    Probe {
        /// Target proxy URL (e.g. socks5://127.0.0.1:1080 or http://127.0.0.1:8118)
        url: String,

        /// Probe timeout in seconds (default: 5)
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,

        /// Output diagnostic result in raw JSON format
        #[arg(long)]
        json: bool,
    },
}


#[derive(Subcommand, Debug, Clone)]
pub enum ExtCommands {
    /// List all registered browser extensions
    #[command(name = "list")]
    List,

    /// Browse and search available extensions from tuquet-scoop-bucket
    #[command(name = "catalog")]
    Catalog {
        /// Optional keyword to filter extensions
        query: Option<String>,
    },

    /// Download and install an extension package from tuquet-scoop-bucket
    #[command(name = "install")]
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

    /// Unregister a browser extension by ID
    #[command(name = "remove")]
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
    /// List all workflows saved in database and vault
    #[command(name = "list")]
    List {
        /// Filter workflows by keyword (name, ID, or description)
        #[arg(short, long)]
        search: Option<String>,

        /// Only list workflows from SQLite database
        #[arg(long)]
        db_only: bool,

        /// Only list workflows from Vault directory (~/.specter/automa/workflows)
        #[arg(long)]
        vault_only: bool,
    },

    /// Import a workflow file (.json) into SQLite database and vault
    #[command(name = "import")]
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

    /// Inspect details, triggers, parameters, and block sequence of a stored workflow
    #[command(name = "info")]
    Info {
        /// Workflow ID or Name
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,
    },

    /// Delete a workflow from database and vault
    #[command(name = "delete")]
    Delete {
        /// Workflow ID to delete
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,

        /// Also delete from vault directory if present
        #[arg(long, default_value_t = true)]
        vault: bool,
    },
}

