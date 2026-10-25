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
use utoipa::ToSchema;

use crate::AppState;
use crate::core::error::AutomationError;
use crate::infrastructure::cloud::SecretClient;
use worker::core::secrets::{DeviceKeypair, SecretStore};

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretRow {
    pub name: String,
    pub key_version: Option<i64>,
}

/// Schema-level status of the Secret Store (`secrets`).
#[derive(Debug, Serialize, ToSchema)]
pub struct SecretStoreStatus {
    pub store_path: String,
    pub device_public_key: Option<String>,
    pub enrolled: bool,
    pub local_count: usize,
    pub cloud_count: usize,
    pub cloud_error: Option<String>,
}

/// Listing of the `secrets.credentials` resource.
#[derive(Debug, Serialize, ToSchema)]
pub struct CredentialList {
    pub local: Vec<SecretRow>,
    pub cloud: Vec<SecretRow>,
    pub cloud_error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetSecretRequest {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretOpResult {
    pub ok: bool,
    pub affected: usize,
    pub message: String,
}

fn store() -> SecretStore {
    SecretStore::new()
}

/// GET /api/v1/secrets — schema-level status (no secret material).
#[utoipa::path(
    tag = "Secrets",
    get,
    path = "/api/v1/secrets",
    operation_id = "get_secret_store_status",
    summary = "Secret Store status",
    description = "Returns the local Secret Store status (store path, device X25519 public key, cloud enrollment, counts). Never returns secret material.",
    responses(
        (status = 200, description = "Secret Store status", body = SecretStoreStatus),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretStoreStatus>, AutomationError> {
    let s = store();
    let local = s.load_all().map_err(AutomationError::Internal)?;
    let device_public_key = DeviceKeypair::load_or_create(s.dir())
        .ok()
        .map(|k| k.public_b64());

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
#[utoipa::path(
    tag = "Secrets",
    get,
    path = "/api/v1/secrets/credentials",
    operation_id = "list_credentials",
    summary = "List credentials",
    description = "Lists credential names of the tenant (local + cloud). Values are never returned (schema `secrets.credentials`).",
    responses(
        (status = 200, description = "Credential names", body = CredentialList),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
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
                    .map(|r| SecretRow {
                        name: r.name,
                        key_version: Some(r.key_version),
                    })
                    .collect()
            }
            Err(e) => cloud_error = Some(e),
        }
    }

    Ok(Json(CredentialList {
        local: local
            .keys()
            .map(|n| SecretRow {
                name: n.clone(),
                key_version: None,
            })
            .collect(),
        cloud,
        cloud_error,
    }))
}

/// POST /api/v1/secrets/credentials — create or update a credential (stored locally, encrypted).
#[utoipa::path(
    tag = "Secrets",
    post,
    path = "/api/v1/secrets/credentials",
    operation_id = "set_credential",
    summary = "Create or update a credential",
    description = "Stores a credential locally, encrypted at rest (AES-256-GCM). Table `secrets.credentials`.",
    request_body = SetSecretRequest,
    responses(
        (status = 200, description = "Stored", body = SecretOpResult),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn set_secret(
    State(_state): State<AppState>,
    Json(req): Json<SetSecretRequest>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    if req.name.trim().is_empty() {
        return Err(AutomationError::BadRequest(
            "Credential name must not be empty".into(),
        ));
    }
    let s = store();
    s.set(req.name.trim(), &req.value)
        .map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: 1,
        message: format!(
            "Credential '{}' stored locally (encrypted).",
            req.name.trim()
        ),
    }))
}

/// DELETE /api/v1/secrets/credentials/{name}
#[utoipa::path(
    tag = "Secrets",
    delete,
    path = "/api/v1/secrets/credentials/{name}",
    operation_id = "delete_credential",
    summary = "Delete a credential",
    description = "Removes a credential from the local store. Table `secrets.credentials`.",
    responses(
        (status = 200, description = "Removed", body = SecretOpResult),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
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
#[utoipa::path(
    tag = "Secrets",
    post,
    path = "/api/v1/secrets/credentials/push",
    operation_id = "push_credentials",
    summary = "Push credentials to Cloud",
    description = "Registers the device X25519 key and uploads client-side-encrypted credentials. Table `secrets.credentials`.",
    responses(
        (status = 200, description = "Pushed", body = SecretOpResult),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn push_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let map = s.load_all().map_err(AutomationError::Internal)?;
    if map.is_empty() {
        return Ok(Json(SecretOpResult {
            ok: true,
            affected: 0,
            message: "No local credentials to push.".into(),
        }));
    }
    let client = SecretClient::connect(s.dir())
        .await
        .map_err(AutomationError::BadRequest)?;
    let n = client.push(&map).await.map_err(AutomationError::Internal)?;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: n,
        message: format!(
            "Pushed {} credential(s) to Cloud (client-side encrypted).",
            n
        ),
    }))
}

/// POST /api/v1/secrets/credentials/pull — receive the sealed DEK, fetch ciphertext, decrypt.
#[utoipa::path(
    tag = "Secrets",
    post,
    path = "/api/v1/secrets/credentials/pull",
    operation_id = "pull_credentials",
    summary = "Pull credentials from Cloud",
    description = "Receives the tenant DEK (sealed) and decrypts the tenant credentials locally. Table `secrets.credentials`.",
    responses(
        (status = 200, description = "Pulled", body = SecretOpResult),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn pull_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let client = SecretClient::connect(s.dir())
        .await
        .map_err(AutomationError::BadRequest)?;
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
#[utoipa::path(
    tag = "Secrets",
    post,
    path = "/api/v1/secrets/tenant-keys/rotate",
    operation_id = "rotate_tenant_dek",
    summary = "Rotate the tenant DEK",
    description = "Rotates the tenant DEK, re-encrypts every credential and re-seals the DEK for all registered devices. Table `secrets.tenant_keys`.",
    responses(
        (status = 200, description = "Rotated", body = SecretOpResult),
        (status = 400, description = "Invalid request", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Store error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn rotate_secrets(
    State(_state): State<AppState>,
) -> Result<Json<SecretOpResult>, AutomationError> {
    let s = store();
    let client = SecretClient::connect(s.dir())
        .await
        .map_err(AutomationError::BadRequest)?;
    let n = client.rotate().await.map_err(AutomationError::Internal)? as usize;
    Ok(Json(SecretOpResult {
        ok: true,
        affected: n,
        message: format!("Rotated tenant DEK — re-encrypted {} credential(s).", n),
    }))
}
