use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub id: String,
    pub name: String,
    pub r#type: String, // 'type' is a reserved keyword in Rust
    pub index: Option<i32>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockData {
    pub settings: Option<Value>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    #[serde(default)]
    pub id: String,
    pub label: Option<String>,
    pub r#type: Option<String>,
    #[serde(default)]
    pub data: BlockData,
    pub position: Option<Value>, // { x, y }
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEdge {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSettings {
    pub on_error: Option<String>,
    pub restart_times: Option<i32>,
    pub notification: Option<bool>,
    pub debug_mode: Option<bool>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub drawflow: Option<Value>, // Legacy structure
    pub nodes: Option<Vec<WorkflowNode>>,
    pub edges: Option<Vec<WorkflowEdge>>,
    pub settings: Option<WorkflowSettings>,
    pub global_data: Option<Value>,
    pub table: Option<Vec<TableColumn>>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}
