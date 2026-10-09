use std::path::{Path, PathBuf};
use serde::Serialize;
use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;

#[derive(Serialize)]
struct WfSummary {
    id: String,
    name: String,
    description: String,
    version: String,
    blocks_count: usize,
    source: String,
}

pub async fn execute_specter_workflow_list(args: &Value) -> Result<String, String> {
    let search = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config = AppConfig::load();

    let mut workflows: Vec<WfSummary> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // 1. SQLite Database
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
        && let Ok(list) = db.workflows().get_workflows(None, None, search.as_deref())
    {
        for wf in list {
            seen_ids.insert(wf.id.clone());
            let parsed: Option<Value> = serde_json::from_str(&wf.data).ok();
            let blocks_count = parsed.as_ref().map(count_workflow_blocks).unwrap_or(0);
            workflows.push(WfSummary {
                id: wf.id,
                name: wf.name,
                description: wf.description.unwrap_or_default(),
                version: wf.version,
                blocks_count,
                source: "Database".to_string(),
            });
        }
    }

    // 2. Vault Directory ~/.specter/automa/workflows
    let specter_vault = crate::config::AutomaConfig::load().resolved_vault_dir();
    let config_vault = PathBuf::from(&config.data_dir).join("workflows");

    for vdir in &[specter_vault, config_vault] {
        if let Ok(mut entries) = tokio::fs::read_dir(vdir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file()
                    && path.extension().map(|e| e == "json").unwrap_or(false)
                    && let Ok(content) = tokio::fs::read_to_string(&path).await
                    && let Ok(val) = serde_json::from_str::<Value>(&content)
                {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wf");
                    let id = val
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| stem.trim_end_matches(".workflow").to_string());
                    let name = val
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| id.clone());
                    let desc = val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if let Some(ref q) = search {
                        let q_lower = q.to_lowercase();
                        if !id.to_lowercase().contains(&q_lower)
                            && !name.to_lowercase().contains(&q_lower)
                            && !desc.to_lowercase().contains(&q_lower)
                        {
                            continue;
                        }
                    }

                    if seen_ids.contains(&id) {
                        if let Some(item) = workflows.iter_mut().find(|w| w.id == id) {
                            item.source = "Database+Vault".to_string();
                        }
                        continue;
                    }

                    seen_ids.insert(id.clone());
                    let version = val
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string();
                    let blocks_count = count_workflow_blocks(&val);

                    workflows.push(WfSummary {
                        id,
                        name,
                        description: desc,
                        version,
                        blocks_count,
                        source: "Vault".to_string(),
                    });
                }
            }
        }
    }

    Ok(serde_json::to_string_pretty(&workflows).unwrap_or_default())
}

pub fn count_workflow_blocks(val: &Value) -> usize {
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        nodes.len()
    } else if let Some(drawflow) = val.get("drawflow") {
        if let Some(s) = drawflow.as_str() {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).map(|a| a.len()))
                .unwrap_or(0)
        } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
            nodes.len()
        } else {
            0
        }
    } else {
        0
    }
}

pub async fn execute_specter_workflow_inspect(args: &Value) -> Result<String, String> {
    let target = args
        .get("workflow")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'workflow' argument".to_string())?;

    let (content, source_label) = resolve_workflow_content(target).await?;

    let val: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow JSON: {}", e))?;

    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or(target);
    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or(id);
    let description = val.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

    let nodes = extract_nodes_array(&val);
    let mut block_types = Vec::new();
    let mut triggers = Vec::new();

    for node in &nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let label = node
            .get("label")
            .or_else(|| node.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        block_types.push(json!({
            "id": node_id,
            "type": label,
        }));

        if label == "trigger" {
            let t_type = node.pointer("/data/type").and_then(|v| v.as_str()).unwrap_or("manual");
            let params = node.pointer("/data/parameters").cloned().unwrap_or(json!([]));
            triggers.push(json!({
                "id": node_id,
                "type": t_type,
                "parameters": params
            }));
        }
    }

    let variables = val.get("variables").cloned().unwrap_or(json!({}));

    let inspect_result = json!({
        "id": id,
        "name": name,
        "description": description,
        "version": version,
        "source": source_label,
        "blocks_count": nodes.len(),
        "triggers": triggers,
        "variables": variables,
        "blocks": block_types
    });

    Ok(serde_json::to_string_pretty(&inspect_result).unwrap_or_default())
}

async fn resolve_workflow_content(target: &str) -> Result<(String, String), String> {
    let target_path = Path::new(target);
    if target_path.exists() && target_path.is_file() {
        let text = tokio::fs::read_to_string(target_path)
            .await
            .map_err(|e| format!("Error reading file {:?}: {}", target_path, e))?;
        return Ok((text, format!("File: {}", target_path.display())));
    }

    let config = AppConfig::load();

    // Check DB
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
        && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(target)
    {
        return Ok((wf.data, format!("Database (ID: {})", wf.id)));
    }

    // Check Vault
    let specter_vault = crate::config::AutomaConfig::load().resolved_vault_dir();
    let config_vault = PathBuf::from(&config.data_dir).join("workflows");

    let candidates = [
        specter_vault.join(format!("{}.workflow.json", target)),
        specter_vault.join(format!("{}.json", target)),
        config_vault.join(format!("{}.workflow.json", target)),
        config_vault.join(format!("{}.json", target)),
    ];

    for c in &candidates {
        if c.exists()
            && c.is_file()
            && let Ok(text) = tokio::fs::read_to_string(c).await
        {
            return Ok((text, format!("Vault: {}", c.display())));
        }
    }

    Err(format!(
        "Workflow '{}' not found in file system, database, or vault.",
        target
    ))
}

fn extract_nodes_array(val: &Value) -> Vec<Value> {
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        nodes.clone()
    } else if let Some(drawflow) = val.get("drawflow") {
        if let Some(s) = drawflow.as_str() {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).cloned())
                .unwrap_or_default()
        } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
            nodes.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

pub async fn execute_specter_workflow_run(args: &Value) -> Result<String, String> {
    let workflow = args
        .get("workflow")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'workflow' argument".to_string())?;

    let headless = args.get("headless").and_then(|v| v.as_bool()).unwrap_or(true);
    let browser = args.get("browser").and_then(|v| v.as_str()).unwrap_or("chrome");
    let timeout = args.get("timeout").and_then(|v| v.as_u64());

    let exe = std::env::current_exe().map_err(|e| format!("Cannot find current exe: {}", e))?;

    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("automa").arg("run").arg(workflow);

    if headless {
        cmd.arg("--headless");
    }

    cmd.arg("--browser").arg(browser);

    if let Some(t) = timeout {
        cmd.arg("--timeout").arg(t.to_string());
    }

    if let Some(vars) = args.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars {
            let val_str = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            cmd.arg("--var").arg(format!("{}={}", k, val_str));
        }
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to execute workflow subprocess: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let status_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();

    let result = json!({
        "success": success,
        "exit_code": status_code,
        "workflow": workflow,
        "stdout": stdout_str,
        "stderr": stderr_str
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}
