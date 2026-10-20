use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::cache::get_cache_file_path;
use super::types::{GITHUB_REPO, UpdateInfo, is_newer_version};

/// Constructs a proxy-aware HTTP client with standard timeouts and User-Agent.
pub fn create_http_client(timeout_secs: u64) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(format!(
            "tuquet/{} ({}; {})",
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
        // 2. Probe if local SOCKS5 mesh bridge (port 1080) is actively listening
        if let Ok(_) = std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], 1080)),
            Duration::from_millis(80),
        ) {
            if let Ok(p) = reqwest::Proxy::all("socks5://127.0.0.1:1080") {
                builder = builder.proxy(p);
            }
        }
    }

    builder.build()
}

/// Fetch URL bytes using `reqwest` first, falling back to OS-native `curl` (Windows Schannel / system CA store)
/// when `rustls` webpki-roots cannot verify an enterprise SSL inspection certificate or system proxy.
pub async fn fetch_bytes_with_fallback(url: &str, timeout_secs: u64) -> Result<Vec<u8>, String> {
    if let Ok(client) = create_http_client(timeout_secs)
        && let Ok(resp) = client.get(url).send().await
        && resp.status().is_success()
        && let Ok(bytes) = resp.bytes().await
    {
        return Ok(bytes.to_vec());
    }

    // Fallback to OS-native curl (uses Windows Schannel / macOS SecureTransport / system CA store)
    let curl_bin = if cfg!(windows) { "curl.exe" } else { "curl" };
    let ua = format!(
        "User-Agent: tuquet/{} ({}; {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let max_time = timeout_secs.to_string();
    let output = tokio::process::Command::new(curl_bin)
        .args([
            "-fsSL",
            "--noproxy",
            "*",
            "--max-time",
            &max_time,
            "-H",
            &ua,
            url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to spawn {}: {}", curl_bin, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to fetch {}: {}", url, stderr.trim()));
    }

    Ok(output.stdout)
}

/// Query GitHub API directly for the latest release and update local cache.
pub async fn check_and_save_latest_version() -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let body_bytes = fetch_bytes_with_fallback(&url, 10).await?;

    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
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
