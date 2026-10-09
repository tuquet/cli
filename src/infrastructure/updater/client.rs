use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::cache::get_cache_file_path;
use super::types::{is_newer_version, UpdateInfo, GITHUB_REPO};

/// Constructs a proxy-aware HTTP client with standard timeouts and User-Agent.
pub fn create_http_client(timeout_secs: u64) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(format!(
            "specter/{} ({}; {})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ));

    // 1. Honor standard environment proxy variables
    if let Ok(proxy_url) = std::env::var("ALL_PROXY")
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .or_else(|_| std::env::var("all_proxy"))
        .or_else(|_| std::env::var("https_proxy"))
    {
        if let Ok(p) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(p);
        }
    } else {
        // 2. Probe if Specter local SOCKS5 mesh bridge (port 1080) is actively listening
        if let Ok(_) = std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], 1080)),
            Duration::from_millis(80),
        ) {
            if let Ok(p) = reqwest::Proxy::all("socks5://127.0.0.1:1080") {
                builder = builder.proxy(p);
            }
        }
    }

    Ok(builder.build()?)
}

/// Query GitHub API directly for the latest release and update local cache.
pub async fn check_and_save_latest_version() -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let client = create_http_client(6)?;
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status: {}", resp.status()).into());
    }

    let json: serde_json::Value = resp.json().await?;
    let raw_tag = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
    let latest_clean = raw_tag.trim().trim_start_matches('v').to_string();
    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/tuquet/cli/releases")
        .to_string();

    let current = env!("CARGO_PKG_VERSION").to_string();
    let has_update = is_newer_version(&current, &latest_clean);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let info = UpdateInfo {
        current_version: current,
        latest_version: latest_clean,
        has_update,
        release_url,
        checked_at: now,
    };

    let path = get_cache_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&info) {
        let _ = std::fs::write(&path, serialized);
    }

    Ok(info)
}
