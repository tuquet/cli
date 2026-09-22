use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"id": "var_1", "name": "API_KEY", "value": "sk_test_123"}))]
/// Global variable stored in Automa SQLite / FS storage
pub struct StorageVariable {
    /// Unique variable identifier
    pub id: Option<String>,
    /// Human-friendly display label
    pub name: Option<String>,
    /// Unique variable key used in workflow expressions
    pub key: Option<String>,
    /// Arbitrary JSON value or primitive stored
    #[schema(value_type = Object)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"id": "cred_1", "name": "Github Auth", "value": "secret"}))]
/// Stored authentication credential
pub struct StorageCredential {
    /// Unique credential identifier
    pub id: Option<String>,
    /// Display name of the credential
    pub name: Option<String>,
    /// Credential lookup key
    pub key: Option<String>,
    /// Encrypted or plaintext secret value
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"id": "tbl_1", "name": "Users"}))]
/// User data table schema in Automa storage
pub struct StorageTable {
    /// Unique table identifier
    pub id: Option<String>,
    /// Table display name
    pub name: Option<String>,
    /// Array of column definitions
    #[schema(value_type = Option<Object>)]
    pub columns: Option<serde_json::Value>,
    /// Optional inline items cache
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub items: Option<serde_json::Value>,
    /// Index mapping for columns
    #[schema(value_type = Option<Object>)]
    pub columns_index: Option<serde_json::Value>,
    /// Creation timestamp in milliseconds
    pub created_at: Option<i64>,
    /// Last modification timestamp in milliseconds
    pub modified_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[schema(example = json!({"id": "row_1", "table_id": "tbl_1", "data": {"name": "Alice"}}))]
/// Single data row entry belonging to a storage table
pub struct TableRow {
    /// Unique row identifier
    pub id: String,
    /// Parent table identifier
    pub table_id: String,
    /// Key-value document payload of row columns
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
}
