use axum::Json;
use serde::{Deserialize, Serialize};
use crate::core::crypto::encrypt_secret as crypto_encrypt;
use crate::core::error::AutomaError;
use std::env;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"plaintext": "my_super_secret_token", "passphrase": null}))]
/// Request payload to encrypt a secret string
pub struct EncryptSecretRequest {
    /// Plaintext string to encrypt
    #[schema(example = "my_super_secret_api_key")]
    pub plaintext: String,
    /// Optional encryption master passphrase (falls back to AUTOMA_PASSPHRASE env)
    #[schema(example = "optional_master_password")]
    pub passphrase: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"encryptedSecret": "enc:aes256:d8f7a6b5..."}))]
/// Response containing ciphertext secret
pub struct EncryptSecretResponse {
    /// Base64 encrypted secret ciphertext
    pub encrypted_secret: String,
}

#[utoipa::path(
    tag = "Secrets",
    post,
    path = "/api/v1/secrets/encrypt",
    operation_id = "encrypt_secret",
    summary = "Encrypt sensitive token or secret with AES-256",
    description = "Encrypts plaintext secrets using PBKDF2 and AES-GCM-256 authenticated encryption.",
    request_body = EncryptSecretRequest,
    responses(
        (status = 200, description = "Secret encrypted successfully", body = EncryptSecretResponse),
        (status = 400, description = "Missing plaintext or passphrase", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Internal encryption failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn encrypt_secret(
    Json(payload): Json<EncryptSecretRequest>,
) -> Result<Json<EncryptSecretResponse>, AutomaError> {
    let passphrase = payload.passphrase.or_else(|| env::var("AUTOMA_PASSPHRASE").ok());
    let pass = passphrase
        .filter(|p| !p.is_empty())
        .ok_or_else(|| AutomaError::BadRequest("Passphrase is required. Provide it in the request or set AUTOMA_PASSPHRASE.".into()))?;

    let plaintext = payload.plaintext;
    let encrypted = tokio::task::spawn_blocking(move || crypto_encrypt(&plaintext, &pass))
        .await
        .map_err(|e| AutomaError::Internal(format!("Encryption task failed: {}", e)))?
        .map_err(|e| AutomaError::Internal(format!("Encryption failed: {}", e)))?;

    Ok(Json(EncryptSecretResponse {
        encrypted_secret: encrypted,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_secret_with_passphrase() {
        let req = EncryptSecretRequest {
            plaintext: "super_secret_token_123".to_string(),
            passphrase: Some("master_pass_abc".to_string()),
        };
        let res = encrypt_secret(Json(req)).await.unwrap();
        assert!(!res.encrypted_secret.is_empty());
        assert!(res.encrypted_secret.contains("Salted__") || res.encrypted_secret.len() > 20);
    }

    #[tokio::test]
    async fn test_encrypt_secret_missing_passphrase_err() {
        unsafe {
            std::env::remove_var("AUTOMA_PASSPHRASE");
        }
        let req = EncryptSecretRequest {
            plaintext: "super_secret_token_123".to_string(),
            passphrase: None,
        };
        let err = encrypt_secret(Json(req)).await.unwrap_err();
        match err {
            AutomaError::BadRequest(msg) => assert!(msg.contains("Passphrase is required")),
            _ => panic!("Expected BadRequest error"),
        }
    }
}
