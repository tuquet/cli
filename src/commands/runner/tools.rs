use std::path::Path;

pub fn print_probe_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = serde_json::json!({
        "protocol": crate::constants::PROTOCOL_AUTOMA_V1,
        "name": "automa-runner",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "chromium-extension-worker",
        "status": "ready",
        "capabilities": [
            "browser:chromium",
            "mv3_extension_worker",
            "isolation:profile_sandbox",
            "headless",
            "automation:workflow_graph"
        ],
        "plugin_type": "runner_driver"
    });
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

pub fn export_openapi(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use utoipa::OpenApi;
    let openapi = crate::api::routes::ApiDoc::openapi();
    let json = openapi.to_pretty_json()?;
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(output_path, json)?;
    println!("OpenAPI spec successfully exported to {:?}", output_path);
    Ok(())
}

pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::config::RunnerConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = crate::config::RunnerConfig::load();
        println!();
        let mut card = crate::ui::Card::new("RUNNER CONFIGURATION");
        card.with_badge(crate::ui::badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        card.add_kv("Listen Host", &config.host);
        card.add_kv("Listen Port", config.port.to_string());
        card.add_kv("Log Level", &config.log_level);
        card.add_kv("Heartbeat", format!("{}s", config.heartbeat_interval_secs));
        card.add_kv("Max Concurrency", config.max_concurrent_jobs.to_string());
        card.add_kv("Auto Restart", if config.auto_restart { "enabled" } else { "disabled" });
        card.with_footer("Tip: edit with 'specter runner config --edit'");
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}
