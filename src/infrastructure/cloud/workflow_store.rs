//! Cloud workflow repository — reads/writes `automation.workflows` through the
//! `public.workflow_*` RPCs.
//!
//! PostgREST in this project only exposes the `public` schema (probed: `PGRST106`),
//! so the supported contract is the RPC layer defined in
//! `cloud/supabase/plugins/automation/install.sql` (§7.5). Those functions are
//! `SECURITY INVOKER`, so RLS still applies (`is_tenant_member` for reads,
//! `automation:workflows:manage` / tenant admin for writes).
//!
//! Replaces the removed local SQLite workflow store (rip-and-replace): the Cloud
//! table is the single source of truth, shared with the workstation console.

use serde_json::{Value, json};

use super::credential_store::{CredentialStore, DEFAULT_SUPABASE_ANON_KEY};
use crate::config::AppConfig;

const DEFAULT_CLOUD_URL: &str = "https://dswhacsoaxgpfnkaxnhz.supabase.co";

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub status: Option<String>,
    pub graph_data: Value,
    pub settings: Value,
    pub updated_at: Option<String>,
}

impl Workflow {
    fn from_row(row: &Value) -> Self {
        Self {
            id: row
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: row
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            description: row
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            version: row
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("1.0.0")
                .to_string(),
            status: row
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string),
            graph_data: row.get("graph_data").cloned().unwrap_or_else(|| json!({})),
            settings: row.get("settings").cloned().unwrap_or_else(|| json!({})),
            updated_at: row
                .get("updated_at")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }

    /// Number of blocks (nodes) in the stored graph.
    pub fn block_count(&self) -> usize {
        self.graph_data
            .get("nodes")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
    }
}

pub struct CloudWorkflowRepository {
    client: reqwest::Client,
    base: String,
    api_key: String,
    token: String,
    tenant_id: String,
}

impl CloudWorkflowRepository {
    /// Connect using the persisted user session. Errors are user-facing strings.
    pub async fn connect() -> Result<Self, String> {
        let config = AppConfig::load();
        let creds = CredentialStore::load(&config.data_dir)
            .await
            .ok_or_else(|| "Not authenticated. Run `tq auth login` first.".to_string())?;
        let token = creds
            .access_token
            .clone()
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| "No user session found. Run `tq auth login` first.".to_string())?;
        let tenant_id = creds
            .tenant_id
            .clone()
            .ok_or_else(|| "No tenant bound. Run `tq auth login` again.".to_string())?;
        let base = creds
            .cloud_url
            .clone()
            .or(config.cloud_url.clone())
            .unwrap_or_else(|| DEFAULT_CLOUD_URL.to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            base: base.trim_end_matches('/').to_string(),
            api_key: DEFAULT_SUPABASE_ANON_KEY.to_string(),
            token,
            tenant_id,
        })
    }

    /// Invoke a `public` RPC and decode the JSON response.
    async fn rpc(&self, function: &str, body: &Value) -> Result<Value, String> {
        let url = format!("{}/rest/v1/rpc/{function}", self.base);
        let res = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Cloud request failed: {e}"))?;

        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "Cloud rejected the request ({}): {}",
                status.as_u16(),
                text.chars().take(300).collect::<String>()
            ));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| format!("Invalid cloud response: {e}"))
    }

    fn rows(value: Value) -> Vec<Workflow> {
        value
            .as_array()
            .map(|a| a.iter().map(Workflow::from_row).collect())
            .unwrap_or_default()
    }

    /// List workflows (newest first), optional keyword search.
    pub async fn list(&self, search: Option<&str>) -> Result<Vec<Workflow>, String> {
        let search = search.map(str::trim).filter(|s| !s.is_empty());
        let value = self
            .rpc("workflow_list", &json!({ "p_search": search }))
            .await?;
        Ok(Self::rows(value))
    }

    /// Find one workflow by UUID or by exact name.
    pub async fn find(&self, target: &str) -> Result<Option<Workflow>, String> {
        let value = self
            .rpc("workflow_get", &json!({ "p_target": target }))
            .await?;
        Ok(Self::rows(value).into_iter().next())
    }

    /// Count visible workflows.
    pub async fn count(&self) -> Result<usize, String> {
        Ok(self.list(None).await?.len())
    }

    /// Create or update a workflow, matched by UUID (`target`) or by name.
    pub async fn upsert(
        &self,
        target: Option<&str>,
        name: &str,
        description: Option<&str>,
        version: &str,
        graph_data: Value,
        settings: Value,
    ) -> Result<Workflow, String> {
        let value = self
            .rpc(
                "workflow_upsert",
                &json!({
                    "p_tenant": self.tenant_id,
                    "p_target": target,
                    "p_name": name,
                    "p_description": description,
                    "p_version": version,
                    "p_graph_data": graph_data,
                    "p_settings": settings,
                }),
            )
            .await?;
        Self::rows(value)
            .into_iter()
            .next()
            .ok_or_else(|| "Cloud returned no workflow after upsert.".to_string())
    }

    /// Soft-delete a workflow (sets `deleted_at`), matched by UUID or name.
    pub async fn delete(&self, target: &str) -> Result<bool, String> {
        let value = self
            .rpc("workflow_delete", &json!({ "p_target": target }))
            .await?;
        Ok(value.as_bool().unwrap_or(false))
    }
}
