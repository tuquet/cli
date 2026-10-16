use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client::check_and_save_latest_version;
use super::types::{CACHE_TTL_SECS, UpdateInfo};

pub fn get_cache_file_path() -> PathBuf {
    crate::config::canonical_specter_dir()
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
    let mut dirs_to_clean = Vec::new();
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(parent) = current_exe.parent()
    {
        dirs_to_clean.push(parent.to_path_buf());
    }
    dirs_to_clean.push(crate::config::canonical_specter_dir().join("bin"));

    for dir in dirs_to_clean {
        for stale in &[
            "specter.exe.old",
            "spc.exe.old",
            "specter.old",
            "spc.old",
            "specter.tmp_new",
            "spc.tmp_new",
        ] {
            let p = dir.join(stale);
            if p.exists() {
                let _ = std::fs::remove_file(p);
            }
        }
    }
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
