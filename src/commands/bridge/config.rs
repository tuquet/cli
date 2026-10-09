use crate::infrastructure::bridge::{BridgeConfig, DiagnosticLevel};
use crate::ui::{badge_online, badge_warn};

pub fn check_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = BridgeConfig::config_path();
    println!("Checking configuration at {}...", config_path.display());

    let config = BridgeConfig::load()?;
    let diagnostics = config.validate();

    if diagnostics.is_empty() {
        println!(
            "{} Configuration is 100% valid! Zero conflicts or missing keys found.",
            badge_online("VALID")
        );
        println!(
            "  Registered servers: {}",
            config
                .servers
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    } else {
        println!(
            "{} Found {} issue(s):",
            badge_warn("WARNING"),
            diagnostics.len()
        );
        for d in diagnostics {
            let badge = match d.level {
                DiagnosticLevel::Error => badge_warn("ERROR"),
                DiagnosticLevel::Warning => badge_warn("WARN"),
                DiagnosticLevel::Info => badge_online("INFO"),
            };
            println!("  {} {}", badge, d.message);
        }
    }

    Ok(())
}

pub fn manage_config(args: &[String], edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("bridge", args, edit, show)
}
