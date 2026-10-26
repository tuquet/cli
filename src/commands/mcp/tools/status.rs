use serde_json::json;

use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;

pub async fn execute_status() -> Result<String, String> {
    let config = AppConfig::load();

    // 1. Cloud
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let cloud_info = if let Some(ref creds) = cloud_creds {
        json!({
            "enrolled": true,
            "device_id": creds.device_id,
            "device_name": creds.name,
            "tenant_id": creds.tenant_id.as_deref().unwrap_or("Personal Workspace"),
            "endpoint": creds.cloud_url.as_deref().unwrap_or("(not configured)")
        })
    } else {
        json!({
            "enrolled": false,
            "message": "Workstation not enrolled with cloud fleet. Run 'tq login' to authenticate."
        })
    };

    // 2. Worker Daemon
    let host = std::env::var("AUTOMATION_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("AUTOMATION_PORT").unwrap_or_else(|_| config.server_port.to_string());
    let daemon_url = format!("http://{}:{}", host, port);
    let health_url = format!("{}/api/v1/health", daemon_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
        .map_err(|e| e.to_string())?;

    let worker_online = match client.get(&health_url).send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    };

    let worker_info = json!({
        "online": worker_online,
        "endpoint": daemon_url,
        "port": port
    });

    // 3. Browser
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let browser_info = json!({
        "installed": browser_status.installed,
        "executable_path": browser_status.executable_path,
        "pinned_version": browser_status.pinned_version,
        "platform": browser_status.platform,
        "size_mb": browser_status.size_mb
    });

    let result = json!({
        "ecosystem": "tuquet",
        "cloud": cloud_info,
        "worker": worker_info,
        "browser": browser_info
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

pub async fn execute_cloud_whoami() -> Result<String, String> {
    let config = AppConfig::load();
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;

    if let Some(creds) = cloud_creds {
        let res = json!({
            "enrolled": true,
            "device_id": creds.device_id,
            "name": creds.name,
            "tenant_id": creds.tenant_id,
            "cloud_url": creds.cloud_url,
            "enrolled_at": creds.registered_at
        });
        Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
    } else {
        let res = json!({
            "enrolled": false,
            "message": "Workstation is not paired with Cloud. Use 'tq login' to connect."
        });
        Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
    }
}

pub async fn execute_browser_status() -> Result<String, String> {
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let res = json!({
        "installed": browser_status.installed,
        "executable_path": browser_status.executable_path,
        "pinned_version": browser_status.pinned_version,
        "platform": browser_status.platform,
        "size_mb": browser_status.size_mb
    });
    Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
}
