use std::path::PathBuf;

use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;

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
        let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMA_SQLITE);
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
        let specter_vault = crate::config::AutomaConfig::load().resolved_vault_dir();
        let config_vault = PathBuf::from(&config.data_dir).join(crate::constants::DIR_WORKFLOWS);

        let vault_dirs = [specter_vault, config_vault];
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
                                        item.source = "DB+Disk".to_string();
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

                                let updated_at = "Disk File".to_string();

                                workflows.push(WfInfo {
                                    id,
                                    name,
                                    version,
                                    source: "Disk".to_string(),
                                    blocks,
                                    updated_at,
                                });
                            }
                }
            }
        }
    }

    println!();
    let columns = vec![
        crate::ui::Column { title: "ID".to_string(), min_width: 18, align_right: false },
        crate::ui::Column { title: "NAME".to_string(), min_width: 24, align_right: false },
        crate::ui::Column { title: "VERSION".to_string(), min_width: 8, align_right: false },
        crate::ui::Column { title: "SOURCE".to_string(), min_width: 10, align_right: false },
        crate::ui::Column { title: "BLOCKS".to_string(), min_width: 6, align_right: true },
        crate::ui::Column { title: "UPDATED".to_string(), min_width: 12, align_right: false },
    ];
    let mut table = crate::ui::Table::new(columns);
    for wf in &workflows {
        table.add_row(vec![
            wf.id.clone(),
            wf.name.clone(),
            wf.version.clone(),
            wf.source.clone(),
            wf.blocks.to_string(),
            wf.updated_at.clone(),
        ]);
    }
    if let Some(first) = workflows.first() {
        table = table.with_footer(format!("Run workflow with: specter automa run {} --headless", first.id));
    } else {
        table = table.with_footer("Import a workflow with: specter automa workflow import <file.json> --id <workflow_id>");
    }
    table.print();
    println!();

    Ok(())
}
