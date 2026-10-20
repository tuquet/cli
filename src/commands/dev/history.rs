use super::scanner::DiscoveredPackage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_HISTORY_ENTRIES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevHistoryEntry {
    pub project_dir: PathBuf,
    pub project_id: String,
    pub script: String,
    pub package_manager: String,
    pub framework: Option<String>,
    pub last_run_at: u64,
    pub run_count: u32,
}

pub fn history_file_path() -> PathBuf {
    crate::config::canonical_tuquet_dir()
        .join(crate::constants::PILLAR_SYSTEM)
        .join("dev_history.json")
}

pub fn load_history() -> Vec<DevHistoryEntry> {
    load_history_from_path(&history_file_path())
}

pub fn load_history_from_path(path: &Path) -> Vec<DevHistoryEntry> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(mut entries) = serde_json::from_str::<Vec<DevHistoryEntry>>(&content) else {
        return Vec::new();
    };
    entries.retain(|e| e.project_dir.exists());
    entries.sort_by_key(|a| std::cmp::Reverse(a.last_run_at));
    entries
}

pub fn record_history(pkg: &DiscoveredPackage, script: &str) {
    record_history_to_path(&history_file_path(), pkg, script);
}

pub fn record_history_to_path(path: &Path, pkg: &DiscoveredPackage, script: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut entries = load_history_from_path(path);
    if let Some(existing) = entries
        .iter_mut()
        .find(|e| e.project_dir == pkg.dir && e.script == script)
    {
        existing.project_id = pkg.id.clone();
        existing.package_manager = pkg.package_manager.clone();
        existing.framework = pkg.framework.clone();
        existing.last_run_at = now;
        existing.run_count = existing.run_count.saturating_add(1);
    } else {
        entries.push(DevHistoryEntry {
            project_dir: pkg.dir.clone(),
            project_id: pkg.id.clone(),
            script: script.to_string(),
            package_manager: pkg.package_manager.clone(),
            framework: pkg.framework.clone(),
            last_run_at: now,
            run_count: 1,
        });
    }

    entries.sort_by_key(|a| std::cmp::Reverse(a.last_run_at));
    entries.truncate(MAX_HISTORY_ENTRIES);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(path, json);
    }
}

pub fn format_relative_time(timestamp_secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(timestamp_secs);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_load_history() {
        let temp_dir = std::env::temp_dir().join(format!("spc_hist_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let hist_file = temp_dir.join("dev_history.json");

        let pkg = DiscoveredPackage {
            id: "tuquet/admin".to_string(),
            name: Some("@tuquet/admin".to_string()),
            dir: temp_dir.clone(),
            package_manager: "pnpm".to_string(),
            framework: Some("Vite".to_string()),
            scripts: vec![("dev".to_string(), "vite".to_string())],
        };

        record_history_to_path(&hist_file, &pkg, "dev");
        let loaded = load_history_from_path(&hist_file);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].project_id, "tuquet/admin");
        assert_eq!(loaded[0].script, "dev");
        assert_eq!(loaded[0].run_count, 1);

        record_history_to_path(&hist_file, &pkg, "dev");
        let loaded2 = load_history_from_path(&hist_file);
        assert_eq!(loaded2.len(), 1);
        assert_eq!(loaded2[0].run_count, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
