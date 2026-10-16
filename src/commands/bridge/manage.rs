use crate::infrastructure::bridge::{BridgeConfig, BridgeSupervisor};
use crate::ui::prompt::{
    clack_cancel, clack_intro, clack_outro, clack_select, clack_step_err, clack_step_ok,
    clack_step_warn, ClackOption,
};

pub fn toggle_server(
    server_opt: Option<&str>,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BridgeConfig::load()?;
    let action_str = if enabled { "enabled" } else { "disabled" };

    clack_intro(
        "spc bridge",
        if enabled {
            "Enable Bridge Server"
        } else {
            "Disable Bridge Server"
        },
    );

    let server_id = match server_opt {
        Some(id) => id.to_string(),
        None => {
            let mut ids: Vec<String> = config.servers.keys().cloned().collect();
            ids.sort();
            if ids.is_empty() {
                clack_step_err("No servers configured in bridge.json");
                return Err("No servers configured in bridge.json".into());
            }
            let options: Vec<ClackOption> = ids
                .iter()
                .map(|id| {
                    let srv = &config.servers[id];
                    let state = if srv.is_enabled() {
                        "enabled"
                    } else {
                        "disabled"
                    };
                    ClackOption::new(
                        id,
                        format!("{} • currently {}", srv.name, state),
                        format!("{} ({})", id, srv.name),
                    )
                })
                .collect();
            let prompt_msg = if enabled {
                "Select a server to ENABLE"
            } else {
                "Select a server to DISABLE"
            };
            match clack_select(prompt_msg, &options, 0, 10)? {
                Some(idx) => ids[idx].clone(),
                None => {
                    clack_cancel("Operation cancelled.");
                    return Ok(());
                }
            }
        }
    };

    if config.set_server_enabled(&server_id, enabled)? {
        clack_step_ok(&format!(
            "Server '{}' has been {} in {}",
            server_id,
            action_str,
            BridgeConfig::config_path().display()
        ));
        clack_outro(&format!("Bridge configuration updated ({} = {}).", server_id, action_str));
    } else {
        clack_step_err(&format!(
            "Server '{}' not found in configuration {}",
            server_id,
            BridgeConfig::config_path().display()
        ));
    }

    Ok(())
}

pub async fn stop_bridge(server_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;
    clack_intro("spc bridge", "Stop Bridge Tunnels & Daemons");

    if let Some(target) = server_opt {
        if target.eq_ignore_ascii_case("all") {
            BridgeSupervisor::stop_all()?;
            clack_step_ok("All bridge processes terminated cleanly.");
            clack_outro("All tunnels offline.");
        } else if let Some(srv) = config.servers.get(target) {
            BridgeSupervisor::stop_server(target, srv)?;
            clack_step_ok(&format!("Bridge for '{}' stopped.", target));
            clack_outro(&format!("Tunnel '{}' offline.", target));
        } else {
            clack_step_warn(&format!("Unknown server '{}'", target));
        }
    } else {
        // Default stop all
        BridgeSupervisor::stop_all()?;
        clack_step_ok("All bridge tunnels and proxy daemons stopped cleanly.");
        clack_outro("All tunnels offline.");
    }

    Ok(())
}
