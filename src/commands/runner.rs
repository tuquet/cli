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
    Path::new(data_dir).join(crate::constants::FILE_RUNNER_PID)
}

pub fn get_log_file_path(data_dir: &str) -> PathBuf {
    Path::new(data_dir).join(crate::constants::DIR_LOGS).join(crate::constants::FILE_RUNNER_LOG)
}

pub async fn handle(command: RunnerSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RunnerSubcommands::Start {
            host,
            port,
            detach,
            data_dir,
            log_level,
            cloud,
        } => run_server(host, port, detach, data_dir, log_level, cloud).await,
        RunnerSubcommands::Worker {
            cloud_profile,
            workflow,
            headless,
            interval,
            once,
        } => run_cloud_worker(cloud_profile, workflow, headless, interval, once).await,
        RunnerSubcommands::Stop { force } => stop_daemon(force).await,
        RunnerSubcommands::Restart { detach } => restart_daemon(detach).await,
        RunnerSubcommands::Status { url, json } => {
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var(crate::constants::ENV_AUTOMA_HOST).unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
                let port = std::env::var(crate::constants::ENV_AUTOMA_PORT).unwrap_or_else(|_| crate::constants::DEFAULT_RUNNER_PORT.to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url, json).await
        }
        RunnerSubcommands::Logs { follow, lines } => show_logs(follow, lines).await,
        RunnerSubcommands::Probe => print_probe_manifest(),
        RunnerSubcommands::ExportOpenapi { output } => export_openapi(&output),
        RunnerSubcommands::Config { edit, show } => manage_config(edit, show),
    }
}

pub fn print_probe_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = serde_json::json!({
        "protocol": crate::constants::PROTOCOL_AUTOMA_V1,
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

pub async fn check_status(url: &str, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let target = format!("{}/api/v1/health", url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()?;

    match client.get(&target).send().await {
        Ok(res) if res.status().is_success() => {
            if json_output {
                let out = serde_json::json!({
                    "status": "online",
                    "code": 200,
                    "endpoint": url,
                    "health_route": target,
                    "driver": "mv3_extension_worker"
                });
                println!("{}", serde_json::to_string(&out)?);
                return Ok(());
            }
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
            if json_output {
                let out = serde_json::json!({
                    "status": "error",
                    "code": res.status().as_u16(),
                    "endpoint": url,
                    "health_route": target
                });
                println!("{}", serde_json::to_string(&out)?);
                return Ok(());
            }
            let mut card = crate::ui::Card::new("RUNNER");
            card.with_badge(crate::ui::badge_error(&format!("HTTP {}", res.status())));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            println!();
            card.print();
            println!();
        }
        Err(e) => {
            if json_output {
                let out = serde_json::json!({
                    "status": "offline",
                    "endpoint": url,
                    "error": format!("{}", e)
                });
                println!("{}", serde_json::to_string(&out)?);
                return Ok(());
            }
            let mut card = crate::ui::Card::new("RUNNER");
            card.with_badge(crate::ui::badge_offline("OFFLINE"));
            card.with_min_width(64);
            card.add_kv("Endpoint", url);
            card.add_kv("Diagnostic", format!("{}", e));
            card.with_footer("Start local daemon with 'specter runner start -d'");
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
    
    let pid_to_kill: Option<u32> = std::fs::read_to_string(&pid_file)
        .ok()
        .and_then(|c| c.trim().parse::<u32>().ok());

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
        if name.contains("specter") {
            let cmd = proc.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");
            if cmd.contains("runner") && cmd.contains("start") && !found_pids.contains(&p_u32) {
                found_pids.push(p_u32);
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
            card.with_footer("Start daemon with 'specter runner start -d'");
            println!();
            card.print();
            println!();
            let _ = std::fs::remove_file(&pid_file);
            return Ok(());
        }
    }

    let mut terminated_pids = Vec::new();
    for &pid in &found_pids {
        let sys_pid = sysinfo::Pid::from(pid as usize);
        let Some(proc) = sys.process(sys_pid) else {
            tracing::warn!("Stale PID {} detected (process not found in system table). Skipping kill.", pid);
            eprintln!("Warning: Stale PID {} detected (process not found). Skipping kill.", pid);
            continue;
        };

        let raw_name = proc.name().to_string_lossy();
        if !is_valid_runner_name(&raw_name) {
            tracing::warn!(
                "Stale PID {} detected: process name '{}' does not match specter or runner. Skipping kill.",
                pid,
                raw_name
            );
            eprintln!(
                "Warning: Stale PID {} detected: process name '{}' does not match specter or runner. Skipping kill.",
                pid,
                raw_name
            );
            continue;
        }

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

        terminated_pids.push(pid);
    }

    let _ = std::fs::remove_file(&pid_file);

    let mut card = crate::ui::Card::new("RUNNER");
    card.with_badge(crate::ui::badge_online("STOPPED"));
    card.with_min_width(64);
    if !terminated_pids.is_empty() {
        let pids_str = terminated_pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
        card.add_kv("Terminated PID(s)", pids_str);
    }
    card.add_line("Runner worker daemon has been gracefully terminated.");
    card.with_footer("Start daemon again with 'specter runner start -d'");
    println!();
    card.print();
    println!();

    Ok(())
}

pub async fn restart_daemon(detach: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::Notify::info(crate::constants::MSG_SERVER_STOPPING);
    let _ = stop_daemon(false).await;
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    crate::ui::Notify::info(crate::constants::MSG_SERVER_STARTING);
    run_server(None, None, detach, None, None, false).await
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
        card.with_footer("Start daemon in background with 'specter runner start -d'");
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
    cloud: bool,
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
        if cloud {
            args.push("-c".to_string());
        }
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
        let mode_desc = if cloud {
            "Background Cloud Fleet Mesh Worker (Detached)"
        } else {
            "Background Daemon (Detached)"
        };
        card.add_kv("Mode", mode_desc);
        card.add_kv("PID", child_pid.to_string());
        card.add_kv("Endpoint", format!("http://{}:{}", config.server_host, config.server_port));
        card.add_kv("Log File", log_file_path.display().to_string());
        card.with_footer("Stop daemon with 'specter runner stop'");
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

    info!("Specter Automa Core Bridge starting in Native Launcher mode...");
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

    // Start background Cloud Telemetry & Inventory Reporter (runs if SPECTER_CLOUD_URL is configured)
    let _reporter_handle =
        crate::infrastructure::cloud_reporter::CloudReporter::start_background_loop(state.clone());

    if cloud {
        let creds = crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&config.data_dir).await;
        let mut card = crate::ui::Card::new("RUNNER CLOUD FLEET MESH");
        card.with_badge(crate::ui::badge_online("FLEET CONNECTED"));
        card.with_min_width(74);
        if let Some(ref c) = creds {
            card.add_kv("Device ID", &c.device_id);
            card.add_kv("Device Name", &c.name);
            if let Some(ref t) = c.tenant_id {
                card.add_kv("Tenant ID", t);
            }
        }
        card.add_kv("Cloud Hub", config.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co"));
        card.add_kv("Proxy Mesh", "Smart Bridge Auto-Detect (127.0.0.1:1080 / 8118)");
        card.add_kv("Local Endpoint", format!("http://{}:{}", config.server_host, config.server_port));
        card.with_footer("Runner daemon active in Cloud Fleet Mesh mode. Ready for distributed workloads.");
        println!();
        card.print();
        println!();
    }

    let addr_str = format!("{}:{}", config.server_host, config.server_port);
    use std::net::ToSocketAddrs;
    let socket_addr = addr_str
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("Could not resolve bind address: {}", addr_str))?;

    let domain = if socket_addr.is_ipv6() {
        socket2::Domain::IPV6
    } else {
        socket2::Domain::IPV4
    };

    let socket = socket2::Socket::new(domain, socket2::Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&socket_addr.into())?;
    socket.listen(1024)?;

    let std_listener: std::net::TcpListener = socket.into();
    let listener = tokio::net::TcpListener::from_std(std_listener)?;
    info!("Server listening on http://{}", listener.local_addr()?);
    let panic_log_path = PathBuf::from(&config.data_dir).join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[SPECTER-PANIC] {:?}\n", info);
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
        crate::ui::Notify::shutdown(crate::constants::MSG_SHUTDOWN_SIGNAL);
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
        crate::ui::Notify::error(format!("Server execution error: {:?}", e));
        return Err(e.into());
    }

    let _ = std::fs::remove_file(&pid_file_path);
    crate::ui::Notify::shutdown_clean(crate::constants::MSG_SHUTDOWN_CLEAN);
    Ok(())
}

pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::config::RunnerConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = crate::config::RunnerConfig::load();
        println!();
        let mut card = crate::ui::Card::new("RUNNER CONFIGURATION");
        card.with_badge(crate::ui::badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        card.add_kv("Listen Host", &config.host);
        card.add_kv("Listen Port", config.port.to_string());
        card.add_kv("Log Level", &config.log_level);
        card.add_kv("Heartbeat", format!("{}s", config.heartbeat_interval_secs));
        card.add_kv("Max Concurrency", config.max_concurrent_jobs.to_string());
        card.add_kv("Auto Restart", if config.auto_restart { "enabled" } else { "disabled" });
        card.with_footer("Tip: edit with 'specter runner config --edit'");
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}

pub async fn run_cloud_worker(
    cloud_profile: Option<String>,
    workflow: Option<String>,
    headless: bool,
    interval_secs: u64,
    once: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let creds = match crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&config.data_dir).await {
        Some(c) => c,
        None => {
            crate::ui::Notify::error(crate::constants::MSG_NOT_ENROLLED);
            eprintln!("Run 'specter runner enroll' to register this device.\n");
            return Err("Missing cloud device credentials".into());
        }
    };

    let client = crate::infrastructure::cloud_reporter::CloudReporter::build_http_client();
    let cloud_url = config.cloud_url.as_deref()
        .or(creds.cloud_url.as_deref())
        .unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co");
    let anon_key = creds.api_key.as_deref().unwrap_or(crate::infrastructure::cloud_reporter::DEFAULT_SUPABASE_ANON_KEY);

    let mut card = crate::ui::Card::new("AUTONOMOUS CLOUD FLEET WORKER");
    card.with_badge(crate::ui::badge_online("ONLINE"));
    card.with_min_width(74);
    card.add_kv("Device ID", &creds.device_id);
    card.add_kv("Device Name", &creds.name);
    card.add_kv("Cloud Endpoint", cloud_url);
    if let Some(ref p) = cloud_profile {
        card.add_kv("Target Profile", p);
    } else {
        card.add_kv("Fleet Scope", "Tenant Profile Pool");
    }
    card.add_kv("Poll Interval", format!("{} seconds", interval_secs));
    card.with_footer("Autonomous cloud mesh worker active. Press Ctrl+C to terminate cleanly.");
    println!();
    card.print();
    println!();

    if let Some(target) = cloud_profile {
        println!(">> Executing autonomous pipeline on cloud profile '{}'...", target);
        crate::infrastructure::cloud_fleet::CloudFleetOrchestrator::run_autonomous_cloud_session(
            &target,
            workflow,
            None,
            headless,
            Vec::new(),
            None,
        ).await?;
        return Ok(());
    }

    loop {
        let list_url = format!("{}/rest/v1/rpc/list_browsers", cloud_url.trim_end_matches('/'));
        let res = client.post(&list_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token
            }))
            .send()
            .await;

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let time_str = format!("{:02}:{:02}:{:02}", (now_epoch / 3600) % 24, (now_epoch / 60) % 60, now_epoch % 60);

        match res {
            Ok(r) if r.status().is_success() => {
                let browsers: Vec<serde_json::Value> = r.json().await.unwrap_or_default();
                let idle_count = browsers.iter().filter(|b| b["status"].as_str() == Some("idle")).count();
                let running_count = browsers.iter().filter(|b| b["status"].as_str() == Some("running")).count();
                println!(
                    "[{}] Fleet mesh online: {} total profiles ({} IDLE, {} RUNNING) | Node: {}",
                    time_str,
                    browsers.len(),
                    idle_count,
                    running_count,
                    creds.name
                );
            }
            Ok(r) => {
                eprintln!("[{}] Failed to query fleet: HTTP {}", time_str, r.status());
            }
            Err(e) => {
                eprintln!("[{}] Network error polling fleet: {}", time_str, e);
            }
        }

        if once {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)) => {},
            _ = tokio::signal::ctrl_c() => {
                println!();
                crate::ui::Notify::shutdown(crate::constants::MSG_SHUTDOWN_SIGNAL);
                break;
            }
        }
    }

    Ok(())
}

pub(crate) fn is_valid_runner_name(raw_name: &str) -> bool {
    let name = raw_name.to_lowercase();
    let base_name = name.trim_end_matches(".exe");
    base_name == "specter" || base_name == "runner"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_runner_name() {
        assert!(is_valid_runner_name("specter"));
        assert!(is_valid_runner_name("specter.exe"));
        assert!(is_valid_runner_name("SPECTER.EXE"));
        assert!(is_valid_runner_name("runner"));
        assert!(is_valid_runner_name("runner.exe"));

        // Innocent applications must be rejected
        assert!(!is_valid_runner_name("chrome.exe"));
        assert!(!is_valid_runner_name("notepad.exe"));
        assert!(!is_valid_runner_name("svchost.exe"));
        assert!(!is_valid_runner_name("cmd.exe"));
        assert!(!is_valid_runner_name("powershell.exe"));
    }

    #[tokio::test]
    async fn test_socket2_reuseaddr_and_listener() {
        let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)
            .expect("Failed to create socket2 socket");
        socket
            .set_reuse_address(true)
            .expect("Failed to set SO_REUSEADDR");
        socket
            .set_nonblocking(true)
            .expect("Failed to set nonblocking");
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("Failed to parse addr");
        socket.bind(&addr.into()).expect("Failed to bind socket");
        socket.listen(128).expect("Failed to listen on socket");

        let std_listener: std::net::TcpListener = socket.into();
        let listener = tokio::net::TcpListener::from_std(std_listener)
            .expect("Failed to convert std TcpListener to tokio TcpListener");
        assert!(listener.local_addr().is_ok());
    }
}


