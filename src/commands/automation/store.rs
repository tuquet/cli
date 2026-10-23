use std::path::PathBuf;

use serde_json::{Value, json};

use crate::infrastructure::cloud::CloudWorkflowRepository;

/// Normalise a workflow document into the Cloud `graph_data` shape `{nodes, edges}`.
///
/// Accepts modern `.workflow.json` (`nodes`/`edges` at the root), drawflow payloads
/// (`drawflow` object) and drawflow strings.
fn extract_graph(val: &Value) -> Value {
    if let Some(s) = val.get("drawflow").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            return json!({
                "nodes": parsed.get("nodes").cloned().unwrap_or_else(|| json!([])),
                "edges": parsed.get("edges").cloned().unwrap_or_else(|| json!([])),
            });
        }
    }
    if let Some(drawflow) = val.get("drawflow") {
        return json!({
            "nodes": drawflow.get("nodes").cloned().unwrap_or_else(|| json!([])),
            "edges": drawflow.get("edges").cloned().unwrap_or_else(|| json!([])),
        });
    }
    json!({
        "nodes": val.get("nodes").cloned().unwrap_or_else(|| json!([])),
        "edges": val.get("edges").cloned().unwrap_or_else(|| json!([])),
    })
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
    let val: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file as JSON: {}", e))?;
    if !val.is_object() {
        return Err("Workflow file must contain a JSON object".into());
    }

    let derived_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".workflow").replace(' ', "_"))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let name = name_opt
        .or_else(|| val.get("name").and_then(Value::as_str).map(str::to_string))
        .unwrap_or(derived_stem);

    let description = desc_opt.or_else(|| {
        val.get("description")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let version = val
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("1.0.0")
        .to_string();

    let settings = val.get("settings").cloned().unwrap_or_else(|| json!({}));
    let graph = extract_graph(&val);

    let repo = CloudWorkflowRepository::connect().await.map_err(|e| -> Box<dyn std::error::Error> {
        e.into()
    })?;
    let wf = repo
        .upsert(
            id_opt.as_deref(),
            &name,
            description.as_deref(),
            &version,
            graph,
            settings,
        )
        .await?;

    crate::ui::Notify::header("Workflow Stored in Cloud");
    crate::ui::Notify::key_val("ID", &wf.id);
    crate::ui::Notify::key_val("Name", &wf.name);
    crate::ui::Notify::key_val("Version", &wf.version);
    if let Some(ref d) = wf.description {
        crate::ui::Notify::key_val("Description", d);
    }
    crate::ui::Notify::key_val("Blocks", wf.block_count());
    crate::ui::Notify::key_val("Table", "automation.workflows");
    crate::ui::Notify::divider();
    println!(" 💡 Ready to execute:");
    println!("    tq automation run {} --headless", wf.name);
    crate::ui::Notify::divider();

    Ok(())
}

pub async fn export_workflow(
    id: String,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CloudWorkflowRepository::connect().await.map_err(|e| -> Box<dyn std::error::Error> {
        e.into()
    })?;
    let wf = repo
        .find(&id)
        .await?
        .ok_or_else(|| format!("Workflow '{}' not found in automation.workflows.", id))?;

    let mut doc = json!({
        "id": wf.id,
        "name": wf.name,
        "description": wf.description,
        "version": wf.version,
        "status": wf.status,
        "settings": wf.settings,
    });
    if let (Some(obj), Some(graph)) = (doc.as_object_mut(), wf.graph_data.as_object()) {
        for (k, v) in graph {
            obj.insert(k.clone(), v.clone());
        }
    }

    let final_content = serde_json::to_string_pretty(&doc)?;
    let safe_name = wf.name.replace(['/', ' '], "_");
    let dest = output.unwrap_or_else(|| PathBuf::from(format!("{}.workflow.json", safe_name)));
    if let Some(parent) = dest.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&dest, final_content).await?;

    crate::ui::Notify::success(format!(
        "Workflow '{}' exported from Cloud to: {:?}",
        wf.name, dest
    ));
    Ok(())
}

pub async fn delete_workflow(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CloudWorkflowRepository::connect().await.map_err(|e| -> Box<dyn std::error::Error> {
        e.into()
    })?;
    if repo.delete(&id).await? {
        crate::ui::Notify::success(format!(
            "Workflow '{}' deleted from automation.workflows.",
            id
        ));
    } else {
        crate::ui::Notify::warn(format!(
            "Workflow '{}' not found in automation.workflows.",
            id
        ));
    }
    Ok(())
}
