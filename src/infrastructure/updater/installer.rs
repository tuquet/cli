use sha2::Digest;
use std::io::{Cursor, Read};
use std::path::Path;

use super::cache::get_cached_update;
use super::client::{check_and_save_latest_version, create_http_client};
use super::types::{UpdateInfo, GITHUB_REPO};

/// Perform upgrade: checks if managed by Scoop, otherwise performs in-place self-update.
pub async fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::Notify::header("⚡ Specter Upgrade Manager");
    println!("Checking for latest release from GitHub ({})...", GITHUB_REPO);

    let current = env!("CARGO_PKG_VERSION");
    let info = match check_and_save_latest_version().await {
        Ok(info) => info,
        Err(e) => {
            crate::ui::Notify::warn(format!(
                "Failed to reach GitHub API: {}. Trying local cache...",
                e
            ));
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
            "{} Specter is already up to date! (Current: \x1b[1;32mv{}\x1b[0m)",
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
            .args([
                "-NoProfile",
                "-Command",
                "Get-Command scoop -ErrorAction SilentlyContinue",
            ])
            .output()
            .await
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);

        if has_scoop {
            println!();
            crate::ui::Notify::info("Detected Scoop installation. Executing package upgrade...");
            println!("\x1b[38;2;148;163;184m> scoop update specter\x1b[0m\n");

            let mut child = tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "scoop update specter"])
                .spawn()?;

            let status = child.wait().await?;
            if status.success() {
                println!();
                crate::ui::Notify::success(format!(
                    "Successfully upgraded Specter to v{} via Scoop!",
                    info.latest_version
                ));
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
        return Err(format!(
            "Failed to retrieve release metadata: HTTP {}",
            release_resp.status()
        )
        .into());
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

    // Find best asset match:
    // 1. Archive matching OS and Arch (e.g. specter-windows-x64.zip, specter-linux-x64.tar.gz)
    // 2. Raw binary matching OS (specter.exe, specter)
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

            // Match full prefix archive (specter)
            if name.starts_with(&specter_prefix)
                && (name.ends_with(".zip") || name.ends_with(".tar.gz"))
            {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
                break;
            }
            // Match naming with arch and os
            if name.contains("specter")
                && name.contains(arch)
                && name.contains(os)
                && name.ends_with(".zip")
            {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
            }
            // Fallback raw binary
            if selected_asset.is_none() && (name == "specter.exe" || name == "specter") {
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
                            println!(
                                "{} Verified SHA256 checksum: \x1b[38;2;34;197;94mMATCH\x1b[0m",
                                crate::ui::badge_step("VERIFY")
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    // Extract binary in-memory from archive if needed
    let binary_bytes = if asset_name.ends_with(".zip") {
        println!(
            "{} Extracting binary from ZIP archive in-memory...",
            crate::ui::badge_step("EXTRACT")
        );
        let cursor = Cursor::new(&downloaded_bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut extracted: Option<Vec<u8>> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();
            if file_name.ends_with("specter.exe") || file_name.ends_with("specter") {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted
            .ok_or("Failed to locate 'specter' executable inside the downloaded ZIP archive")?
    } else if asset_name.ends_with(".tar.gz") {
        println!(
            "{} Extracting binary from Tarball archive in-memory...",
            crate::ui::badge_step("EXTRACT")
        );
        let cursor = Cursor::new(&downloaded_bytes);
        let gz_decoder = flate2::read::GzDecoder::new(cursor);
        let mut tar_archive = tar::Archive::new(gz_decoder);
        let mut extracted: Option<Vec<u8>> = None;

        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            if path.ends_with("specter") || path.ends_with("specter.exe") {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted
            .ok_or("Failed to locate 'specter' executable inside the downloaded tarball archive")?
    } else {
        // Direct raw binary
        downloaded_bytes.to_vec()
    };

    // Swap executable in-place
    println!(
        "{} Performing atomic in-place binary swap...",
        crate::ui::badge_step("SWAP")
    );

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
