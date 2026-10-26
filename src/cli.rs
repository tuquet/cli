use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Args, Parser, Subcommand};
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

/// - Autonomous Browser Automation & Distributed Mesh Runtime
#[derive(Parser, Debug)]
#[command(
    name = "tuquet",
    author = "Ecosystem <tuquet@users.noreply.github.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Autonomous Browser Automation & Distributed Mesh Runtime",
    styles = get_styles(),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Authenticate with cloud ecosystem and manage credentials
    #[command(name = "auth")]
    Auth {
        #[command(subcommand)]
        command: Option<CloudSubcommands>,
    },

    /// Distributed worker daemon and cloud execution node
    #[command(name = "worker")]
    Worker {
        #[command(subcommand)]
        command: Option<WorkerSubcommands>,
    },

    /// Manage and execute automation workflows
    #[command(name = "automation")]
    Automation {
        #[command(subcommand)]
        command: Option<AutomationSubcommands>,
    },

    /// Secure network gateway, VPS tunnel mesh & proxy manager
    #[command(name = "gateway")]
    Gateway {
        #[command(subcommand)]
        command: Option<GatewaySubcommands>,
    },

    /// Manage environment variables, local secrets vault, and workspace sync
    #[command(name = "env")]
    Env {
        /// Project to inspect/switch (e.g. admin, bot, cloud)
        project: Option<String>,

        /// Check environment integrity and report missing/empty keys
        #[arg(short = 'c', long)]
        check: bool,

        /// Switch profile (.env.<profile> -> .env)
        #[arg(short = 's', long)]
        switch: Option<String>,

        /// List all discovered projects with .env files
        #[arg(short = 'l', long)]
        list: bool,

        /// Custom root path to scan
        #[arg(short = 'p', long)]
        path: Option<std::path::PathBuf>,

        /// Format: table, json, or card
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// Dedicated isolated antidetect browser runtime & profile sandbox
    #[command(name = "browser")]
    Browser {
        #[command(subcommand)]
        command: Option<BrowserCommands>,
    },

    /// Manage the local encrypted Secret Store (referenced as {{secrets@name}})
    #[command(name = "secret", aliases = ["secrets"])]
    Secret {
        #[command(subcommand)]
        command: Option<SecretSubcommands>,
    },

    /// Run Model Context Protocol (MCP) JSON-RPC 2.0 stdio server for AI agents
    #[command(name = "mcp", hide = true)]
    Mcp,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SecretSubcommands {
    /// Initialise the local Secret Store key (~/.tq/system/.secret_key)
    #[command(name = "init")]
    Init,
    /// Store or update a secret value
    #[command(name = "set")]
    Set {
        /// Secret name (referenced as {{secrets@name}})
        name: String,
        /// Secret value (`@path/to/file` to read a file, `-` to read stdin)
        value: String,
    },
    /// List stored secret names (values are never printed)
    #[command(name = "list", aliases = ["ls"])]
    List {
        /// Format: table or json
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },
    /// Remove a secret
    #[command(name = "rm", aliases = ["remove", "delete"])]
    Rm {
        /// Secret name
        name: String,
    },
    /// Push local secrets to the Cloud secret store (device-authenticated)
    #[command(name = "push", aliases = ["upload"])]
    Push,
    /// Pull secrets from the Cloud onto this device (portable, no re-entry)
    #[command(name = "pull", aliases = ["download"])]
    Pull,
    /// List secrets stored in Cloud (names only)
    #[command(name = "remote", aliases = ["list-remote"])]
    Remote,
    /// Rotate the tenant DEK (re-encrypts values, re-seals for all devices)
    #[command(name = "rotate")]
    Rotate,
    /// Print the Secret Store file path
    #[command(name = "path")]
    Path,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SchemaSubcommands {
    /// List or export all commands declared in the schema manifest
    #[command(name = "commands", aliases = ["cmds"])]
    Commands {
        /// Filter commands by microservice pillar domain (system, automation, browser, gateway, faker, runner, cloud)
        #[arg(short, long)]
        pillar: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Export the JSON Schema definition for a microservice configuration file
    #[command(name = "config")]
    Config {
        /// Service configuration schema to output (gateway, browser, automation, runner, faker, system)
        #[arg(value_name = "SERVICE")]
        service: Option<String>,
    },

    /// Export AI Agent Model Context Protocol (MCP) tool schemas generated from manifest
    #[command(name = "mcp")]
    Mcp,
}

#[derive(Subcommand, Debug, Clone)]
pub enum GatewaySubcommands {
    /// Probe proxy egress connectivity, latency, and WebRTC leak shield
    #[command(name = "probe")]
    Probe {
        /// Target proxy URL to probe (defaults to socks5://127.0.0.1:1080)
        #[arg(value_name = "URL")]
        url: Option<String>,

        /// Probe timeout in seconds
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,

        /// Output results in raw JSON format
        #[arg(short, long)]
        json: bool,
    },
    /// Show health check dashboard of all configured VPSs, ports, and workloads
    #[command(name = "status")]
    Status {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Start a gateway connection for configured servers (default: all enabled)
    #[command(name = "start")]
    Start {
        /// Target server identifier or pattern (e.g. my-vps, vps-*, or omit for all enabled)
        #[arg(value_name = "SERVER")]
        server: Option<String>,

        /// Filter servers by tag (e.g. mmo, git, clean-ip)
        #[arg(short = 't', long)]
        tag: Option<String>,

        /// Also start the pure Rust embedded HTTP-to-SOCKS5 gateway (port 8118)
        #[arg(long)]
        http: bool,

        /// Also start local SSH port forwarding (e.g. port 2222) for direct SSH access
        #[arg(long)]
        ssh: bool,

        /// Run supervisor in foreground (keeps running until Ctrl+C, auto-heals dropped tunnels)
        #[arg(short = 'f', long)]
        foreground: bool,
    },

    /// Stop running gateway tunnels and proxy daemons
    #[command(name = "stop")]
    Stop {
        /// Server identifier to stop (or 'all' to terminate all gateway processes)
        #[arg(value_name = "SERVER")]
        server: Option<String>,
    },

    /// Enable a server in gateway configuration (~/.tq/gateway/gateway.json)
    #[command(name = "enable")]
    Enable {
        /// Server identifier to enable (e.g. my-vps, may-b; omit to select interactively)
        #[arg(value_name = "SERVER")]
        server: Option<String>,
    },

    /// Disable a server in gateway configuration (~/.tq/gateway/gateway.json)
    #[command(name = "disable")]
    Disable {
        /// Server identifier to disable (e.g. my-vps, may-b; omit to select interactively)
        #[arg(value_name = "SERVER")]
        server: Option<String>,
    },

    /// Validate ~/.tq/gateway/gateway.json configuration schema, ports, and workloads
    #[command(name = "check")]
    Check,

    /// Display, inspect, or edit the canonical gateway configuration file
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Open ~/.tq/gateway/gateway.json in your default editor
        #[arg(short, long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short, long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum InboxSubcommands {
    /// Retrieve latest OTP code for a recipient email address
    #[command(name = "otp")]
    Otp {
        /// Target email recipient (e.g. acc1@domain.com)
        #[arg(value_name = "RECIPIENT")]
        recipient: String,

        /// Wait and poll until OTP arrives
        #[arg(short, long)]
        wait: bool,

        /// Maximum wait time in seconds (default: 60)
        #[arg(short, long, default_value_t = 60)]
        timeout: u64,

        /// Output raw OTP code only (ideal for shell pipes & automation)
        #[arg(short, long)]
        raw: bool,

        /// Do not mark OTP as consumed
        #[arg(long)]
        no_consume: bool,

        /// Auto-copy intercepted OTP code to system clipboard
        #[arg(long)]
        copy: bool,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Retrieve latest verification or magic link for a recipient email address
    #[command(name = "link")]
    Link {
        /// Target email recipient (e.g. acc1@domain.com)
        #[arg(value_name = "RECIPIENT")]
        recipient: String,

        /// Optional link type filter (verification, reset_password, magic_link)
        #[arg(short, long)]
        link_type: Option<String>,

        /// Open link automationtically in default browser
        #[arg(short, long)]
        open: bool,

        /// Output raw URL only
        #[arg(short, long)]
        raw: bool,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// List recent emails, OTPs, and verification links
    #[command(name = "list", aliases = ["ls"])]
    List {
        /// Filter by recipient email address
        #[arg(short, long)]
        recipient: Option<String>,

        /// Maximum records to display (default: 20)
        #[arg(short, long, default_value_t = 20)]
        limit: usize,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Start the local catch-all webhook listener daemon
    #[command(name = "start")]
    Start {
        /// Custom listening port (default from inbox.json: 9123)
        #[arg(short, long)]
        port: Option<u16>,

        /// Custom listening host (default: 127.0.0.1)
        #[arg(long)]
        host: Option<String>,

        /// Run in foreground instead of background daemon
        #[arg(short, long)]
        foreground: bool,
    },

    /// Stop the running inbox daemon
    #[command(name = "stop")]
    Stop,

    /// Show status of the inbox subsystem, daemon health, and database metrics
    #[command(name = "status")]
    Status {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Execute one-click acceptance test verifying local storage, cloud discovery, and OTP interception
    #[command(name = "test")]
    Test {
        /// Force simulated local loop test even when online
        #[arg(short, long)]
        simulate: bool,

        /// Verify connectivity to remote cloud endpoint
        #[arg(short, long)]
        remote: bool,

        /// Output results in JSON format
        #[arg(short, long)]
        json: bool,
    },

    /// Generate Cloudflare Email Routing Worker script for 1-click catch-all deployment
    #[command(name = "setup-worker")]
    SetupWorker {
        /// Write generated script to target file path
        #[arg(short, long)]
        output: Option<String>,

        /// Custom webhook secret token
        #[arg(short, long)]
        secret: Option<String>,
    },

    /// Synchronize recent emails and OTPs from Supabase Cloud
    #[command(name = "sync")]
    Sync {
        /// Number of recent emails to pull from cloud (default: 50)
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },

    /// Inspect or edit inbox configuration (~/.tq/inbox/inbox.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Open configuration file in default editor
        #[arg(short, long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short, long)]
        show: bool,
    },
}

#[derive(Args, Debug, Clone)]
pub struct FakerGenerateArgs {
    /// Number of profiles to generate (default: 1)
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// Filter by gender: male, female, or all
    #[arg(short = 'g', long)]
    pub gender: Option<String>,

    /// Filter by nationality: US, VN, JP, or all
    #[arg(long)]
    pub nat: Option<String>,

    /// Avatar style: real (portrait photo) or svg (vector avatar)
    #[arg(long, default_value = "real")]
    pub avatar: Option<String>,

    /// Custom email domain (e.g. flowup.io.vn), overrides faker.json
    #[arg(short = 'd', long)]
    pub domain: Option<String>,

    /// Output format: auto, json, table, card, or csv (default: auto)
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,

    /// Display rich visual identity card (shortcut for -f card)
    #[arg(short = 'c', long)]
    pub card: bool,

    /// Display rounded ANSI tabular overview (shortcut for -f table)
    #[arg(short = 't', long)]
    pub table: bool,

    /// Export to RFC-compliant CSV (shortcut for -f csv)
    #[arg(long)]
    pub csv: bool,

    /// Output results in raw JSON format (shortcut for -f json)
    #[arg(short = 'j', long)]
    pub json: bool,

    /// Optional file path to export output to
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum FakerSubcommands {
    /// Generate synthetic identity profiles with validated national ID and addresses
    #[command(name = "generate", aliases = ["gen", "g"])]
    Generate(FakerGenerateArgs),

    /// Inspect a single detailed persona card with full credentials
    #[command(name = "card", aliases = ["show", "inspect"])]
    Card {
        /// Filter by gender: male, female, or all
        #[arg(short = 'g', long)]
        gender: Option<String>,

        /// Filter by nationality: US, VN, JP, or all
        #[arg(long)]
        nat: Option<String>,

        /// Avatar style: real or svg
        #[arg(long, default_value = "real")]
        avatar: Option<String>,

        /// Custom email domain (e.g. flowup.io.vn), overrides faker.json
        #[arg(short = 'd', long)]
        domain: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// List supported nationalities, ID schemas, and provider metadata
    #[command(name = "nationalities", aliases = ["nats", "locales"])]
    Nationalities {
        /// Display catalog in formatted ANSI table
        #[arg(short = 't', long)]
        table: bool,

        /// Output catalog in raw JSON format
        #[arg(short = 'j', long)]
        json: bool,
    },

    /// Generate isolated national ID with mathematically valid checksum (SSN, CCCD, My Number)
    #[command(name = "id", aliases = ["ssn", "cccd", "national-id"])]
    Id {
        /// Filter by nationality: US, VN, JP (default from faker.json)
        #[arg(long)]
        nat: Option<String>,

        /// Output raw ID string only (ideal for scripts & automation)
        #[arg(short = 'r', long)]
        raw: bool,
    },

    /// Manage synthetic persona configuration (~/.tq/faker/faker.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

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
pub enum AutomationSubcommands {
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

        /// Automationtically ensure Gateway is running and verify WebRTC leak shield before execution
        #[arg(long)]
        safe: bool,
    },

    /// Manage stored workflows in local SQLite database
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

    /// Launch Automation Web Studio in default system browser
    Studio,

    /// Probe Automation manifest capabilities
    Probe,

    /// Display, inspect, or edit Automation configuration (~/.tq/automation/automation.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Open automation.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorkerSubcommands {
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
        /// Daemon server base URL (defaults to http://{AUTOMATION_HOST}:{AUTOMATION_PORT} or http://127.0.0.1:8765)
        #[arg(short, long)]
        url: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
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

    /// Display, inspect, or edit Runner daemon configuration (~/.tq/automation/runner.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

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
    /// Authenticate operator account, resolve multi-tenant workspace, and pair workstation
    Login {
        /// Cloud / Supabase endpoint URL (e.g. https://dswhacsoaxgpfnkaxnhz.supabase.co)
        #[arg(short, long)]
        url: Option<String>,

        /// Operator account email for Supabase GoTrue authentication
        #[arg(short, long)]
        email: Option<String>,

        /// Operator account password
        #[arg(short, long)]
        password: Option<String>,

        /// Request and authenticate via One-Time Passcode (OTP / Magic Code)
        #[arg(long)]
        otp: bool,

        /// Pre-supplied 6-digit One-Time Passcode (OTP)
        #[arg(short, long)]
        code: Option<String>,

        /// JWT access token or organization enrollment token
        #[arg(short, long)]
        token: Option<String>,

        /// Pre-select target workspace by slug or ID
        #[arg(long)]
        tenant: Option<String>,

        /// Custom workstation name (defaults to machine hostname)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional custom Supabase API key (publishable or anon key)
        #[arg(long)]
        api_key: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Log out and disconnect this workstation from Cloud
    Logout,

    /// Show current Cloud authentication and enrollment status
    #[command(name = "whoami", aliases = ["status"])]
    Whoami {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Manage accessible multi-tenant workspaces and active context
    #[command(name = "tenant", aliases = ["workspace", "ws"])]
    Tenant {
        #[command(subcommand)]
        command: Option<TenantSubcommands>,
    },

    /// Display, inspect, or edit System & Cloud configuration (~/.tq/system/system.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Open system.json in default editor
        #[arg(short = 'e', long)]
        edit: bool,

        /// Display structured configuration details and summary card
        #[arg(short = 's', long)]
        show: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum TenantSubcommands {
    /// List all accessible multi-tenant workspaces for the authenticated operator
    #[command(name = "list", aliases = ["ls"])]
    List {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Switch active workspace context for this workstation
    #[command(name = "switch", aliases = ["use", "select"])]
    Switch {
        /// Target workspace slug or UUID (omit to select interactively)
        #[arg(value_name = "TARGET")]
        target: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Display current active workspace details and membership role
    #[command(name = "current", aliases = ["show"])]
    Current {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
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
    Status {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Search and list available engine releases from the curated manifest
    #[command(name = "search", aliases = ["releases"])]
    Search {
        /// Force refreshing remote manifest from GitHub CDN
        #[arg(short, long)]
        remote: bool,
    },

    /// List all locally installed browser runtimes on this machine
    #[command(name = "list", aliases = ["ls"])]
    List {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

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

    /// Manage and configure browser extensions (Automation MV3 and custom extensions)
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

    /// Launch an antidetect browser profile (zero-port stealth & detached by default)
    #[command(name = "launch", aliases = ["start", "open"])]
    Launch {
        /// Target profile ID or target URL if single argument is a URL
        #[arg(value_name = "PROFILE_OR_URL")]
        target: Option<String>,

        /// Optional target URL to navigate to when profile is specified
        #[arg(value_name = "URL")]
        url: Option<String>,

        /// Enable Chrome DevTools Protocol (CDP) for automation (auto-allocates free port if --port is omitted)
        #[arg(long)]
        cdp: bool,

        /// Chrome DevTools Protocol (CDP) port (defaults to 0 for stealth; when --cdp is set without port, picks next free from 9222)
        #[arg(short, long, default_value_t = 0)]
        port: u16,

        /// Run in foreground and keep terminal attached (default is detached background)
        #[arg(short = 'f', long)]
        foreground: bool,

        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,

        /// Run in background without keeping terminal attached (default behavior)
        #[arg(short = 'd', long)]
        detach: bool,

        /// Override proxy server (e.g. socks5://127.0.0.1:1080)
        #[arg(long)]
        proxy: Option<String>,

        /// Automation mode: 'extension' (Zero-port ultra-stealth default) or 'driver' (CDP DevTools gateway)
        #[arg(short, long, default_value = "extension")]
        mode: String,

        /// Shortcut for zero-port extension stealth mode
        #[arg(long)]
        no_cdp: bool,

        /// Force launch even if profile is already running
        #[arg(long)]
        force: bool,

        /// Bypass pre-flight proxy healthcheck and launch immediately
        #[arg(long)]
        skip_proxy_check: bool,
    },

    /// List active running antidetect browser profiles
    #[command(name = "ps", aliases = ["running", "processes", "ls-running"])]
    Ps {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Terminate running browser profile process tree cleanly
    #[command(name = "stop", aliases = ["kill", "close"])]
    Stop {
        /// Target profile ID or name
        #[arg(value_name = "PROFILE")]
        profile: Option<String>,

        /// Stop all running browser profiles
        #[arg(short, long)]
        all: bool,

        /// Force immediate termination
        #[arg(short, long)]
        force: bool,
    },

    /// Live visual verification & stealth presentation (Cloudflare Turnstile, Bézier mouse, smooth scroll)
    #[command(name = "verify", aliases = ["demo", "inspect-live", "live"])]
    Verify {
        /// Target URL to test (defaults to Cloudflare Turnstile managed challenge test)
        #[arg(value_name = "URL")]
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

    /// Display, inspect, or edit dedicated browser configuration (~/.tq/browser/browser.json)
    #[command(name = "config")]
    Config {
        /// Optional option key and value (e.g. 'key', 'key value', or 'set key value')
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

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
    List {
        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

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

        #[command(flatten)]
        format: crate::ui::FormatArgs,
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

    /// Push browser profile archive to Cloudflare R2 via Storage Hub Presigned URL
    #[command(name = "push", aliases = ["upload"])]
    Push {
        /// Profile ID or name
        id: String,

        /// Storage Hub URL (defaults to env STORAGE_URL or http://127.0.0.1:8080)
        #[arg(long)]
        storage_url: Option<String>,

        /// Output result in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Pull browser profile archive from Cloudflare R2 via Storage Hub Presigned URL
    #[command(name = "pull", aliases = ["download"])]
    Pull {
        /// Profile ID or name
        id: String,

        /// Storage Hub URL (defaults to env STORAGE_URL or http://127.0.0.1:8080)
        #[arg(long)]
        storage_url: Option<String>,

        /// Output result in raw JSON format
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

    /// Push browser profile snapshot directly to Cloudflare R2 via Storage Hub Presigned URL
    #[command(name = "push", aliases = ["upload"])]
    Push {
        /// Target Profile ID or name
        id: String,

        /// Storage Hub URL (defaults to env STORAGE_URL or http://127.0.0.1:8080)
        #[arg(long)]
        storage_url: Option<String>,

        /// Output result in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Pull browser profile snapshot directly from Cloudflare R2 via Storage Hub Presigned URL
    #[command(name = "pull", aliases = ["download"])]
    Pull {
        /// Target Profile ID or name
        id: String,

        /// Storage Hub URL (defaults to env STORAGE_URL or http://127.0.0.1:8080)
        #[arg(long)]
        storage_url: Option<String>,

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

    /// Browse and search available extensions from scoop catalog
    #[command(name = "catalog")]
    Catalog {
        /// Optional keyword to filter extensions
        query: Option<String>,
    },

    /// Download and install an extension package from scoop catalog
    #[command(name = "install")]
    Install {
        /// Extension ID from catalog (e.g. 'automation', 'ublock', 'cookie-injector')
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

    /// Enable an extension for automationted browser sessions
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
        /// Extension ID to inspect (defaults to 'automation')
        #[arg(default_value = "automation")]
        id: String,
    },

    /// Print the absolute filesystem path of an extension (defaults to 'automation')
    #[command(name = "path")]
    Path {
        /// Extension ID (defaults to 'automation')
        #[arg(default_value = "automation")]
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
    /// List all workflows stored in the Cloud (automation.workflows)
    #[command(name = "list")]
    List {
        /// Filter workflows by keyword (name, ID, or description)
        #[arg(short, long)]
        search: Option<String>,

        #[command(flatten)]
        format: crate::ui::FormatArgs,
    },

    /// Import a workflow file (.json) into the Cloud (automation.workflows)
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

    /// Export a workflow from the Cloud to a JSON file
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

    /// Delete a workflow from the Cloud (soft delete)
    #[command(name = "delete")]
    Delete {
        /// Workflow ID to delete
        #[arg(value_name = "WORKFLOW_ID")]
        id: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct SetupArgs {
    /// Target sub-component to set up directly (core, mcp, gateway, browser, cloud)
    #[command(subcommand)]
    pub component: Option<SetupComponent>,

    /// Quick non-interactive core setup (SSOT + MCP only, skips optional modules)
    #[arg(short = 'q', long)]
    pub quick: bool,

    /// Force re-provisioning / overwrite existing configurations
    #[arg(short, long)]
    pub force: bool,

    /// Assume yes to all interactive prompts (automationted onboarding)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SetupComponent {
    /// Initialize SSOT directory (~/.tq) and database
    #[command(name = "core")]
    Core,

    /// Auto-wire MCP server into AI agent editors (Antigravity, Cursor, Claude, Hermes)
    #[command(name = "mcp")]
    Mcp,

    /// Configure Specter Gateway VPS tunnel & verify SOCKS5 mesh proxy
    #[command(name = "gateway")]
    Gateway,

    /// Provision Antidetect Chromium browser sandbox
    #[command(name = "browser")]
    Browser,

    /// Pair workstation with TQ Cloud (Supabase)
    #[command(name = "cloud")]
    Cloud,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_debug_assert() {
        Cli::command().debug_assert();
    }
}
