use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client::check_and_save_latest_version;
use super::types::{UpdateInfo, CACHE_TTL_SECS};

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
