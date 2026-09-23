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

    if let Some(ref path_opt) = cli.export_openapi {
        let output_path = path_opt.clone().unwrap_or_else(|| std::path::PathBuf::from("openapi.json"));
        return export_openapi(&output_path);
    }

    match cli.command {
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

    info!("Automa Core Daemon starting in Clean Architecture mode...");
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
