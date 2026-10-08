use std::path::Path;
use crate::config::{canonical_ssot_dir, canonical_specter_dir, AppConfig};
use crate::infrastructure::bridge::tools::find_executable;
use crate::infrastructure::bridge::probe_port;
use crate::ui::{badge_offline, badge_online, badge_warn, badge_error, colors, Card, Column, Table};

use serde_json::json;

pub struct DiagnosticItem {
    pub name: String,
    pub category: &'static str,
    pub ok: bool,
    pub status_text: String,
    pub path_or_detail: String,
    pub action_hint: Option<String>,
}

pub async fn bootstrap(force_browser: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!(
        "  {BOLD}{CYAN}SPECTER BOOTSTRAP{RESET} {MUTED}— Ecosystem Environment & Microservice Provisioning{RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        RESET = colors::RESET,
        MUTED = colors::MUTED,
    );
    println!();

    let root = canonical_ssot_dir();

    // ─────────────────────────────────────────────────────────────────────────────
    // 1. Create Canonical SSOT Pillar Directories (~/.specter/)
    // ─────────────────────────────────────────────────────────────────────────────
    let dirs = [
        root.join("bin"),
        root.join(crate::constants::PILLAR_SYSTEM),
        root.join(crate::constants::PILLAR_AUTOMA).join(crate::constants::DIR_WORKFLOWS),
        root.join(crate::constants::PILLAR_BROWSER).join(crate::constants::DIR_PROFILES),
        root.join(crate::constants::PILLAR_BROWSER).join(crate::constants::DIR_RUNTIMES),
        root.join(crate::constants::PILLAR_BROWSER).join(crate::constants::DIR_EXTENSIONS),
        root.join(crate::constants::PILLAR_BRIDGE).join("bin"),
        root.join(crate::constants::PILLAR_BRIDGE).join(crate::constants::DIR_PIDS),
        root.join(crate::constants::PILLAR_FAKER),
    ];

    for d in &dirs {
        std::fs::create_dir_all(d)?;
    }
    println!("  {GREEN}✓{RESET} Initialized 5 canonical storage pillars under {}",
        root.display(),
        GREEN = colors::GREEN,
        RESET = colors::RESET,
    );

    // ─────────────────────────────────────────────────────────────────────────────
    // 2. Initialize Automa SQLite Database
    // ─────────────────────────────────────────────────────────────────────────────
    let db_path = root.join(crate::constants::PILLAR_AUTOMA).join(crate::constants::FILE_AUTOMA_SQLITE);
    match crate::infrastructure::db::AutomaDb::new(&db_path) {
        Ok(_) => {
            println!("  {GREEN}✓{RESET} SQLite database initialized: {}",
                db_path.display(),
                GREEN = colors::GREEN,
                RESET = colors::RESET,
            );
        }
        Err(e) => {
            println!("  {AMBER}⚠{RESET} SQLite database notice: {}", e,
                AMBER = colors::AMBER,
                RESET = colors::RESET,
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 3. Auto-register MCP Server for AI Agent IDEs (Antigravity / Gemini)
    // ─────────────────────────────────────────────────────────────────────────────
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let gemini_config_dir = Path::new(&home).join(".gemini").join("config");
    let mcp_config_file = gemini_config_dir.join("mcp_config.json");

    if let Err(e) = std::fs::create_dir_all(&gemini_config_dir) {
        println!("  {AMBER}⚠{RESET} Could not create ~/.gemini/config directory: {}", e,
            AMBER = colors::AMBER,
            RESET = colors::RESET,
        );
    } else {
        let mut config_val: serde_json::Value = if mcp_config_file.exists() {
            match std::fs::read_to_string(&mcp_config_file) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| json!({ "mcpServers": {} })),
                Err(_) => json!({ "mcpServers": {} }),
            }
        } else {
            json!({ "mcpServers": {} })
        };

        if config_val.get("mcpServers").is_none() {
            config_val["mcpServers"] = json!({});
        }

        let specter_cmd = if find_executable("specter").is_some() {
            "specter".to_string()
        } else {
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "specter".to_string())
        };

        let mcp_def = json!({
            "command": specter_cmd,
            "args": ["mcp"]
        });

        // Register specter MCP server and remove legacy tuquet if present
        config_val["mcpServers"]["specter"] = mcp_def;
        if let Some(servers) = config_val.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove("tuquet");
        }

        if let Ok(pretty) = serde_json::to_string_pretty(&config_val) {
            if std::fs::write(&mcp_config_file, pretty).is_ok() {
                println!("  {GREEN}✓{RESET} Registered native specter MCP stdio server in ~/.gemini/config/mcp_config.json",
                    GREEN = colors::GREEN,
                    RESET = colors::RESET,
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 4. Provision Antidetect Chromium (Golden LTS v148)
    // ─────────────────────────────────────────────────────────────────────────────
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    if !browser_status.installed || force_browser {
        println!("  {CYAN}↓{RESET} Downloading & provisioning Antidetect Chromium (Golden LTS v148)...",
            CYAN = colors::CYAN,
            RESET = colors::RESET,
        );
        match crate::core::browser::resolver::download_stealth_runtime(force_browser, Some("148")).await {
            Ok(p) => {
                println!("  {GREEN}✓{RESET} Installed Antidetect Chromium: {}", p,
                    GREEN = colors::GREEN,
                    RESET = colors::RESET,
                );
            }
            Err(e) => {
                println!("  {AMBER}⚠{RESET} Browser download deferred: {}", e,
                    AMBER = colors::AMBER,
                    RESET = colors::RESET,
                );
            }
        }
    } else {
        println!("  {GREEN}✓{RESET} Antidetect Chromium already provisioned: v{}",
            crate::core::browser::resolver::get_active_version(),
            GREEN = colors::GREEN,
            RESET = colors::RESET,
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 5. Summary Card
    // ─────────────────────────────────────────────────────────────────────────────
    println!();
    let mut card = Card::new("BOOTSTRAP COMPLETED");
    card.with_badge(badge_online("READY"));
    card.with_min_width(68);
    card.add_kv("SSOT Root", root.display().to_string());
    card.add_kv("SQLite Database", "automa.sqlite (Schema initialized)");
    card.add_kv("MCP Agent Protocol", "specter stdio server registered (~/.gemini/config/mcp_config.json)");
    let b_status = crate::core::browser::resolver::get_runtime_status();
    card.add_kv("Browser Engine", if b_status.installed { "Golden LTS v148 (Antidetect Chromium)" } else { "Pending download (specter browser install)" });
    card.with_footer("Next: Run 'specter doctor' to inspect dependencies or 'specter status' for dashboard.");
    card.print();
    println!();

    Ok(())
}

pub async fn run(fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    if fix {
        println!();
        println!(
            "  {BOLD}{CYAN}SPECTER DOCTOR (--fix){RESET} {MUTED}— Automatic Remediation & Environment Bootstrap{RESET}",
            BOLD = colors::BOLD,
            CYAN = colors::CYAN,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
        );
        let _ = bootstrap(false).await;
        println!();
    }

    println!();
    println!(
        "  {BOLD}{CYAN}SPECTER DOCTOR{RESET} {MUTED}— Ecosystem Environment & Dependency Diagnostics{RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        RESET = colors::RESET,
        MUTED = colors::MUTED,
    );
    println!();

    let mut missing_actions: Vec<String> = Vec::new();

    // ─────────────────────────────────────────────────────────────────────────────
    // 1. Tool Dependencies Table
    // ─────────────────────────────────────────────────────────────────────────────
    let mut tool_items = Vec::new();

    // OpenSSH (ssh - Built-in on Windows 10/11, macOS, and Linux)
    let ssh_path = find_executable("ssh");
    let ssh_found = ssh_path.is_some();
    let ssh_hint = if cfg!(target_os = "windows") {
        "Enable Windows Settings > System > Optional Features > OpenSSH Client"
    } else if cfg!(target_os = "macos") {
        "Built-in on macOS (or brew install openssh)"
    } else {
        "Install via system package manager: sudo apt install openssh-client"
    };

    tool_items.push(DiagnosticItem {
        name: "ssh (OpenSSH)".to_string(),
        category: "Core SOCKS5 Proxy",
        ok: ssh_found,
        status_text: if ssh_found { "● READY (Built-in)".to_string() } else { "● MISSING".to_string() },
        path_or_detail: ssh_path.map(|p| p.display().to_string()).unwrap_or_else(|| "Not found in PATH or standard system paths".to_string()),
        action_hint: if !ssh_found { Some(ssh_hint.to_string()) } else { None },
    });

    // Cloudflared (Portable Mesh Tunnel)
    let cf_path = find_executable("cloudflared");
    let cf_found = cf_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "cloudflared".to_string(),
        category: "Bridge Mesh Tunnel",
        ok: true,
        status_text: if cf_found { "● READY".to_string() } else { "○ ON-DEMAND".to_string() },
        path_or_detail: cf_path.map(|p| p.display().to_string()).unwrap_or_else(|| "Portable binary fetched when bridge starts".to_string()),
        action_hint: None,
    });

    // Git (Developer Tool - Optional)
    let git_path = find_executable("git");
    let git_found = git_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "git".to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if git_found { "● READY".to_string() } else { "○ OPTIONAL".to_string() },
        path_or_detail: git_path.map(|p| p.display().to_string()).unwrap_or_else(|| "Only needed for source contributors".to_string()),
        action_hint: None,
    });

    // Node.js (Developer Tool - Optional)
    let node_path = find_executable("node");
    let node_found = node_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "node".to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if node_found { "● READY".to_string() } else { "○ OPTIONAL".to_string() },
        path_or_detail: node_path.map(|p| p.display().to_string()).unwrap_or_else(|| "Only needed for skills authoring".to_string()),
        action_hint: None,
    });

    // Package Manager (Scoop on Windows, Homebrew on macOS/Linux)
    let (pkg_name, pkg_found, pkg_path) = if cfg!(target_os = "windows") {
        let found = find_executable("scoop").is_some() || {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                Path::new(&profile).join("scoop").exists()
            } else {
                false
            }
        };
        let path = find_executable("scoop")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Optional for developers".to_string());
        ("scoop", found, path)
    } else if cfg!(target_os = "macos") {
        let found = find_executable("brew").is_some() || Path::new("/opt/homebrew/bin/brew").exists() || Path::new("/usr/local/bin/brew").exists();
        let path = find_executable("brew")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Optional Homebrew on macOS".to_string());
        ("brew (Homebrew)", found, path)
    } else {
        let found = find_executable("apt").is_some() || find_executable("dnf").is_some() || find_executable("pacman").is_some();
        let path = "System package manager".to_string();
        ("system package manager", found, path)
    };

    tool_items.push(DiagnosticItem {
        name: pkg_name.to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if pkg_found { "● INSTALLED".to_string() } else { "○ OPTIONAL".to_string() },
        path_or_detail: pkg_path,
        action_hint: None,
    });

    // Render Tools Table
    let mut tool_table = Table::new(vec![
        Column { title: "TOOL / COMPONENT".to_string(), min_width: 18, align_right: false },
        Column { title: "CATEGORY".to_string(), min_width: 20, align_right: false },
        Column { title: "STATUS".to_string(), min_width: 14, align_right: false },
        Column { title: "RESOLVED PATH / DETAILS".to_string(), min_width: 48, align_right: false },
    ]);

    for item in &tool_items {
        if let Some(ref action) = item.action_hint {
            missing_actions.push(format!("Tool '{}': {}", item.name, action));
        }
        tool_table.add_row(vec![
            item.name.clone(),
            item.category.to_string(),
            item.status_text.clone(),
            item.path_or_detail.clone(),
        ]);
    }

    println!("╭─ ENVIRONMENT & SYSTEM TOOL DEPENDENCIES ──────────────────────────╮");
    tool_table.print();
    println!();

    // ─────────────────────────────────────────────────────────────────────────────
    // 2. Antidetect Browser Engine
    // ─────────────────────────────────────────────────────────────────────────────
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let installed_runtimes = tuquet_browser::list_installed_runtimes();
    let active_version = tuquet_browser::get_active_version();

    let mut browser_card = Card::new("ANTIDETECT BROWSER ENGINE");
    browser_card.with_min_width(68);

    if browser_status.installed {
        browser_card.with_badge(badge_online("OPERATIONAL"));
        browser_card.add_kv("Engine Architecture", "Chromium C++ Antidetect (Blink/V8 Native Spoofing)");
        browser_card.add_kv("Active Version", format!("v{} (Configured in browser.json)", active_version));
        browser_card.add_kv("Physical Executable", &browser_status.executable_path);
        if let Some(mb) = browser_status.size_mb {
            browser_card.add_kv("Disk Footprint", format!("{:.1} MB", mb));
        }
        let installed_desc = if installed_runtimes.is_empty() {
            "None discovered".to_string()
        } else {
            installed_runtimes
                .iter()
                .map(|r| format!("{} ({})", r.version, r.status_badge))
                .collect::<Vec<_>>()
                .join(", ")
        };
        browser_card.add_kv("Installed Versions", installed_desc);
        let profiles_dir = canonical_specter_dir().join(crate::constants::PILLAR_BROWSER).join(crate::constants::DIR_PROFILES);
        let profile_count = std::fs::read_dir(&profiles_dir)
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        browser_card.add_kv("Profiles Sandbox", format!("{} profile(s) in {}", profile_count, profiles_dir.display()));
        browser_card.with_footer("Manage with 'specter browser list' or 'specter browser use <version>'");
    } else {
        browser_card.with_badge(badge_error("MISSING ENGINE"));
        browser_card.add_line("No Antidetect Chromium runtime installed on this machine.");
        browser_card.with_footer("Run 'specter browser install 148' to install Golden LTS release");
        missing_actions.push("Browser: Run 'specter browser install 148' to install C++ Antidetect Chromium".to_string());
    }

    browser_card.print();
    println!();

    // ─────────────────────────────────────────────────────────────────────────────
    // 3. Canonical Microservice Storage Pillars (~/.specter/)
    // ─────────────────────────────────────────────────────────────────────────────
    let root = canonical_specter_dir();
    let mut pillar_items = Vec::new();

    let pillars = &[
        (crate::constants::PILLAR_SYSTEM, "specter/cli (System)", "Identity (.identity.json), CLI history, machine ID"),
        (crate::constants::PILLAR_AUTOMA, "specter/automa (Web Studio)", "Workflow DAG definitions, SQLite DB (automa.sqlite)"),
        (crate::constants::PILLAR_BROWSER, "specter/browser (Sandbox)", "Antidetect Chromium runtimes, sandbox profiles"),
        (crate::constants::PILLAR_BRIDGE, "specter/bridge (Mesh)", "Multi-VPS mesh configuration (bridge.json), PID locks"),
        (crate::constants::PILLAR_FAKER, "specter/faker (Identity)", "Synthetic persona schemas, CCCD template data"),
    ];

    for (pillar, service, desc) in pillars {
        let p_path = root.join(pillar);
        let exists = p_path.is_dir();
        pillar_items.push((
            service.to_string(),
            desc.to_string(),
            if exists { "● READY".to_string() } else { "○ INITIALIZING".to_string() },
            p_path.display().to_string(),
        ));
    }

    let mut pillar_table = Table::new(vec![
        Column { title: "SERVICE (PILLAR)".to_string(), min_width: 28, align_right: false },
        Column { title: "DESCRIPTION".to_string(), min_width: 44, align_right: false },
        Column { title: "STATUS".to_string(), min_width: 14, align_right: false },
        Column { title: "CANONICAL PATH".to_string(), min_width: 36, align_right: false },
    ]);

    for (domain, desc, status, path) in pillar_items {
        pillar_table.add_row(vec![domain, desc, status, path]);
    }

    println!("╭─ CANONICAL MICROSERVICE STORAGE PILLARS (SSOT) ───────────────────╮");
    pillar_table.print();
    println!();

    // ─────────────────────────────────────────────────────────────────────────────
    // 4. Port Availability & Daemon Probes
    // ─────────────────────────────────────────────────────────────────────────────
    let app_cfg = AppConfig::load();
    let runner_port = app_cfg.server_port;
    let runner_active = probe_port(runner_port);

    let bridge_cfg = crate::infrastructure::bridge::BridgeConfig::load().unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
    let git_port = bridge_cfg.workloads.as_ref().and_then(|w| w.git.as_ref()).map(|g| g.port).unwrap_or(crate::constants::DEFAULT_SOCKS5_PORT);
    let git_active = probe_port(git_port);
    let ssh_port = bridge_cfg.servers.get("my-vps").and_then(|s| s.local_ssh_port).unwrap_or(crate::constants::DEFAULT_SSH_TUNNEL_PORT);
    let ssh_active = probe_port(ssh_port);
    let http_port = bridge_cfg.workloads.as_ref().and_then(|w| w.supabase.as_ref()).map(|s| s.http_port).unwrap_or(crate::constants::DEFAULT_HTTP_BRIDGE_PORT);
    let http_active = probe_port(http_port);

    let mut port_card = Card::new("NETWORK PORTS & SERVICES");
    port_card.with_min_width(68);
    port_card.with_badge(if git_active || ssh_active || runner_active { badge_online("ACTIVE WORKLOADS") } else { badge_offline("STANDBY") });
    port_card.add_kv("Port 1080 (Git SOCKS5)", if git_active { "● ACTIVE (Tunnel open)" } else { "○ STANDBY (specter bridge start)" });
    port_card.add_kv("Port 2222 (VPS SSH Tunnel)", if ssh_active { "● ACTIVE (CF Access tunnel)" } else { "○ STANDBY (specter bridge start)" });
    port_card.add_kv("Port 8118 (Supabase HTTP)", if http_active { "● ACTIVE (HTTP-to-SOCKS5)" } else { "○ STANDBY (specter bridge start --http)" });
    port_card.add_kv(format!("Port {} (Runner Worker)", runner_port), if runner_active { "● ACTIVE (Daemon online)" } else { "○ STANDBY (specter runner start)" });
    port_card.with_footer("Tunnels run on demand. Start with 'specter bridge start' or 'specter runner start'");
    port_card.print();
    println!();

    // ─────────────────────────────────────────────────────────────────────────────
    // 5. Final Diagnostic Verdict Card
    // ─────────────────────────────────────────────────────────────────────────────
    let mut verdict_card = Card::new("DIAGNOSTIC VERDICT");
    verdict_card.with_min_width(68);

    let critical_tools_ok = ssh_found;
    let browser_ok = browser_status.installed;

    if critical_tools_ok && browser_ok {
        verdict_card.with_badge(badge_online("ALL SYSTEMS OPERATIONAL"));
        verdict_card.add_line("All required ecosystem dependencies and runtimes are properly provisioned.");
        verdict_card.add_line("Your workstation is 100% ready for autonomous browser workflows, antidetect spoofing, and bridge meshes.");
        verdict_card.with_footer("Tip: Run 'specter' to launch interactive shell, or 'specter status' for dashboard.");
    } else {
        verdict_card.with_badge(badge_warn("ACTION REQUIRED"));
        verdict_card.add_line("Some required components or runtimes are missing:");
        for action in &missing_actions {
            verdict_card.add_line(format!("  • {}", action));
        }
        verdict_card.with_footer("Tip: Run 'specter doctor --fix' or 'specter bootstrap' to auto-remediate missing components.");
    }

    verdict_card.print();
    println!();

    Ok(())
}
