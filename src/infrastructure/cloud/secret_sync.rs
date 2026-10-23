//! Cloud Secret Store client — device-authenticated sync of `{{secrets.*}}` values.
//!
//! Shared by the `tq secret` CLI command **and** the workstation TUI (`tq tui` → SECRETS tab).
//!
//! Zero-knowledge over wire: values are XChaCha20-Poly1305 encrypted client-side with the
//! tenant DEK; the DEK itself is received as a libsodium sealed box (X25519) bound to this
//! device's public key. The server never sees a plaintext value.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use std::collections::BTreeMap;
use std::path::Path;

use super::credential_store::{CredentialStore, DEFAULT_SUPABASE_ANON_KEY};
use crate::config::AppConfig;
use worker::core::secrets::{DeviceKeypair, aead_decrypt, aead_encrypt, unseal_box};

fn decode_b64_clean(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s.replace(['\n', '\r', ' '], "")).map_err(|e| e.to_string())
}

/// A secret row as exposed by the cloud (metadata only, never the plaintext).
#[derive(Debug, Clone)]
pub struct RemoteSecret {
    pub name: String,
    pub key_version: i64,
}

pub struct SecretClient {
    client: reqwest::Client,
    cloud_url: String,
    anon_key: String,
    device_id: String,
    device_token: String,
    pub keypair: DeviceKeypair,
}

impl SecretClient {
    /// Builds a client from the enrolled device identity (`~/.tq/system/.identity.json`)
    /// and the local device keypair (`<store_dir>/.device_key`).
    pub async fn connect(store_dir: &Path) -> Result<Self, String> {
        let cfg = AppConfig::load();
        let creds = CredentialStore::load(&cfg.data_dir)
            .await
            .ok_or_else(|| "Workstation not enrolled with Cloud. Run `tq auth login` first.".to_string())?;

        let cloud_url = cfg
            .cloud_url
            .clone()
            .or(creds.cloud_url.clone())
            .unwrap_or_else(|| "https://dswhacsoaxgpfnkaxnhz.supabase.co".to_string());
        let anon_key = creds
            .api_key
            .clone()
            .or_else(|| std::env::var("SPECTER_API_KEY").ok())
            .unwrap_or_else(|| DEFAULT_SUPABASE_ANON_KEY.to_string());
        let keypair = DeviceKeypair::load_or_create(store_dir)?;

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .map_err(|e| e.to_string())?,
            cloud_url,
            anon_key,
            device_id: creds.device_id,
            device_token: creds.device_token,
            keypair,
        })
    }

    async fn rpc(
        &self,
        fn_name: &str,
        mut body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!(
            "{}/rest/v1/rpc/{}",
            self.cloud_url.trim_end_matches('/'),
            fn_name
        );
        body["p_device_id"] = serde_json::Value::String(self.device_id.clone());
        body["p_device_token"] = serde_json::Value::String(self.device_token.clone());

        let res = self
            .client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", self.anon_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Cloud '{}' failed ({}): {}", fn_name, status, text));
        }
        Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
    }

    /// Registers this device's public key (idempotent) and returns the tenant DEK.
    pub async fn register_and_get_dek(&self) -> Result<[u8; 32], String> {
        let raw = self
            .rpc(
                "secret_register_device",
                serde_json::json!({ "p_public_key": self.keypair.public_b64() }),
            )
            .await?;
        let sealed_b64 = raw.as_str().ok_or("register_device returned no sealed DEK")?;
        let sealed = decode_b64_clean(sealed_b64)?;
        let dek = unseal_box(&self.keypair.secret, &self.keypair.public, &sealed)
            .map_err(|e| format!("Failed to unseal DEK: {}", e))?;
        let mut out = [0u8; 32];
        if dek.len() != 32 {
            return Err("Unsealed DEK has wrong length".to_string());
        }
        out.copy_from_slice(&dek);
        Ok(out)
    }

    /// Pushes the given secrets, encrypted client-side. Returns the count pushed.
    pub async fn push(&self, map: &BTreeMap<String, String>) -> Result<usize, String> {
        let dek = self.register_and_get_dek().await?;
        let mut n = 0usize;
        for (name, value) in map {
            let (ct, nonce) = aead_encrypt(&dek, value.as_bytes());
            self.rpc(
                "secret_put",
                serde_json::json!({
                    "p_name": name,
                    "p_ciphertext": B64.encode(&ct),
                    "p_nonce": B64.encode(nonce),
                }),
            )
            .await?;
            n += 1;
        }
        Ok(n)
    }

    /// Fetches and decrypts all secrets for this tenant.
    pub async fn pull(&self) -> Result<BTreeMap<String, String>, String> {
        let dek = self.register_and_get_dek().await?;
        let rows = self.rpc("secret_fetch", serde_json::json!({})).await?;
        let arr = rows.as_array().cloned().unwrap_or_default();
        let mut map = BTreeMap::new();
        for row in arr {
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let ct_b64 = row.get("ciphertext").and_then(|v| v.as_str()).unwrap_or("");
            let nonce_b64 = row.get("nonce").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let ct = decode_b64_clean(ct_b64)?;
            let nonce = decode_b64_clean(nonce_b64)?;
            let pt = aead_decrypt(&dek, &ct, &nonce)?;
            map.insert(name.to_string(), String::from_utf8(pt).map_err(|e| e.to_string())?);
        }
        Ok(map)
    }

    /// Lists cloud secrets (names + key_version only).
    pub async fn remote_list(&self) -> Result<Vec<RemoteSecret>, String> {
        let rows = self.rpc("secret_list", serde_json::json!({})).await?;
        let arr = rows.as_array().cloned().unwrap_or_default();
        Ok(arr
            .into_iter()
            .map(|row| RemoteSecret {
                name: row.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                key_version: row.get("key_version").and_then(|v| v.as_i64()).unwrap_or(0),
            })
            .collect())
    }

    /// Rotates the tenant DEK (server re-encrypts values, re-seals for all devices).
    pub async fn rotate(&self) -> Result<i64, String> {
        let raw = self.rpc("secret_rotate", serde_json::json!({})).await?;
        Ok(raw.as_i64().unwrap_or(0))
    }
}
