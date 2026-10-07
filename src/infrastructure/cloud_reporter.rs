use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use crate::AppState;

pub const DEFAULT_SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImRzd2hhY3NvYXhncGZua2F4bmh6Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3OTAyOTEzMzcsImV4cCI6MjEwNTg2NzMzN30.QRdxE3CPCF8CtliOtSUcSFO-jbKi99uM2AKlJgKt6RQ";

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

pub struct CloudReporter;

impl CloudReporter {
    pub fn get_credentials_path(_data_dir: &str) -> PathBuf {
        crate::config::canonical_tuquet_dir().join("system").join(".identity.json")
    }

    pub fn generate_machine_fingerprint() -> String {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string());
        let os_name = sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
        let os_ver = sysinfo::System::os_version().unwrap_or_else(|| "unknown".to_string());
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "user".to_string());

        let raw = format!("{}:{}:{}:{}", hostname, os_name, os_ver, username);
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        format!("fp_{}", hex::encode(&hasher.finalize()[..16]))
    }

    pub fn build_http_client() -> reqwest::Client {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15));

        if std::env::var("ALL_PROXY").is_err() && std::env::var("all_proxy").is_err() && std::env::var("HTTPS_PROXY").is_err() && std::env::var("https_proxy").is_err() {
            if std::net::TcpStream::connect_timeout(&"127.0.0.1:1080".parse().unwrap(), std::time::Duration::from_millis(50)).is_ok() {
                if let Ok(proxy) = reqwest::Proxy::all("socks5://127.0.0.1:1080") {
                    builder = builder.proxy(proxy);
                }
            } else if std::net::TcpStream::connect_timeout(&"127.0.0.1:8118".parse().unwrap(), std::time::Duration::from_millis(50)).is_ok() {
                if let Ok(proxy) = reqwest::Proxy::all("http://127.0.0.1:8118") {
                    builder = builder.proxy(proxy);
                }
            }
        }
        builder.build().unwrap_or_default()
    }

    pub async fn load_credentials(data_dir: &str) -> Option<DeviceCredentials> {
        let path = Self::get_credentials_path(data_dir);
        if path.exists()
            && let Ok(content) = tokio::fs::read_to_string(&path).await
                && let Ok(creds) = serde_json::from_str::<DeviceCredentials>(&content) {
                    return Some(creds);
                }
        None
    }

    pub async fn save_credentials(data_dir: &str, creds: &DeviceCredentials) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::get_credentials_path(data_dir);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let json = serde_json::to_string_pretty(creds)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
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

        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "PC-WORKSTATION".to_string());
        let fingerprint = Self::generate_machine_fingerprint();
        let os_info = format!("{} {}", sysinfo::System::name().unwrap_or_default(), sysinfo::System::os_version().unwrap_or_default());
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();

        let cpu_cores = sys.cpus().len().max(1) as i32;
        let ram_mb = (sys.total_memory() / (1024 * 1024)).max(256) as i32;

        let payload = serde_json::json!({
            "p_machine_fingerprint": fingerprint,
            "p_name": hostname,
            "p_os_info": os_info,
            "p_cpu_cores": cpu_cores,
            "p_ram_mb": ram_mb,
            "p_capabilities": ["chromium", "automa", "gui", "worker", "runner"],
            "p_metadata": {
                "automa_version": env!("CARGO_PKG_VERSION"),
                "architecture": std::env::consts::ARCH
            },
            "p_enrollment_token": enrollment_token
        });

        let target_url = format!("{}/rest/v1/rpc/enroll_device", cloud_url.trim_end_matches('/'));
        info!("[CloudReporter] Enrolling workstation with Tuquet Cloud at: {}", target_url);

        let anon_key = std::env::var("TUQUET_API_KEY").unwrap_or_else(|_| DEFAULT_SUPABASE_ANON_KEY.to_string());
        let res = client.post(&target_url)
            .header("apikey", &anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Failed to enroll device with Tuquet Cloud: {}", err_text).into());
        }

        let body: serde_json::Value = res.json().await?;
        let device_id = body.get("device_id").and_then(|v| v.as_str()).ok_or("Missing device_id in enrollment response")?.to_string();
        let device_token = body.get("device_token").and_then(|v| v.as_str()).ok_or("Missing device_token in enrollment response")?.to_string();
        let tenant_id = body.get("tenant_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let assigned_name = body.get("name").and_then(|v| v.as_str()).unwrap_or(&hostname).to_string();

        let creds = DeviceCredentials {
            cloud_url: Some(cloud_url.to_string()),
            device_id,
            device_token,
            tenant_id,
            name: assigned_name,
            api_key: Some(anon_key),
            machine_fingerprint: fingerprint,
            registered_at: Some(epoch_secs_now()),
        };

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

        let hostname = custom_name.unwrap_or("").trim();
        let default_host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "PC-WORKSTATION".to_string());
        let device_name = if hostname.is_empty() { default_host } else { hostname.to_string() };

        let fingerprint = Self::generate_machine_fingerprint();
        let os_info = format!("{} {}", sysinfo::System::name().unwrap_or_default(), sysinfo::System::os_version().unwrap_or_default());
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();

        let cpu_cores = sys.cpus().len().max(1) as i32;
        let ram_mb = (sys.total_memory() / (1024 * 1024)).max(256) as i32;

        let payload = serde_json::json!({
            "p_machine_fingerprint": fingerprint,
            "p_name": device_name,
            "p_os_info": os_info,
            "p_cpu_cores": cpu_cores,
            "p_ram_mb": ram_mb,
            "p_capabilities": ["chromium", "automa", "gui", "worker", "runner"],
            "p_metadata": {
                "automa_version": env!("CARGO_PKG_VERSION"),
                "architecture": std::env::consts::ARCH,
                "login_source": "cli_login"
            },
            "p_enrollment_token": enrollment_token
        });

        let target_url = format!("{}/rest/v1/rpc/enroll_device", cloud_url.trim_end_matches('/'));
        let anon_key = std::env::var("TUQUET_API_KEY").unwrap_or_else(|_| DEFAULT_SUPABASE_ANON_KEY.to_string());
        let res = client.post(&target_url)
            .header("apikey", &anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .json(&payload)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Cloud enrollment failed (HTTP {}): {}", status, err_text).into());
        }

        let body: serde_json::Value = res.json().await?;
        let device_id = body.get("device_id").and_then(|v| v.as_str()).ok_or("Missing device_id in enrollment response")?.to_string();
        let device_token = body.get("device_token").and_then(|v| v.as_str()).ok_or("Missing device_token in enrollment response")?.to_string();
        let tenant_id = body.get("tenant_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let assigned_name = body.get("name").and_then(|v| v.as_str()).unwrap_or(&device_name).to_string();

        let creds = DeviceCredentials {
            cloud_url: Some(cloud_url.to_string()),
            device_id,
            device_token,
            tenant_id,
            name: assigned_name,
            api_key: Some(anon_key),
            machine_fingerprint: fingerprint,
            registered_at: Some(epoch_secs_now()),
        };

        Self::save_credentials(data_dir, &creds).await?;
        Ok(creds)
    }

    pub async fn logout(data_dir: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::get_credentials_path(data_dir);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            Ok(true)
        } else {
            Ok(false)
        }
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
                    message: "Tuquet Cloud URL not configured (standalone mode)".to_string(),
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

        let env_key = std::env::var("TUQUET_API_KEY").ok();
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
                info!("[CloudReporter] Synced {} local browser profiles to Tuquet Cloud", browsers_count);
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
            message: format!("Successfully synced {} browsers and heartbeat to Tuquet Cloud", browsers_synced),
        })
    }

    pub fn start_background_loop(state: AppState) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval_secs = state.config.cloud_heartbeat_interval_secs.max(10);
            
            let saved_creds = Self::load_credentials(&state.config.data_dir).await;
            let active_url = state.config.cloud_url.clone()
                .or_else(|| saved_creds.as_ref().and_then(|c| c.cloud_url.clone()));

            if active_url.is_none() {
                info!("[CloudReporter] No TUQUET_CLOUD_URL or saved login found. Running in offline/standalone mode.");
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

fn epoch_secs_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
