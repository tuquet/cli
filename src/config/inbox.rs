use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::config::expander::{canonical_specter_dir, EnvExpander};
use crate::constants::paths::{FILE_INBOX_JSON, FILE_INBOX_SQLITE};
use crate::constants::pillars::PILLAR_INBOX;
use crate::constants::ports::DEFAULT_INBOX_PORT;
use crate::constants::endpoints::DEFAULT_HOST;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub webhook_secret: Option<String>,
    #[serde(default = "default_true")]
    pub cloud_sync: bool,
    #[serde(default)]
    pub supabase_url: Option<String>,
    #[serde(default)]
    pub supabase_anon_key: Option<String>,
    #[serde(default)]
    pub catch_all_domains: Vec<String>,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
}

fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

fn default_port() -> u16 {
    DEFAULT_INBOX_PORT
}

fn default_true() -> bool {
    true
}

fn default_retention_days() -> u32 {
    14
}

fn default_timeout_secs() -> u64 {
    30
}

impl Default for InboxConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            webhook_secret: None,
            cloud_sync: true,
            supabase_url: None,
            supabase_anon_key: None,
            catch_all_domains: Vec::new(),
            retention_days: default_retention_days(),
            default_timeout_secs: default_timeout_secs(),
        }
    }
}

impl InboxConfig {
    pub fn config_path() -> PathBuf {
        let dir = canonical_specter_dir().join(PILLAR_INBOX);
        let _ = fs::create_dir_all(&dir);
        dir.join(FILE_INBOX_JSON)
    }

    pub fn sqlite_path() -> PathBuf {
        let dir = canonical_specter_dir().join(PILLAR_INBOX);
        let _ = fs::create_dir_all(&dir);
        dir.join(FILE_INBOX_SQLITE)
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path) {
                let expanded = EnvExpander::expand(&content);
                if let Ok(cfg) = serde_json::from_str::<Self>(&expanded) {
                    return cfg;
                }
            }
        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbox_config_defaults() {
        let cfg = InboxConfig::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 9123);
        assert!(cfg.cloud_sync);
        assert_eq!(cfg.retention_days, 14);
        assert_eq!(cfg.default_timeout_secs, 30);
        assert!(cfg.catch_all_domains.is_empty());
    }

    #[test]
    fn test_inbox_config_paths() {
        let cfg_path = InboxConfig::config_path();
        assert!(cfg_path.ends_with(PathBuf::from("inbox").join("inbox.json")));

        let db_path = InboxConfig::sqlite_path();
        assert!(db_path.ends_with(PathBuf::from("inbox").join("inbox.sqlite")));
    }
}
