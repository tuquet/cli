//! HTTP surface for the local encrypted **Secret Store** (schema `secrets`).
//!
//! Route naming follows the database schema / table names (see
//! `docs/en/admin/specs/SPEC-core-route-foundation.md` §5.1):
//!
//! - `/secrets`                       → schema-level status
//! - `/secrets/credentials`           → table `secrets.credentials`
//! - `/secrets/credentials/{name}`    → one credential row
//! - `/secrets/tenant-keys/rotate`    → table `secrets.tenant_keys` (rotate the tenant DEK)
//!
//! Consumed by the workstation console (`tq desktop`). Values are never returned in
//! plaintext by this API — only names/metadata (zero-knowledge; see SPEC-secret-store).

use axum::{
    Json,
    extract::{Path as AxumPath, State},
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::core::error::AutomationError;
use crate::infrastructure::cloud::SecretClient;
use worker::core::secrets::{DeviceKeypair, SecretStore};

#[derive(Debug, Serialize)]
pub struct SecretRow {
    pub name: String,
    pub key_version: Option<i64>,
}

/// Schema-level status of the Secret Store (`secrets`).
#[derive(Debug, Serialize)]
pub struct SecretStoreStatus {
    pub store_path: String,
    pub device_public_key: Option<String>,
    pub enrolled: bool,
    pub local_count: usize,
    pub cloud_count: usize,
    pub cloud_error: Option<String>,
}

/// Listing of the `secrets.credentials` resource.
#[derive(Debug, Serialize)]
pub struct CredentialList {
    pub local: Vec<SecretRow>,
    pub cloud: Vec<SecretRow>,
    pub cloud_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetSecretRequest {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct SecretOpResult {
    pub ok: bool,
    pub affected: usize,
    pub message: String,
}

fn store() -> SecretStore {
    SecretStore::new()
}

/// GET /api/v1/secrets — schema-level status (no secret material).
pub async fn get_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretStoreStatus>, AutomationError> {
    let s = store();
    let local = s.load_all().map_err(AutomationError::Internal)?;
    let device_public_key = DeviceKeypair::load_or_create(s.dir()).ok().map(|k| k.public_b64());

    let mut cloud_count = 0;
    let mut cloud_error = None;
    let mut enrolled = false;
    match SecretClient::connect(s.dir()).await {
        Ok(client) => {
            enrolled = true;
            match client.remote_list().await {
                Ok(rows) => cloud_count = rows.len(),
                Err(e) => cloud_error = Some(e),
            }
        }
        Err(e) => cloud_error = Some(e),
    }

    Ok(Json(SecretStoreStatus {
        store_path: s.dir().join("secrets.json").display().to_string(),
        device_public_key,
        enrolled,
        local_count: local.len(),
        cloud_count,
        cloud_error,
    }))
}

/// GET /api/v1/secrets/credentials — list `secrets.credentials` (local + cloud names).
pub async fn list_credentials(
    State(_state): State<AppState>,
) -> Result<Json<CredentialList>, AutomationError> {
    let s = store();
    let local = s.load_all().map_err(AutomationError::Internal)?;

    let mut cloud = Vec::new();
    let mut cloud_error = None;
    if let Ok(client) = SecretClient::connect(s.dir()).await {
        match client.remote_list().await {
            Ok(rows) => {
                cloud = rows
                    .into_iter()
                    .map(|r| SecretRow { name: r.name, key_version: Some(r.key_version) })
                    .collect()
            }
            Err(e) => cloud_error = Some(e),
        }
    }

    Ok(Json(CredentialList {
        local: local
            .keys()
            .map(|n| SecretRow { name: n.clone(), key_version: None })
            .collect(),
        cloud,
        cloud_error,
    }))
}

/// POST /api/v1/secrets/credentials — create or update a credential (stored locally, encrypted).
pub async fn set_secret(
    State(_state): State<AppState>,
    Json(req): Json<SetSecretRequest>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    if req.name.trim().is_empty() {
        return Err(AutomationError::BadRequest("Credential name must not be empty".into()));
    }
    let s = store();
    s.set(req.name.trim(), &req.value).map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: 1,
        message: format!("Credential '{}' stored locally (encrypted).", req.name.trim()),
    }))
}

/// DELETE /api/v1/secrets/credentials/{name}
pub async fn delete_secret(
    State(_state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let existed = s.remove(&name).map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: existed,
        affected: usize::from(existed),
        message: if existed {
            format!("Credential '{}' removed.", name)
        } else {
            format!("Credential '{}' not found.", name)
        },
    }))
}

/// POST /api/v1/secrets/credentials/push — upload credentials, client-side encrypted.
pub async fn push_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let map = s.load_all().map_err(AutomationError::Internal)?;
    if map.is_empty() {
        return Ok(Json(SecretOpResult { ok: true, affected: 0, message: "No local credentials to push.".into() }));
    }
    let client = SecretClient::connect(s.dir()).await.map_err(AutomationError::BadRequest)?;
    let n = client.push(&map).await.map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: n,
        message: format!("Pushed {} credential(s) to Cloud (client-side encrypted).", n),
    }))
}

/// POST /api/v1/secrets/credentials/pull — receive the sealed DEK, fetch ciphertext, decrypt.
pub async fn pull_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let client = SecretClient::connect(s.dir()).await.map_err(AutomationError::BadRequest)?;
    let map = client.pull().await.map_err(AutomationError::Internal)?;
    let n = map.len();
    s.save_all(&map).map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: n,
        message: format!("Pulled {} credential(s) from Cloud (device-portable).", n),
    }))
}

/// POST /api/v1/secrets/tenant-keys/rotate — rotate the tenant DEK (`secrets.tenant_keys`).
pub async fn rotate_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let client = SecretClient::connect(s.dir()).await.map_err(AutomationError::BadRequest)?;
    let n = client.rotate().await.map_err(AutomationError::Internal)? as usize;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: n,
        message: format!("Rotated tenant DEK — re-encrypted {} credential(s).", n),
    }))
}
