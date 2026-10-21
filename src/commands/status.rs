use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::infrastructure::db::AutomaDb;
use crate::ui::{
    OutputFormat, TabularRow, badge_offline, badge_online, badge_warn,
    create_network_topology_card, create_tabular_card, default_workstation_endpoints, respond_with,
};
use std::path::PathBuf;

pub async fn show_dashboard(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();

    // 1. SYSTEM & CLOUD
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "WORKSTATION".to_string());

    let (system_badge, cloud_target_val, cloud_status) = if let Some(ref creds) = cloud_creds {
        (
            badge_online("ENROLLED (PROD)"),
            creds
                .cloud_url
                .as_deref()
                .unwrap_or(crate::constants::DEFAULT_DEV_SUPABASE_URL),
            badge_online("CONNECTED"),
        )
    } else {
        (
            badge_offline("LOCAL ONLY"),
            "Not paired with Cloud",
            badge_offline("STANDBY"),
        )
    };

    let workstation_val = cloud_creds
        .as_ref()
        .map(|c| c.name.as_str())
        .unwrap_or(&hostname);
    let tenant_val = cloud_creds
        .as_ref()
        .and_then(|c| c.tenant_id.as_deref())
        .unwrap_or("Personal Workspace");

    let cloud_display = cloud_target_val
        .trim_start_matches("https://")
        .trim_end_matches('/');
    let system_rows = vec![
        TabularRow::new(
            "Workstation",
            workstation_val,
            "Local Node Host",
            badge_online("READY"),
        ),
        TabularRow::new(
            "SSOT Storage",
            "~/.tq",
            "Microservices Root",
            badge_online("READY"),
        ),
        TabularRow::new(
            "Cloud Mesh",
            cloud_display,
            "Control Plane Gateway",
            cloud_status,
        ),
        TabularRow::new(
            "Tenant Scope",
            tenant_val,
            "Workspace Organization",
            if cloud_creds.is_some() {
                badge_online("ACTIVE")
            } else {
                badge_offline("STANDBY")
            },
        ),
    ];

    let system_card = create_tabular_card(
        "SYSTEM & MESH",
        Some(system_badge),
        ["COMPONENT", "IDENTITY / TARGET", "ROLE / DETAILS", "STATUS"],
        &system_rows,
        Some(if cloud_creds.is_some() {
            "Workstation enrolled in Cloud Mesh"
        } else {
            "Run 'tq auth login' to authenticate with Cloud"
        }),
        72,
    );

    // 2. AUTOMA (Runner Daemon + SQLite Database + Workflows)
    let host = std::env::var(crate::constants::ENV_AUTOMA_HOST)
        .unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
    let port = std::env::var(crate::constants::ENV_AUTOMA_PORT)
        .unwrap_or_else(|_| config.server_port.to_string());
    let daemon_url = format!("http://{}:{}", host, port);
    let health_url = format!("{}/api/v1/health", daemon_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()?;

    let daemon_running = match client.get(&health_url).send().await {
        Ok(res) => res.status().is_success(),
        _ => false,
    };

    let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMATION_SQLITE);
    let (db_size_str, workflow_count) = if db_path.exists() {
        let size_str = if let Ok(meta) = std::fs::metadata(&db_path) {
            format!("{:.2} KB", meta.len() as f64 / 1024.0)
        } else {
            "Present".to_string()
        };
        let count = if let Ok(db) = AutomaDb::new(&db_path) {
            db.workflows().get_workflows(None, None, None).map(|w| w.len()).unwrap_or(0)
        } else {
            0
        };
        (size_str, count)
    } else {
        ("Not initialized".to_string(), 0)
    };

    let daemon_status = if daemon_running {
        badge_online("ONLINE")
    } else {
        badge_offline("OFFLINE")
    };
    let automa_rows = vec![
        TabularRow::new(
            "Runner RPC",
            format!("127.0.0.1:{}", port),
            "Local Daemon Bridge",
            daemon_status,
        ),
        TabularRow::new(
            "SQLite State",
            format!("{} ({})", crate::constants::FILE_AUTOMATION_SQLITE, db_size_str),
            "Workflow & Job DB",
            if db_path.exists() {
                badge_online("READY")
            } else {
                badge_offline("STANDBY")
            },
        ),
        TabularRow::new(
            "Workflows",
            format!("{} workflow(s)", workflow_count),
            "SQLite Stored Workflows",
            if db_path.exists() {
                badge_online("READY")
            } else {
                badge_offline("STANDBY")
            },
        ),
    ];

    let automa_card = create_tabular_card(
        "AUTOMA & RUNNER",
        Some(if daemon_running {
            badge_online("RUNNING")
        } else {
            badge_offline("STANDBY")
        }),
        [
            "COMPONENT",
            "ENDPOINT / RESOURCE",
            "ROLE / DETAILS",
            "STATUS",
        ],
        &automa_rows,
        Some(if daemon_running {
            "Runner daemon active on local endpoint"
        } else {
            "Start runner daemon with 'tq runner start --port 8765'"
        }),
        72,
    );

    // 3. BROWSER
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let (browser_badge, engine_status, binary_status) = if browser_status.installed {
        (
            badge_online("READY"),
            badge_online("READY"),
            badge_online("READY"),
        )
    } else {
        (
            badge_warn("NOT INSTALLED"),
            badge_warn("MISSING"),
            badge_warn("MISSING"),
        )
    };

    let profile_dir = crate::config::canonical_dir()
        .join("browser")
        .join("profiles");
    let profile_count = std::fs::read_dir(&profile_dir)
        .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).count())
        .unwrap_or(0);

    let footprint_str = browser_status
        .size_mb
        .map(|mb| format!("{:.1} MB Physical Disk", mb))
        .unwrap_or_else(|| "Physical Disk".to_string());

    let version_clean = browser_status
        .pinned_version
        .split_whitespace()
        .next()
        .unwrap_or(&browser_status.pinned_version);

    let ssot_dir = crate::constants::DEFAULT_SSOT_ROOT_DIR;
    let exec_clean = if browser_status.executable_path.contains(ssot_dir) {
        format!(
            "~/{}//{}",
            ssot_dir,
            browser_status
                .executable_path
                .split(ssot_dir)
                .nth(1)
                .unwrap_or("")
                .trim_start_matches(['/', '\\'])
                .replace('\\', "/")
        )
    } else if browser_status.executable_path.contains(".tq") {
        format!(
            "~/.tq{}",
            browser_status
                .executable_path
                .split(".tq")
                .nth(1)
                .unwrap_or("")
                .replace('\\', "/")
        )
    } else {
        browser_status.executable_path.clone()
    };

    let browser_rows = vec![
        TabularRow::new(
            "Engine V8",
            version_clean,
            format!("Chromium Stealth ({})", browser_status.platform),
            engine_status,
        ),
        TabularRow::new("Runtime Exec", exec_clean, footprint_str, binary_status),
        TabularRow::new(
            "Sandbox Fleet",
            format!("{} profile sandbox(es)", profile_count),
            "Isolated Antidetect Paths",
            badge_online("READY"),
        ),
    ];

    let browser_card = create_tabular_card(
        "ANTIDETECT BROWSER",
        Some(browser_badge),
        ["COMPONENT", "VERSION / PATH", "ROLE / DETAILS", "STATUS"],
        &browser_rows,
        Some(if browser_status.installed {
            "Engine ready. Run 'tq browser list' to manage profiles"
        } else {
            "Install dedicated runtime with 'tq browser install'"
        }),
        72,
    );

    // 4. NETWORK BRIDGE
    let bridge_config = crate::infrastructure::bridge::BridgeConfig::load()
        .unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
    let git_port = bridge_config
        .workloads
        .as_ref()
        .and_then(|w| w.git.as_ref())
        .map(|g| g.port)
        .unwrap_or(crate::constants::DEFAULT_SOCKS5_PORT);
    let primary_ssh_port = bridge_config
        .servers
        .get("my-vps")
        .and_then(|s| s.local_ssh_port)
        .unwrap_or(crate::constants::DEFAULT_SSH_TUNNEL_PORT);
    let http_port = bridge_config
        .workloads
        .as_ref()
        .and_then(|w| w.supabase.as_ref())
        .map(|s| s.http_port)
        .unwrap_or(crate::constants::DEFAULT_HTTP_BRIDGE_PORT);

    let (git_active, ssh_active, http_active) = tokio::join!(
        crate::infrastructure::bridge::probe_port_async(git_port),
        crate::infrastructure::bridge::probe_port_async(primary_ssh_port),
        crate::infrastructure::bridge::probe_port_async(http_port),
    );

    let endpoints = default_workstation_endpoints(
        git_port,
        git_active,
        http_port,
        http_active,
        primary_ssh_port,
        ssh_active,
        None,
    );
    let mut bridge_card = create_network_topology_card(
        "NETWORK BRIDGE",
        &endpoints,
        Some("Manage with 'tq bridge status' or 'tq bridge start'"),
    );
    if git_active || ssh_active || http_active {
        bridge_card.with_badge(badge_online("CONNECTED"));
    } else {
        bridge_card.with_badge(badge_offline("DISCONNECTED"));
    }

    let payload = serde_json::json!({
        "ecosystem": "tuquet",
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

    respond_with(format, &payload, |_| {
        crate::ui::prompt::clack_intro("tq status", "Ecosystem Status");
        if let Some(info) = crate::infrastructure::updater::get_cached_update()
            && info.has_update
        {
            crate::ui::prompt::clack_step_warn(format!(
                "Update available: v{} -> v{} (run 'tq upgrade')",
                info.current_version, info.latest_version
            ));
        }

        system_card.print();
        automa_card.print();
        browser_card.print();
        bridge_card.print();
        crate::ui::prompt::clack_outro("Run 'tq tui' for full-screen live dashboard.");
    })
}
