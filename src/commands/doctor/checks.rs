use std::path::Path;

use crate::config::{canonical_specter_dir, AppConfig};
use crate::infrastructure::bridge::probe_port;
use crate::infrastructure::bridge::tools::find_executable;
use crate::ui::{
    badge_error, badge_online, badge_warn, create_network_topology_card,
    default_workstation_endpoints, Card, Column, Table,
};
use super::types::DiagnosticItem;

pub fn check_tools(missing_actions: &mut Vec<String>) -> (Table, bool) {
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
        status_text: if ssh_found {
            "● READY (Built-in)".to_string()
        } else {
            "● MISSING".to_string()
        },
        path_or_detail: ssh_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Not found in PATH or standard system paths".to_string()),
        action_hint: if !ssh_found {
            Some(ssh_hint.to_string())
        } else {
            None
        },
    });

    // Cloudflared (Portable Mesh Tunnel)
    let cf_path = find_executable("cloudflared");
    let cf_found = cf_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "cloudflared".to_string(),
        category: "Bridge Mesh Tunnel",
        ok: true,
        status_text: if cf_found {
            "● READY".to_string()
        } else {
            "○ ON-DEMAND".to_string()
        },
        path_or_detail: cf_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Portable binary fetched when bridge starts".to_string()),
        action_hint: None,
    });

    // Git (Developer Tool - Optional)
    let git_path = find_executable("git");
    let git_found = git_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "git".to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if git_found {
            "● READY".to_string()
        } else {
            "○ OPTIONAL".to_string()
        },
        path_or_detail: git_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Only needed for source contributors".to_string()),
        action_hint: None,
    });

    // Node.js (Developer Tool - Optional)
    let node_path = find_executable("node");
    let node_found = node_path.is_some();
    tool_items.push(DiagnosticItem {
        name: "node".to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if node_found {
            "● READY".to_string()
        } else {
            "○ OPTIONAL".to_string()
        },
        path_or_detail: node_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Only needed for skills authoring".to_string()),
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
        let found = find_executable("brew").is_some()
            || Path::new("/opt/homebrew/bin/brew").exists()
            || Path::new("/usr/local/bin/brew").exists();
        let path = find_executable("brew")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Optional Homebrew on macOS".to_string());
        ("brew (Homebrew)", found, path)
    } else {
        let found = find_executable("apt").is_some()
            || find_executable("dnf").is_some()
            || find_executable("pacman").is_some();
        let path = "System package manager".to_string();
        ("system package manager", found, path)
    };

    tool_items.push(DiagnosticItem {
        name: pkg_name.to_string(),
        category: "Dev Tool (Optional)",
        ok: true,
        status_text: if pkg_found {
            "● INSTALLED".to_string()
        } else {
            "○ OPTIONAL".to_string()
        },
        path_or_detail: pkg_path,
        action_hint: None,
    });

    let mut tool_table = Table::new(vec![
        Column {
            title: "TOOL / COMPONENT".to_string(),
            min_width: 18,
            align_right: false,
        },
        Column {
            title: "CATEGORY".to_string(),
            min_width: 20,
            align_right: false,
        },
        Column {
            title: "STATUS".to_string(),
            min_width: 14,
            align_right: false,
        },
        Column {
            title: "RESOLVED PATH / DETAILS".to_string(),
            min_width: 48,
            align_right: false,
        },
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

    (tool_table, ssh_found)
}

pub fn check_browser(missing_actions: &mut Vec<String>) -> (Card, bool) {
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let installed_runtimes = specter_browser::list_installed_runtimes();
    let active_version = specter_browser::get_active_version();

    let mut browser_card = Card::new("ANTIDETECT BROWSER ENGINE");
    browser_card.with_min_width(68);

    let browser_ok = browser_status.installed;

    if browser_ok {
        browser_card.with_badge(badge_online("OPERATIONAL"));
        browser_card.add_kv(
            "Engine Architecture",
            "Chromium C++ Antidetect (Blink/V8 Native Spoofing)",
        );
        browser_card.add_kv(
            "Active Version",
            format!("v{} (Configured in browser.json)", active_version),
        );
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
        let profiles_dir = canonical_specter_dir()
            .join(crate::constants::PILLAR_BROWSER)
            .join(crate::constants::DIR_PROFILES);
        let profile_count = std::fs::read_dir(&profiles_dir)
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        browser_card.add_kv(
            "Profiles Sandbox",
            format!("{} profile(s) in {}", profile_count, profiles_dir.display()),
        );
        browser_card.with_footer(
            "Manage with 'specter browser list' or 'specter browser use <version>'",
        );
    } else {
        browser_card.with_badge(badge_error("MISSING ENGINE"));
        browser_card.add_line("No Antidetect Chromium runtime installed on this machine.");
        browser_card
            .with_footer("Run 'specter browser install 148' to install Golden LTS release");
        missing_actions.push(
            "Browser: Run 'specter browser install 148' to install C++ Antidetect Chromium"
                .to_string(),
        );
    }

    (browser_card, browser_ok)
}

pub fn check_storage_pillars() -> Table {
    let root = canonical_specter_dir();
    let mut pillar_items = Vec::new();

    let pillars = &[
        (
            crate::constants::PILLAR_SYSTEM,
            "specter/cli (System)",
            "Identity (.identity.json), CLI history, machine ID",
        ),
        (
            crate::constants::PILLAR_AUTOMA,
            "specter/automa (Web Studio)",
            "Workflow DAG definitions, SQLite DB (automa.sqlite)",
        ),
        (
            crate::constants::PILLAR_BROWSER,
            "specter/browser (Sandbox)",
            "Antidetect Chromium runtimes, sandbox profiles",
        ),
        (
            crate::constants::PILLAR_BRIDGE,
            "specter/bridge (Mesh)",
            "Multi-VPS mesh configuration (bridge.json), PID locks",
        ),
        (
            crate::constants::PILLAR_FAKER,
            "specter/faker (Identity)",
            "Synthetic persona schemas, CCCD template data",
        ),
    ];

    for (pillar, service, desc) in pillars {
        let p_path = root.join(pillar);
        let exists = p_path.is_dir();
        pillar_items.push((
            service.to_string(),
            desc.to_string(),
            if exists {
                "● READY".to_string()
            } else {
                "○ INITIALIZING".to_string()
            },
            p_path.display().to_string(),
        ));
    }

    let mut pillar_table = Table::new(vec![
        Column {
            title: "SERVICE (PILLAR)".to_string(),
            min_width: 28,
            align_right: false,
        },
        Column {
            title: "DESCRIPTION".to_string(),
            min_width: 44,
            align_right: false,
        },
        Column {
            title: "STATUS".to_string(),
            min_width: 14,
            align_right: false,
        },
        Column {
            title: "CANONICAL PATH".to_string(),
            min_width: 36,
            align_right: false,
        },
    ]);

    for (domain, desc, status, path) in pillar_items {
        pillar_table.add_row(vec![domain, desc, status, path]);
    }

    pillar_table
}

pub fn check_network_topology() -> Card {
    let app_cfg = AppConfig::load();
    let runner_port = app_cfg.server_port;
    let runner_active = probe_port(runner_port);

    let bridge_cfg = crate::infrastructure::bridge::BridgeConfig::load().unwrap_or_else(|_| {
        crate::infrastructure::bridge::BridgeConfig::default_config()
    });
    let git_port = bridge_cfg
        .workloads
        .as_ref()
        .and_then(|w| w.git.as_ref())
        .map(|g| g.port)
        .unwrap_or(crate::constants::DEFAULT_SOCKS5_PORT);
    let git_active = probe_port(git_port);
    let ssh_port = bridge_cfg
        .servers
        .get("my-vps")
        .and_then(|s| s.local_ssh_port)
        .unwrap_or(crate::constants::DEFAULT_SSH_TUNNEL_PORT);
    let ssh_active = probe_port(ssh_port);
    let http_port = bridge_cfg
        .workloads
        .as_ref()
        .and_then(|w| w.supabase.as_ref())
        .map(|s| s.http_port)
        .unwrap_or(crate::constants::DEFAULT_HTTP_BRIDGE_PORT);
    let http_active = probe_port(http_port);

    let endpoints = default_workstation_endpoints(
        git_port,
        git_active,
        http_port,
        http_active,
        ssh_port,
        ssh_active,
        Some((runner_port, runner_active)),
    );

    create_network_topology_card(
        "NETWORK TOPOLOGY & LISTENERS",
        &endpoints,
        Some("Tunnels run on demand. Start with 'specter bridge start' or 'specter runner start'"),
    )
}

pub fn check_verdict(critical_tools_ok: bool, browser_ok: bool, missing_actions: &[String]) -> Card {
    let mut verdict_card = Card::new("DIAGNOSTIC VERDICT");
    verdict_card.with_min_width(68);

    if critical_tools_ok && browser_ok {
        verdict_card.with_badge(badge_online("ALL SYSTEMS OPERATIONAL"));
        verdict_card.add_line(
            "All required ecosystem dependencies and runtimes are properly provisioned.",
        );
        verdict_card.add_line("Your workstation is 100% ready for autonomous browser workflows, antidetect spoofing, and bridge meshes.");
        verdict_card.with_footer(
            "Tip: Run 'specter' to launch interactive shell, or 'specter status' for dashboard.",
        );
    } else {
        verdict_card.with_badge(badge_warn("ACTION REQUIRED"));
        verdict_card.add_line("Some required components or runtimes are missing:");
        for action in missing_actions {
            verdict_card.add_line(format!("  • {}", action));
        }
        verdict_card.with_footer("Tip: Run 'specter doctor --fix' or 'specter bootstrap' to auto-remediate missing components.");
    }

    verdict_card
}
