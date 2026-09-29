use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::cli::RunnerSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::db::AutomaDb;
use crate::AppState;

pub async fn handle(command: RunnerSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RunnerSubcommands::Start {
            host,
            port,
            data_dir,
            log_level,
        } => run_server(host, port, data_dir, log_level).await,
        RunnerSubcommands::Status { url } => {
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| "8765".to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url).await
        }
        RunnerSubcommands::Probe => print_probe_manifest(),
        RunnerSubcommands::ExportOpenapi { output } => export_openapi(&output),
        RunnerSubcommands::SetupExt {
            browser,
            extension_path,
        } => setup_extension(&browser, extension_path).await,
    }
}

pub fn print_probe_manifest() -> Result<(), Box<dyn std::error::Error>> {
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

pub fn export_openapi(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use utoipa::OpenApi;
    let openapi = crate::api::routes::ApiDoc::openapi();
    let json = openapi.to_pretty_json()?;
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(output_path, json)?;
    println!("OpenAPI spec successfully exported to {:?}", output_path);
    Ok(())
}

pub async fn check_status(url: &str) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn setup_extension(
    browser: &str,
    extension_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext_dir = extension_path.unwrap_or_else(|| {
        PathBuf::from(crate::core::browser::worker_coordinator::resolve_cli_runner_extension_path())
    });

    println!("============================================================");
    println!(" Automa Web Extension Setup Utility");
    println!("============================================================");
    println!("Target Browser: {}", browser);
    println!("Extension Path: {}", ext_dir.display());

    if !ext_dir.exists() {
        println!("Status: Extension path does not exist yet.");
        println!("Hint: Build extension first with: pnpm --filter @automa/runner build");
    } else {
        println!("Status: Extension directory verified.");
        println!("To launch Chrome manually with extension loaded:");
        println!("  chrome.exe --load-extension=\"{}\"", ext_dir.display());
    }
    Ok(())
}

pub async fn run_server(
    host_override: Option<String>,
    port_override: Option<u16>,
    data_dir_override: Option<PathBuf>,
    log_level_override: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::load();
    if let Some(host) = host_override {
        config.server_host = host;
    }
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

    let db_path = Path::new(&config.data_dir).join("automa.sqlite");
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

    let app = crate::api::routes::create_router(state.clone());

    // Start background Cloud Telemetry & Inventory Reporter (runs if TUQUET_CLOUD_URL is configured)
    let _reporter_handle =
        crate::infrastructure::cloud_reporter::CloudReporter::start_background_loop(state.clone());

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", listener.local_addr()?);
    let panic_log_path = PathBuf::from(&config.data_dir).join("panic.log");
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
        crate::core::browser::manager::BrowserManager::destroy_all().await;
        info!("All browser processes cleaned up.");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
    {
        eprintln!("[AUTOMA-CORE SERVER ERROR] {:?}", e);
        return Err(e.into());
    }

    eprintln!("[AUTOMA-CORE EXITED MAIN OK]");
    Ok(())
}
