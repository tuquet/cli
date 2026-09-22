use dotenvy::dotenv;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server_port: u16,
    pub environment: String,
    pub log_level: String,
    pub data_dir: String,
}

impl AppConfig {
    pub fn load() -> Self {
        // Ignore dotenv error if file doesn't exist
        let _ = dotenv();

        let mut server_port = env::var("AUTOMA_PORT")
            .unwrap_or_else(|_| "8765".to_string())
            .parse()
            .unwrap_or(8765);

        // Check command line arguments for --port or -p
        let args: Vec<String> = env::args().collect();
        for i in 0..args.len() {
            if (args[i] == "--port" || args[i] == "-p") && i + 1 < args.len() {
                if let Ok(p) = args[i + 1].parse::<u16>() {
                    server_port = p;
                }
            }
        }

        let environment = env::var("AUTOMA_ENV").unwrap_or_else(|_| "development".to_string());
        
        let log_level = env::var("AUTOMA_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        
        let data_dir = env::var("AUTOMA_DATA_DIR").unwrap_or_else(|_| {
            let env_type = env::var("AUTOMA_ENV").unwrap_or_else(|_| "development".to_string());
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            let mut path = std::path::PathBuf::from(home);
            if env_type == "development" {
                path.push(".automa/core-dev");
            } else {
                path.push(".automa/core");
            }
            path.to_string_lossy().to_string()
        });

        Self {
            server_port,
            environment,
            log_level,
            data_dir,
        }
    }
}
