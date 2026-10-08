use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const MANIFEST_RAW_JSON: &str = include_str!("../../schema/cli.manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliManifest {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub name: String,
    pub version: String,
    pub description: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub pillars: Vec<String>,
    pub commands: Vec<CommandDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub id: String,
    pub route: Vec<String>,
    pub pillar: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<ArgumentDef>,
    #[serde(default)]
    pub options: Vec<OptionDef>,
    #[serde(default)]
    pub mcp: Option<McpDef>,
    #[serde(default)]
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentDef {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    pub description: String,
    #[serde(default)]
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionDef {
    pub name: String,
    #[serde(default)]
    pub short: Option<String>,
    #[serde(rename = "type")]
    pub opt_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    pub description: String,
    #[serde(default)]
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDef {
    pub tool_name: String,
    #[serde(default)]
    pub exported: bool,
}

static MANIFEST_INSTANCE: OnceLock<CliManifest> = OnceLock::new();

/// Get global parsed instance of the Specter CLI Manifest
pub fn manifest() -> &'static CliManifest {
    MANIFEST_INSTANCE.get_or_init(|| {
        serde_json::from_str(MANIFEST_RAW_JSON)
            .expect("Embedded cli.manifest.json must be valid JSON adhering to schema")
    })
}

/// Filter commands by pillar name (e.g. "browser", "bridge", "automa")
pub fn list_by_pillar(pillar: &str) -> Vec<&'static CommandDef> {
    manifest()
        .commands
        .iter()
        .filter(|cmd| cmd.pillar.eq_ignore_ascii_case(pillar))
        .collect()
}

/// Find a command definition matching an exact route array (e.g. &["browser", "launch"])
pub fn find_by_route(route: &[&str]) -> Option<&'static CommandDef> {
    manifest().commands.iter().find(|cmd| {
        if cmd.route.len() != route.len() {
            return false;
        }
        cmd.route
            .iter()
            .zip(route.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

/// Export raw JSON representation
pub fn raw_json() -> serde_json::Value {
    serde_json::from_str(MANIFEST_RAW_JSON).unwrap_or(serde_json::json!({}))
}
