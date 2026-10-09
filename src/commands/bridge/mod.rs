pub mod config;
pub mod manage;
pub mod start;
pub mod status;

pub use config::{check_config, manage_config};
pub use manage::{stop_bridge, toggle_server};
pub use start::{start_bridge, BridgeStartOptions};
pub use status::show_status;

use crate::cli::BridgeSubcommands;

pub async fn handle(subcmd: Option<BridgeSubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        Some(BridgeSubcommands::Status) | None => show_status().await?,
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
        Some(BridgeSubcommands::Enable { server }) => toggle_server(&server, true)?,
        Some(BridgeSubcommands::Disable { server }) => toggle_server(&server, false)?,
        Some(BridgeSubcommands::Check) => check_config()?,
        Some(BridgeSubcommands::Config { edit, show }) => manage_config(edit, show)?,
    }
    Ok(())
}
