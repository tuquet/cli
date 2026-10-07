use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::config::expander::{canonical_tuquet_dir, EnvExpander};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    #[serde(default = "default_machine_name")]
    pub machine_name: String,
    #[serde(default = "default_cloud_url")]
    pub cloud_url: String,
    #[serde(default = "default_true")]
    pub auto_check_update: bool,
    #[serde(default = "default_channel")]
    pub update_channel: String,
    #[serde(default = "default_env")]
    pub environment: String,
}

fn default_machine_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "WORKSTATION".to_string())
}
fn default_cloud_url() -> String { "https://cloud.tuquet.com".to_string() }
fn default_true() -> bool { true }
fn default_channel() -> String { "stable".to_string() }
fn default_env() -> String { "production".to_string() }

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            machine_name: default_machine_name(),
            cloud_url: default_cloud_url(),
            auto_check_update: true,
            update_channel: default_channel(),
            environment: default_env(),
        }
    }
}

impl SystemConfig {
    pub fn config_path() -> PathBuf {
        let dir = canonical_tuquet_dir().join("system");
        let _ = fs::create_dir_all(&dir);
        dir.join("system.json")
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
            let _ = fs::create_dir_all(parent);
        }
        let json_str = serde_json::to_string_pretty(self)?;
        fs::write(path, json_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_config_defaults() {
        let config = SystemConfig::default();
        assert_eq!(config.cloud_url, "https://cloud.tuquet.com");
        assert_eq!(config.update_channel, "stable");
        assert!(config.auto_check_update);
    }
}
