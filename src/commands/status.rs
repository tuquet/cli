use std::path::PathBuf;
use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{badge_offline, badge_online, badge_warn, Card};

pub async fn show_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();

    // 1. Cloud Authentication & Identity
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let cloud_card = if let Some(ref creds) = cloud_creds {
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_online("ENROLLED (PROD)"));
        card.with_min_width(68);
        card.add_kv("Device ID", &creds.device_id);
        card.add_kv("Device Name", &creds.name);
        card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal Workspace"));
        card.add_kv("Endpoint", creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co"));
        card
    } else {
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_offline("DISCONNECTED"));
        card.with_min_width(68);
        card.add_line("Workstation not enrolled with cloud fleet.");
        card.with_footer("Run 'tuquet login' to authenticate with Tuquet Cloud");
        card
    };

    // 2. Runner Daemon State (Check 127.0.0.1:8765)
    let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| config.server_port.to_string());
    let daemon_url = format!("http://{}:{}", host, port);
    let health_url = format!("{}/api/v1/health", daemon_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()?;

    let daemon_card = match client.get(&health_url).send().await {
        Ok(res) if res.status().is_success() => {
            let mut card = Card::new("RUNNER");
            card.with_badge(badge_online("ONLINE (HTTP 200)"));
            card.with_min_width(68);
            card.add_kv("Endpoint", &daemon_url);
            card.add_kv("Engine", "Chromium MV3 Extension Worker (CDP)");
            card.add_kv("Protocol", "tuquet.automa.v1");
            card
        }
        _ => {
            let mut card = Card::new("RUNNER");
            card.with_badge(badge_offline("OFFLINE"));
            card.with_min_width(68);
            card.add_kv("Endpoint", &daemon_url);
            card.add_kv("Status", "Local worker service is not currently running");
            card.with_footer(format!("Start daemon with 'tuquet runner start --port {}'", port));
            card
        }
    };

    // 3. Browser Runtime State
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let browser_card = if browser_status.installed {
        let mut card = Card::new("BROWSER");
        card.with_badge(badge_online("READY"));
        card.with_min_width(68);
        card.add_kv("Engine", "Chromium (Open Source - BSD-3-Clause)");
        card.add_kv("Revision", &browser_status.pinned_version);
        card.add_kv("Platform", &browser_status.platform);
        card.add_kv("Executable", &browser_status.executable_path);
        if let Some(mb) = browser_status.size_mb {
            card.add_kv("Disk Usage", format!("{:.1} MB", mb));
        }
        card
    } else {
        let mut card = Card::new("BROWSER");
        card.with_badge(badge_warn("NOT INSTALLED"));
        card.with_min_width(68);
        card.add_line("Dedicated Chromium binary not found in ~/.tuquet/runtimes/");
        card.with_footer("Install dedicated runtime with 'tuquet browser install'");
        card
    };

    // 4. Storage & Vault Summary
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string());
    let vault_dir = PathBuf::from(&home).join(".tuquet").join("workflows");
    let mut workflow_count = 0;
    if let Ok(entries) = std::fs::read_dir(&vault_dir) {
        for e in entries.flatten() {
            if e.path().extension().map(|ext| ext == "json").unwrap_or(false) {
                workflow_count += 1;
            }
        }
    }

    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    let db_size_str = if db_path.exists() {
        if let Ok(meta) = std::fs::metadata(&db_path) {
            format!("{:.2} KB", meta.len() as f64 / 1024.0)
        } else {
            "Present".to_string()
        }
    } else {
        "Not initialized".to_string()
    };

    let mut storage_card = Card::new("VAULT");
    storage_card.with_badge(badge_online("ACTIVE"));
    storage_card.with_min_width(68);
    storage_card.add_kv("Canonical Root", format!("{}/.tuquet", home.replace('\\', "/")));
    storage_card.add_kv("Workflow Vault", format!("{} workflows saved in {}", workflow_count, vault_dir.display()));
    storage_card.add_kv("SQLite Database", format!("automa.sqlite ({})", db_size_str));

    println!();
    cloud_card.print();
    println!();
    daemon_card.print();
    println!();
    browser_card.print();
    println!();
    storage_card.print();
    println!();

    Ok(())
}
