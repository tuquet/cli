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

pub fn get_pid_file_path(data_dir: &str) -> PathBuf {
    Path::new(data_dir).join("runner.pid")
}

pub fn get_log_file_path(data_dir: &str) -> PathBuf {
    Path::new(data_dir).join("logs").join("runner.log")
}

pub async fn handle(command: RunnerSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RunnerSubcommands::Start {
            host,
            port,
            detach,
            data_dir,
            log_level,
        } => run_server(host, port, detach, data_dir, log_level).await,
        RunnerSubcommands::Stop { force } => stop_daemon(force).await,
        RunnerSubcommands::Restart { detach } => restart_daemon(detach).await,
        RunnerSubcommands::Status { url } => {
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| "8765".to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url).await
        }
        RunnerSubcommands::Logs { follow, lines } => show_logs(follow, lines).await,
        RunnerSubcommands::Probe => print_probe_manifest(),
        RunnerSubcommands::ExportOpenapi { output } => export_openapi(&output),
        RunnerSubcommands::SetupExt {
            browser,
            extension_path,
        } => crate::commands::browser::setup_extension(&browser, extension_path).await,
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
            let mut card = crate::ui::Card::new("RUNNER");
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
            let mut card = crate::ui::Card::new("RUNNER");
            card.with_badge(crate::ui::badge_error(&format!("HTTP {}", res.status())));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            println!();
            card.print();
            println!();
        }
        Err(e) => {
            let mut card = crate::ui::Card::new("RUNNER");
            card.with_badge(crate::ui::badge_offline("OFFLINE"));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            card.add_kv("Diagnostic", format!("{}", e));
            card.with_footer("Start local daemon with 'tuquet runner start -d'");
            println!();
            card.print();
            println!();
        }
    }
    Ok(())
}

pub async fn stop_daemon(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let _ = force;
    let config = AppConfig::load();
    let pid_file = get_pid_file_path(&config.data_dir);
    
    let mut pid_to_kill: Option<u32> = None;
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(p) = content.trim().parse::<u32>() {
                pid_to_kill = Some(p);
            }
        }
    }

    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    
    let current_pid = std::process::id();
    let mut found_pids = Vec::new();
    if let Some(p) = pid_to_kill {
        found_pids.push(p);
    }

    for (p, proc) in sys.processes() {
        let p_u32 = p.as_u32();
        if p_u32 == current_pid {
            continue;
        }
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("tuquet") {
            let cmd = proc.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");
            if cmd.contains("runner") && cmd.contains("start") {
                if !found_pids.contains(&p_u32) {
                    found_pids.push(p_u32);
                }
            }
        }
    }

    if found_pids.is_empty() {
        let health_url = format!("http://{}:{}/api/v1/health", config.server_host, config.server_port);
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_millis(500)).build()?;
        let is_running = client.get(&health_url).send().await.map(|r| r.status().is_success()).unwrap_or(false);
        if !is_running {
            let mut card = crate::ui::Card::new("RUNNER");
            card.with_badge(crate::ui::badge_offline("NOT RUNNING"));
            card.with_min_width(64);
            card.add_line("No active runner daemon process found on this machine.");
            card.with_footer("Start daemon with 'tuquet runner start -d'");
            println!();
            card.print();
            println!();
            let _ = std::fs::remove_file(&pid_file);
            return Ok(());
        }
    }

    for &pid in &found_pids {
        #[cfg(target_os = "windows")]
        {
            let pid_str = pid.to_string();
            let mut cmd = tokio::process::Command::new("taskkill");
            // Windows console processes have no GUI window for WM_CLOSE, requiring /F to terminate
            let args = vec!["/F", "/T", "/PID", pid_str.as_str()];
            cmd.args(&args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let _ = cmd.status().await;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let sig = if force { "-9" } else { "-15" };
            let _ = tokio::process::Command::new("kill")
                .args([sig, &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }
    }

    let _ = std::fs::remove_file(&pid_file);

    let mut card = crate::ui::Card::new("RUNNER");
    card.with_badge(crate::ui::badge_online("STOPPED"));
    card.with_min_width(64);
    if !found_pids.is_empty() {
        let pids_str = found_pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
        card.add_kv("Terminated PID(s)", pids_str);
    }
    card.add_line("Runner worker daemon has been gracefully terminated.");
    card.with_footer("Start daemon again with 'tuquet runner start -d'");
    println!();
    card.print();
    println!();

    Ok(())
}

pub async fn restart_daemon(detach: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Stopping existing runner daemon...");
    let _ = stop_daemon(false).await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    println!("Starting runner daemon...");
    run_server(None, None, detach, None, None).await
}

pub async fn show_logs(follow: bool, lines: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let log_file = get_log_file_path(&config.data_dir);
    if !log_file.exists() {
        let mut card = crate::ui::Card::new("RUNNER");
        card.with_badge(crate::ui::badge_warn("NO LOGS FOUND"));
        card.with_min_width(64);
        card.add_kv("Expected Log File", log_file.display().to_string());
        card.add_line("Daemon has not written any background logs yet.");
        card.with_footer("Start daemon in background with 'tuquet runner start -d'");
        println!();
        card.print();
        println!();
        return Ok(());
    }

    let content = std::fs::read_to_string(&log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start_idx = all_lines.len().saturating_sub(lines);
    
    println!("\x1b[1;36m╭─ RUNNER LOGS ({}) ─\x1b[0m", log_file.display());
    for line in &all_lines[start_idx..] {
        println!("│ {}", line);
    }
    println!("\x1b[1;36m╰───────────────────────────────────────────────────\x1b[0m");

    if follow {
        use std::io::{BufRead, BufReader, Seek, SeekFrom};
        let file = std::fs::File::open(&log_file)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(0))?;

        let mut line_buf = String::new();
        loop {
            line_buf.clear();
            let bytes = reader.read_line(&mut line_buf)?;
            if bytes > 0 {
                print!("│ {}", line_buf);
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
        }
    }

    Ok(())
}

pub async fn run_server(
    host_override: Option<String>,
    port_override: Option<u16>,
    detach: bool,
    data_dir_override: Option<PathBuf>,
    log_level_override: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::load();
    if let Some(host) = host_override.clone() {
        config.server_host = host;
    }
    if let Some(port) = port_override {
        config.server_port = port;
    }
    if let Some(dir) = data_dir_override.clone() {
        config.data_dir = dir.to_string_lossy().to_string();
    }
    if let Some(level) = log_level_override.clone() {
        config.log_level = level;
    }

    if detach {
        let log_file_path = get_log_file_path(&config.data_dir);
        if let Some(parent) = log_file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exe = std::env::current_exe()?;
        let mut args = vec!["runner".to_string(), "start".to_string()];
        if let Some(ref h) = host_override {
            args.push("-H".to_string());
            args.push(h.clone());
        }
        if let Some(p) = port_override {
            args.push("-p".to_string());
            args.push(p.to_string());
        }
        if let Some(ref d) = data_dir_override {
            args.push("--data-dir".to_string());
            args.push(d.to_string_lossy().to_string());
        }
        if let Some(ref l) = log_level_override {
            args.push("--log-level".to_string());
            args.push(l.clone());
        }

        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;
        let err_file = out_file.try_clone()?;

        let mut cmd = std::process::Command::new(exe);
        cmd.args(&args)
            .stdout(std::process::Stdio::from(out_file))
            .stderr(std::process::Stdio::from(err_file));

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008)
            cmd.creation_flags(0x08000008);
        }

        let child = cmd.spawn()?;
        let child_pid = child.id();
        let pid_file_path = get_pid_file_path(&config.data_dir);
        let _ = std::fs::write(&pid_file_path, child_pid.to_string());

        tokio::time::sleep(std::time::Duration::from_millis(600)).await;

        let mut card = crate::ui::Card::new("RUNNER");
        card.with_badge(crate::ui::badge_online("STARTED (DETACHED)"));
        card.with_min_width(64);
        card.add_kv("Mode", "Background Daemon (Detached)");
        card.add_kv("PID", child_pid.to_string());
        card.add_kv("Endpoint", format!("http://{}:{}", config.server_host, config.server_port));
        card.add_kv("Log File", log_file_path.display().to_string());
        card.with_footer("Stop daemon with 'tuquet runner stop'");
        println!();
        card.print();
        println!();
        return Ok(());
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

    let pid = std::process::id();
    let pid_file_path = get_pid_file_path(&config.data_dir);
    let _ = std::fs::write(&pid_file_path, pid.to_string());

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

    let pid_file_clone = pid_file_path.clone();
    let shutdown_signal = async move {
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
        let _ = std::fs::remove_file(&pid_file_clone);
        info!("All browser processes cleaned up.");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
    {
        let _ = std::fs::remove_file(&pid_file_path);
        eprintln!("[AUTOMA-CORE SERVER ERROR] {:?}", e);
        return Err(e.into());
    }

    let _ = std::fs::remove_file(&pid_file_path);
    eprintln!("[AUTOMA-CORE EXITED MAIN OK]");
    Ok(())
}
