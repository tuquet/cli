use sha2::Digest;
use std::io::{Cursor, Read};
use std::path::Path;

use super::cache::get_cached_update;
use super::client::{check_and_save_latest_version, fetch_bytes_with_fallback};
use super::types::{GITHUB_REPO, UpdateInfo};

/// Perform upgrade: checks if managed by Scoop, otherwise performs in-place self-update.
pub async fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::prompt::{
        ClackSpinner, clack_intro, clack_outro, clack_step_info, clack_step_ok, clack_step_warn,
    };

    clack_intro("tq upgrade", "CLI Self-Updater & Release Manager");
    let sp = ClackSpinner::start(format!(
        "Checking for latest release from GitHub ({})...",
        GITHUB_REPO
    ));

    let current = env!("CARGO_PKG_VERSION");
    let info = match check_and_save_latest_version().await {
        Ok(info) => {
            sp.finish(format!(
                "Checked GitHub release channel (latest: v{})",
                info.latest_version
            ));
            info
        }
        Err(e) => {
            sp.clear();
            clack_step_warn(format!(
                "Failed to reach GitHub API ({}). Checking local cache...",
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
        clack_step_ok(format!(
            "is already up to date! (Current: \x1b[1;32mv{}\x1b[0m)",
            current
        ));
        clack_outro("No upgrade required.");
        return Ok(());
    }

    clack_step_warn(format!(
        "New version available: \x1b[38;2;251;191;36mv{}\x1b[0m -> \x1b[1;32mv{}\x1b[0m",
        current, info.latest_version
    ));

    let current_exe = std::env::current_exe()?;
    let exe_lower = current_exe.to_string_lossy().to_lowercase();
    let is_scoop_install =
        exe_lower.contains("scoop\\apps\\tuquet") || exe_lower.contains("scoop/apps/tuquet");

    // 1. If installed as a dedicated Scoop app, delegate to scoop update
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
            clack_step_info("Detected Scoop installation. Executing package upgrade...");

            if let Ok(output) = tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "scoop update tuquet"])
                .output()
                .await
            {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                if output.status.success() && !stdout_str.contains("isn't installed") {
                    clack_step_ok(format!(
                        "Successfully upgraded to v{} via Scoop!",
                        info.latest_version
                    ));
                    clack_outro("Upgrade complete.");
                    return Ok(());
                }
            }
        }
    }

    // 2. Perform native in-place self-update
    perform_in_place_self_update(&info, &current_exe).await?;
    clack_outro("Upgrade complete.");

    Ok(())
}

/// Pure Rust in-place self-updater: downloads release asset, extracts binary in-memory,
/// and atomically swaps the currently executing binary on disk.
async fn perform_in_place_self_update(
    info: &UpdateInfo,
    current_exe: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("Initiating in-place self-update...");
    println!("  Current version   : v{}", info.current_version);
    println!("  Target version    : v{}", info.latest_version);
    println!("  Target executable : {}", current_exe.display());
    println!();

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let release_bytes = fetch_bytes_with_fallback(&url, 30).await?;

    let release_json: serde_json::Value = serde_json::from_slice(&release_bytes)?;
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

    let prefix = format!("{}-{}", os, arch);

    // Find best asset match:
    // 1. Archive matching OS and Arch (e.g. windows-x64.zip, linux-x64.tar.gz)
    // 2. Raw binary matching OS (tuquet.exe, tuquet)
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

            // Match full prefix archive (tuquet)
            if name.starts_with(&prefix)
                && (name.ends_with(".zip") || name.ends_with(".tar.gz"))
            {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
                break;
            }
            // Match naming with arch and os
            if name.contains("tuquet")
                && name.contains(arch)
                && name.contains(os)
                && name.ends_with(".zip")
            {
                selected_asset = Some((name.to_string(), download_url.to_string(), size));
            }
            // Fallback raw binary
            if selected_asset.is_none()
                && (name == "tq.exe"
                    || name == "tq"
                    || name == "tuquet.exe"
                    || name == "tuquet")
            {
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

    let downloaded_bytes = fetch_bytes_with_fallback(&download_url, 120).await?;
    println!(
        "{} Successfully received {} bytes from GitHub Releases",
        crate::ui::badge_step("RECEIVED"),
        downloaded_bytes.len()
    );

    // Optional SHA-256 verification when checksum asset exists
    if let Some((_, csum_url)) = checksum_asset
        && let Ok(csum_bytes) = fetch_bytes_with_fallback(&csum_url, 30).await
        && let Ok(csum_text) = String::from_utf8(csum_bytes)
    {
        let actual_hash = hex::encode(sha2::Sha256::digest(&downloaded_bytes));
        for line in csum_text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].contains(&asset_name) {
                let expected_hash = parts[0].trim();
                if !expected_hash.eq_ignore_ascii_case(&actual_hash) {
                    return Err(format!(
                        "Security Alert: SHA256 checksum mismatch for {}! Expected {}, got {}",
                        asset_name, expected_hash, actual_hash
                    )
                    .into());
                }
                println!(
                    "{} Verified SHA256 checksum: \x1b[38;2;34;197;94mMATCH\x1b[0m",
                    crate::ui::badge_step("VERIFY")
                );
                break;
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
            if file_name.ends_with("tq.exe")
                || file_name.ends_with("tq")
                || file_name.ends_with("tuquet.exe")
                || file_name.ends_with("tuquet")
            {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted.ok_or("Failed to locate 'tq' executable inside the downloaded ZIP archive")?
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
            if path.ends_with("tq")
                || path.ends_with("tq.exe")
                || path.ends_with("tuquet")
                || path.ends_with("tuquet.exe")
            {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                extracted = Some(buf);
                break;
            }
        }
        extracted
            .ok_or("Failed to locate 'tq' executable inside the downloaded tarball archive")?
    } else {
        // Direct raw binary
        downloaded_bytes.to_vec()
    };

    // Swap executable in-place for `tq` only, and purge any legacy `tuquet.exe`
    println!(
        "{} Performing atomic in-place binary swap (tq)...",
        crate::ui::badge_step("SWAP")
    );

    let bin_name = platform_binary_name();
    let target_spc = current_exe
        .parent()
        .map(|p| p.join(bin_name))
        .unwrap_or_else(|| current_exe.to_path_buf());

    atomic_swap_binary(&target_spc, &binary_bytes)?;

    // If the user ran `tuquet.exe update` from an older install, remove the legacy `tuquet.exe`
    if current_exe != target_spc {
        let old_legacy = current_exe.with_extension("exe.old");
        let _ = std::fs::rename(current_exe, &old_legacy);
        let _ = std::fs::remove_file(&old_legacy);
    }

    if let Some(parent) = target_spc.parent() {
        let legacy = parent.join(legacy_binary_name());
        if legacy.exists() {
            let _ = std::fs::remove_file(&legacy);
        }
    }

    let ssot_bin = crate::config::canonical_dir().join("bin");
    if ssot_bin.exists() {
        let candidate = ssot_bin.join(bin_name);
        if candidate != target_spc && candidate.exists() {
            let _ = atomic_swap_binary(&candidate, &binary_bytes);
        }
        let legacy_ssot = ssot_bin.join(legacy_binary_name());
        if legacy_ssot.exists() {
            let _ = std::fs::remove_file(&legacy_ssot);
        }
    }

    println!();
    println!("In-place upgrade completed successfully.");
    println!("  Previous version  : v{}", info.current_version);
    println!("  Active version    : v{}", info.latest_version);
    println!("  Target executable : {}", target_spc.display());
    println!();

    Ok(())
}

fn platform_binary_name() -> &'static str {
    #[cfg(windows)]
    {
        "tq.exe"
    }
    #[cfg(not(windows))]
    {
        "tq"
    }
}

fn legacy_binary_name() -> &'static str {
    #[cfg(windows)]
    {
        "tuquet.exe"
    }
    #[cfg(not(windows))]
    {
        "tuquet"
    }
}

fn atomic_swap_binary(
    target_exe: &Path,
    binary_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let old_exe = target_exe.with_extension("exe.old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(&old_exe);
        }

        if target_exe.exists() {
            // On Windows NTFS, an open executable handle can be renamed, but not overwritten directly.
            std::fs::rename(target_exe, &old_exe).map_err(|e| {
                format!(
                    "Failed to rename running executable '{}' to backup: {}",
                    target_exe.display(),
                    e
                )
            })?;
        }

        if let Err(e) = std::fs::write(target_exe, binary_bytes) {
            if old_exe.exists() {
                let _ = std::fs::rename(&old_exe, target_exe);
            }
            return Err(format!(
                "Failed to write new binary to '{}': {}. Reverted to previous version.",
                target_exe.display(),
                e
            )
            .into());
        }
        let _ = std::fs::remove_file(&old_exe);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp_new = target_exe.with_extension("tmp_new");
        std::fs::write(&tmp_new, binary_bytes)?;
        let _ = std::fs::set_permissions(&tmp_new, std::fs::Permissions::from_mode(0o755));

        std::fs::rename(&tmp_new, target_exe).map_err(|e| {
            format!(
                "Failed to atomically swap binary at '{}': {}",
                target_exe.display(),
                e
            )
        })?;
    }

    Ok(())
}
