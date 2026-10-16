pub mod config;
pub mod manage;
pub mod start;
pub mod status;

pub use config::{check_config, manage_config};
pub use manage::{stop_bridge, toggle_server};
pub use start::{start_bridge, BridgeStartOptions};
pub use status::show_status;

use crate::cli::BridgeSubcommands;
use crate::ui::{
    clack_cancel, clack_intro, clack_outro, clack_select, is_interactive_tty, ClackOption,
    FormatArgs, OutputFormat,
};

pub async fn handle(subcmd: Option<BridgeSubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        Some(BridgeSubcommands::Status { format }) => {
            let fmt = format.resolve();
            if !fmt.is_json() {
                clack_intro("spc bridge status", "Multi-VPS Mesh & Workload Topology");
            }
            show_status(fmt).await?;
            if !fmt.is_json() {
                clack_outro("Bridge mesh topology inspected.");
            }
        }
        None => {
            if is_interactive_tty() {
                run_interactive_bridge().await?;
            } else {
                show_status(OutputFormat::Json).await?;
            }
        }
        Some(BridgeSubcommands::Start {
            server,
            tag,
            http,
            ssh,
            foreground,
        }) => {
            start_bridge(BridgeStartOptions {
                server: server.as_deref(),
                tag: tag.as_deref(),
                http,
                ssh,
                foreground,
            })
            .await?
        }
        Some(BridgeSubcommands::Stop { server }) => stop_bridge(server.as_deref()).await?,
        Some(BridgeSubcommands::Enable { server }) => toggle_server(server.as_deref(), true)?,
        Some(BridgeSubcommands::Disable { server }) => toggle_server(server.as_deref(), false)?,
        Some(BridgeSubcommands::Check) => check_config()?,
        Some(BridgeSubcommands::Config { edit, show, args }) => manage_config(&args, edit, show)?,
    }
    Ok(())
}

async fn run_interactive_bridge() -> Result<(), Box<dyn std::error::Error>> {
    clack_intro("spc bridge", "Network Bridge & Multi-VPS Mesh Controller");

    let opts = vec![
        ClackOption::new(
            "Inspect Bridge Mesh Status & Topology",
            "Probe all configured VPS tunnels, SSH ports & SOCKS5/HTTP listeners",
            "Inspect Bridge Mesh Status",
        ),
        ClackOption::new(
            "Start Enabled Bridge Tunnels",
            "Activate Cloudflare & SSH SOCKS5 proxy tunnels for enabled servers",
            "Start Enabled Bridge Tunnels",
        ),
        ClackOption::new(
            "Stop Active Bridge Tunnels",
            "Gracefully terminate running bridge tunnel daemons",
            "Stop Active Bridge Tunnels",
        ),
        ClackOption::new(
            "Enable a Bridge Server",
            "Interactively enable a server in ~/.specter/bridge/bridge.json",
            "Enable a Bridge Server",
        ),
        ClackOption::new(
            "Disable a Bridge Server",
            "Interactively disable a server in ~/.specter/bridge/bridge.json",
            "Disable a Bridge Server",
        ),
        ClackOption::new(
            "Validate bridge.json Configuration",
            "Check SSH IdentityFile paths, port conflicts & schema",
            "Validate bridge.json Configuration",
        ),
        ClackOption::new(
            "Configure Bridge Settings",
            "Inspect or edit ~/.specter/bridge/bridge.json",
            "Configure Bridge Settings",
        ),
    ];

    let Some(idx) = clack_select("Select a Bridge operation", &opts, 0, 8)? else {
        clack_cancel("Operation cancelled.");
        return Ok(());
    };

    match idx {
        0 => {
            show_status(FormatArgs::default().resolve()).await?;
            clack_outro("Bridge mesh topology inspected.");
        }
        1 => {
            start_bridge(BridgeStartOptions {
                server: None,
                tag: None,
                http: false,
                ssh: false,
                foreground: false,
            })
            .await?;
        }
        2 => stop_bridge(None).await?,
        3 => toggle_server(None, true)?,
        4 => toggle_server(None, false)?,
        5 => check_config()?,
        6 => manage_config(&[], false, false)?,
        _ => {}
    }

    Ok(())
}
