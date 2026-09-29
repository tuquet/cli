use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cli::{AutomaSubcommands, WorkflowCommands};
use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;
use crate::AppState;

pub async fn handle(command: AutomaSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AutomaSubcommands::Run {
            workflow_pos,
            workflow,
            workflow_json,
            headless,
            browser,
            browser_id,
            variables,
            timeout,
        } => {
            let target_workflow = workflow.or(workflow_pos);
            run_workflow(
                target_workflow,
                workflow_json,
                headless,
                browser,
                browser_id,
                variables,
                timeout,
            )
            .await
        }
        AutomaSubcommands::Workflow { command } => match command {
            WorkflowCommands::List {
                search,
                db_only,
                vault_only,
            } => list_workflows(search, db_only, vault_only).await,
            WorkflowCommands::Import {
                file,
                id,
                name,
                description,
            } => import_workflow(file, id, name, description).await,
            WorkflowCommands::Export { id, output } => export_workflow(id, output).await,
            WorkflowCommands::Info { id } => inspect_workflow(&id),
            WorkflowCommands::Delete { id, vault } => delete_workflow(id, vault).await,
        },
        AutomaSubcommands::Inspect { workflow } => inspect_workflow(&workflow),
        AutomaSubcommands::Studio => open_studio(),
        AutomaSubcommands::Probe => crate::commands::runner::print_probe_manifest(),
    }
}

pub fn open_studio() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let port = config.server_port;
    let url = std::env::var("AUTOMA_STUDIO_URL").unwrap_or_else(|_| {
        format!("https://automa-studio.vercel.app?port={}", port)
    });
    println!("Opening Automa Web Studio at: {}", url);
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", &url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    Ok(())
}

pub async fn list_workflows(
    search: Option<String>,
    db_only: bool,
    vault_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    struct WfInfo {
        id: String,
        name: String,
        version: String,
        source: String,
        blocks: usize,
        updated_at: String,
    }

    let config = AppConfig::load();
    let mut workflows: Vec<WfInfo> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. Fetch from SQLite Database if not vault_only
    if !vault_only {
        let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
        if db_path.exists()
            && let Ok(db) = AutomaDb::new(&db_path)
                && let Ok(list) = db.workflows().get_workflows(None, None, search.as_deref()) {
                    for wf in list {
                        let parsed: Option<serde_json::Value> = serde_json::from_str(&wf.data).ok();
                        let blocks = parsed
                            .as_ref()
                            .map(|v| {
                                if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
                                    nodes.len()
                                } else if let Some(drawflow) = v.get("drawflow") {
                                    if let Some(s) = drawflow.as_str() {
                                        serde_json::from_str::<serde_json::Value>(s)
                                            .ok()
                                            .and_then(|d| {
                                                d.get("nodes")
                                                    .and_then(|n| n.as_array())
                                                    .map(|a| a.len())
                                            })
                                            .unwrap_or(0)
                                    } else if let Some(nodes) =
                                        drawflow.get("nodes").and_then(|n| n.as_array())
                                    {
                                        nodes.len()
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                }
                            })
                            .unwrap_or(0);

                        seen_ids.insert(wf.id.clone());
                        workflows.push(WfInfo {
                            id: wf.id,
                            name: wf.name,
                            version: wf.version,
                            source: "Database".to_string(),
                            blocks,
                            updated_at: wf.updated_at,
                        });
                    }
                }
    }

    // 2. Fetch from Vault if not db_only
    if !db_only {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = PathBuf::from(&config.data_dir).join("workflows");

        let vault_dirs = [user_vault, config_vault];
        for vdir in &vault_dirs {
            if let Ok(mut entries) = tokio::fs::read_dir(vdir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() && (path.extension().map(|e| e == "json").unwrap_or(false))
                        && let Ok(content) = tokio::fs::read_to_string(&path).await
                            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                let stem =
                                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("wf");
                                let id = val
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        stem.trim_end_matches(".workflow").to_string()
                                    });
                                let name = val
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| id.clone());
                                let version = val
                                    .get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("1.0.0")
                                    .to_string();
                                let desc =
                                    val.get("description").and_then(|v| v.as_str()).unwrap_or("");

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
                                        item.source = "DB+Vault".to_string();
                                    }
                                    continue;
                                }

                                seen_ids.insert(id.clone());

                                let blocks = if let Some(nodes) =
                                    val.get("nodes").and_then(|n| n.as_array())
                                {
                                    nodes.len()
                                } else if let Some(drawflow) = val.get("drawflow") {
                                    if let Some(s) = drawflow.as_str() {
                                        serde_json::from_str::<serde_json::Value>(s)
                                            .ok()
                                            .and_then(|d| {
                                                d.get("nodes")
                                                    .and_then(|n| n.as_array())
                                                    .map(|a| a.len())
                                            })
                                            .unwrap_or(0)
                                    } else if let Some(nodes) =
                                        drawflow.get("nodes").and_then(|n| n.as_array())
                                    {
                                        nodes.len()
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };

                                let updated_at = "Vault File".to_string();

                                workflows.push(WfInfo {
                                    id,
                                    name,
                                    version,
                                    source: "Vault".to_string(),
                                    blocks,
                                    updated_at,
                                });
                            }
                }
            }
        }
    }

    println!("========================================================================================");
    println!(" 📂 AUTOMA SAVED WORKFLOWS ({} Workflows Found)", workflows.len());
    println!("========================================================================================");
    if workflows.is_empty() {
        println!(" (No workflows found)");
        println!("----------------------------------------------------------------------------------------");
        println!("💡 Import a workflow with:");
        println!("   tuquet automa workflow import <file.json> --id <workflow_id>");
    } else {
        println!(
            " {:<20} {:<24} {:<8} {:<10} {:<8} UPDATED AT",
            "ID", "NAME", "VERSION", "SOURCE", "BLOCKS"
        );
        println!("----------------------------------------------------------------------------------------");
        for wf in &workflows {
            let id_display = if wf.id.len() > 19 {
                format!("{}...", &wf.id[..16])
            } else {
                wf.id.clone()
            };
            let name_display = if wf.name.len() > 23 {
                format!("{}...", &wf.name[..20])
            } else {
                wf.name.clone()
            };
            println!(
                " {:<20} {:<24} {:<8} {:<10} {:<8} {}",
                id_display, name_display, wf.version, wf.source, wf.blocks, wf.updated_at
            );
        }
        println!("========================================================================================");
        println!("💡 Run with:  tuquet automa run <ID>");
        if let Some(first) = workflows.first() {
            println!("   Example:   tuquet automa run {} --headless", first.id);
        }
    }
    println!("========================================================================================");

    Ok(())
}

pub async fn import_workflow(
    file: PathBuf,
    id_opt: Option<String>,
    name_opt: Option<String>,
    desc_opt: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() || !file.is_file() {
        return Err(format!("File does not exist or is not a file: {}", file.display()).into());
    }

    let content = tokio::fs::read_to_string(&file).await?;
    let mut val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file as JSON: {}", e))?;

    if !val.is_object() {
        return Err("Workflow file must contain a JSON object".into());
    }

    let derived_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".workflow").replace(' ', "_"))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let id = id_opt
        .or_else(|| val.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or(derived_stem);

    let name = name_opt
        .or_else(|| val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| id.clone());

    let description = desc_opt
        .or_else(|| val.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()));

    let version = val
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();
    let icon = val.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Synchronize ID and Name into JSON data
    if let Some(obj) = val.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
        obj.insert("name".to_string(), serde_json::Value::String(name.clone()));
        if let Some(ref desc) = description {
            obj.insert("description".to_string(), serde_json::Value::String(desc.clone()));
        }
    }

    let serialized = serde_json::to_string_pretty(&val)?;

    let config = AppConfig::load();
    let db_dir = PathBuf::from(&config.data_dir);
    tokio::fs::create_dir_all(&db_dir).await?;
    let db_path = db_dir.join("automa.sqlite");

    let db = AutomaDb::new(&db_path)?;
    db.workflows().create_workflow(
        &id,
        &name,
        description.as_deref(),
        &serialized,
        Some(&version),
        icon.as_deref(),
    )?;

    // Also copy to Vault ~/.automa/workflows/
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let vault_dir = PathBuf::from(home).join(".automa").join("workflows");
    let _ = tokio::fs::create_dir_all(&vault_dir).await;
    let vault_file = vault_dir.join(format!("{}.workflow.json", id));
    tokio::fs::write(&vault_file, &serialized).await?;

    let block_count = val
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .or_else(|| {
            val.pointer("/drawflow/nodes")
                .and_then(|n| n.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);

    println!("============================================================");
    println!(" ✔ Workflow Successfully Imported");
    println!("============================================================");
    println!(" ID:          {}", id);
    println!(" Name:        {}", name);
    println!(" Version:     {}", version);
    if let Some(ref d) = description {
        println!(" Description: {}", d);
    }
    println!(" Blocks:      {}", block_count);
    println!(" Vault File:  {}", vault_file.display());
    println!(" Database:    {}", db_path.display());
    println!("------------------------------------------------------------");
    println!(" 💡 Ready to execute:");
    println!("    tuquet automa run {} --headless", id);
    println!("============================================================");

    Ok(())
}

pub async fn export_workflow(
    id: String,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut workflow_json: Option<String> = None;

    // 1. Check persistent SQLite DB
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
            && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(&id) {
                workflow_json = Some(wf.data);
            }

    // 2. Check Vault
    if workflow_json.is_none() {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = PathBuf::from(&config.data_dir).join("workflows");

        let candidates = [
            user_vault.join(format!("{}.workflow.json", id)),
            user_vault.join(format!("{}.json", id)),
            config_vault.join(format!("{}.workflow.json", id)),
            config_vault.join(format!("{}.json", id)),
        ];

        for c in &candidates {
            if c.exists() && c.is_file()
                && let Ok(content) = tokio::fs::read_to_string(c).await {
                    workflow_json = Some(content);
                    break;
                }
        }
    }

    let raw_data = match workflow_json {
        Some(d) => d,
        None => {
            return Err(format!("Workflow '{}' not found in database or vault.", id).into());
        }
    };

    let final_content = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_data) {
        serde_json::to_string_pretty(&val)?
    } else {
        raw_data
    };

    let dest = output.unwrap_or_else(|| PathBuf::from(format!("{}.workflow.json", id)));
    if let Some(parent) = dest.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&dest, final_content).await?;

    println!("✔ Workflow '{}' exported successfully to: {:?}", id, dest);
    Ok(())
}

pub async fn delete_workflow(
    id: String,
    delete_from_vault: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut deleted_db = false;
    let mut deleted_vault = false;

    // 1. Delete from SQLite DB
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
            && let Ok(res) = db.workflows().delete_workflow(&id) {
                deleted_db = res;
            }

    // 2. Delete from Vault if requested
    if delete_from_vault {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = PathBuf::from(&config.data_dir).join("workflows");

        let candidates = [
            user_vault.join(format!("{}.workflow.json", id)),
            user_vault.join(format!("{}.json", id)),
            config_vault.join(format!("{}.workflow.json", id)),
            config_vault.join(format!("{}.json", id)),
        ];

        for c in &candidates {
            if c.exists() && c.is_file()
                && tokio::fs::remove_file(c).await.is_ok() {
                    deleted_vault = true;
                }
        }
    }

    if deleted_db || deleted_vault {
        println!(
            "✔ Workflow '{}' deleted successfully (Database: {}, Vault: {}).",
            id, deleted_db, deleted_vault
        );
    } else {
        println!("Workflow '{}' was not found in database or vault.", id);
    }

    Ok(())
}

pub fn inspect_workflow(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!(" Automa Workflow Inspector");
    println!("============================================================");

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
            let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
            if db_path.exists()
                && let Ok(db) = AutomaDb::new(&db_path)
                    && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(target) {
                        resolved = Some((wf.data, format!("Database (ID: {})", wf.id)));
                    }

            // Check Vault
            if resolved.is_none() {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                let user_vault = PathBuf::from(&home).join(".automa").join("workflows");
                let config_vault = PathBuf::from(&config.data_dir).join("workflows");

                let candidates = [
                    user_vault.join(format!("{}.workflow.json", target)),
                    user_vault.join(format!("{}.json", target)),
                    config_vault.join(format!("{}.workflow.json", target)),
                    config_vault.join(format!("{}.json", target)),
                ];

                for c in &candidates {
                    if c.exists() && c.is_file()
                        && let Ok(text) = std::fs::read_to_string(c) {
                            resolved = Some((text, format!("Vault File: {}", c.display())));
                            break;
                        }
                }
            }

            match resolved {
                Some(r) => r,
                None => {
                    eprintln!(
                        "Error: Workflow file or ID '{}' not found in file system, database, or vault.",
                        target
                    );
                    std::process::exit(1);
                }
            }
        }
    };

    println!("Target:      {}", target);
    println!("Source:      {}", source_label);

    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Failed to parse workflow file as valid JSON: {}", e);
            std::process::exit(1);
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
                && !param_arr.is_empty() {
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

    // Dynamic expressions scan
    let re = regex::Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("Invalid regex: {}", e))?;
    let mut expressions = std::collections::BTreeSet::new();
    for cap in re.captures_iter(&content) {
        if let Some(matched) = cap.get(1) {
            expressions.insert(matched.as_str().trim().to_string());
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
    println!("  tuquet automa run \"{}\"", target);
    println!("============================================================");

    Ok(())
}

pub async fn run_workflow(
    workflow_path_opt: Option<String>,
    workflow_json_opt: Option<String>,
    headless: bool,
    browser_opt: Option<String>,
    browser_id_opt: Option<String>,
    variables: Vec<String>,
    timeout_opt: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout_duration = timeout_opt
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(300));
    let mut config = AppConfig::load();
    let data_dir = std::env::temp_dir().join(format!("automa_run_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&data_dir);
    config.data_dir = data_dir.to_string_lossy().to_string();

    let db_path = data_dir.join("automa_run.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path)?));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let bound_port = listener.local_addr()?.port();
    config.server_port = bound_port;

    // Set environment variable so browser extension connects to this ephemeral bridge port
    unsafe {
        std::env::set_var("AUTOMA_PORT", bound_port.to_string());
    }

    let (tx, mut rx) = tokio::sync::broadcast::channel(1000);
    let (worker_tx, _) = tokio::sync::broadcast::channel(1000);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx: tx.clone(),
        worker_tx,
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = crate::api::routes::create_router(state.clone());

    let _server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let workflow_json_val: Option<serde_json::Value> = if let Some(ref raw_json) = workflow_json_opt {
        serde_json::from_str(raw_json).ok()
    } else {
        None
    };

    let vars_map: Option<serde_json::Value> = if !variables.is_empty() {
        let mut map = serde_json::Map::new();
        for var_str in variables {
            if let Some((k, v)) = var_str.split_once('=') {
                let key = k.trim().to_string();
                let val_trimmed = v.trim();
                let parsed_val =
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(val_trimmed) {
                        val
                    } else {
                        serde_json::Value::String(val_trimmed.to_string())
                    };
                map.insert(key, parsed_val);
            } else {
                eprintln!(
                    "Warning: Invalid variable format '{}'. Expected KEY=VALUE",
                    var_str
                );
            }
        }
        Some(serde_json::Value::Object(map))
    } else {
        None
    };

    let submit_options = crate::api::handlers::jobs::SubmitJobOptions {
        browser_id: browser_id_opt.or_else(|| Some("run_worker".to_string())),
        headless: Some(headless),
        default_browser: browser_opt,
        variables: vars_map,
        debug: Some(true),
        close_browser_on_finish: Some(true),
    };

    println!(
        ">> Submitting workflow to browser worker (Bridge Port: {})...",
        bound_port
    );

    use crate::core::engine::job_coordinator::JobCoordinator;
    let job_id = match JobCoordinator::submit(
        &state,
        None,
        workflow_path_opt.as_deref(),
        workflow_json_val.as_ref(),
        Some(submit_options),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error submitting workflow: {}", e);
            std::process::exit(1);
        }
    };

    println!(
        ">> Workflow dispatched [Job ID: {}]. Waiting for worker execution...",
        job_id
    );

    // Stream logs to console until job finishes or timeout/ctrl-c occurs
    let execution_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\n>> Execution interrupted by user (Ctrl+C). Cleaning up...");
            Err("Interrupted by user (SIGINT)")
        }
        _ = tokio::time::sleep(timeout_duration) => {
            eprintln!("\n>> Execution timed out after {:?}.", timeout_duration);
            Err("Workflow execution timed out")
        }
        res = async {
            let mut failed = false;
            while let Ok(msg) = rx.recv().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if let Some(event_type) = val.get("type").and_then(|v| v.as_str()) {
                        if event_type == "job_failed" {
                            failed = true;
                            println!(">> Job finished with event: {}", event_type);
                            break;
                        } else if event_type == "job_finish" || event_type == "job_completed" || event_type == "workflow_finished" {
                            println!(">> Job finished with event: {}", event_type);
                            break;
                        }
                    }
                    if let Some(log_msg) = val.get("message").and_then(|v| v.as_str()) {
                        println!("[worker] {}", log_msg);
                    } else if let Some(data) = val.get("data") {
                        println!("[worker] {}", data);
                    }
                }
            }
            if failed {
                Err("Workflow job failed during execution")
            } else {
                Ok(())
            }
        } => res
    };

    // Clean up server and browser child processes
    _server_handle.abort();
    crate::core::browser::manager::BrowserManager::destroy_all().await;

    // Explicitly drop state to release SQLite file handles
    drop(state);

    #[cfg(target_os = "windows")]
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Cleanup ephemeral data directory with retry
    for _ in 0..5 {
        if std::fs::remove_dir_all(&data_dir).is_ok() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    match execution_result {
        Ok(_) => {
            println!(">> Run completed successfully.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!(">> Run failed: {}", e);
            std::process::exit(1);
        }
    }
}
