use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::AppState;
use crate::infrastructure::cloud::{
    CloudApiClient, CredentialStore, TenancyManager, TenantInfo,
};

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "success": true,
    "activeTenantId": "uuid-1234",
    "activeTenantSlug": "default-workspace",
    "activeTenantName": "Default Workspace",
    "tenants": [
        {
            "id": "uuid-1234",
            "slug": "default-workspace",
            "name": "Default Workspace",
            "role": "owner",
            "status": "active",
            "isActive": true
        }
    ]
}))]
/// Response descriptor containing current and accessible workspaces/tenants
pub struct TenantListResponse {
    /// Whether query succeeded
    pub success: bool,
    /// Currently active tenant ID
    pub active_tenant_id: Option<String>,
    /// Currently active tenant vanity slug
    pub active_tenant_slug: Option<String>,
    /// Currently active tenant display name
    pub active_tenant_name: Option<String>,
    /// List of all accessible tenants
    pub tenants: Vec<TenantInfo>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"target": "alpha-corp"}))]
/// Request payload to switch current active tenant
pub struct SwitchTenantRequest {
    /// Tenant identifier UUID or vanity slug to switch into
    pub target: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "success": true,
    "message": "Switched active tenant to 'alpha-corp'",
    "activeTenant": {
        "id": "uuid-1234",
        "slug": "alpha-corp",
        "name": "Alpha Corp",
        "role": "owner",
        "status": "active",
        "isActive": true
    }
}))]
/// Response returned after successfully switching active tenant
pub struct SwitchTenantResponse {
    /// Whether switch succeeded
    pub success: bool,
    /// Status confirmation message
    pub message: String,
    /// Details of newly selected active tenant
    pub active_tenant: Option<TenantInfo>,
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/system/tenants",
    operation_id = "get_system_tenants",
    summary = "List all accessible tenants and active workspace",
    description = "Queries cached and cloud-synced tenants for the authenticated operator without exposing cloud keys to frontend.",
    responses(
        (status = 200, description = "List of accessible tenants", body = TenantListResponse),
        (status = 401, description = "Workstation not enrolled or unauthenticated", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_tenants(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let creds = match CredentialStore::load(&state.config.data_dir).await {
        Some(c) => c,
        None => {
            return (
                StatusCode::OK,
                Json(TenantListResponse {
                    success: false,
                    active_tenant_id: None,
                    active_tenant_slug: None,
                    active_tenant_name: None,
                    tenants: Vec::new(),
                }),
            ).into_response();
        }
    };

    let cloud_url = creds.cloud_url.clone()
        .or_else(|| state.config.cloud_url.clone())
        .unwrap_or_else(|| "https://dswhacsoaxgpfnkaxnhz.supabase.co".to_string());
    let api_key = creds.api_key.clone()
        .unwrap_or_else(|| crate::infrastructure::cloud::DEFAULT_SUPABASE_ANON_KEY.to_string());

    let client = CloudApiClient::build_http_client();
    let user_id = creds.user_id.clone().unwrap_or_default();

    let mut tenants = if let Some(ref access_token) = creds.access_token {
        TenancyManager::fetch_user_tenants(&client, &cloud_url, &api_key, access_token, &user_id)
            .await
            .unwrap_or_else(|_| creds.available_tenants.clone().unwrap_or_default())
    } else {
        creds.available_tenants.clone().unwrap_or_default()
    };

    let active_id = creds.tenant_id.as_deref().unwrap_or("");
    let active_slug = creds.tenant_slug.as_deref().unwrap_or("");

    for t in tenants.iter_mut() {
        t.is_active = (!active_id.is_empty() && t.id == active_id)
            || (!active_slug.is_empty() && t.slug.eq_ignore_ascii_case(active_slug));
    }

    (
        StatusCode::OK,
        Json(TenantListResponse {
            success: true,
            active_tenant_id: creds.tenant_id,
            active_tenant_slug: creds.tenant_slug,
            active_tenant_name: creds.tenant_name,
            tenants,
        }),
    ).into_response()
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/tenants/switch",
    operation_id = "switch_system_tenant",
    summary = "Switch active tenant workspace",
    description = "Re-enrolls this runner device under the target tenant, updates local credentials, and emits a tenant_switched event to SSE.",
    request_body = SwitchTenantRequest,
    responses(
        (status = 200, description = "Successfully switched active tenant", body = SwitchTenantResponse),
        (status = 400, description = "Invalid tenant ID or slug", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Failed to switch tenant", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn switch_tenant(
    State(state): State<AppState>,
    Json(payload): Json<SwitchTenantRequest>,
) -> impl IntoResponse {
    match TenancyManager::switch_tenant(&state.config.data_dir, &payload.target).await {
        Ok(active_tenant) => {
            // Broadcast tenant switch event to global SSE stream
            let _ = state.tx.send(serde_json::json!({
                "type": "tenant_switched",
                "tenantId": active_tenant.id,
                "tenantSlug": active_tenant.slug,
                "tenantName": active_tenant.name
            }).to_string());

            (
                StatusCode::OK,
                Json(SwitchTenantResponse {
                    success: true,
                    message: format!("Switched active workspace to '{}'", active_tenant.name),
                    active_tenant: Some(active_tenant),
                }),
            ).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(SwitchTenantResponse {
                success: false,
                message: format!("Failed to switch workspace: {}", e),
                active_tenant: None,
            }),
        ).into_response(),
    }
}
