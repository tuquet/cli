use std::path::PathBuf;

use crate::config::AppConfig;
use crate::infrastructure::db::AutomationDb;

pub async fn list_workflows(
    search: Option<String>,
    format: crate::ui::OutputFormat,
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

    // 1. Fetch strictly from SQLite Database
    let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMATION_SQLITE);
    if db_path.exists()
        && let Ok(db) = AutomationDb::new(&db_path)
        && let Ok(list) = db.workflows().get_workflows(None, None, search.as_deref())
    {
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
                                    d.get("nodes").and_then(|n| n.as_array()).map(|a| a.len())
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
    let json_workflows: Vec<serde_json::Value> = workflows
        .iter()
        .map(|wf| {
            serde_json::json!({
                "id": wf.id,
                "name": wf.name,
                "version": wf.version,
                "source": wf.source,
                "blocks": wf.blocks,
                "updated_at": wf.updated_at
            })
        })
        .collect();

    crate::ui::respond_with(format, &json_workflows, |_| {
        println!();
        let columns = vec![
            crate::ui::Column {
                title: "ID".to_string(),
                min_width: 18,
                align_right: false,
            },
            crate::ui::Column {
                title: "NAME".to_string(),
                min_width: 24,
                align_right: false,
            },
            crate::ui::Column {
                title: "VERSION".to_string(),
                min_width: 8,
                align_right: false,
            },
            crate::ui::Column {
                title: "SOURCE".to_string(),
                min_width: 10,
                align_right: false,
            },
            crate::ui::Column {
                title: "BLOCKS".to_string(),
                min_width: 6,
                align_right: true,
            },
            crate::ui::Column {
                title: "UPDATED".to_string(),
                min_width: 12,
                align_right: false,
            },
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
            table = table.with_footer(format!(
                "Run workflow with: tq automation run {} --headless",
                first.id
            ));
        } else {
            table = table.with_footer("Import a workflow with: tq automation workflow import <file.json> --id <workflow_id>");
        }
        table.print();
        println!();
    })
}
