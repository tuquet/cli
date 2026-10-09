use std::path::Path;
use serde_json::json;

use crate::config::canonical_ssot_dir;
use crate::infrastructure::bridge::tools::find_executable;
use crate::ui::{badge_online, colors, Card};

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
    println!(
        "  {GREEN}✓{RESET} Initialized 5 canonical storage pillars under {}",
        root.display(),
        GREEN = colors::GREEN,
        RESET = colors::RESET,
    );

    // ─────────────────────────────────────────────────────────────────────────────
    // 2. Initialize Automa SQLite Database
    // ─────────────────────────────────────────────────────────────────────────────
    let db_path = root
        .join(crate::constants::PILLAR_AUTOMA)
        .join(crate::constants::FILE_AUTOMA_SQLITE);
    match crate::infrastructure::db::AutomaDb::new(&db_path) {
        Ok(_) => {
            println!(
                "  {GREEN}✓{RESET} SQLite database initialized: {}",
                db_path.display(),
                GREEN = colors::GREEN,
                RESET = colors::RESET,
            );
        }
        Err(e) => {
            println!(
                "  {AMBER}⚠{RESET} SQLite database notice: {}",
                e,
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
        println!(
            "  {AMBER}⚠{RESET} Could not create ~/.gemini/config directory: {}",
            e,
            AMBER = colors::AMBER,
            RESET = colors::RESET,
        );
    } else {
        let mut config_val: serde_json::Value = if mcp_config_file.exists() {
            match std::fs::read_to_string(&mcp_config_file) {
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or_else(|_| json!({ "mcpServers": {} }))
                }
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
                println!(
                    "  {GREEN}✓{RESET} Registered native specter MCP stdio server in ~/.gemini/config/mcp_config.json",
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
        println!(
            "  {CYAN}↓{RESET} Downloading & provisioning Antidetect Chromium (Golden LTS v148)...",
            CYAN = colors::CYAN,
            RESET = colors::RESET,
        );
        match crate::core::browser::resolver::download_stealth_runtime(force_browser, Some("148"))
            .await
        {
            Ok(p) => {
                println!(
                    "  {GREEN}✓{RESET} Installed Antidetect Chromium: {}",
                    p,
                    GREEN = colors::GREEN,
                    RESET = colors::RESET,
                );
            }
            Err(e) => {
                println!(
                    "  {AMBER}⚠{RESET} Browser download deferred: {}",
                    e,
                    AMBER = colors::AMBER,
                    RESET = colors::RESET,
                );
            }
        }
    } else {
        println!(
            "  {GREEN}✓{RESET} Antidetect Chromium already provisioned: v{}",
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
    card.add_kv(
        "MCP Agent Protocol",
        "specter stdio server registered (~/.gemini/config/mcp_config.json)",
    );
    let b_status = crate::core::browser::resolver::get_runtime_status();
    card.add_kv(
        "Browser Engine",
        if b_status.installed {
            "Golden LTS v148 (Antidetect Chromium)"
        } else {
            "Pending download (specter browser install)"
        },
    );
    card.with_footer(
        "Next: Run 'specter doctor' to inspect dependencies or 'specter status' for dashboard.",
    );
    card.print();
    println!();

    Ok(())
}
