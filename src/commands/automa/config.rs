pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::config::AutomaConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = crate::config::AutomaConfig::load();
        println!();
        let mut card = crate::ui::Card::new("AUTOMA CONFIGURATION");
        card.with_badge(crate::ui::badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        card.add_kv("Vault Directory", config.resolved_vault_dir().display().to_string());
        card.add_kv("Default Browser", &config.default_browser);
        card.add_kv("Default Headless", if config.default_headless { "true" } else { "false" });
        card.add_kv("Default Timeout", format!("{}s", config.default_timeout_secs));
        card.add_kv("Studio Port", config.studio_port.to_string());
        card.add_kv("Auto Backup", if config.auto_backup { "enabled" } else { "disabled" });
        card.with_footer("Tip: edit with 'specter automa config --edit'");
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}
