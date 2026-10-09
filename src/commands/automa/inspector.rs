use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;

pub fn inspect_workflow(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::Notify::header("Automa Workflow Inspector");

    let (content, source_label) = {
        let target_path = Path::new(target);
        if target_path.exists() && target_path.is_file() {
            (
                std::fs::read_to_string(target_path)?,
                format!("File: {}", target_path.display()),
            )
        } else {
            // Try resolving from DB or Vault
            let config = AppConfig::load();
            let mut resolved = None;

            // Check SQLite DB
            let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMA_SQLITE);
            if db_path.exists()
                && let Ok(db) = AutomaDb::new(&db_path)
                && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(target)
            {
                resolved = Some((wf.data, format!("Database (ID: {})", wf.id)));
            }

            // Check Vault
            if resolved.is_none() {
                let specter_vault = crate::config::AutomaConfig::load().resolved_vault_dir();
                let config_vault =
                    PathBuf::from(&config.data_dir).join(crate::constants::DIR_WORKFLOWS);

                let candidates = [
                    specter_vault.join(format!("{}.workflow.json", target)),
                    specter_vault.join(format!("{}.json", target)),
                    config_vault.join(format!("{}.workflow.json", target)),
                    config_vault.join(format!("{}.json", target)),
                ];

                for c in &candidates {
                    if c.exists()
                        && c.is_file()
                        && let Ok(text) = std::fs::read_to_string(c)
                    {
                        resolved = Some((text, format!("Vault File: {}", c.display())));
                        break;
                    }
                }
            }

            match resolved {
                Some(r) => r,
                None => {
                    return Err(format!(
                        "Workflow file or ID '{}' not found in file system, database, or vault.",
                        target
                    )
                    .into());
                }
            }
        }
    };

    println!("Target:      {}", target);
    println!("Source:      {}", source_label);

    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("Failed to parse workflow file as valid JSON: {}", e).into());
        }
    };

    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("(Unnamed Workflow)");
    let description = val
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("(No description)");
    println!("Name:        {}", name);
    println!("Description: {}", description);

    let parsed_drawflow_opt = val
        .get("drawflow")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

    let nodes: Vec<serde_json::Value> = if let Some(arr) =
        val.pointer("/drawflow/nodes").and_then(|v| v.as_array())
    {
        arr.clone()
    } else if let Some(arr) = parsed_drawflow_opt
        .as_ref()
        .and_then(|d| d.get("nodes"))
        .and_then(|v| v.as_array())
    {
        arr.clone()
    } else if let Some(arr) = val.get("nodes").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };

    let edge_count = if let Some(arr) = val.pointer("/drawflow/edges").and_then(|v| v.as_array()) {
        arr.len()
    } else if let Some(arr) = parsed_drawflow_opt
        .as_ref()
        .and_then(|d| d.get("edges"))
        .and_then(|v| v.as_array())
    {
        arr.len()
    } else if let Some(arr) = val.get("edges").and_then(|v| v.as_array()) {
        arr.len()
    } else {
        0
    };

    println!("Structure:   {} nodes, {} connections", nodes.len(), edge_count);
    println!("------------------------------------------------------------");

    let mut triggers = Vec::new();
    let mut block_list = Vec::new();

    for node in &nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let label = node
            .get("label")
            .or_else(|| node.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let data = node.get("data");
        block_list.push((node_id, label, data));

        if label == "trigger" {
            let trigger_type = node
                .pointer("/data/type")
                .and_then(|v| v.as_str())
                .unwrap_or("manual");
            triggers.push((node_id, trigger_type, node.pointer("/data/parameters")));
        }
    }

    if triggers.is_empty() {
        println!("Triggers: [!] NO TRIGGER NODE FOUND");
    } else {
        println!("Triggers ({}):", triggers.len());
        for (id, t_type, params) in &triggers {
            println!("  - [{}] Type: {}", id, t_type);
            if let Some(param_arr) = params.and_then(|p| p.as_array())
                && !param_arr.is_empty()
            {
                println!("    Parameters:");
                for p in param_arr {
                    let pname = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let pval = p
                        .get("defaultValue")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "null".to_string());
                    println!("      * {} (default: {})", pname, pval);
                }
            }
        }
    }

    println!("\nBlocks Sequence ({}):", block_list.len());
    for (i, (id, label, data)) in block_list.iter().enumerate() {
        let mut detail = String::new();
        if let Some(d) = data {
            if let Some(url) = d.get("url").and_then(|v| v.as_str()) {
                detail = format!("url: {}", url);
            } else if let Some(time) = d.get("time").and_then(|v| v.as_i64()) {
                detail = format!("delay: {}ms", time);
            } else if let Some(sel) = d.get("selector").and_then(|v| v.as_str()) {
                detail = format!("selector: {}", sel);
            } else if let Some(code) = d.get("code").and_then(|v| v.as_str()) {
                let first_line = code
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect::<String>();
                detail = format!("js: {}...", first_line);
            }
        }
        if detail.is_empty() {
            println!("  {:2}. [{:<16}] id: {}", i + 1, label, id);
        } else {
            println!("  {:2}. [{:<16}] id: {} ({})", i + 1, label, id, detail);
        }
    }

    println!("\nPredefined Variables:");
    if let Some(vars) = val.get("variables") {
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                println!("  - {}: {}", k, v);
            }
        } else if let Some(arr) = vars.as_array() {
            for item in arr {
                println!("  - {}", item);
            }
        }
    } else {
        println!("  (None)");
    }

    // Dynamic expressions scan (pure stdlib, zero regex dependency)
    let mut expressions = BTreeSet::new();
    let mut remaining = content.as_str();
    while let Some(start) = remaining.find("{{") {
        remaining = &remaining[start + 2..];
        if let Some(end) = remaining.find("}}") {
            let expr = remaining[..end].trim();
            if !expr.is_empty() {
                expressions.insert(expr.to_string());
            }
            remaining = &remaining[end + 2..];
        } else {
            break;
        }
    }

    if !expressions.is_empty() {
        println!("\nDynamic Expressions Detected:");
        for expr in &expressions {
            println!("  - {{{{ {} }}}}", expr);
        }
    }

    println!("============================================================");
    println!("Status: VALID WORKFLOW");
    println!("Execution command:");
    println!("  specter automa run \"{}\"", target);
    println!("============================================================");

    Ok(())
}
