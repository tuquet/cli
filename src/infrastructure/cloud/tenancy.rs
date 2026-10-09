use reqwest::Client;
use serde_json::Value;
use super::credential_store::{CredentialStore, TenantInfo};
use super::client::CloudApiClient;
use super::supabase_auth::SupabaseAuthClient;

pub struct TenancyManager;

impl TenancyManager {
    /// Fetch all workspaces/tenants the authenticated user belongs to via Supabase PostgREST
    pub async fn fetch_user_tenants(
        client: &Client,
        cloud_url: &str,
        api_key: &str,
        access_token: &str,
        user_id: &str,
    ) -> Result<Vec<TenantInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let base = cloud_url.trim_end_matches('/');

        // 1. Fetch tenants filtered by RLS
        let tenants_url = format!("{}/rest/v1/tenants?select=id,slug,name,status,created_by", base);
        let res = client
            .get(&tenants_url)
            .header("apikey", api_key)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Failed to fetch tenants from Supabase: {}", err_text).into());
        }

        let tenants_json: Vec<Value> = res.json().await?;

        // 2. Fetch user's membership and roles in tenants
        let members_url = format!(
            "{}/rest/v1/tenant_members?select=tenant_id,status,member_roles(role_id,roles(name,display_name))&user_id=eq.{}",
            base, user_id
        );
        let member_res = client
            .get(&members_url)
            .header("apikey", api_key)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await;

        let mut role_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        if let Ok(m_res) = member_res
            && m_res.status().is_success()
            && let Ok(members_json) = m_res.json::<Vec<Value>>().await {
                for m in members_json {
                    if let Some(tid) = m.get("tenant_id").and_then(|v| v.as_str()) {
                        // Extract role name if present
                        if let Some(mr_array) = m.get("member_roles").and_then(|v| v.as_array()) {
                            for mr in mr_array {
                                if let Some(role_obj) = mr.get("roles")
                                    && let Some(name) = role_obj.get("name").and_then(|v| v.as_str()) {
                                        role_map.insert(tid.to_string(), name.to_string());
                                        break;
                                    }
                            }
                        }
                    }
                }
            }

        let mut results = Vec::new();
        for t in tenants_json {
            let id = t.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let slug = t.get("slug").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let status = t.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());
            let created_by = t.get("created_by").and_then(|v| v.as_str()).unwrap_or_default();

            // Default role is 'owner' if created_by matches user_id, or mapped role, or 'member'
            let role = if created_by == user_id {
                Some("owner".to_string())
            } else {
                role_map.get(&id).cloned().or_else(|| Some("member".to_string()))
            };

            results.push(TenantInfo {
                id,
                slug,
                name,
                role,
                status,
                is_active: false,
            });
        }

        Ok(results)
    }

    /// Select or resolve the active tenant from available tenants
    pub fn resolve_active_tenant<'a>(
        tenants: &'a mut [TenantInfo],
        requested: Option<&str>,
        fallback_tenant_id: Option<&str>,
    ) -> Result<&'a mut TenantInfo, Box<dyn std::error::Error + Send + Sync>> {
        if tenants.is_empty() {
            return Err("No accessible workspaces found for this user account".into());
        }

        let target_idx = if let Some(req) = requested {
            let req_clean = req.trim();
            tenants
                .iter()
                .position(|t| t.slug.eq_ignore_ascii_case(req_clean) || t.id.eq_ignore_ascii_case(req_clean))
                .ok_or_else(|| format!("Requested workspace '{}' not found in your accessible tenants", req))?
        } else if let Some(fb) = fallback_tenant_id {
            tenants
                .iter()
                .position(|t| t.id.eq_ignore_ascii_case(fb) || t.slug.eq_ignore_ascii_case(fb))
                .unwrap_or(0)
        } else {
            0
        };

        for (i, t) in tenants.iter_mut().enumerate() {
            t.is_active = i == target_idx;
        }

        Ok(&mut tenants[target_idx])
    }

    /// Switch active tenant, re-enroll device under target tenant, and persist credentials
    pub async fn switch_tenant(
        data_dir: &str,
        target_slug_or_id: &str,
    ) -> Result<TenantInfo, Box<dyn std::error::Error + Send + Sync>> {
        let mut creds = CredentialStore::load(data_dir).await
            .ok_or("No active cloud session found on this machine. Run 'specter login' first.")?;

        let cloud_url = creds.cloud_url.clone()
            .unwrap_or_else(|| "https://dswhacsoaxgpfnkaxnhz.supabase.co".to_string());
        let api_key = creds.api_key.clone()
            .unwrap_or_else(|| super::credential_store::DEFAULT_SUPABASE_ANON_KEY.to_string());

        let client = CloudApiClient::build_http_client();

        // 1. Refresh access token if expired
        let access_token = match (creds.access_token.as_ref(), creds.refresh_token.as_ref()) {
            (Some(tok), Some(refresh)) if creds.is_token_expired() => {
                if let Ok(new_sess) = SupabaseAuthClient::refresh_access_token(&client, &cloud_url, &api_key, refresh).await {
                    creds.access_token = Some(new_sess.access_token.clone());
                    creds.refresh_token = new_sess.refresh_token.or(Some(refresh.clone()));
                    creds.token_expires_at = new_sess.expires_at;
                    new_sess.access_token
                } else {
                    tok.clone()
                }
            }
            (Some(tok), _) => tok.clone(),
            _ => {
                return Err("No active user session found. Please run 'specter login' to authenticate.".into());
            }
        };

        let user_id = creds.user_id.clone().unwrap_or_default();

        // 2. Refresh tenants list
        let mut tenants = Self::fetch_user_tenants(&client, &cloud_url, &api_key, &access_token, &user_id).await
            .unwrap_or_else(|_| creds.available_tenants.clone().unwrap_or_default());

        let target_tenant = Self::resolve_active_tenant(&mut tenants, Some(target_slug_or_id), None)?
            .clone();

        // 3. Re-enroll device under target tenant
        let new_device_creds = CloudApiClient::enroll(
            &client,
            &cloud_url,
            Some(&creds.name),
            Some(&target_tenant.slug),
            Some("cli_tenant_switch"),
            Some(&access_token),
        ).await?;

        // 4. Update credentials
        creds.device_id = new_device_creds.device_id;
        creds.device_token = new_device_creds.device_token;
        creds.tenant_id = Some(target_tenant.id.clone());
        creds.tenant_slug = Some(target_tenant.slug.clone());
        creds.tenant_name = Some(target_tenant.name.clone());
        creds.tenant_role = target_tenant.role.clone();
        creds.available_tenants = Some(tenants);

        CredentialStore::save(data_dir, &creds).await?;

        Ok(target_tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tenants() -> Vec<TenantInfo> {
        vec![
            TenantInfo {
                id: "uuid-111".to_string(),
                slug: "alpha-corp".to_string(),
                name: "Alpha Corp".to_string(),
                role: Some("owner".to_string()),
                status: Some("active".to_string()),
                is_active: false,
            },
            TenantInfo {
                id: "uuid-222".to_string(),
                slug: "beta-labs".to_string(),
                name: "Beta Labs".to_string(),
                role: Some("member".to_string()),
                status: Some("active".to_string()),
                is_active: false,
            },
            TenantInfo {
                id: "uuid-333".to_string(),
                slug: "gamma-dev".to_string(),
                name: "Gamma Dev".to_string(),
                role: Some("admin".to_string()),
                status: Some("active".to_string()),
                is_active: false,
            },
        ]
    }

    #[test]
    fn test_resolve_active_tenant_by_slug() {
        let mut tenants = sample_tenants();
        let active = TenancyManager::resolve_active_tenant(&mut tenants, Some("beta-labs"), None)
            .expect("Should resolve beta-labs");
        assert_eq!(active.id, "uuid-222");
        assert_eq!(active.slug, "beta-labs");
        assert!(active.is_active);

        // Verify other tenants are not active
        assert!(!tenants[0].is_active);
        assert!(tenants[1].is_active);
        assert!(!tenants[2].is_active);
    }

    #[test]
    fn test_resolve_active_tenant_by_id_case_insensitive() {
        let mut tenants = sample_tenants();
        let active = TenancyManager::resolve_active_tenant(&mut tenants, Some("UUID-333"), None)
            .expect("Should resolve UUID-333 case insensitively");
        assert_eq!(active.slug, "gamma-dev");
        assert!(active.is_active);
        assert!(tenants[2].is_active);
    }

    #[test]
    fn test_resolve_active_tenant_fallback() {
        let mut tenants = sample_tenants();
        let active = TenancyManager::resolve_active_tenant(&mut tenants, None, Some("uuid-222"))
            .expect("Should fallback to existing tenant id");
        assert_eq!(active.slug, "beta-labs");
        assert!(active.is_active);
    }

    #[test]
    fn test_resolve_active_tenant_default_first() {
        let mut tenants = sample_tenants();
        let active = TenancyManager::resolve_active_tenant(&mut tenants, None, None)
            .expect("Should default to first tenant");
        assert_eq!(active.slug, "alpha-corp");
        assert!(active.is_active);
    }

    #[test]
    fn test_resolve_active_tenant_not_found_err() {
        let mut tenants = sample_tenants();
        let res = TenancyManager::resolve_active_tenant(&mut tenants, Some("non-existent"), None);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_active_tenant_empty_err() {
        let mut tenants: Vec<TenantInfo> = Vec::new();
        let res = TenancyManager::resolve_active_tenant(&mut tenants, None, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("No accessible workspaces"));
    }
}

