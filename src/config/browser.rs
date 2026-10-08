use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::config::expander::{canonical_specter_dir, EnvExpander};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default = "default_browser_name")]
    pub default_browser: String,
    #[serde(default)]
    pub chromium_revision: Option<String>,
    #[serde(default)]
    pub headless: bool,
    #[serde(default = "default_width")]
    pub viewport_width: u32,
    #[serde(default = "default_height")]
    pub viewport_height: u32,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default = "default_args")]
    pub extra_args: Vec<String>,
    #[serde(default = "default_extensions")]
    pub autoload_extensions: Vec<String>,
}

fn default_browser_name() -> String { "chromium".to_string() }
fn default_width() -> u32 { 1280 }
fn default_height() -> u32 { 800 }
fn default_args() -> Vec<String> {
    vec![
        "--disable-blink-features=AutomationControlled".to_string(),
        "--no-sandbox".to_string(),
    ]
}
fn default_extensions() -> Vec<String> {
    vec!["automa".to_string()]
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            default_browser: default_browser_name(),
            chromium_revision: Some("134.0.6998.35".to_string()),
            headless: false,
            viewport_width: default_width(),
            viewport_height: default_height(),
            user_agent: None,
            extra_args: default_args(),
            autoload_extensions: default_extensions(),
        }
    }
}

impl BrowserConfig {
    pub fn config_path() -> PathBuf {
        let dir = canonical_specter_dir().join("browser");
        let _ = fs::create_dir_all(&dir);
        dir.join("browser.json")
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
    fn test_browser_config_defaults() {
        let config = BrowserConfig::default();
        assert_eq!(config.default_browser, "chromium");
        assert_eq!(config.viewport_width, 1280);
        assert!(!config.headless);
        assert!(config.autoload_extensions.contains(&"automa".to_string()));
    }
}
