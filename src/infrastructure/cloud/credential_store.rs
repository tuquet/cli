use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SUPABASE_ANON_KEY: &str = "specter-placeholder-supabase-anon-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCredentials {
    #[serde(default)]
    pub cloud_url: Option<String>,
    pub device_id: String,
    pub device_token: String,
    pub tenant_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub machine_fingerprint: String,
    #[serde(default, alias = "enrolled_at")]
    pub registered_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResult {
    pub success: bool,
    pub enrolled: bool,
    pub device_id: String,
    pub browsers_synced: usize,
    pub heartbeat_sent: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudStatusInfo {
    pub enabled: bool,
    pub cloud_url: Option<String>,
    pub enrolled: bool,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub machine_fingerprint: String,
    pub last_sync_at: Option<String>,
}

/// Local persistence store for Specter Cloud identity in ~/.specter/system/.identity.json
pub struct CredentialStore;

impl CredentialStore {
    pub fn get_path(_data_dir: &str) -> PathBuf {
        crate::config::canonical_specter_dir().join("system").join(".identity.json")
    }

    pub async fn load(data_dir: &str) -> Option<DeviceCredentials> {
        let path = Self::get_path(data_dir);
        if path.exists()
            && let Ok(content) = tokio::fs::read_to_string(&path).await
                && let Ok(creds) = serde_json::from_str::<DeviceCredentials>(&content) {
                    return Some(creds);
                }
        None
    }

    pub async fn save(data_dir: &str, creds: &DeviceCredentials) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::get_path(data_dir);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let json = serde_json::to_string_pretty(creds)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }

    pub async fn purge(data_dir: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::get_path(data_dir);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn epoch_secs_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
