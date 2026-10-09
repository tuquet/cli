use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;
use crate::AppState;

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

    let workflow_json_val: Option<serde_json::Value> =
        if let Some(ref raw_json) = workflow_json_opt {
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

    crate::ui::Notify::info(format!(
        "Submitting workflow to browser worker (Bridge Port: {})...",
        bound_port
    ));

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
            return Err(format!("Error submitting workflow: {}", e).into());
        }
    };

    crate::ui::Notify::info(format!(
        "Workflow dispatched [Job ID: {}]. Waiting for worker execution...",
        job_id
    ));

    // Stream logs to console until job finishes or timeout/ctrl-c occurs
    let execution_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!();
            crate::ui::Notify::warn(crate::constants::MSG_JOB_CANCELLED);
            Err("Interrupted by user (SIGINT)")
        }
        _ = tokio::time::sleep(timeout_duration) => {
            eprintln!();
            crate::ui::Notify::error(format!("Execution timed out after {:?}.", timeout_duration));
            Err("Workflow execution timed out")
        }
        res = async {
            let mut failed = false;
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                            if let Some(event_type) = val.get("type").and_then(|v| v.as_str()) {
                                if event_type == "job_failed" {
                                    failed = true;
                                    crate::ui::Notify::info(format!("Job finished with event: {}", event_type));
                                    break;
                                } else if event_type == "job_finish" || event_type == "job_completed" || event_type == "workflow_finished" {
                                    crate::ui::Notify::info(format!("Job finished with event: {}", event_type));
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
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Workflow log stream lagged; skipped {} events", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
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
            crate::ui::Notify::success(crate::constants::MSG_JOB_COMPLETED);
            Ok(())
        }
        Err(e) => {
            crate::ui::Notify::error(format!("Run failed: {}", e));
            Err(e.into())
        }
    }
}
