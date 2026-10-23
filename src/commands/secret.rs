//! `tq secret` — manage the Secret Store (local encrypted + cloud device-portable).
//!
//! Local store : `~/.tq/system/secrets.json` (AES-256-GCM).
//! Cloud store : schema `secrets` (envelope: Vault KEK -> per-tenant DEK -> values),
//!               reached device-authenticated via `public.set_secret / fetch_secrets / list_secrets`.
//!
//! Workflows reference values as `{{secrets.<name>}}` — never persisted in the graph.

use crate::cli::SecretSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud::{CredentialStore, DEFAULT_SUPABASE_ANON_KEY};
use worker::core::secrets::SecretStore;

struct DeviceCtx {
    client: reqwest::Client,
    cloud_url: String,
    anon_key: String,
    device_id: String,
    device_token: String,
}

async fn device_ctx() -> Result<DeviceCtx, Box<dyn std::error::Error>> {
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

    Ok(DeviceCtx {
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()?,
        cloud_url,
        anon_key,
        device_id: creds.device_id,
        device_token: creds.device_token,
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
}

pub async fn handle(cmd: SecretSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    let store = SecretStore::new();

    match cmd {
        SecretSubcommands::Init => {
            store.ensure_key()?;
            println!("Secret Store ready.");
            println!("  dir : {}", store.dir().display());
            println!("  file: {}", store.dir().join("secrets.json").display());
            println!("  key : {} (0600)", store.dir().join(".secret_key").display());
            println!();
            println!("Add a secret:  tq secret set <name> <value>");
            println!("Sync to Cloud: tq secret push   |  Pull on a new device: tq secret pull");
        }
        SecretSubcommands::Set { name, value } => {
            store.set(&name, &value)?;
            println!("Secret '{}' stored (encrypted).", name);
        }
        SecretSubcommands::List { format } => {
            let names = store.list()?;
            if format.eq_ignore_ascii_case("json") {
                println!("{}", serde_json::to_string_pretty(&names)?);
            } else if names.is_empty() {
                println!("No local secrets in {}.", store.dir().display());
            } else {
                println!(
                    "Secret Store — {} entr(ies) in {}",
                    names.len(),
                    store.dir().display()
                );
                for n in names {
                    println!("  • {}  →  ********", n);
                }
            }
        }
        SecretSubcommands::Rm { name } => {
            if store.remove(&name)? {
                println!("Secret '{}' removed.", name);
            } else {
                println!("Secret '{}' not found.", name);
            }
        }
        SecretSubcommands::Path => {
            println!("{}", store.dir().join("secrets.json").display());
        }
        SecretSubcommands::Push => {
            let ctx = device_ctx().await?;
            let map = store.load_all()?;
            if map.is_empty() {
                println!("No local secrets to push.");
                return Ok(());
            }
            let mut n = 0usize;
            for (name, value) in &map {
                ctx.rpc(
                    "set_secret",
                    serde_json::json!({ "p_name": name, "p_value": value }),
                )
                .await?;
                n += 1;
            }
            println!(
                "Pushed {} secret(s) to Cloud (tenant-scoped, encrypted at rest).",
                n
            );
        }
        SecretSubcommands::Pull => {
            let ctx = device_ctx().await?;
            let rows = ctx.rpc("fetch_secrets", serde_json::json!({})).await?;
            let arr = rows.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
                return Ok(());
            }
            let mut map = std::collections::BTreeMap::new();
            for row in arr {
                if let (Some(name), Some(value)) = (
                    row.get("name").and_then(|v| v.as_str()),
                    row.get("value").and_then(|v| v.as_str()),
                ) {
                    map.insert(name.to_string(), value.to_string());
                }
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
            let ctx = device_ctx().await?;
            let rows = ctx.rpc("list_secrets", serde_json::json!({})).await?;
            let arr = rows.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
            } else {
                println!("Cloud Secret Store — {} entr(ies):", arr.len());
                for row in arr {
                    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let upd = row.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
                    println!("  • {}  →  ********  ({})", name, upd);
                }
            }
        }
    }

    Ok(())
}
