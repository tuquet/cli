use std::path::PathBuf;
use tracing::{info, warn};
use crate::AppState;

pub use super::cloud::{
    CloudApiClient, CredentialStore, MachineFingerprint,
    DeviceCredentials, CloudSyncResult, CloudStatusInfo,
    DEFAULT_SUPABASE_ANON_KEY,
};

/// Facade coordinator for Specter Cloud communication, local identity, and background telemetry
pub struct CloudReporter;

impl CloudReporter {
    pub fn get_credentials_path(data_dir: &str) -> PathBuf {
        CredentialStore::get_path(data_dir)
    }

    pub fn generate_machine_fingerprint() -> String {
        MachineFingerprint::generate()
    }

    pub fn build_http_client() -> reqwest::Client {
        CloudApiClient::build_http_client()
    }

    pub async fn load_credentials(data_dir: &str) -> Option<DeviceCredentials> {
        CredentialStore::load(data_dir).await
    }

    pub async fn save_credentials(data_dir: &str, creds: &DeviceCredentials) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        CredentialStore::save(data_dir, creds).await
    }

    pub async fn ensure_enrolled(
        client: &reqwest::Client,
        cloud_url: &str,
        data_dir: &str,
        enrollment_token: Option<&str>,
    ) -> Result<DeviceCredentials, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(existing) = Self::load_credentials(data_dir).await {
            return Ok(existing);
        }

        let creds = CloudApiClient::enroll(client, cloud_url, None, enrollment_token, None).await?;
        Self::save_credentials(data_dir, &creds).await?;
        info!("[CloudReporter] Workstation enrolled successfully [Device ID: {}]", creds.device_id);
        Ok(creds)
    }

    pub async fn login(
        cloud_url: &str,
        enrollment_token: Option<&str>,
        custom_name: Option<&str>,
        data_dir: &str,
    ) -> Result<DeviceCredentials, Box<dyn std::error::Error + Send + Sync>> {
        let client = Self::build_http_client();
        let creds = CloudApiClient::enroll(&client, cloud_url, custom_name, enrollment_token, Some("cli_login")).await?;
        Self::save_credentials(data_dir, &creds).await?;
        Ok(creds)
    }

    pub async fn logout(data_dir: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        CredentialStore::purge(data_dir).await
    }

    pub async fn whoami(data_dir: &str) -> Option<DeviceCredentials> {
        Self::load_credentials(data_dir).await
    }

    pub async fn sync_inventory_and_heartbeat(
        state: &AppState,
    ) -> Result<CloudSyncResult, Box<dyn std::error::Error + Send + Sync>> {
        let saved_creds = Self::load_credentials(&state.config.data_dir).await;
        let cloud_url_owned = state.config.cloud_url.clone()
            .or_else(|| saved_creds.as_ref().and_then(|c| c.cloud_url.clone()));

        let cloud_url = match cloud_url_owned.as_deref() {
            Some(u) if !u.trim().is_empty() => u.trim(),
            _ => {
                return Ok(CloudSyncResult {
                    success: false,
                    enrolled: false,
                    device_id: "".to_string(),
                    browsers_synced: 0,
                    heartbeat_sent: false,
                    message: "Specter Cloud URL not configured (standalone mode)".to_string(),
                });
            }
        };

        let client = Self::build_http_client();

        let creds = Self::ensure_enrolled(
            &client,
            cloud_url,
            &state.config.data_dir,
            state.config.cloud_enrollment_token.as_deref(),
        ).await?;

        // 1. Gather all local browser profiles from SQLite
        let local_browsers = {
            let db = state.db.lock().await;
            db.browsers().get_browsers(None, None, None).unwrap_or_default()
        };

        let active_sessions = {
            crate::core::browser::manager::browser_sessions().read().await.clone()
        };

        let browsers_payload: Vec<serde_json::Value> = local_browsers.into_iter().map(|b| {
            let is_running = active_sessions.contains_key(&b.id);
            serde_json::json!({
                "id": b.id,
                "name": b.name,
                "browserType": "chromium",
                "status": if is_running { "running" } else { "idle" },
                "userAgent": b.user_agent,
                "timezone": b.timezone,
                "proxy": b.proxy
            })
        }).collect();

        let browsers_count = browsers_payload.len();

        let env_key = std::env::var("SPECTER_API_KEY").ok();
        let anon_key = creds.api_key.as_deref()
            .or(env_key.as_deref())
            .unwrap_or(DEFAULT_SUPABASE_ANON_KEY);

        // 2. Report Browser Inventory to Central Hub
        let report_url = format!("{}/rest/v1/rpc/report_browser_inventory", cloud_url.trim_end_matches('/'));
        let report_res = client.post(&report_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .json(&serde_json::json!({
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token,
                "p_browsers": browsers_payload
            }))
            .send()
            .await;

        let browsers_synced = match report_res {
            Ok(res) if res.status().is_success() => {
                info!("[CloudReporter] Synced {} local browser profiles to Specter Cloud", browsers_count);
                browsers_count
            }
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                warn!("[CloudReporter] Failed to sync browser inventory: {}", text);
                0
            }
            Err(e) => {
                warn!("[CloudReporter] Network error syncing browser inventory: {}", e);
                0
            }
        };

        // 3. Send Heartbeat & Live Telemetry
        let mut sys = sysinfo::System::new_all();
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let active_jobs_count = state.active_jobs.read().await.len() as i32;
        let cpu_usage = sys.global_cpu_usage();
        let ram_used = (sys.used_memory() / (1024 * 1024)) as f64;
        let ram_total = (sys.total_memory() / (1024 * 1024)) as f64;

        let heartbeat_url = format!("{}/rest/v1/rpc/heartbeat", cloud_url.trim_end_matches('/'));
        let heartbeat_res = client.post(&heartbeat_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .json(&serde_json::json!({
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token,
                "p_active_jobs": active_jobs_count,
                "p_telemetry": {
                    "cpu_usage": cpu_usage,
                    "ram_used_mb": ram_used,
                    "ram_total_mb": ram_total,
                    "local_browsers_count": browsers_count
                }
            }))
            .send()
            .await;

        let heartbeat_sent = match heartbeat_res {
            Ok(res) if res.status().is_success() => true,
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                warn!("[CloudReporter] Heartbeat rejected: {}", text);
                false
            }
            Err(e) => {
                warn!("[CloudReporter] Network error sending heartbeat: {}", e);
                false
            }
        };

        Ok(CloudSyncResult {
            success: browsers_synced > 0 || heartbeat_sent,
            enrolled: true,
            device_id: creds.device_id,
            browsers_synced,
            heartbeat_sent,
            message: format!("Successfully synced {} browsers and heartbeat to Specter Cloud", browsers_synced),
        })
    }

    pub fn start_background_loop(state: AppState) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval_secs = state.config.cloud_heartbeat_interval_secs.max(10);
            
            let saved_creds = Self::load_credentials(&state.config.data_dir).await;
            let active_url = state.config.cloud_url.clone()
                .or_else(|| saved_creds.as_ref().and_then(|c| c.cloud_url.clone()));

            if active_url.is_none() {
                info!("[CloudReporter] No SPECTER_CLOUD_URL or saved login found. Running in offline/standalone mode.");
                return;
            }

            info!(
                "[CloudReporter] Cloud Telemetry Reporter active. Sync interval: {}s. Cloud: {}",
                interval_secs,
                active_url.as_deref().unwrap_or("none")
            );

            // Initial sync after 3 seconds startup grace period
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            if let Err(e) = Self::sync_inventory_and_heartbeat(&state).await {
                warn!("[CloudReporter] Initial cloud sync failed: {}", e);
            }

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
                if let Err(e) = Self::sync_inventory_and_heartbeat(&state).await {
                    warn!("[CloudReporter] Scheduled cloud sync failed: {}", e);
                }
            }
        })
    }
}
