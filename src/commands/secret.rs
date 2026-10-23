//! `tq secret` — Secret Store (local encrypted + cloud, device-portable, zero-knowledge over wire).
//!
//! Local : `~/.tq/system/secrets.json` (AES-256-GCM) + `~/.tq/system/.device_key` (X25519).
//! Cloud : schema `secrets` (Vault KEK -> per-tenant DEK -> sealed to each device).
//!         Values are XChaCha20-Poly1305 encrypted **client-side**; the server never
//!         receives a plaintext value.
//!
//! Workflows reference values as `{{secrets.<name>}}`.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

use crate::cli::SecretSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud::{CredentialStore, DEFAULT_SUPABASE_ANON_KEY};
use worker::core::secrets::{
    DeviceKeypair, SecretStore, aead_decrypt, aead_encrypt, unseal_box,
};

struct DeviceCtx {
    client: reqwest::Client,
    cloud_url: String,
    anon_key: String,
    device_id: String,
    device_token: String,
    keypair: DeviceKeypair,
}

fn decode_b64_clean(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s.replace(['\n', '\r', ' '], "")).map_err(|e| e.to_string())
}

async fn device_ctx(store: &SecretStore) -> Result<DeviceCtx, Box<dyn std::error::Error>> {
    let cfg = AppConfig::load();
    let creds = CredentialStore::load(&cfg.data_dir)
        .await
        .ok_or("Workstation not enrolled with Cloud. Run `tq auth login` first.")?;

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

    let keypair = DeviceKeypair::load_or_create(store.dir())?;

    Ok(DeviceCtx {
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()?,
        cloud_url,
        anon_key,
        device_id: creds.device_id,
        device_token: creds.device_token,
        keypair,
    })
}

impl DeviceCtx {
    async fn rpc(
        &self,
        fn_name: &str,
        mut body: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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
            .await?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Cloud RPC '{}' failed ({}): {}", fn_name, status, text).into());
        }
        Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
    }

    /// Registers this device's X25519 public key and returns the tenant DEK.
    async fn register_and_get_dek(&self) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        let raw = self
            .rpc(
                "secret_register_device",
                serde_json::json!({ "p_public_key": self.keypair.public_b64() }),
            )
            .await?;
        let sealed_b64 = raw
            .as_str()
            .ok_or("secret_register_device returned no sealed DEK")?;
        let sealed = decode_b64_clean(sealed_b64)?;
        let dek = unseal_box(&self.keypair.secret, &self.keypair.public, &sealed)
            .map_err(|e| format!("Failed to unseal DEK: {}", e))?;
        if dek.len() != 32 {
            return Err("Unsealed DEK has wrong length".into());
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&dek);
        Ok(out)
    }

    /// Re-fetches the sealed DEK for an already-registered device.
    #[allow(dead_code)]
    async fn deliver_dek(&self) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        let raw = self.rpc("secret_deliver_dek", serde_json::json!({})).await?;
        let sealed_b64 = raw
            .as_str()
            .ok_or("Device public key not registered — run `tq secret push` once.")?;
        let sealed = decode_b64_clean(sealed_b64)?;
        let dek = unseal_box(&self.keypair.secret, &self.keypair.public, &sealed)
            .map_err(|e| format!("Failed to unseal DEK: {}", e))?;
        let mut out = [0u8; 32];
        out.copy_from_slice(&dek);
        Ok(out)
    }
}

pub async fn handle(cmd: SecretSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    let store = SecretStore::new();

    match cmd {
        SecretSubcommands::Init => {
            store.ensure_key()?;
            let kp = DeviceKeypair::load_or_create(store.dir())?;
            println!("Secret Store ready.");
            println!("  dir        : {}", store.dir().display());
            println!("  store      : {}", store.dir().join("secrets.json").display());
            println!("  store key  : {} (0600)", store.dir().join(".secret_key").display());
            println!("  device key : {} (0600, X25519)", store.dir().join(".device_key").display());
            println!("  device pub : {}", B64.encode(kp.public));
            println!();
            println!("Add a secret:  tq secret set <name> <value>");
            println!("Sync to Cloud: tq secret push   |  On a new device: tq secret pull");
        }
        SecretSubcommands::Set { name, value } => {
            store.set(&name, &value)?;
            println!("Secret '{}' stored locally (encrypted).", name);
        }
        SecretSubcommands::List { format } => {
            let names = store.list()?;
            if format.eq_ignore_ascii_case("json") {
                println!("{}", serde_json::to_string_pretty(&names)?);
            } else if names.is_empty() {
                println!("No local secrets in {}.", store.dir().display());
            } else {
                println!("Local Secret Store — {} entr(ies) in {}:", names.len(), store.dir().display());
                for n in names {
                    println!("  • {}  →  ********", n);
                }
            }
        }
        SecretSubcommands::Rm { name } => {
            println!(
                "{}",
                if store.remove(&name)? { format!("Secret '{}' removed.", name) } else { format!("Secret '{}' not found.", name) }
            );
        }
        SecretSubcommands::Path => println!("{}", store.dir().join("secrets.json").display()),
        SecretSubcommands::Push => {
            let ctx = device_ctx(&store).await?;
            let dek = ctx.register_and_get_dek().await?;
            let map = store.load_all()?;
            if map.is_empty() {
                println!("No local secrets to push (device key registered).");
                return Ok(());
            }
            let mut n = 0usize;
            for (name, value) in &map {
                let (ct, nonce) = aead_encrypt(&dek, value.as_bytes());
                ctx.rpc(
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
            println!("Pushed {} secret(s) — client-side encrypted (XChaCha20-Poly1305).", n);
        }
        SecretSubcommands::Pull => {
            let ctx = device_ctx(&store).await?;
            // Idempotent: registers this device's public key on first use, then
            // returns the tenant DEK sealed to it.
            let dek = ctx.register_and_get_dek().await?;
            let rows = ctx.rpc("secret_fetch", serde_json::json!({})).await?;
            let arr = rows.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
                return Ok(());
            }
            let mut map = std::collections::BTreeMap::new();
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
                map.insert(name.to_string(), String::from_utf8(pt)?);
            }
            let count = map.len();
            store.save_all(&map)?;
            println!(
                "Pulled {} secret(s) from Cloud into {} (device-portable, no re-entry).",
                count,
                store.dir().display()
            );
        }
        SecretSubcommands::Remote => {
            let ctx = device_ctx(&store).await?;
            let rows = ctx.rpc("secret_list", serde_json::json!({})).await?;
            let arr = rows.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
            } else {
                println!("Cloud Secret Store — {} entr(ies):", arr.len());
                for row in arr {
                    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let kv = row.get("key_version").and_then(|v| v.as_i64()).unwrap_or(0);
                    println!("  • {}  →  ********  (key_version {})", name, kv);
                }
            }
        }
        SecretSubcommands::Rotate => {
            let ctx = device_ctx(&store).await?;
            let raw = ctx.rpc("secret_rotate", serde_json::json!({})).await?;
            let n = raw.as_i64().unwrap_or(0);
            println!("Rotated tenant DEK — re-encrypted {} value(s) and re-sealed for all devices.", n);
        }
    }

    Ok(())
}
