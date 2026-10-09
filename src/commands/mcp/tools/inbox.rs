use serde_json::{json, Value};
use crate::config::inbox::InboxConfig;
use crate::commands::inbox::cloud_client::InboxCloudClient;
use crate::commands::inbox::storage::InboxStorage;
use crate::commands::inbox::server::read_pid;

pub async fn execute_specter_inbox_otp(args: &Value) -> Result<String, String> {
    let recipient = args
        .get("recipient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'recipient'".to_string())?;

    let wait = args.get("wait").and_then(|v| v.as_bool()).unwrap_or(true);
    let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);

    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())
        .map_err(|e| format!("Failed to open local storage: {}", e))?;
    let cloud = InboxCloudClient::new(&config);

    let otp_res = if wait {
        cloud
            .poll_otp(&storage, recipient, timeout, true)
            .await
            .map_err(|e| format!("Cloud poll error: {}", e))?
    } else {
        storage
            .get_latest_otp(recipient, true)
            .map_err(|e| format!("Storage error: {}", e))?
    };

    match otp_res {
        Some(otp) => {
            let out = json!({
                "recipient": otp.recipient,
                "otp_code": otp.otp_code,
                "service": otp.service_name.unwrap_or_else(|| "Unknown".to_string()),
                "created_at": otp.created_at,
                "consumed": true
            });
            Ok(serde_json::to_string_pretty(&out).unwrap_or_default())
        }
        None => Err(format!("No OTP found for recipient '{}' within timeout", recipient)),
    }
}

pub async fn execute_specter_inbox_status() -> Result<String, String> {
    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())
        .map_err(|e| format!("Failed to open local storage: {}", e))?;
    let stats = storage.stats().unwrap_or_default();
    let cloud = InboxCloudClient::new(&config);
    let pid = read_pid();

    let out = json!({
        "daemon": {
            "running": pid.is_some(),
            "pid": pid,
            "host": config.host,
            "port": config.port
        },
        "database": {
            "path": InboxConfig::sqlite_path().display().to_string(),
            "total_emails": stats.total_emails,
            "total_otps": stats.total_otps,
            "unconsumed_otps": stats.unconsumed_otps,
            "total_links": stats.total_links
        },
        "cloud": {
            "connected": cloud.is_configured(),
            "sync_enabled": config.cloud_sync
        }
    });

    Ok(serde_json::to_string_pretty(&out).unwrap_or_default())
}
