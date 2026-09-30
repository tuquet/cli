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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()?;

    match client.get(&target).send().await {
        Ok(res) if res.status().is_success() => {
            let mut card = crate::ui::Card::new("RUNNER DAEMON");
            card.with_badge(crate::ui::badge_online("ONLINE (HTTP 200)"));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            card.add_kv("Worker Driver", "mv3_extension_worker (CDP Bridge)");
            card.add_kv("Health Route", &target);
            card.with_footer("Worker daemon is ready to receive and execute jobs");
            println!();
            card.print();
            println!();
        }
        Ok(res) => {
            let mut card = crate::ui::Card::new("RUNNER DAEMON");
            card.with_badge(crate::ui::badge_error(&format!("HTTP {}", res.status())));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            println!();
            card.print();
            println!();
        }
        Err(e) => {
            let mut card = crate::ui::Card::new("RUNNER DAEMON");
            card.with_badge(crate::ui::badge_offline("OFFLINE"));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            card.add_kv("Diagnostic", format!("{}", e));
            card.with_footer("Start local daemon with 'tuquet runner start --port 8765'");
            println!();
            card.print();
            println!();
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

    let exists = ext_dir.exists();
    let badge = if exists {
        crate::ui::badge_online("READY")
    } else {
        crate::ui::badge_warn("NOT BUILT")
    };

    let mut card = crate::ui::Card::new("BROWSER EXTENSION RUNNER");
    card.with_badge(badge);
    card.with_min_width(64);
    card.add_kv("Target Browser", browser);
    card.add_kv("Extension Path", ext_dir.display().to_string());

    if !exists {
        card.add_line("Status: Extension unpacked directory does not exist yet.");
        card.with_footer("Build extension with: pnpm --filter @automa/runner build");
    } else {
        card.add_line(format!("Launch command: chrome.exe --load-extension=\"{}\"", ext_dir.display()));
        card.with_footer("Ready to launch and attach to worker daemon");
    }
    println!();
    card.print();
    println!();
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
