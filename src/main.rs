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
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--export-openapi") {
        use utoipa::OpenApi;
        let openapi = api::routes::ApiDoc::openapi();
        let json = openapi.to_pretty_json()?;
        let output_path = args.iter()
            .position(|arg| arg == "--export-openapi")
            .and_then(|idx| args.get(idx + 1))
            .filter(|p| !p.starts_with("--"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("openapi.json"));
        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&output_path, json)?;
        println!("OpenAPI spec successfully exported to {:?}", output_path);
        return Ok(());
    }

    let config = AppConfig::load();
    
    // Khởi tạo Logger
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
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");

    info!("Automa Core Daemon starting in Clean Architecture mode...");
    info!("Environment: {}", config.environment);
    info!("Data Directory: {}", config.data_dir);
    
    // Đảm bảo thư mục data tồn tại
    std::fs::create_dir_all(&config.data_dir)?;

    // Khởi tạo Database
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
