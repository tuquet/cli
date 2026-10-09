use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SUPABASE_ANON_KEY: &str = "specter-placeholder-supabase-anon-key";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantInfo {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub is_active: bool,
}

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

    // Supabase GoTrue Auth & Multi-Tenancy additions:
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_expires_at: Option<i64>,
    #[serde(default)]
    pub tenant_slug: Option<String>,
    #[serde(default)]
    pub tenant_name: Option<String>,
    #[serde(default)]
    pub tenant_role: Option<String>,
    #[serde(default)]
    pub available_tenants: Option<Vec<TenantInfo>>,
}

impl DeviceCredentials {
    pub fn is_token_expired(&self) -> bool {
        if let Some(exp) = self.token_expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now >= (exp - 60)
        } else {
            false
        }
    }

    pub fn has_user_session(&self) -> bool {
        self.access_token.is_some() || self.email.is_some()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_credentials_backward_compatibility() {
        // Legacy identity JSON without auth/tenant fields
        let legacy_json = r#"{
            "device_id": "dev_123",
            "device_token": "token_abc",
            "tenant_id": "tenant_xyz",
            "name": "workstation-1",
            "machine_fingerprint": "fp_hash",
            "enrolled_at": "1700000000"
        }"#;

        let creds: DeviceCredentials = serde_json::from_str(legacy_json).expect("Must deserialize legacy json");
        assert_eq!(creds.device_id, "dev_123");
        assert_eq!(creds.device_token, "token_abc");
        assert_eq!(creds.tenant_id, Some("tenant_xyz".to_string()));
        assert_eq!(creds.name, "workstation-1");
        assert_eq!(creds.machine_fingerprint, "fp_hash");
        assert_eq!(creds.registered_at, Some("1700000000".to_string()));
        assert!(creds.user_id.is_none());
        assert!(creds.email.is_none());
        assert!(creds.access_token.is_none());
        assert!(creds.refresh_token.is_none());
        assert!(creds.tenant_slug.is_none());
        assert!(creds.available_tenants.is_none());
        assert!(!creds.has_user_session());
    }

    #[test]
    fn test_device_credentials_full_roundtrip() {
        let creds = DeviceCredentials {
            device_id: "dev_999".to_string(),
            device_token: "tok_secret".to_string(),
            tenant_id: Some("tenant_main".to_string()),
            name: "dev-laptop".to_string(),
            cloud_url: Some("https://example.supabase.co".to_string()),
            api_key: Some("sb_publishable_test".to_string()),
            machine_fingerprint: "hw_fingerprint_abc".to_string(),
            registered_at: Some("1720000000".to_string()),
            user_id: Some("usr_456".to_string()),
            email: Some("dev@example.com".to_string()),
            access_token: Some("jwt.access.token".to_string()),
            refresh_token: Some("refresh_token_value".to_string()),
            token_expires_at: Some(1730000000),
            tenant_slug: Some("acme-corp".to_string()),
            tenant_name: Some("Acme Corp".to_string()),
            tenant_role: Some("owner".to_string()),
            available_tenants: Some(vec![
                TenantInfo {
                    id: "tenant_main".to_string(),
                    slug: "acme-corp".to_string(),
                    name: "Acme Corp".to_string(),
                    role: Some("owner".to_string()),
                    status: Some("active".to_string()),
                    is_active: true,
                },
                TenantInfo {
                    id: "tenant_sub".to_string(),
                    slug: "acme-labs".to_string(),
                    name: "Acme Labs".to_string(),
                    role: Some("member".to_string()),
                    status: Some("active".to_string()),
                    is_active: false,
                },
            ]),
        };

        assert!(creds.has_user_session());
        let json = serde_json::to_string(&creds).expect("Serialize creds");
        let decoded: DeviceCredentials = serde_json::from_str(&json).expect("Deserialize creds");
        assert_eq!(decoded.device_id, "dev_999");
        assert_eq!(decoded.email, Some("dev@example.com".to_string()));
        assert_eq!(decoded.tenant_slug, Some("acme-corp".to_string()));
        let tenants = decoded.available_tenants.expect("Must have available_tenants");
        assert_eq!(tenants.len(), 2);
        assert_eq!(tenants[0].slug, "acme-corp");
        assert!(tenants[0].is_active);
        assert!(!tenants[1].is_active);
    }
}

