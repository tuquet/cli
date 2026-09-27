#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use automa_core::infrastructure::db::AutomaDb;
use automa_core::config::AppConfig;
use automa_core::AppState;
use automa_core::api;
use automa_core::cli::{Cli, Commands};
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.probe || matches!(cli.command, Some(Commands::Probe)) {
        return print_probe_manifest();
    }

    if let Some(ref path_opt) = cli.export_openapi {
        let output_path = path_opt.clone().unwrap_or_else(|| std::path::PathBuf::from("openapi.json"));
        return export_openapi(&output_path);
    }

    match cli.command {
        Some(Commands::Probe) => {
            print_probe_manifest()
        }
        Some(Commands::Run { workflow, workflow_json, headless, browser_id }) => {
            run_workflow(workflow, workflow_json, headless, browser_id).await
        }
        Some(Commands::ExportOpenapi { output }) => {
            export_openapi(&output)
        }
        Some(Commands::Status { url }) => {
            check_status(&url).await
        }
        Some(Commands::SetupExt { browser, extension_path }) => {
            setup_extension(&browser, extension_path).await
        }
        Some(Commands::Server { port, data_dir, log_level }) => {
            run_server(port, data_dir, log_level).await
        }
        None => {
            run_server(None, None, None).await
        }
    }
}

fn print_probe_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = serde_json::json!({
        "protocol": "tuquet.automa.v1",
        "name": "automa-runner",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "chromium-extension-worker",
        "status": "ready",
        "capabilities": [
            "browser:chromium",
            "mv3_extension_worker",
            "isolation:profile_sandbox",
            "headless",
            "automation:workflow_graph"
        ],
        "plugin_type": "runner_driver"
    });
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

fn export_openapi(output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use utoipa::OpenApi;
    let openapi = api::routes::ApiDoc::openapi();
    let json = openapi.to_pretty_json()?;
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(output_path, json)?;
    println!("OpenAPI spec successfully exported to {:?}", output_path);
    Ok(())
}

async fn check_status(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let target = format!("{}/api/v1/health", url.trim_end_matches('/'));
    println!("Querying Automa Core daemon status at {} ...", target);
    match reqwest::get(&target).await {
        Ok(res) if res.status().is_success() => {
            println!("Status: ONLINE [HTTP 200]");
            let text = res.text().await?;
            println!("Response: {}", text);
        }
        Ok(res) => {
            println!("Status: ERROR [HTTP {}]", res.status());
        }
        Err(e) => {
            println!("Status: OFFLINE or UNREACHABLE ({})", e);
        }
    }
    Ok(())
}

async fn setup_extension(browser: &str, extension_path: Option<std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let ext_dir = extension_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("apps")
            .join("webe")
            .join("dist")
    });

    println!("============================================================");
    println!(" Automa Web Extension Setup Utility");
    println!("============================================================");
    println!("Target Browser: {}", browser);
    println!("Extension Path: {}", ext_dir.display());

    if !ext_dir.exists() {
        println!("Status: Extension path does not exist yet.");
        println!("Hint: Build extension first with: pnpm --filter @automa/webe build");
    } else {
        println!("Status: Extension directory verified.");
        println!("To launch Chrome manually with extension loaded:");
        println!("  chrome.exe --load-extension=\"{}\"", ext_dir.display());
    }
    Ok(())
}

async fn run_workflow(
    workflow_path_opt: Option<String>,
    workflow_json_opt: Option<String>,
    headless: bool,
    browser_id_opt: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let app = api::routes::create_router(state.clone());

    let _server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let workflow_json_val: Option<serde_json::Value> = if let Some(ref raw_json) = workflow_json_opt {
        serde_json::from_str(raw_json).ok()
    } else {
        None
    };

    let submit_options = automa_core::api::handlers::jobs::SubmitJobOptions {
        browser_id: browser_id_opt.or_else(|| Some("run_worker".to_string())),
        headless: Some(headless),
        default_browser: None,
        variables: None,
        debug: Some(true),
        close_browser_on_finish: Some(true),
    };

    println!(">> Submitting workflow to browser worker (Bridge Port: {})...", bound_port);

    use automa_core::core::engine::job_coordinator::JobCoordinator;
    let job_id = match JobCoordinator::submit(
        &state,
        None,
        workflow_path_opt.as_deref(),
        workflow_json_val.as_ref(),
        Some(submit_options),
    ).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error submitting workflow: {}", e);
            std::process::exit(1);
        }
    };

    println!(">> Workflow dispatched [Job ID: {}]. Waiting for worker execution...", job_id);

    // Stream logs to console until job finishes
    while let Ok(msg) = rx.recv().await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
            if let Some(event_type) = val.get("type").and_then(|v| v.as_str()) {
                if event_type == "job_finish" || event_type == "job_completed" || event_type == "job_failed" || event_type == "workflow_finished" {
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

    // Clean up server and browser child processes
    _server_handle.abort();
    automa_core::core::browser::manager::BrowserManager::destroy_all().await;

    // Cleanup ephemeral data directory
    let _ = std::fs::remove_dir_all(&data_dir);
    println!(">> Run completed successfully.");
    std::process::exit(0);
}

async fn run_server(
    port_override: Option<u16>,
    data_dir_override: Option<std::path::PathBuf>,
    log_level_override: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::load();
    if let Some(port) = port_override {
        config.server_port = port;
    }
    if let Some(dir) = data_dir_override {
        config.data_dir = dir.to_string_lossy().to_string();
    }
    if let Some(level) = log_level_override {
        config.log_level = level;
    }

    let log_level = match config.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Tuquet Automa Core Bridge starting in Native Launcher mode...");
    info!("Environment: {}", config.environment);
    info!("Data Directory: {}", config.data_dir);
    
    std::fs::create_dir_all(&config.data_dir)?;

    let db_path = std::path::Path::new(&config.data_dir).join("automa.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path)?));

    let (tx, _) = tokio::sync::broadcast::channel(10000);
    let (worker_tx, _) = tokio::sync::broadcast::channel(10000);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx,
        worker_tx,
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = api::routes::create_router(state);

    let addr = format!("127.0.0.1:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", listener.local_addr()?);
    let panic_log_path = std::path::PathBuf::from(&config.data_dir).join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[AUTOMA-CORE PANIC] {:?}\n", info);
        eprintln!("{}", msg);
        let _ = std::fs::write(&panic_log_path, &msg);
    }));

    let shutdown_signal = async {
        #[cfg(windows)]
        {
            if std::env::var("AUTOMA_NO_CTRLC_SHUTDOWN").is_ok() {
                std::future::pending::<()>().await;
            } else {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
        #[cfg(not(windows))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        eprintln!("[AUTOMA-CORE SHUTDOWN TRIGGERED] SIGINT/Ctrl-C received!");
        info!("Shutdown signal received. Cleaning up child processes...");
        automa_core::core::browser::manager::BrowserManager::destroy_all().await;
        info!("All browser processes cleaned up.");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await {
        eprintln!("[AUTOMA-CORE SERVER ERROR] {:?}", e);
        return Err(e.into());
    }
    
    eprintln!("[AUTOMA-CORE EXITED MAIN OK]");
    Ok(())
}
