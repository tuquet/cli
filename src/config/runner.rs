use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::config::expander::{canonical_tuquet_dir, EnvExpander};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
    #[serde(default = "default_concurrency")]
    pub max_concurrent_jobs: usize,
    #[serde(default = "default_true")]
    pub auto_restart: bool,
}

fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8765 }
fn default_log_level() -> String { "info".to_string() }
fn default_heartbeat() -> u64 { 30 }
fn default_concurrency() -> usize { 2 }
fn default_true() -> bool { true }

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            heartbeat_interval_secs: default_heartbeat(),
            max_concurrent_jobs: default_concurrency(),
            auto_restart: true,
        }
    }
}

impl RunnerConfig {
    pub fn config_path() -> PathBuf {
        let dir = canonical_tuquet_dir().join("automa");
        let _ = fs::create_dir_all(&dir);
        dir.join("runner.json")
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
    fn test_runner_config_defaults() {
        let config = RunnerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8765);
        assert_eq!(config.max_concurrent_jobs, 2);
    }
}
