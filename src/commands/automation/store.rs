use std::path::PathBuf;

use crate::config::AppConfig;
use crate::infrastructure::db::AutomationDb;

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
        .or_else(|| {
            val.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or(derived_stem);

    let name = name_opt
        .or_else(|| {
            val.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| id.clone());

    let description = desc_opt.or_else(|| {
        val.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });

    let version = val
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();
    let icon = val
        .get("icon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Synchronize ID and Name into JSON data
    if let Some(obj) = val.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
        obj.insert("name".to_string(), serde_json::Value::String(name.clone()));
        if let Some(ref desc) = description {
            obj.insert(
                "description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }
    }

    let serialized = serde_json::to_string_pretty(&val)?;

    let config = AppConfig::load();
    let db_dir = PathBuf::from(&config.data_dir);
    tokio::fs::create_dir_all(&db_dir).await?;
    let db_path = db_dir.join(crate::constants::FILE_AUTOMATION_SQLITE);

    let db = AutomationDb::new(&db_path)?;
    db.workflows().create_workflow(
        &id,
        &name,
        description.as_deref(),
        &serialized,
        Some(&version),
        icon.as_deref(),
    )?;

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

    crate::ui::Notify::header("Workflow Successfully Imported");
    crate::ui::Notify::key_val("ID", &id);
    crate::ui::Notify::key_val("Name", &name);
    crate::ui::Notify::key_val("Version", &version);
    if let Some(ref d) = description {
        crate::ui::Notify::key_val("Description", d);
    }
    crate::ui::Notify::key_val("Blocks", block_count);
    crate::ui::Notify::key_val("Database", db_path.display());
    crate::ui::Notify::divider();
    println!(" 💡 Ready to execute:");
    println!("    tq automation run {} --headless", id);
    crate::ui::Notify::divider();

    Ok(())
}

pub async fn export_workflow(
    id: String,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut workflow_json: Option<String> = None;

    // Check persistent SQLite DB
    let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMATION_SQLITE);
    if db_path.exists()
        && let Ok(db) = AutomationDb::new(&db_path)
        && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(&id)
    {
        workflow_json = Some(wf.data);
    }

    let raw_data = match workflow_json {
        Some(d) => d,
        None => {
            return Err(format!("Workflow '{}' not found in database.", id).into());
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

    crate::ui::Notify::success(format!(
        "Workflow '{}' exported successfully to: {:?}",
        id, dest
    ));
    Ok(())
}

pub async fn delete_workflow(
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut deleted_db = false;

    // Delete from SQLite DB
    let db_path = PathBuf::from(&config.data_dir).join(crate::constants::FILE_AUTOMATION_SQLITE);
    if db_path.exists()
        && let Ok(db) = AutomationDb::new(&db_path)
        && let Ok(res) = db.workflows().delete_workflow(&id)
    {
        deleted_db = res;
    }

    if deleted_db {
        crate::ui::Notify::success(format!(
            "Workflow '{}' deleted successfully from database.",
            id
        ));
    } else {
        crate::ui::Notify::warn(format!(
            "Workflow '{}' was not found in database.",
            id
        ));
    }

    Ok(())
}
