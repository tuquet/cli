use crate::infrastructure::bridge::{BridgeConfig, BridgeSupervisor};
use crate::ui::{badge_online, badge_warn};

pub fn toggle_server(server_id: &str, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BridgeConfig::load()?;
    let action_str = if enabled { "enabled" } else { "disabled" };
    let badge = if enabled {
        badge_online("ENABLED")
    } else {
        badge_warn("DISABLED")
    };

    if config.set_server_enabled(server_id, enabled)? {
        println!(
            "{} Server '{}' has been {} in {}",
            badge,
            server_id,
            action_str,
            BridgeConfig::config_path().display()
        );
    } else {
        eprintln!(
            "{} Server '{}' not found in configuration {}",
            badge_warn("NOT FOUND"),
            server_id,
            BridgeConfig::config_path().display()
        );
    }

    Ok(())
}

pub async fn stop_bridge(server_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;

    if let Some(target) = server_opt {
        if target.eq_ignore_ascii_case("all") {
            BridgeSupervisor::stop_all()?;
            println!(
                "{} All bridge processes terminated cleanly.",
                badge_online("STOPPED")
            );
        } else if let Some(srv) = config.servers.get(target) {
            BridgeSupervisor::stop_server(target, srv)?;
            println!("{} Bridge for '{}' stopped.", badge_online("STOPPED"), target);
        } else {
            eprintln!("{} Unknown server '{}'", badge_warn("WARN"), target);
        }
    } else {
        // Default stop all
        BridgeSupervisor::stop_all()?;
        println!(
            "{} All bridge tunnels and proxy daemons stopped cleanly.",
            badge_online("STOPPED")
        );
    }

    Ok(())
}
