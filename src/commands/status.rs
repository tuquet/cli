use std::path::PathBuf;
use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{badge_offline, badge_online, badge_warn, Card};

pub async fn show_dashboard(json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let canonical_root = crate::config::canonical_specter_dir().display().to_string();

    // 1. SYSTEM & CLOUD
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "WORKSTATION".to_string());
    
    let mut system_card = Card::new("SYSTEM & CLOUD");
    if let Some(ref creds) = cloud_creds {
        system_card.with_badge(badge_online("ENROLLED (PROD)"));
        system_card.with_min_width(68);
        system_card.add_kv("Workstation", &creds.name);
        system_card.add_kv("Canonical Root", &canonical_root);
        system_card.add_kv("Device ID", &creds.device_id);
        system_card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal Workspace"));
        system_card.add_kv("Cloud Target", creds.cloud_url.as_deref().unwrap_or(crate::constants::DEFAULT_DEV_SUPABASE_URL));
    } else {
        system_card.with_badge(badge_offline("LOCAL ONLY"));
        system_card.with_min_width(68);
        system_card.add_kv("Workstation", &hostname);
        system_card.add_kv("Canonical Root", &canonical_root);
        system_card.add_kv("Cloud Target", "Not paired with Specter Cloud");
        system_card.with_footer("Run 'specter login' to authenticate with Specter Cloud");
    }

    // 2. AUTOMA (Runner Daemon + SQLite Database + Workflows)
    let host = std::env::var(crate::constants::ENV_AUTOMA_HOST).unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
    let port = std::env::var(crate::constants::ENV_AUTOMA_PORT).unwrap_or_else(|_| config.server_port.to_string());
    let daemon_url = format!("http://{}:{}", host, port);
    let health_url = format!("{}/api/v1/health", daemon_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()?;

    let daemon_running = match client.get(&health_url).send().await {
        Ok(res) => res.status().is_success(),
        _ => false,
    };

    let vault_dir = crate::config::AutomaConfig::load().resolved_vault_dir();
    let workflow_count = std::fs::read_dir(&vault_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .count()
        })
        .unwrap_or(0);

    let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMA_SQLITE);
    let db_size_str = if db_path.exists() {
        if let Ok(meta) = std::fs::metadata(&db_path) {
            format!("{:.2} KB", meta.len() as f64 / 1024.0)
        } else {
            "Present".to_string()
        }
    } else {
        "Not initialized".to_string()
    };

    let mut automa_card = Card::new("AUTOMA");
    if daemon_running {
        automa_card.with_badge(badge_online("RUNNING"));
    } else {
        automa_card.with_badge(badge_online("READY"));
    }
    automa_card.with_min_width(68);
    automa_card.add_kv(
        "Worker Daemon",
        if daemon_running {
            format!("{} (● ONLINE)", daemon_url)
        } else {
            format!("{} (○ OFFLINE)", daemon_url)
        },
    );
    automa_card.add_kv("Database", format!("automa.sqlite ({})", db_size_str));
    automa_card.add_kv("Workflows", format!("{} workflows in {}", workflow_count, vault_dir.display()));
    if !daemon_running {
        automa_card.with_footer(format!("Start worker daemon with 'specter runner start --port {}'", port));
    }

    // 3. BROWSER
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let browser_card = if browser_status.installed {
        let mut card = Card::new("BROWSER");
        card.with_badge(badge_online("READY"));
        card.with_min_width(68);
        card.add_kv("Engine", "Chromium C++ Antidetect Engine (Blink/V8 Native Spoofing)");
        card.add_kv("Active Version", &browser_status.pinned_version);
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
        card.add_line("Dedicated Chromium binary not found in ~/.specter/browser/runtimes/");
        card.with_footer("Install dedicated runtime with 'specter browser install'");
        card
    };

    // 4. NETWORK BRIDGE
    let bridge_config = crate::infrastructure::bridge::BridgeConfig::load().unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
    let git_port = bridge_config.workloads.as_ref().and_then(|w| w.git.as_ref()).map(|g| g.port).unwrap_or(crate::constants::DEFAULT_SOCKS5_PORT);
    let git_active = crate::infrastructure::bridge::probe_port(git_port);
    let primary_ssh_port = bridge_config.servers.get("my-vps").and_then(|s| s.local_ssh_port).unwrap_or(crate::constants::DEFAULT_SSH_TUNNEL_PORT);
    let ssh_active = crate::infrastructure::bridge::probe_port(primary_ssh_port);
    let http_port = bridge_config.workloads.as_ref().and_then(|w| w.supabase.as_ref()).map(|s| s.http_port).unwrap_or(crate::constants::DEFAULT_HTTP_BRIDGE_PORT);
    let http_active = crate::infrastructure::bridge::probe_port(http_port);

    let mut bridge_card = Card::new("NETWORK BRIDGE");
    if git_active || ssh_active || http_active {
        bridge_card.with_badge(badge_online("CONNECTED"));
    } else {
        bridge_card.with_badge(badge_offline("DISCONNECTED"));
    }
    bridge_card.with_min_width(68);
    bridge_card.add_kv("Config Schema", crate::infrastructure::bridge::BridgeConfig::config_path().to_string_lossy().to_string());
    bridge_card.add_kv("Git SOCKS5 (1080)", if git_active { "● ONLINE (Git push ready)" } else { "○ OFFLINE" });
    bridge_card.add_kv("SSH VPS Tunnel (2222)", if ssh_active { "● ONLINE (CF Access active)" } else { "○ OFFLINE" });
    bridge_card.add_kv("Supabase HTTP (8118)", if http_active { "● ONLINE (Pure Rust Adapter)" } else { "○ OFFLINE" });
    bridge_card.with_footer("Manage with 'specter bridge status' or 'specter bridge start'");

    if json_output {
        let out = serde_json::json!({
            "ecosystem": "specter",
            "cloud": {
                "enrolled": cloud_creds.is_some(),
                "device_id": cloud_creds.as_ref().map(|c| c.device_id.as_str()),
                "device_name": cloud_creds.as_ref().map(|c| c.name.as_str()),
                "tenant_id": cloud_creds.as_ref().and_then(|c| c.tenant_id.as_deref()),
                "endpoint": cloud_creds.as_ref().and_then(|c| c.cloud_url.as_deref())
            },
            "runner": {
                "online": daemon_running,
                "endpoint": daemon_url,
                "port": port
            },
            "automa": {
                "workflows_count": workflow_count,
                "database": db_size_str
            },
            "browser": {
                "installed": browser_status.installed,
                "platform": browser_status.platform,
                "executable_path": browser_status.executable_path,
                "pinned_version": browser_status.pinned_version,
                "size_mb": browser_status.size_mb
            },
            "bridge": {
                "git_1080": git_active,
                "ssh_2222": ssh_active,
                "http_8118": http_active
            }
        });
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }

    if let Some(info) = crate::infrastructure::updater::get_cached_update()
        && info.has_update
    {
        println!();
        println!("{}", crate::ui::render_update_banner(&info.current_version, &info.latest_version));
    }

    println!();
    system_card.print();
    println!();
    automa_card.print();
    println!();
    browser_card.print();
    println!();
    bridge_card.print();
    println!();

    Ok(())
}
