use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::config::expander::{canonical_specter_dir, EnvExpander};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaConfig {
    #[serde(default)]
    pub vault_dir: Option<String>,
    #[serde(default = "default_timeout")]
    pub default_timeout_secs: u64,
    #[serde(default = "default_browser")]
    pub default_browser: String,
    #[serde(default)]
    pub default_headless: bool,
    #[serde(default = "default_studio_port")]
    pub studio_port: u16,
    #[serde(default = "default_true")]
    pub auto_backup: bool,
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

fn default_timeout() -> u64 { 300 }
fn default_browser() -> String { "chromium".to_string() }
fn default_studio_port() -> u16 { 8765 }
fn default_true() -> bool { true }

impl Default for AutomaConfig {
    fn default() -> Self {
        let mut variables = HashMap::new();
        variables.insert("BASE_URL".to_string(), "https://example.com".to_string());
        Self {
            vault_dir: None,
            default_timeout_secs: default_timeout(),
            default_browser: default_browser(),
            default_headless: false,
            studio_port: default_studio_port(),
            auto_backup: true,
            variables,
        }
    }
}

impl AutomaConfig {
    pub fn config_path() -> PathBuf {
        let dir = canonical_specter_dir().join("automa");
        let _ = fs::create_dir_all(&dir);
        dir.join("automa.json")
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

    pub fn resolved_vault_dir(&self) -> PathBuf {
        if let Some(ref custom) = self.vault_dir {
            let expanded = EnvExpander::expand(custom);
            PathBuf::from(expanded)
        } else {
            canonical_specter_dir().join("automa").join("workflows")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automa_config_defaults() {
        let config = AutomaConfig::default();
        assert_eq!(config.default_timeout_secs, 300);
        assert_eq!(config.studio_port, 8765);
        assert!(config.auto_backup);
        assert!(config.resolved_vault_dir().to_string_lossy().contains("automa"));
    }
}
