use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub user_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub user: AuthUser,
}

#[derive(Debug, Deserialize)]
struct GoTrueError {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub struct SupabaseAuthClient;

impl SupabaseAuthClient {
    /// Authenticate using email and password against Supabase GoTrue
    pub async fn login_with_password(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthSession, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("{}/auth/v1/token?grant_type=password", cloud_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "email": email.trim(),
            "password": password
        });

        let res = client
            .post(&endpoint)
            .header("apikey", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            let parsed_msg = serde_json::from_str::<GoTrueError>(&err_text)
                .ok()
                .and_then(|e| e.error_description.or(e.msg).or(e.message).or(e.error))
                .unwrap_or(err_text);
            return Err(format!("Supabase authentication failed: {}", parsed_msg).into());
        }

        let mut session: AuthSession = res.json().await?;
        if session.expires_at.is_none() && let Some(in_secs) = session.expires_in {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            session.expires_at = Some(now + in_secs);
        }

        Ok(session)
    }

    /// Request a One-Time Passcode (OTP / Magic Code) sent to the user's email
    pub async fn request_otp(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        email: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("{}/auth/v1/otp", cloud_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "email": email.trim(),
            "create_user": false
        });

        let res = client
            .post(&endpoint)
            .header("apikey", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            let parsed_msg = serde_json::from_str::<GoTrueError>(&err_text)
                .ok()
                .and_then(|e| e.error_description.or(e.msg).or(e.message).or(e.error))
                .unwrap_or(err_text);
            return Err(format!("Failed to request OTP from Supabase: {}", parsed_msg).into());
        }

        Ok(())
    }

    /// Verify the 6-digit One-Time Passcode (OTP) and exchange for session JWT
    pub async fn verify_otp(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        email: &str,
        code: &str,
    ) -> Result<AuthSession, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("{}/auth/v1/verify", cloud_url.trim_end_matches('/'));
        
        // Try type "email" first (standard for OTP codes)
        let payload_email = serde_json::json!({
            "type": "email",
            "email": email.trim(),
            "token": code.trim()
        });

        let res = client
            .post(&endpoint)
            .header("apikey", api_key)
            .header("Content-Type", "application/json")
            .json(&payload_email)
            .send()
            .await?;

        if res.status().is_success() {
            let mut session: AuthSession = res.json().await?;
            if session.expires_at.is_none() && let Some(in_secs) = session.expires_in {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                session.expires_at = Some(now + in_secs);
            }
            return Ok(session);
        }

        // Fallback try type "magiclink" if "email" fails
        let payload_magic = serde_json::json!({
            "type": "magiclink",
            "email": email.trim(),
            "token": code.trim()
        });

        let res_magic = client
            .post(&endpoint)
            .header("apikey", api_key)
            .header("Content-Type", "application/json")
            .json(&payload_magic)
            .send()
            .await?;

        if res_magic.status().is_success() {
            let mut session: AuthSession = res_magic.json().await?;
            if session.expires_at.is_none() && let Some(in_secs) = session.expires_in {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                session.expires_at = Some(now + in_secs);
            }
            return Ok(session);
        }

        let err_text = res.text().await.unwrap_or_default();
        let parsed_msg = serde_json::from_str::<GoTrueError>(&err_text)
            .ok()
            .and_then(|e| e.error_description.or(e.msg).or(e.message).or(e.error))
            .unwrap_or(err_text);
        Err(format!("OTP verification failed: {}", parsed_msg).into())
    }

    /// Validate an existing JWT token and retrieve the user identity
    pub async fn validate_token(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        token: &str,
    ) -> Result<AuthUser, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("{}/auth/v1/user", cloud_url.trim_end_matches('/'));

        let res = client
            .get(&endpoint)
            .header("apikey", api_key)
            .header("Authorization", format!("Bearer {}", token.trim()))
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Invalid or expired access token: {}", err_text).into());
        }

        let user: AuthUser = res.json().await?;
        Ok(user)
    }

    /// Refresh an existing session access token using a refresh token
    pub async fn refresh_access_token(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        refresh_token: &str,
    ) -> Result<AuthSession, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("{}/auth/v1/token?grant_type=refresh_token", cloud_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "refresh_token": refresh_token.trim()
        });

        let res = client
            .post(&endpoint)
            .header("apikey", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Failed to refresh session: {}", err_text).into());
        }

        let mut session: AuthSession = res.json().await?;
        if session.expires_at.is_none() && let Some(in_secs) = session.expires_in {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            session.expires_at = Some(now + in_secs);
        }

        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_session_deserialization() {
        let json = r#"{
            "access_token": "eyJhbGciOiJIUzI1NiIsIn...",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "ref_12345",
            "user": {
                "id": "00000000-0000-0000-0000-000000000001",
                "email": "user@example.com",
                "user_metadata": { "name": "Test User" }
            }
        }"#;

        let session: AuthSession = serde_json::from_str(json).expect("Deserialize AuthSession");
        assert_eq!(session.access_token, "eyJhbGciOiJIUzI1NiIsIn...");
        assert_eq!(session.token_type, "bearer");
        assert_eq!(session.expires_in, Some(3600));
        assert_eq!(session.refresh_token, Some("ref_12345".to_string()));
        assert_eq!(session.user.id, "00000000-0000-0000-0000-000000000001");
        assert_eq!(session.user.email, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_gotrue_error_deserialization_formats() {
        // Format 1: error_description
        let json1 = r#"{"error": "invalid_grant", "error_description": "Invalid login credentials"}"#;
        let err1: GoTrueError = serde_json::from_str(json1).expect("Deserialize err1");
        let msg1 = err1.error_description.or(err1.msg).or(err1.message).or(err1.error);
        assert_eq!(msg1, Some("Invalid login credentials".to_string()));

        // Format 2: msg
        let json2 = r#"{"msg": "Email rate limit exceeded"}"#;
        let err2: GoTrueError = serde_json::from_str(json2).expect("Deserialize err2");
        let msg2 = err2.error_description.or(err2.msg).or(err2.message).or(err2.error);
        assert_eq!(msg2, Some("Email rate limit exceeded".to_string()));

        // Format 3: message
        let json3 = r#"{"message": "User not found"}"#;
        let err3: GoTrueError = serde_json::from_str(json3).expect("Deserialize err3");
        let msg3 = err3.error_description.or(err3.msg).or(err3.message).or(err3.error);
        assert_eq!(msg3, Some("User not found".to_string()));
    }
}

