use tracing::info;
use super::credential_store::{epoch_secs_now, DeviceCredentials, DEFAULT_SUPABASE_ANON_KEY};
use super::fingerprint::MachineFingerprint;

/// HTTP RPC Client for communicating with Specter Cloud control plane
pub struct CloudApiClient;

impl CloudApiClient {
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

    pub async fn enroll(
        client: &reqwest::Client,
        cloud_url: &str,
        custom_name: Option<&str>,
        enrollment_token: Option<&str>,
        login_source: Option<&str>,
        bearer_token: Option<&str>,
    ) -> Result<DeviceCredentials, Box<dyn std::error::Error + Send + Sync>> {
        let hostname = custom_name.unwrap_or("").trim();
        let default_host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "PC-WORKSTATION".to_string());
        let device_name = if hostname.is_empty() { default_host } else { hostname.to_string() };

        let fingerprint = MachineFingerprint::generate();
        let os_info = format!("{} {}", sysinfo::System::name().unwrap_or_default(), sysinfo::System::os_version().unwrap_or_default());
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();

        let cpu_cores = sys.cpus().len().max(1) as i32;
        let ram_mb = (sys.total_memory() / (1024 * 1024)).max(256) as i32;

        let mut metadata = serde_json::json!({
            "automa_version": env!("CARGO_PKG_VERSION"),
            "architecture": std::env::consts::ARCH
        });
        if let Some(src) = login_source {
            metadata["login_source"] = serde_json::Value::String(src.to_string());
        }

        let payload = serde_json::json!({
            "p_machine_fingerprint": fingerprint,
            "p_name": device_name,
            "p_os_info": os_info,
            "p_cpu_cores": cpu_cores,
            "p_ram_mb": ram_mb,
            "p_capabilities": ["chromium", "automa", "gui", "worker", "runner"],
            "p_metadata": metadata,
            "p_enrollment_token": enrollment_token
        });

        let target_url = format!("{}/rest/v1/rpc/enroll_device", cloud_url.trim_end_matches('/'));
        info!("[CloudReporter] Enrolling workstation with Specter Cloud at: {}", target_url);

        let anon_key = std::env::var("SPECTER_API_KEY")
            .unwrap_or_else(|_| DEFAULT_SUPABASE_ANON_KEY.to_string());
        let auth_header = match bearer_token {
            Some(tok) if !tok.trim().is_empty() => format!("Bearer {}", tok.trim()),
            _ => format!("Bearer {}", anon_key),
        };

        let res = client.post(&target_url)
            .header("apikey", &anon_key)
            .header("Authorization", auth_header)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Failed to enroll device with Specter Cloud: {}", err_text).into());
        }

        let body: serde_json::Value = res.json().await?;
        let device_id = body.get("device_id").and_then(|v| v.as_str()).ok_or("Missing device_id in enrollment response")?.to_string();
        let device_token = body.get("device_token").and_then(|v| v.as_str()).ok_or("Missing device_token in enrollment response")?.to_string();
        let tenant_id = body.get("tenant_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let assigned_name = body.get("name").and_then(|v| v.as_str()).unwrap_or(&device_name).to_string();

        Ok(DeviceCredentials {
            cloud_url: Some(cloud_url.to_string()),
            device_id,
            device_token,
            tenant_id,
            name: assigned_name,
            api_key: Some(anon_key),
            machine_fingerprint: fingerprint,
            registered_at: Some(epoch_secs_now()),
            user_id: None,
            email: None,
            access_token: bearer_token.map(|s| s.to_string()),
            refresh_token: None,
            token_expires_at: None,
            tenant_slug: None,
            tenant_name: None,
            tenant_role: None,
            available_tenants: None,
        })
    }
}
