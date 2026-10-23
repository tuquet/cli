use serde_json::json;
use std::path::{Path, PathBuf};
use crate::infrastructure::gateway::tools::find_executable;
use crate::ui::prompt::{ClackOption, clack_multiselect, clack_step_ok, is_interactive_tty};

pub struct AgentTarget {
    pub id: &'static str,
    pub name: &'static str,
    pub config_path: PathBuf,
    pub detected: bool,
}

pub fn get_available_targets() -> Vec<AgentTarget> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let home_path = Path::new(&home);

    let mut targets = Vec::new();

    // 1. Antigravity / Gemini CLI
    let gemini_dir = home_path.join(".gemini");
    let gemini_config = gemini_dir.join("config").join("mcp_config.json");
    let gemini_detected = gemini_dir.exists();
    targets.push(AgentTarget {
        id: "gemini",
        name: "Antigravity & Gemini CLI (~/.gemini/config/mcp_config.json)",
        config_path: gemini_config,
        detected: gemini_detected,
    });

    // 2. Cursor
    let cursor_dir = home_path.join(".cursor");
    let cursor_config = cursor_dir.join("mcp.json");
    let cursor_detected = cursor_dir.exists();
    targets.push(AgentTarget {
        id: "cursor",
        name: "Cursor AI Editor (~/.cursor/mcp.json)",
        config_path: cursor_config,
        detected: cursor_detected,
    });

    // 3. Claude Desktop
    #[cfg(windows)]
    let claude_config = {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(appdata).join("Claude").join("claude_desktop_config.json")
    };
    #[cfg(target_os = "macos")]
    let claude_config = home_path
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let claude_config = home_path.join(".config").join("Claude").join("claude_desktop_config.json");

    let claude_detected = claude_config.parent().map(|p| p.exists()).unwrap_or(false);
    targets.push(AgentTarget {
        id: "claude",
        name: "Claude Desktop (claude_desktop_config.json)",
        config_path: claude_config,
        detected: claude_detected,
    });

    // 4. Hermes Agent
    let hermes_dir = home_path.join(".hermes");
    let hermes_config = hermes_dir.join("mcp.json");
    let hermes_detected = hermes_dir.exists();
    targets.push(AgentTarget {
        id: "hermes",
        name: "Hermes Agent (~/.hermes/mcp.json)",
        config_path: hermes_config,
        detected: hermes_detected,
    });

    targets
}

fn inject_mcp_config(config_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut config_val: serde_json::Value = if config_file.exists() {
        match std::fs::read_to_string(config_file) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| json!({ "mcpServers": {} })),
            Err(_) => json!({ "mcpServers": {} }),
        }
    } else {
        json!({ "mcpServers": {} })
    };

    if config_val.get("mcpServers").is_none() {
        config_val["mcpServers"] = json!({});
    }

    let cmd = if find_executable("tq").is_some() {
        "tq".to_string()
    } else {
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "tq".to_string())
    };

    let mcp_def = json!({
        "command": cmd,
        "args": ["mcp"]
    });

    // Set tuquet and remove old specter key if present
    config_val["mcpServers"]["tuquet"] = mcp_def.clone();
    if let Some(servers) = config_val.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove("specter");
    }

    let pretty = serde_json::to_string_pretty(&config_val)?;
    std::fs::write(config_file, pretty)?;
    Ok(())
}

pub fn run(interactive: bool) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let targets = get_available_targets();
    let mut selected_indices = Vec::new();

    if interactive && is_interactive_tty() {
        let options: Vec<ClackOption> = targets
            .iter()
            .map(|t| {
                let hint = if t.detected { "detected" } else { "not found" };
                ClackOption::new(t.name, hint, t.id)
            })
            .collect();

        let default_indices: Vec<usize> = targets
            .iter()
            .enumerate()
            .filter(|(_, t)| t.detected)
            .map(|(i, _)| i)
            .collect();

        let chosen = clack_multiselect(
            "Select AI Agent editors to wire with TQ MCP tools:",
            &options,
            &default_indices,
            8,
        )?;

        if let Some(indices) = chosen {
            selected_indices = indices;
        }
    } else {
        // Non-interactive / quick mode: select all detected targets (or at least gemini)
        for (i, t) in targets.iter().enumerate() {
            if t.detected || t.id == "gemini" {
                selected_indices.push(i);
            }
        }
    }

    let mut configured = Vec::new();
    for &idx in &selected_indices {
        if let Some(target) = targets.get(idx) {
            match inject_mcp_config(&target.config_path) {
                Ok(_) => {
                    clack_step_ok(format!("Wired MCP server for {}", target.name));
                    configured.push(target.id.to_string());
                }
                Err(e) => {
                    eprintln!("  Notice: Failed to write MCP config for {}: {}", target.name, e);
                }
            }
        }
    }

    Ok(configured)
}
