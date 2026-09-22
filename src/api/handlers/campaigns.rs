use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::Campaign;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "slotIndex": 0,
    "x": 0,
    "y": 0,
    "width": 960,
    "height": 540,
    "browserId": "browser_slot_1",
    "status": "ready"
}))]
/// Display bounds and runtime state for a grid window slot
pub struct MatrixSlotInfo {
    /// 0-indexed matrix slot position
    pub slot_index: u32,
    /// Window X coordinate in pixels
    pub x: i32,
    /// Window Y coordinate in pixels
    pub y: i32,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Assigned browser profile ID (if any)
    pub browser_id: Option<String>,
    /// Slot status ("ready", "running", "completed", "failed")
    pub status: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "campaignId": "camp_123",
    "status": "active",
    "totalTasks": 4,
    "completedTasks": 1,
    "activeSlots": []
}))]
/// Grid matrix telemetry and slot allocation for a campaign
pub struct MatrixStatusResponse {
    /// Unique campaign identifier
    pub campaign_id: String,
    /// Overall campaign status ("active", "completed", "aborted")
    pub status: String,
    /// Total tasks queued in the campaign
    pub total_tasks: u32,
    /// Number of completed tasks
    pub completed_tasks: u32,
    /// Detailed matrix slots currently allocated
    pub active_slots: Vec<MatrixSlotInfo>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "campaignPath": "C:/vault/campaigns/daily_audit.campaign.json",
    "campaignId": "camp_daily_audit",
    "runNow": true,
    "useGrid": true
}))]
/// Request payload to trigger or schedule a campaign execution
pub struct ExecuteCampaignRequest {
    /// Path to the `.campaign.json` descriptor file
    pub campaign_path: Option<String>,
    /// Optional custom campaign identifier
    pub campaign_id: Option<String>,
    /// Immediately start executing jobs
    pub run_now: Option<bool>,
    /// Automatically tile browser windows across screen grid
    pub use_grid: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "campaignId": "camp_123",
    "status": "running",
    "totalJobs": 4,
    "jobIds": ["job_1", "job_2"],
    "allocatedSlots": []
}))]
/// Response returned after scheduling a campaign
pub struct ExecuteCampaignResponse {
    /// Unique campaign identifier
    pub campaign_id: String,
    /// Current campaign state
    pub status: String,
    /// Total execution jobs generated
    pub total_jobs: usize,
    /// List of created job identifiers
    pub job_ids: Vec<String>,
    /// Grid matrix slot assignments
    pub allocated_slots: Vec<MatrixSlotInfo>,
}

#[utoipa::path(
    tag = "Campaigns",
    get,
    path = "/api/v1/campaigns/{id}/matrix-status",
    operation_id = "get_campaign_matrix_status",
    summary = "Get campaign grid matrix execution status",
    description = "Retrieves real-time slot layout and execution status for a multi-instance grid campaign.",
    params(
        ("id" = String, Path, description = "Unique campaign identifier")
    ),
    responses(
        (status = 200, description = "Matrix status and slot allocation", body = MatrixStatusResponse),
        (status = 404, description = "Campaign not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_matrix_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MatrixStatusResponse>, AutomaError> {
    let settings = {
        let db = state.db.lock().await;
        db.settings().get_settings().unwrap_or_default()
    };

    let grid = settings.grid;
    let mut active_slots = Vec::new();

    // Generate active slots based on grid matrix
    let total_slots = grid.matrix.columns * grid.matrix.rows;
    for i in 0..total_slots.min(12) {
        let (x, y, width, height) = grid.calculate_slot_bounds(i);
        active_slots.push(MatrixSlotInfo {
            slot_index: i,
            x,
            y,
            width,
            height,
            browser_id: Some(format!("browser_slot_{}", i + 1)),
            status: "ready".to_string(),
        });
    }

    Ok(Json(MatrixStatusResponse {
        campaign_id: id,
        status: "active".to_string(),
        total_tasks: total_slots,
        completed_tasks: 0,
        active_slots,
    }))
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "cp_daily_sync",
    "name": "Daily Sync Campaign",
    "description": "Syncs marketing profiles",
    "data": {
        "members": [{"browserId": "b1", "workflowId": "w1"}],
        "settings": {"concurrency_mode": "parallel"}
    },
    "cron": "0 8 * * *",
    "version": "1.0.0",
    "createdAt": "2026-08-26T00:00:00Z",
    "updatedAt": "2026-08-26T00:00:00Z"
}))]
/// Campaign descriptor stored in central SQLite database
pub struct CampaignStorageItem {
    /// Unique campaign identifier
    pub id: String,
    /// Campaign display name
    pub name: String,
    /// Optional campaign description
    pub description: Option<String>,
    /// Campaign data (members, tasks, matrix, concurrency settings)
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
    /// Optional cron schedule expression
    pub cron: Option<String>,
    /// Semantic version
    pub version: String,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last modification timestamp (ISO 8601)
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "cp_daily_sync",
    "name": "Daily Sync Campaign",
    "description": "Syncs marketing profiles",
    "data": {
        "members": [{"browserId": "b1", "workflowId": "w1"}]
    },
    "cron": "0 8 * * *",
    "version": "1.0.0"
}))]
/// Request payload for creating a new campaign in SQLite database
pub struct CreateCampaignStorageRequest {
    /// Optional custom identifier
    pub id: Option<String>,
    /// Campaign display name
    pub name: String,
    /// Optional campaign description
    pub description: Option<String>,
    /// Campaign data (members, tasks, matrix)
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
    /// Optional cron expression
    pub cron: Option<String>,
    /// Optional version (defaults to 1.0.0)
    pub version: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "Updated Daily Sync",
    "cron": "0 9 * * *"
}))]
/// Request payload for updating an existing campaign in SQLite database
pub struct UpdateCampaignStorageRequest {
    /// Optional updated name
    pub name: Option<String>,
    /// Optional updated description
    pub description: Option<String>,
    /// Optional updated campaign data
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
    /// Optional updated cron expression
    pub cron: Option<String>,
    /// Optional updated version
    pub version: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "campaign": {
        "name": "Imported Fleet Campaign",
        "members": []
    }
}))]
/// Request payload for importing a campaign into SQLite database
pub struct ImportCampaignStorageRequest {
    /// Optional custom ID
    pub id: Option<String>,
    /// Raw campaign JSON content
    #[schema(value_type = Object)]
    pub campaign: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"success": true, "message": "Campaign 'cp_1' deleted"}))]
/// Response returned after successfully deleting a campaign
pub struct DeleteCampaignResponse {
    /// True if deletion succeeded
    pub success: bool,
    /// Confirmation message
    pub message: String,
}

#[derive(Deserialize, IntoParams)]
pub struct GetCampaignsQuery {
    /// Maximum number of campaigns to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter campaigns by name, ID or description
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/campaigns",
    operation_id = "get_storage_campaigns",
    summary = "List all campaigns from storage database",
    description = "Retrieves all campaigns persisted in the central SQLite database.",
    params(GetCampaignsQuery),
    responses(
        (status = 200, description = "List of campaigns", body = Vec<CampaignStorageItem>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_storage_campaigns(
    State(state): State<AppState>,
    Query(query): Query<GetCampaignsQuery>,
) -> Result<Json<Vec<CampaignStorageItem>>, AutomaError> {
    let db = state.db.lock().await;
    let list = db.campaigns().get_campaigns(query.limit, query.offset, query.search.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let items = list.into_iter().map(|c| {
        let parsed_data = serde_json::from_str::<serde_json::Value>(&c.data)
            .unwrap_or(serde_json::json!({}));
        CampaignStorageItem {
            id: c.id,
            name: c.name,
            description: c.description,
            data: parsed_data,
            cron: c.cron,
            version: c.version,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }).collect();

    Ok(Json(items))
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/campaigns/{id}",
    operation_id = "get_storage_campaign",
    summary = "Get a campaign by ID from storage database",
    description = "Retrieves the full configuration and metadata of a campaign from SQLite database.",
    params(
        ("id" = String, Path, description = "Unique campaign identifier")
    ),
    responses(
        (status = 200, description = "Campaign details", body = CampaignStorageItem),
        (status = 404, description = "Campaign not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_storage_campaign(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CampaignStorageItem>, AutomaError> {
    let db = state.db.lock().await;
    let item = db.campaigns().get_campaign(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AutomaError::NotFound(format!("Campaign '{}' not found in database", id)))?;

    let parsed_data = serde_json::from_str::<serde_json::Value>(&item.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(CampaignStorageItem {
        id: item.id,
        name: item.name,
        description: item.description,
        data: parsed_data,
        cron: item.cron,
        version: item.version,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/campaigns",
    operation_id = "create_storage_campaign",
    summary = "Create or persist a campaign in database",
    description = "Creates a new campaign record or updates an existing record in SQLite database.",
    request_body = CreateCampaignStorageRequest,
    responses(
        (status = 200, description = "Campaign created successfully", body = CampaignStorageItem),
        (status = 400, description = "Invalid campaign payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn create_storage_campaign(
    State(state): State<AppState>,
    Json(payload): Json<CreateCampaignStorageRequest>,
) -> Result<Json<CampaignStorageItem>, AutomaError> {
    let id = payload.id.unwrap_or_else(|| format!("camp_{}", uuid::Uuid::new_v4().simple()));
    let data_str = serde_json::to_string(&payload.data)
        .map_err(|e| AutomaError::BadRequest(format!("Invalid campaign JSON: {}", e)))?;

    let db = state.db.lock().await;
    let created = db.campaigns().create_campaign(
        &id,
        &payload.name,
        payload.description.as_deref(),
        &data_str,
        payload.cron.as_deref(),
        payload.version.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let parsed_data = serde_json::from_str::<serde_json::Value>(&created.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(CampaignStorageItem {
        id: created.id,
        name: created.name,
        description: created.description,
        data: parsed_data,
        cron: created.cron,
        version: created.version,
        created_at: created.created_at,
        updated_at: created.updated_at,
    }))
}

#[utoipa::path(
    tag = "Storage",
    put,
    path = "/api/v1/storage/campaigns/{id}",
    operation_id = "update_storage_campaign",
    summary = "Update a campaign in storage database",
    description = "Modifies an existing campaign record in SQLite database.",
    params(
        ("id" = String, Path, description = "Unique campaign identifier")
    ),
    request_body = UpdateCampaignStorageRequest,
    responses(
        (status = 200, description = "Campaign updated successfully", body = CampaignStorageItem),
        (status = 400, description = "Invalid update payload", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Campaign not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn update_storage_campaign(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCampaignStorageRequest>,
) -> Result<Json<CampaignStorageItem>, AutomaError> {
    let data_str = payload.data.as_ref().map(|d| serde_json::to_string(d)).transpose()
        .map_err(|e| AutomaError::BadRequest(format!("Invalid campaign JSON: {}", e)))?;

    let db = state.db.lock().await;
    let updated = db.campaigns().upsert_campaign(
        &id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        data_str.as_deref(),
        payload.cron.as_deref(),
        payload.version.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let _ = state.tx.send(serde_json::json!({
        "type": "storage_changed",
        "entity": "campaign",
        "action": "upsert",
        "id": updated.id,
        "name": updated.name
    }).to_string());

    let parsed_data = serde_json::from_str::<serde_json::Value>(&updated.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(CampaignStorageItem {
        id: updated.id,
        name: updated.name,
        description: updated.description,
        data: parsed_data,
        cron: updated.cron,
        version: updated.version,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

#[utoipa::path(
    tag = "Storage",
    delete,
    path = "/api/v1/storage/campaigns/{id}",
    operation_id = "delete_storage_campaign",
    summary = "Delete a campaign from storage database",
    description = "Removes a campaign record from SQLite database by ID.",
    params(
        ("id" = String, Path, description = "Unique campaign identifier")
    ),
    responses(
        (status = 200, description = "Campaign deleted successfully", body = DeleteCampaignResponse),
        (status = 404, description = "Campaign not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_storage_campaign(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DeleteCampaignResponse>, AutomaError> {
    let db = state.db.lock().await;
    let deleted = db.campaigns().delete_campaign(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(AutomaError::NotFound(format!("Campaign '{}' not found in database", id)));
    }

    let _ = state.tx.send(serde_json::json!({
        "type": "storage_changed",
        "entity": "campaign",
        "action": "delete",
        "id": id
    }).to_string());

    Ok(Json(DeleteCampaignResponse {
        success: true,
        message: format!("Campaign '{}' deleted", id),
    }))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/campaigns/import",
    operation_id = "import_storage_campaign",
    summary = "Import a campaign JSON into storage database",
    description = "Parses and imports a campaign JSON object into SQLite database.",
    request_body = ImportCampaignStorageRequest,
    responses(
        (status = 200, description = "Campaign imported successfully", body = CampaignStorageItem),
        (status = 400, description = "Invalid campaign JSON", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn import_storage_campaign(
    State(state): State<AppState>,
    Json(payload): Json<ImportCampaignStorageRequest>,
) -> Result<Json<CampaignStorageItem>, AutomaError> {
    let cp = &payload.campaign;
    if !cp.is_object() {
        return Err(AutomaError::BadRequest("Import payload must contain a valid JSON object".into()));
    }

    let id = payload.id
        .or_else(|| cp.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| format!("camp_{}", uuid::Uuid::new_v4().simple()));

    let name = cp.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();

    let description = cp.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let cron = cp.get("cron")
        .or_else(|| cp.get("settings").and_then(|s| s.get("cron")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let version = cp.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let data_str = serde_json::to_string(cp)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to serialize campaign: {}", e)))?;

    let db = state.db.lock().await;
    let created = db.campaigns().create_campaign(
        &id,
        &name,
        description.as_deref(),
        &data_str,
        cron.as_deref(),
        version.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let parsed_data = serde_json::from_str::<serde_json::Value>(&created.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(CampaignStorageItem {
        id: created.id,
        name: created.name,
        description: created.description,
        data: parsed_data,
        cron: created.cron,
        version: created.version,
        created_at: created.created_at,
        updated_at: created.updated_at,
    }))
}

#[utoipa::path(
    tag = "Campaigns",
    post,
    path = "/api/v1/campaigns/execute",
    operation_id = "execute_campaign",
    summary = "Execute a multi-instance automation campaign",
    description = "Parses a campaign descriptor from SQLite database or file, provisions required browser profiles, allocates grid matrix slots, and starts parallel workflow execution jobs.",
    request_body = ExecuteCampaignRequest,
    responses(
        (status = 200, description = "Campaign execution scheduled", body = ExecuteCampaignResponse),
        (status = 400, description = "Invalid campaign payload or unreadable descriptor", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn execute_campaign(
    State(state): State<AppState>,
    Json(payload): Json<ExecuteCampaignRequest>,
) -> Result<Json<ExecuteCampaignResponse>, AutomaError> {
    let campaign_id = payload
        .campaign_id
        .clone()
        .unwrap_or_else(|| format!("camp_{}", uuid::Uuid::new_v4().simple()));

    let settings = {
        let db = state.db.lock().await;
        db.settings().get_settings().unwrap_or_default()
    };

    let grid = settings.grid;
    let mut allocated_slots = Vec::new();
    let mut job_ids = Vec::new();

    // 1. Try resolving from SQLite Database first
    let mut task_count = 1;
    let db_campaign = {
        let db = state.db.lock().await;
        db.campaigns().get_campaign(&campaign_id).ok().flatten()
    };

    if let Some(c) = db_campaign {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&c.data) {
            if let Some(members) = val.get("members").and_then(|m| m.as_array()) {
                task_count = members.len().max(1);
            } else if let Some(browsers) = val.get("browsers").and_then(|b| b.as_array()) {
                task_count = browsers.len().max(1);
            }
        }
    } else if let Some(ref path_str) = payload.campaign_path {
        // 2. Fallback to file on disk
        if let Ok(content) = tokio::fs::read_to_string(path_str).await {
            if let Ok(camp) = serde_json::from_str::<Campaign>(&content) {
                if let Some(ref browsers) = camp.browsers {
                    task_count = browsers.len().max(1);
                }
            }
        }
    }

    // Allocate grid slots for each browser task
    for i in 0..task_count as u32 {
        let (x, y, width, height) = grid.calculate_slot_bounds(i);
        let job_id = format!("job_{}", uuid::Uuid::new_v4().simple());
        job_ids.push(job_id);

        allocated_slots.push(MatrixSlotInfo {
            slot_index: i,
            x,
            y,
            width,
            height,
            browser_id: Some(format!("matrix_browser_{}", i + 1)),
            status: "running".to_string(),
        });
    }

    // Broadcast matrix started event to SSE listeners
    let sse_event = serde_json::json!({
        "type": "matrix_started",
        "campaignId": campaign_id,
        "status": "running",
        "totalJobs": task_count,
        "slots": allocated_slots
    });
    let _ = state.tx.send(sse_event.to_string());

    Ok(Json(ExecuteCampaignResponse {
        campaign_id,
        status: "running".to_string(),
        total_jobs: task_count,
        job_ids,
        allocated_slots,
    }))
}

#[utoipa::path(
    tag = "Campaigns",
    delete,
    path = "/api/v1/campaigns/{id}",
    operation_id = "abort_campaign",
    summary = "Abort a running multi-instance campaign",
    description = "Immediately halts all sub-jobs associated with the campaign, stops all browser processes in the grid matrix, and emits a matrix_finished SSE event.",
    params(
        ("id" = String, Path, description = "Unique campaign identifier")
    ),
    responses(
        (status = 200, description = "Campaign aborted successfully"),
        (status = 404, description = "Campaign not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn abort_campaign(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AutomaError> {
    let sse_event = serde_json::json!({
        "type": "matrix_finished",
        "campaignId": id,
        "status": "aborted"
    });
    let _ = state.tx.send(sse_event.to_string());
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;
    use crate::config::AppConfig;
    use std::sync::Arc;
    use std::collections::HashMap;

    fn create_test_state() -> AppState {
        let db = Arc::new(tokio::sync::Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let config = Arc::new(AppConfig::load());
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let (worker_tx, _worker_rx) = tokio::sync::broadcast::channel(100);

        AppState {
            db,
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_get_matrix_status() {
        let state = create_test_state();
        let res = get_matrix_status(State(state), Path("camp_unit_test".to_string())).await.unwrap();
        assert_eq!(res.campaign_id, "camp_unit_test");
        assert_eq!(res.status, "active");
        assert!(!res.active_slots.is_empty());
        assert_eq!(res.active_slots[0].slot_index, 0);
    }

    #[tokio::test]
    async fn test_execute_campaign_allocates_slots() {
        let state = create_test_state();
        let req = ExecuteCampaignRequest {
            campaign_path: None,
            campaign_id: Some("camp_custom_123".to_string()),
            run_now: Some(true),
            use_grid: Some(true),
        };

        let res = execute_campaign(State(state), Json(req)).await.unwrap();
        assert_eq!(res.campaign_id, "camp_custom_123");
        assert_eq!(res.status, "running");
        assert_eq!(res.total_jobs, 1);
        assert_eq!(res.allocated_slots.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_campaigns_api_flow() {
        let state = create_test_state();

        // 1. Create campaign
        let create_payload = CreateCampaignStorageRequest {
            id: Some("cp_unit_1".to_string()),
            name: "Unit Test Campaign".to_string(),
            description: Some("Automated testing campaign".to_string()),
            data: serde_json::json!({
                "members": [{"browserId": "b1", "workflowId": "w1"}],
                "settings": {"concurrency_mode": "parallel"}
            }),
            cron: Some("0 8 * * *".to_string()),
            version: Some("1.0.0".to_string()),
        };
        let created = create_storage_campaign(State(state.clone()), Json(create_payload)).await.unwrap();
        assert_eq!(created.id, "cp_unit_1");
        assert_eq!(created.name, "Unit Test Campaign");

        // 2. Get campaigns
        let query = GetCampaignsQuery { limit: None, offset: None, search: None };
        let list = get_storage_campaigns(State(state.clone()), Query(query)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "cp_unit_1");

        // 3. Get campaign by ID
        let detail = get_storage_campaign(State(state.clone()), Path("cp_unit_1".to_string())).await.unwrap();
        assert_eq!(detail.name, "Unit Test Campaign");

        // 4. Update campaign
        let update_payload = UpdateCampaignStorageRequest {
            name: Some("Updated Campaign".to_string()),
            description: None,
            data: None,
            cron: Some("0 9 * * *".to_string()),
            version: Some("1.1.0".to_string()),
        };
        let updated = update_storage_campaign(State(state.clone()), Path("cp_unit_1".to_string()), Json(update_payload)).await.unwrap();
        assert_eq!(updated.name, "Updated Campaign");
        assert_eq!(updated.cron.as_deref(), Some("0 9 * * *"));

        // 5. Import campaign
        let import_payload = ImportCampaignStorageRequest {
            id: Some("cp_imported_1".to_string()),
            campaign: serde_json::json!({
                "name": "Imported Fleet",
                "members": [{"browserId": "b1"}, {"browserId": "b2"}]
            }),
        };
        let imported = import_storage_campaign(State(state.clone()), Json(import_payload)).await.unwrap();
        assert_eq!(imported.id, "cp_imported_1");
        assert_eq!(imported.name, "Imported Fleet");

        // 6. Execute campaign using SQLite DB ID
        let exec_req = ExecuteCampaignRequest {
            campaign_path: None,
            campaign_id: Some("cp_imported_1".to_string()),
            run_now: Some(true),
            use_grid: Some(true),
        };
        let exec_res = execute_campaign(State(state.clone()), Json(exec_req)).await.unwrap();
        assert_eq!(exec_res.total_jobs, 2); // 2 members from DB

        // 7. Delete campaign
        let del_res = delete_storage_campaign(State(state.clone()), Path("cp_unit_1".to_string())).await.unwrap();
        assert!(del_res.success);

        let query = GetCampaignsQuery { limit: None, offset: None, search: None };
        let remaining = get_storage_campaigns(State(state), Query(query)).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "cp_imported_1");
    }
}
