//! `tq secret` — Secret Store (local encrypted + cloud, device-portable, zero-knowledge over wire).
//!
//! Local : `~/.tq/system/secrets.json` (AES-256-GCM) + `~/.tq/system/.device_key` (X25519).
//! Cloud : schema `secrets` (Vault KEK -> per-tenant DEK -> sealed to each device).
//!         Values are XChaCha20-Poly1305 encrypted **client-side**; the server never
//!         receives a plaintext value. See SPEC-secret-store.
//!
//! Workflows reference values as `{{secrets@name}}` (Automa dialect).

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

use crate::cli::SecretSubcommands;
use crate::infrastructure::cloud::SecretClient;
use worker::core::secrets::{DeviceKeypair, SecretStore};

/// Reads a secret value from a file (`@path`) or stdin (`-`).
///
/// Multi-line secrets — a Google service-account JSON being the motivating case — are
/// painful (and unsafe: shell history, `ps` output) to pass as a command-line argument.
fn resolve_secret_input(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        return Ok(buf.trim_end().to_string());
    }
    if let Some(path) = value.strip_prefix('@') {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read secret file '{path}': {e}"))?;
        return Ok(content.trim_end().to_string());
    }
    Ok(value.to_string())
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
            let value = resolve_secret_input(&value)?;
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
            println!("{}", if store.remove(&name)? {
                format!("Secret '{}' removed.", name)
            } else {
                format!("Secret '{}' not found.", name)
            });
        }
        SecretSubcommands::Path => println!("{}", store.dir().join("secrets.json").display()),
        SecretSubcommands::Push => {
            let client = SecretClient::connect(store.dir()).await?;
            let map = store.load_all()?;
            if map.is_empty() {
                println!("No local secrets to push.");
                return Ok(());
            }
            let n = client.push(&map).await?;
            println!("Pushed {} secret(s) — client-side encrypted (XChaCha20-Poly1305).", n);
        }
        SecretSubcommands::Pull => {
            let client = SecretClient::connect(store.dir()).await?;
            let map = client.pull().await?;
            if map.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
                return Ok(());
            }
            let count = map.len();
            store.save_all(&map)?;
            println!("Pulled {} secret(s) from Cloud into {} (device-portable, no re-entry).", count, store.dir().display());
        }
        SecretSubcommands::Remote => {
            let client = SecretClient::connect(store.dir()).await?;
            let rows = client.remote_list().await?;
            if rows.is_empty() {
                println!("No secrets stored in Cloud for this tenant.");
            } else {
                println!("Cloud Secret Store — {} entr(ies):", rows.len());
                for r in rows {
                    println!("  • {}  →  ********  (key_version {})", r.name, r.key_version);
                }
            }
        }
        SecretSubcommands::Rotate => {
            let client = SecretClient::connect(store.dir()).await?;
            let n = client.rotate().await?;
            println!("Rotated tenant DEK — re-encrypted {} value(s) and re-sealed for all devices.", n);
        }
    }

    Ok(())
}
