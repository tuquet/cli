use dotenvy::dotenv;
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
        // Ignore dotenv error if file doesn't exist
        let _ = dotenv();

        let mut server_host = env::var("TUQUET_HOST")
            .or_else(|_| env::var("AUTOMA_HOST"))
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let mut server_port = env::var("TUQUET_PORT")
            .or_else(|_| env::var("AUTOMA_PORT"))
            .unwrap_or_else(|_| "8765".to_string())
            .parse()
            .unwrap_or(8765);

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

        let environment = env::var("TUQUET_ENV")
            .or_else(|_| env::var("AUTOMA_ENV"))
            .unwrap_or_else(|_| "development".to_string());
        
        let log_level = env::var("TUQUET_LOG_LEVEL")
            .or_else(|_| env::var("AUTOMA_LOG_LEVEL"))
            .unwrap_or_else(|_| "info".to_string());
        
        let data_dir = env::var("TUQUET_DATA_DIR")
            .unwrap_or_else(|_| {
                let home = env::var("HOME")
                    .or_else(|_| env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                let home_buf = std::path::PathBuf::from(home);
                let env_suffix = if environment == "development" { "core-dev" } else { "core" };
                home_buf.join(".tuquet").join(env_suffix).to_string_lossy().to_string()
            });

        let mut cloud_url = env::var("TUQUET_CLOUD_URL")
            .or_else(|_| env::var("AUTOMA_CLOUD_URL"))
            .ok()
            .filter(|s| !s.trim().is_empty());

        if cloud_url.is_none() {
            let device_path = std::path::Path::new(&data_dir).join("device.json");
            if device_path.exists()
                && let Ok(content) = std::fs::read_to_string(&device_path)
                    && let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content)
                        && let Some(u) = creds.get("cloud_url").and_then(|v| v.as_str())
                            && !u.trim().is_empty() {
                                cloud_url = Some(u.to_string());
                            }
        }

        let cloud_enrollment_token = env::var("TUQUET_ENROLLMENT_TOKEN")
            .or_else(|_| env::var("AUTOMA_ENROLLMENT_TOKEN"))
            .ok()
            .filter(|s| !s.trim().is_empty());

        let cloud_heartbeat_interval_secs = env::var("AUTOMA_HEARTBEAT_INTERVAL")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

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
