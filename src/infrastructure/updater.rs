use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_REPO: &str = "tuquet/cli";
const CACHE_TTL_SECS: u64 = 86400; // 24 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
    pub checked_at: u64,
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse_ver = |v: &str| -> Vec<u32> {
        let clean = v.trim().trim_start_matches('v');
        clean
            .split(['.', '-'])
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let cur_parts = parse_ver(current);
    let lat_parts = parse_ver(latest);

    for (c, l) in cur_parts.iter().zip(lat_parts.iter()) {
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    lat_parts.len() > cur_parts.len()
}

pub fn get_cache_file_path() -> PathBuf {
    crate::config::canonical_tuquet_dir()
        .join("system")
        .join("update_check.json")
}

pub fn get_cached_update() -> Option<UpdateInfo> {
    let path = get_cache_file_path();
    if let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(info) = serde_json::from_str::<UpdateInfo>(&content)
    {
        return Some(info);
    }
    None
}

/// Clean up leftover temporary or .old backup binaries from previous in-place updates.
pub fn cleanup_stale_update_files() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_exe = current_exe.with_extension("exe.old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(old_exe);
        }
        let old_posix = current_exe.with_extension("old");
        if old_posix.exists() {
            let _ = std::fs::remove_file(old_posix);
        }
        let tmp_new = current_exe.with_extension("tmp_new");
        if tmp_new.exists() {
            let _ = std::fs::remove_file(tmp_new);
        }
    }
}

/// Constructs a proxy-aware HTTP client with standard timeouts and User-Agent.
fn create_http_client(timeout_secs: u64) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(format!(
            "tuquet-cli/{} ({}; {})",
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
        // 2. Probe if Tuquet local SOCKS5 mesh bridge (port 1080) is actively listening
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

/// Spawns a non-blocking background check for updates if the local cache is older than 24h.
pub fn spawn_background_update_check() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(cached) = get_cached_update()
        && now.saturating_sub(cached.checked_at) < CACHE_TTL_SECS
    {
        return;
    }

    tokio::spawn(async move {
        let _ = check_and_save_latest_version().await;
    });
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

/// Perform upgrade: checks if managed by Scoop, otherwise performs in-place self-update.
pub async fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[38;2;56;189;248m⚡ Tuquet CLI Upgrade Manager\x1b[0m");
    println!("Checking for latest release from GitHub ({})...", GITHUB_REPO);

    let current = env!("CARGO_PKG_VERSION");
    let info = match check_and_save_latest_version().await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("\x1b[38;2;248;113;113m[WARN]\x1b[0m Failed to reach GitHub API: {}. Trying local cache...", e);
            if let Some(cached) = get_cached_update() {
                cached
            } else {
                return Err(format!("Unable to determine latest version: {}", e).into());
            }
        }
    };

    if !info.has_update {
        println!();
        println!(
            "{} Tuquet CLI is already up to date! (Current: \x1b[1;32mv{}\x1b[0m)",
            crate::ui::badge_online("UP TO DATE"),
            current
        );
        return Ok(());
    }

    println!();
    println!(
        "{} New version available: \x1b[38;2;251;191;36mv{}\x1b[0m → \x1b[1;32mv{}\x1b[0m",
        crate::ui::badge_warn("UPDATE FOUND"),
        current,
        info.latest_version
    );

    let current_exe = std::env::current_exe()?;
    let is_scoop_install = current_exe.to_string_lossy().to_lowercase().contains("scoop");

    // 1. If installed inside Scoop directory, delegate to scoop update
    if is_scoop_install {
        let has_scoop = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Command scoop -ErrorAction SilentlyContinue"])
            .output()
            .await
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);

        if has_scoop {
            println!();
            println!("Detected \x1b[38;2;56;189;248mScoop\x1b[0m installation. Executing package upgrade...");
            println!("\x1b[38;2;148;163;184m> scoop update specter\x1b[0m\n");

            let mut child = tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "scoop update specter"])
                .spawn()?;

            let status = child.wait().await?;
            if status.success() {
                println!(
                    "\n{} Successfully upgraded Tuquet CLI to v{} via Scoop!",
                    crate::ui::badge_online("SUCCESS"),
                    info.latest_version
                );
                return Ok(());
            }
        }
    }

    // 2. Perform native in-place self-update
    perform_in_place_self_update(&info, &current_exe).await?;

    Ok(())
}

/// Pure Rust in-place self-updater: downloads release asset, extracts binary in-memory,
/// and atomically swaps the currently executing binary on disk.
async fn perform_in_place_self_update(
    info: &UpdateInfo,
    current_exe: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("╭─ IN-PLACE SELF-UPDATE ────────────────────────────────────── ● INITIATED ─╮");
    println!("│  Current Version    v{:<52} │", info.current_version);
    println!("│  Target Version     v{:<52} │", info.latest_version);
    println!("│  Target Executable  {:<53} │", current_exe.display());
    println!("╰───────────────────────────────────────────────────────────────────────────╯");
    println!();

    let client = create_http_client(60)?;
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let release_resp = client.get(&url).send().await?;

    if !release_resp.status().is_success() {
        return Err(format!("Failed to retrieve release metadata: HTTP {}", release_resp.status()).into());
    }

    let release_json: serde_json::Value = release_resp.json().await?;
    let assets = release_json
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or("No assets found in latest GitHub release")?;

    // Determine target platform asset patterns
    let os = std::env::consts::OS;
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };

    let specter_prefix = format!("specter-{}-{}", os, arch);
    let tuquet_prefix = format!("tuquet-{}-{}", os, arch);

    // Find best asset match:
    // 1. Archive matching OS and Arch (e.g. specter-windows-x64.zip, tuquet-linux-x64.tar.gz)
    // 2. Raw binary matching OS (specter.exe, specter, tuquet.exe, tuquet)
    let mut selected_asset: Option<(String, String, u64)> = None;
    let mut checksum_asset: Option<(String, String)> = None;

    for asset in assets {
        if let (Some(name), Some(download_url)) = (
            asset.get("name").and_then(|v| v.as_str()),
            asset.get("browser_download_url").and_then(|v| v.as_str()),
        ) {
            let size = asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

            // Track checksum manifest if available
            if name.eq_ignore_ascii_case("sha256sums.txt")
                || name.eq_ignore_ascii_case("checksums.txt")
                || name.ends_with(".sha256")
            {
                checksum_asset = Some((name.to_string(), download_url.to_string()));
            }

            // Match full prefix archive (specter or legacy tuquet)
            if (name.starts_with(&specter_prefix) || name.starts_with(&tuquet_prefix))
                && (name.ends_with(".zip") || name.ends_with(".tar.gz"))
            {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
                break;
            }
            // Match legacy naming (tuquet-v1.0.0-windows-x64.zip)
            if name.contains(arch) && name.contains(os) && name.ends_with(".zip") {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
            }
            // Fallback raw binary
            if selected_asset.is_none() && (name == "specter.exe" || name == "specter" || name == "tuquet.exe" || name == "tuquet") {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
            }
        }
    }

    let (asset_name, download_url, asset_size) = selected_asset.ok_or_else(|| {
        format!(
            "No compatible release binary found for target: {} ({}) on GitHub",
            os, arch
        )
    })?;

    let size_mb = asset_size as f64 / 1_048_576.0;
    println!(
        "{} Downloading release asset: \x1b[38;2;56;189;248m{}\x1b[0m ({:.1} MB)...",
        crate::ui::badge_step("DOWNLOAD"),
        asset_name,
        size_mb
    );

    let downloaded_bytes = client.get(&download_url).send().await?.bytes().await?;
    println!(
        "{} Successfully received {} bytes from GitHub Releases",
        crate::ui::badge_step("RECEIVED"),
        downloaded_bytes.len()
    );

    // Optional SHA-256 verification when checksum asset exists
    if let Some((_, csum_url)) = checksum_asset {
        if let Ok(csum_resp) = client.get(&csum_url).send().await {
            if csum_resp.status().is_success() {
                if let Ok(csum_text) = csum_resp.text().await {
                    let actual_hash = hex::encode(sha2::Sha256::digest(&downloaded_bytes));
                    for line in csum_text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 && parts[1].contains(&asset_name) {
                            let expected_hash = parts[0].trim();
                            if !expected_hash.eq_ignore_ascii_case(&actual_hash) {
                                return Err(format!(
                                    "Security Alert: SHA256 checksum mismatch for {}! Expected {}, got {}",
                                    asset_name, expected_hash, actual_hash
                                ).into());
                            }
                            println!("{} Verified SHA256 checksum: \x1b[38;2;34;197;94mMATCH\x1b[0m", crate::ui::badge_step("VERIFY"));
                            break;
                        }
                    }
                }
            }
        }
    }

    // Extract binary in-memory from archive if needed
    let binary_bytes = if asset_name.ends_with(".zip") {
        println!("{} Extracting binary from ZIP archive in-memory...", crate::ui::badge_step("EXTRACT"));
        let cursor = Cursor::new(&downloaded_bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut extracted: Option<Vec<u8>> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();
            if file_name.ends_with("specter.exe") || file_name.ends_with("specter") || file_name.ends_with("tuquet.exe") || file_name.ends_with("tuquet") {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted.ok_or("Failed to locate 'specter' executable inside the downloaded ZIP archive")?
    } else if asset_name.ends_with(".tar.gz") {
        println!("{} Extracting binary from Tarball archive in-memory...", crate::ui::badge_step("EXTRACT"));
        let cursor = Cursor::new(&downloaded_bytes);
        let gz_decoder = flate2::read::GzDecoder::new(cursor);
        let mut tar_archive = tar::Archive::new(gz_decoder);
        let mut extracted: Option<Vec<u8>> = None;

        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            if path.ends_with("specter") || path.ends_with("specter.exe") || path.ends_with("tuquet") || path.ends_with("tuquet.exe") {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted.ok_or("Failed to locate 'specter' executable inside the downloaded tarball archive")?
    } else {
        // Direct raw binary
        downloaded_bytes.to_vec()
    };

    // Swap executable in-place
    println!("{} Performing atomic in-place binary swap...", crate::ui::badge_step("SWAP"));

    #[cfg(windows)]
    {
        let old_exe = current_exe.with_extension("exe.old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(&old_exe);
        }

        // On Windows NTFS, an open executable handle can be renamed, but not overwritten directly.
        // Step 1: Rename current binary to .exe.old
        std::fs::rename(current_exe, &old_exe).map_err(|e| {
            format!(
                "Failed to rename running executable '{}' to backup: {}",
                current_exe.display(),
                e
            )
        })?;

        // Step 2: Write new binary to the original canonical path
        if let Err(e) = std::fs::write(current_exe, &binary_bytes) {
            // Rollback if writing the new binary fails
            let _ = std::fs::rename(&old_exe, current_exe);
            return Err(format!(
                "Failed to write new binary to '{}': {}. Reverted to previous version.",
                current_exe.display(),
                e
            )
            .into());
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp_new = current_exe.with_extension("tmp_new");
        std::fs::write(&tmp_new, &binary_bytes)?;
        let _ = std::fs::set_permissions(&tmp_new, std::fs::Permissions::from_mode(0o755));

        // Atomic rename replaces the existing file inode cleanly
        std::fs::rename(&tmp_new, current_exe).map_err(|e| {
            format!(
                "Failed to atomically swap binary at '{}': {}",
                current_exe.display(),
                e
            )
        })?;
    }

    println!();
    println!("╭─ UPGRADE COMPLETED ────────────────────────────────────────── ● SUCCESS ─╮");
    println!("│  Previous Version   v{:<52} │", info.current_version);
    println!("│  Active Version     v{:<52} │", info.latest_version);
    println!("│  Binary Executable  {:<53} │", current_exe.display());
    println!("╰─ In-place upgrade applied successfully! Run 'specter doctor' to inspect. ──╯");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "v1.1.0"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(is_newer_version("1.2.3", "1.3.0"));

        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "v1.0.0"));
        assert!(!is_newer_version("1.1.0", "1.0.5"));
        assert!(!is_newer_version("2.0.0", "1.9.9"));
    }
}
