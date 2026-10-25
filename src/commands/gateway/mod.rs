pub mod probe;
pub use probe::run_probe;
pub mod config;
pub mod manage;
pub mod start;
pub mod status;

pub use config::{check_config, manage_config};
pub use manage::{stop_gateway, toggle_server};
pub use start::{GatewayStartOptions, start_gateway};
pub use status::show_status;

use crate::cli::GatewaySubcommands;
use crate::ui::{
    ClackOption, FormatArgs, OutputFormat, clack_cancel, clack_intro, clack_outro, clack_select,
    is_interactive_tty,
};

pub async fn handle(subcmd: Option<GatewaySubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        Some(GatewaySubcommands::Probe { url, timeout, json }) => {
            run_probe(url, timeout, json).await?
        }
        Some(GatewaySubcommands::Status { format }) => {
            let fmt = format.resolve();
            if !fmt.is_json() {
                clack_intro("tq gateway status", "Multi-VPS Mesh & Workload Topology");
            }
            show_status(fmt).await?;
            if !fmt.is_json() {
                clack_outro("Gateway mesh topology inspected.");
            }
        }
        None => {
            if is_interactive_tty() {
                run_interactive_gateway().await?;
            } else {
                show_status(OutputFormat::Json).await?;
            }
        }
        Some(GatewaySubcommands::Start {
            server,
            tag,
            http,
            ssh,
            foreground,
        }) => {
            start_gateway(GatewayStartOptions {
                server: server.as_deref(),
                tag: tag.as_deref(),
                http,
                ssh,
                foreground,
            })
            .await?
        }
        Some(GatewaySubcommands::Stop { server }) => stop_gateway(server.as_deref()).await?,
        Some(GatewaySubcommands::Enable { server }) => toggle_server(server.as_deref(), true)?,
        Some(GatewaySubcommands::Disable { server }) => toggle_server(server.as_deref(), false)?,
        Some(GatewaySubcommands::Check) => check_config()?,
        Some(GatewaySubcommands::Config { edit, show, args }) => manage_config(&args, edit, show)?,
    }
    Ok(())
}

async fn run_interactive_gateway() -> Result<(), Box<dyn std::error::Error>> {
    clack_intro("tq gateway", "Network Gateway & Multi-VPS Mesh Controller");

    let opts = vec![
        ClackOption::new(
            "Inspect Gateway Mesh Status & Topology",
            "Probe all configured VPS tunnels, SSH ports & SOCKS5/HTTP listeners",
            "Inspect Gateway Mesh Status",
        ),
        ClackOption::new(
            "Start Enabled Gateway Tunnels",
            "Activate Cloudflare & SSH SOCKS5 proxy tunnels for enabled servers",
            "Start Enabled Gateway Tunnels",
        ),
        ClackOption::new(
            "Stop Active Gateway Tunnels",
            "Gracefully terminate running gateway tunnel daemons",
            "Stop Active Gateway Tunnels",
        ),
        ClackOption::new(
            "Enable a Gateway Server",
            "Interactively enable a server in ~/.tq/gateway/gateway.json",
            "Enable a Gateway Server",
        ),
        ClackOption::new(
            "Disable a Gateway Server",
            "Interactively disable a server in ~/.tq/gateway/gateway.json",
            "Disable a Gateway Server",
        ),
        ClackOption::new(
            "Validate gateway.json Configuration",
            "Check SSH IdentityFile paths, port conflicts & schema",
            "Validate gateway.json Configuration",
        ),
        ClackOption::new(
            "Configure Gateway Settings",
            "Inspect or edit ~/.tq/gateway/gateway.json",
            "Configure Gateway Settings",
        ),
    ];

    let Some(idx) = clack_select("Select a Gateway operation", &opts, 0, 8)? else {
        clack_cancel("Operation cancelled.");
        return Ok(());
    };

    match idx {
        0 => {
            show_status(FormatArgs::default().resolve()).await?;
            clack_outro("Gateway mesh topology inspected.");
        }
        1 => {
            start_gateway(GatewayStartOptions {
                server: None,
                tag: None,
                http: false,
                ssh: false,
                foreground: false,
            })
            .await?;
        }
        2 => stop_gateway(None).await?,
        3 => toggle_server(None, true)?,
        4 => toggle_server(None, false)?,
        5 => check_config()?,
        6 => manage_config(&[], false, false)?,
        _ => {}
    }

    Ok(())
}
