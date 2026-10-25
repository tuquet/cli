use crate::ui::prompt::{
    ClackOption, clack_select, clack_step_ok, clack_step_warn, clack_text, is_interactive_tty,
};
use std::path::{Path, PathBuf};

pub fn list_ssh_keys() -> Vec<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let ssh_dir = Path::new(&home).join(".ssh");

    let mut keys = Vec::new();
    if let Ok(entries) = std::fs::read_dir(ssh_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.ends_with(".pub")
                    && !name.starts_with("known_hosts")
                    && name != "config"
                    && name != "authorized_keys"
                {
                    keys.push(path);
                }
            }
        }
    }
    keys
}

pub async fn run(interactive: bool) -> Result<(), Box<dyn std::error::Error>> {
    let keys = list_ssh_keys();
    let mut selected_key_path = String::new();

    if keys.is_empty() {
        clack_step_warn(
            "No SSH private keys found in ~/.ssh/. You may generate one using 'ssh-keygen'.",
        );
    } else if interactive && is_interactive_tty() {
        let options: Vec<ClackOption> = keys
            .iter()
            .map(|k| {
                let fname = k.file_name().and_then(|n| n.to_str()).unwrap_or("key");
                ClackOption::new(fname, k.display().to_string(), fname)
            })
            .collect();

        // Default to id_ed25519 if present
        let default_idx = keys
            .iter()
            .position(|k| k.ends_with("id_ed25519"))
            .unwrap_or(0);

        if let Some(chosen_idx) = clack_select(
            "Select SSH Private Key for Gateway SOCKS5 Tunnel:",
            &options,
            default_idx,
            8,
        )? {
            selected_key_path = format!("~/.ssh/{}", options[chosen_idx].summary);
        }
    } else {
        // Non-interactive / default
        if let Some(ed) = keys.iter().find(|k| k.ends_with("id_ed25519")) {
            let fname = ed
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("id_ed25519");
            selected_key_path = format!("~/.ssh/{}", fname);
        } else if let Some(first) = keys.first() {
            let fname = first
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("id_rsa");
            selected_key_path = format!("~/.ssh/{}", fname);
        }
    }

    let default_endpoint = "cdn.flowup.io.vn";
    let endpoint = if interactive && is_interactive_tty() {
        clack_text(
            "Cloudflare Access Tunnel Hostname:",
            Some(default_endpoint),
            None,
        )?
        .unwrap_or_else(|| default_endpoint.to_string())
    } else {
        default_endpoint.to_string()
    };

    // Update gateway.json
    let gateway_cfg_path = crate::config::canonical_ssot_dir()
        .join(crate::constants::PILLAR_GATEWAY)
        .join(crate::constants::FILE_GATEWAY_JSON);

    if let Some(parent) = gateway_cfg_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut cfg: serde_json::Value = if gateway_cfg_path.exists() {
        match std::fs::read_to_string(&gateway_cfg_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        }
    } else {
        serde_json::json!({})
    };

    if cfg.get("servers").is_none() {
        cfg["servers"] = serde_json::json!({});
    }

    let key_to_save = if selected_key_path.is_empty() {
        "~/.ssh/id_ed25519".to_string()
    } else {
        selected_key_path
    };

    cfg["servers"]["my-vps"] = serde_json::json!({
        "type": "cloudflare",
        "name": "my-vps",
        "description": "Primary SOCKS5 dynamic proxy (port 1080) & SSH tunnel via Cloudflare Access",
        "cf_hostname": endpoint,
        "remote_user": "root",
        "identity_file": key_to_save,
        "socks_port": 1080,
        "local_ssh_port": 2222,
        "enabled": true,
        "priority": 1,
        "server_alive_interval": 15,
        "server_alive_count_max": 3,
        "ciphers": [
            "chacha20-poly1305@openssh.com",
            "aes128-gcm@openssh.com"
        ],
        "tags": ["primary", "git"]
    });

    if cfg.get("workloads").is_none() {
        cfg["workloads"] = serde_json::json!({
            "git": { "server": "my-vps", "port": 1080 },
            "supabase": { "server": "my-vps", "socks_port": 1080, "http_port": 8118 }
        });
    }

    let pretty = serde_json::to_string_pretty(&cfg)?;
    std::fs::write(&gateway_cfg_path, pretty)?;

    clack_step_ok(format!(
        "Configured Specter Gateway: {} with key {}",
        endpoint, key_to_save
    ));

    Ok(())
}
