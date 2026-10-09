pub mod automa;
pub mod browser;
pub mod expander;
pub mod inbox;
pub mod inspector;
pub mod registry;
pub mod runner;
pub mod system;

pub use automa::AutomaConfig;
pub use browser::BrowserConfig;
pub use expander::{canonical_specter_dir, canonical_ssot_dir, EnvExpander};
pub use inbox::InboxConfig;
pub use inspector::{ConfigController, ConfigOptionDef};
pub use registry::ConfigRegistry;
pub use runner::RunnerConfig;
pub use system::SystemConfig;

use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub log_level: String,
    pub data_dir: String,
    pub cloud_url: Option<String>,
    pub cloud_enrollment_token: Option<String>,
    pub cloud_heartbeat_interval_secs: u64,
}

impl AppConfig {
    pub fn load() -> Self {
        let mut server_host = env::var(crate::constants::ENV_SPECTER_HOST)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_HOST))
            .unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());

        let mut server_port = env::var(crate::constants::ENV_SPECTER_PORT)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_PORT))
            .unwrap_or_else(|_| crate::constants::DEFAULT_RUNNER_PORT.to_string())
            .parse()
            .unwrap_or(crate::constants::DEFAULT_RUNNER_PORT);

        // Check command line arguments for --port / -p, --host / -H
        let args: Vec<String> = env::args().collect();
        for i in 0..args.len() {
            if (args[i] == "--port" || args[i] == "-p") && i + 1 < args.len()
                && let Ok(p) = args[i + 1].parse::<u16>() {
                    server_port = p;
                }
            if (args[i] == "--host" || args[i] == "-H") && i + 1 < args.len() {
                server_host = args[i + 1].clone();
            }
        }

        let environment = env::var(crate::constants::ENV_SPECTER_ENV)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_ENV))
            .unwrap_or_else(|_| "development".to_string());
        
        let log_level = env::var(crate::constants::ENV_SPECTER_LOG_LEVEL)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_LOG_LEVEL))
            .unwrap_or_else(|_| "info".to_string());
        
        let data_dir = env::var(crate::constants::ENV_SPECTER_DATA_DIR)
            .unwrap_or_else(|_| canonical_ssot_dir().join(crate::constants::PILLAR_AUTOMA).to_string_lossy().to_string());

        let mut cloud_url = env::var(crate::constants::ENV_SPECTER_CLOUD_URL)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_CLOUD_URL))
            .ok()
            .filter(|s| !s.trim().is_empty());

        if cloud_url.is_none() {
            let identity_path = canonical_ssot_dir().join(crate::constants::PILLAR_SYSTEM).join(crate::constants::FILE_IDENTITY_JSON);
            let device_path = std::path::Path::new(&data_dir).join("device.json");
            let target_path = if identity_path.exists() { identity_path } else { device_path };
            if target_path.exists()
                && let Ok(content) = std::fs::read_to_string(&target_path)
                    && let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content)
                        && let Some(u) = creds.get("cloud_url").and_then(|v| v.as_str())
                            && !u.trim().is_empty() {
                                cloud_url = Some(u.to_string());
                            }
        }

        let cloud_enrollment_token = env::var(crate::constants::ENV_SPECTER_ENROLLMENT_TOKEN)
            .or_else(|_| env::var(crate::constants::ENV_AUTOMA_ENROLLMENT_TOKEN))
            .ok()
            .filter(|s| !s.trim().is_empty());

        let cloud_heartbeat_interval_secs = env::var(crate::constants::ENV_AUTOMA_HEARTBEAT_INTERVAL)
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(crate::constants::DEFAULT_HEARTBEAT_INTERVAL_SECS);

        Self {
            server_host,
            server_port,
            environment,
            log_level,
            data_dir,
            cloud_url,
            cloud_enrollment_token,
            cloud_heartbeat_interval_secs,
        }
    }
}
