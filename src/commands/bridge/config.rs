use crate::infrastructure::bridge::{BridgeConfig, DiagnosticLevel};
use crate::ui::{badge_online, badge_warn, Card};

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

pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = BridgeConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = BridgeConfig::load().unwrap_or_else(|_| BridgeConfig::default_config());
        println!();
        let mut card = Card::new("BRIDGE CONFIGURATION");
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        if let Some(ref w) = config.workstation {
            card.add_kv("Workstation", &w.name);
            card.add_kv(
                "Default Server",
                w.default_server.as_deref().unwrap_or("none"),
            );
        }
        card.add_kv("Configured VPS", format!("{} server(s)", config.servers.len()));
        card.with_footer(
            "Tip: edit with 'specter bridge config --edit' or validate via 'specter bridge check'",
        );
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}
