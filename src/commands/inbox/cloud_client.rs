use std::time::{Duration, Instant};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::commands::inbox::parser::EmailParser;
use crate::commands::inbox::storage::{EmailRecord, InboxStorage, LinkRecord, OtpRecord};
use crate::config::inbox::InboxConfig;
use crate::infrastructure::cloud::MachineFingerprint;

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("Reqwest HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Supabase cloud not configured (set SUPABASE_URL and SUPABASE_ANON_KEY in ~/.specter/inbox/inbox.json)")]
    NotConfigured,
    #[error("Local storage error: {0}")]
    Storage(#[from] crate::commands::inbox::storage::StorageError),
    #[error("Cloud error response ({status}): {body}")]
    Api { status: u16, body: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudOtpRow {
    pub id: String,
    pub email_id: String,
    pub recipient: String,
    pub otp_code: String,
    pub service_name: Option<String>,
    pub expires_at: Option<String>,
    pub consumed_at: Option<String>,
    pub consumed_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEmailRow {
    pub id: String,
    pub message_id: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub domain: String,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub received_at: String,
    pub raw_headers: Option<serde_json::Value>,
    pub created_at: String,
}

pub struct InboxCloudClient {
    client: reqwest::Client,
    supabase_url: String,
    supabase_anon_key: String,
    device_id: Option<String>,
    tenant_id: Option<String>,
    auto_sync: bool,
}

impl InboxCloudClient {
    pub fn new(config: &InboxConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // 1. Auto-discover from SSOT identity file (~/.specter/system/.identity.json)
        let identity_path = crate::config::canonical_specter_dir().join("system").join(".identity.json");
        let (discovered_url, discovered_key, discovered_dev_id, discovered_tenant_id) = if identity_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&identity_path) {
                if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content) {
                    (
                        creds.get("cloud_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        creds.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        creds.get("device_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        creds.get("tenant_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                } else {
                    (None, None, None, None)
                }
            } else {
                (None, None, None, None)
            }
        } else {
            (None, None, None, None)
        };

        // 2. Cascade priority: inbox.json -> ENV -> identity.json -> Dev Default
        let supabase_url = config
            .supabase_url
            .clone()
            .or_else(|| std::env::var("SPECTER_CLOUD_URL").ok())
            .or_else(|| std::env::var("SUPABASE_URL").ok())
            .or(discovered_url)
            .unwrap_or_else(|| crate::constants::DEFAULT_DEV_SUPABASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        let supabase_anon_key = config
            .supabase_anon_key
            .clone()
            .or_else(|| std::env::var("SPECTER_API_KEY").ok())
            .or_else(|| std::env::var("SUPABASE_ANON_KEY").ok())
            .or(discovered_key)
            .unwrap_or_else(|| crate::infrastructure::cloud::DEFAULT_SUPABASE_ANON_KEY.to_string());

        Self {
            client,
            supabase_url,
            supabase_anon_key,
            device_id: discovered_dev_id,
            tenant_id: discovered_tenant_id,
            auto_sync: config.cloud_sync,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.supabase_url.is_empty()
            && !self.supabase_url.contains("<YOUR_PROJECT_ID>")
            && !self.supabase_anon_key.is_empty()
            && !self.supabase_anon_key.contains("<YOUR_ANON_KEY>")
            && self.supabase_anon_key != crate::infrastructure::cloud::DEFAULT_SUPABASE_ANON_KEY
    }

    pub fn supabase_url(&self) -> &str {
        &self.supabase_url
    }

    pub fn supabase_anon_key(&self) -> &str {
        &self.supabase_anon_key
    }

    pub fn device_id(&self) -> Option<&str> {
        self.device_id.as_deref()
    }

    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    pub fn auto_sync(&self) -> bool {
        self.auto_sync
    }

    pub async fn ping(&self) -> Result<u16, CloudError> {
        if !self.is_configured() {
            return Err(CloudError::NotConfigured);
        }
        let url = format!("{}/rest/v1/", self.supabase_url);
        let res = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", self.supabase_anon_key))
            .send()
            .await?;
        Ok(res.status().as_u16())
    }

    pub async fn poll_otp(
        &self,
        storage: &InboxStorage,
        recipient: &str,
        timeout_secs: u64,
        mark_consumed: bool,
    ) -> Result<Option<OtpRecord>, CloudError> {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let poll_interval = Duration::from_millis(1500);

        // 1. Initial quick local check
        if let Ok(Some(local_otp)) = storage.get_latest_otp(recipient, true) {
            if mark_consumed {
                let device_id = MachineFingerprint::generate();
                let _ = storage.consume_otp(&local_otp.id, &device_id);
                if self.is_configured() {
                    let _ = self.consume_otp_remote(&local_otp.id, &device_id).await;
                }
            }
            return Ok(Some(local_otp));
        }

        if !self.is_configured() {
            // Offline/Local only polling mode
            while Instant::now() < deadline {
                tokio::time::sleep(poll_interval).await;
                if let Ok(Some(local_otp)) = storage.get_latest_otp(recipient, true) {
                    if mark_consumed {
                        let device_id = MachineFingerprint::generate();
                        let _ = storage.consume_otp(&local_otp.id, &device_id);
                    }
                    return Ok(Some(local_otp));
                }
            }
            return Ok(None);
        }

        // 2. Cloud polling loop
        let url = format!(
            "{}/rest/v1/inbox_otps?recipient=eq.{}&consumed_at=is.null&order=created_at.desc&limit=1",
            self.supabase_url, recipient
        );

        while Instant::now() < deadline {
            let res = self
                .client
                .get(&url)
                .header("apikey", &self.supabase_anon_key)
                .header("Authorization", format!("Bearer {}", self.supabase_anon_key))
                .header("Accept", "application/json")
                .send()
                .await;

            match res {
                Ok(resp) if resp.status().is_success() => {
                    let rows: Vec<CloudOtpRow> = resp.json().await.unwrap_or_default();
                    if let Some(row) = rows.into_iter().next() {
                        let otp_rec = OtpRecord {
                            id: row.id.clone(),
                            email_id: row.email_id.clone(),
                            recipient: row.recipient.clone(),
                            otp_code: row.otp_code.clone(),
                            service_name: row.service_name.clone(),
                            expires_at: row.expires_at.clone(),
                            consumed_at: row.consumed_at.clone(),
                            consumed_by: row.consumed_by.clone(),
                            created_at: row.created_at.clone(),
                        };

                        // Cache in local SQLite
                        let _ = storage.save_otp(&otp_rec);

                        if mark_consumed {
                            let device_id = MachineFingerprint::generate();
                            let _ = storage.consume_otp(&otp_rec.id, &device_id);
                            let _ = self.consume_otp_remote(&otp_rec.id, &device_id).await;
                        }

                        return Ok(Some(otp_rec));
                    }
                }
                _ => {}
            }

            tokio::time::sleep(poll_interval).await;
        }

        Ok(None)
    }

    pub async fn consume_otp_remote(&self, otp_id: &str, device_id: &str) -> Result<bool, CloudError> {
        if !self.is_configured() {
            return Err(CloudError::NotConfigured);
        }

        let url = format!("{}/rest/v1/rpc/consume_otp", self.supabase_url);
        let payload = serde_json::json!({
            "p_otp_id": otp_id,
            "p_device_id": device_id
        });

        let res = self
            .client
            .post(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", self.supabase_anon_key))
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status: 400,
                body,
            });
        }

        Ok(true)
    }

    pub async fn sync_recent(&self, storage: &InboxStorage, limit: usize) -> Result<usize, CloudError> {
        if !self.is_configured() {
            return Err(CloudError::NotConfigured);
        }

        let url = format!(
            "{}/rest/v1/inbox_emails?order=received_at.desc&limit={}",
            self.supabase_url, limit
        );

        let res = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", self.supabase_anon_key))
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let rows: Vec<CloudEmailRow> = res.json().await.unwrap_or_default();
        let mut synced_count = 0;

        for row in rows {
            let email_rec = EmailRecord {
                id: row.id.clone(),
                message_id: row.message_id.clone(),
                sender: row.sender.clone(),
                recipient: row.recipient.clone(),
                domain: row.domain.clone(),
                subject: row.subject.clone(),
                body_text: row.body_text.clone(),
                body_html: row.body_html.clone(),
                received_at: row.received_at.clone(),
                raw_headers: row.raw_headers.map(|v| v.to_string()),
                created_at: row.created_at.clone(),
            };

            let _ = storage.save_email(&email_rec);

            // Parse for OTPs and Links and cache locally
            let parsed = EmailParser::parse(
                &email_rec.sender,
                email_rec.subject.as_deref(),
                email_rec.body_text.as_deref(),
                email_rec.body_html.as_deref(),
            );

            for otp in parsed.otps {
                let otp_rec = OtpRecord {
                    id: format!("{}-{}", email_rec.id, otp.code),
                    email_id: email_rec.id.clone(),
                    recipient: email_rec.recipient.clone(),
                    otp_code: otp.code,
                    service_name: otp.service.or(parsed.detected_service.clone()),
                    expires_at: None,
                    consumed_at: None,
                    consumed_by: None,
                    created_at: email_rec.received_at.clone(),
                };
                let _ = storage.save_otp(&otp_rec);
            }

            for link in parsed.links {
                let link_rec = LinkRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    email_id: email_rec.id.clone(),
                    recipient: email_rec.recipient.clone(),
                    url: link.url,
                    link_type: link.link_type,
                    created_at: email_rec.received_at.clone(),
                };
                let _ = storage.save_link(&link_rec);
            }

            synced_count += 1;
        }

        Ok(synced_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_client_initialization_defaults() {
        let config = InboxConfig::default();
        let client = InboxCloudClient::new(&config);
        // Should resolve either to device identity or fallback dev default
        assert!(!client.supabase_url().is_empty());
        assert!(!client.supabase_anon_key().is_empty());
    }

    #[test]
    fn test_cloud_client_explicit_override() {
        let config = InboxConfig {
            supabase_url: Some("https://custom.supabase.co".to_string()),
            supabase_anon_key: Some("custom-key-123".to_string()),
            ..Default::default()
        };

        let client = InboxCloudClient::new(&config);
        assert_eq!(client.supabase_url(), "https://custom.supabase.co");
        assert_eq!(client.supabase_anon_key(), "custom-key-123");
        assert!(client.is_configured());
    }
}

