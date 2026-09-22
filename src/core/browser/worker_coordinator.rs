use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex as TokioMutex, RwLock};

pub fn connected_browsers() -> &'static Arc<RwLock<HashSet<String>>> {
    static BROWSERS: OnceLock<Arc<RwLock<HashSet<String>>>> = OnceLock::new();
    BROWSERS.get_or_init(|| Arc::new(RwLock::new(HashSet::new())))
}

fn launcher_mutexes() -> &'static RwLock<HashMap<String, Arc<TokioMutex<()>>>> {
    static MUTEXES: OnceLock<RwLock<HashMap<String, Arc<TokioMutex<()>>>>> = OnceLock::new();
    MUTEXES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_browser_launcher_lock(browser_id: &str) -> Arc<TokioMutex<()>> {
    let mut map = launcher_mutexes().write().await;
    map.entry(browser_id.to_string())
        .or_insert_with(|| Arc::new(TokioMutex::new(())))
        .clone()
}

pub fn resolve_cli_runner_extension_path() -> String {
    let mut possible_paths = vec![
        std::path::PathBuf::from("automa-webe/dist/cli-runner"),
        std::path::PathBuf::from("./automa-webe/dist/cli-runner"),
        std::path::PathBuf::from("../automa-webe/dist/cli-runner"),
        std::path::PathBuf::from("../../automa-webe/dist/cli-runner"),
        std::path::PathBuf::from("../../../automa-webe/dist/cli-runner"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("automa-webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../automa-webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../../automa-webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../../../automa-webe/dist/cli-runner"));
        }
    }

    if let Ok(env_ext) = std::env::var("AUTOMA_EXTENSION_PATH") {
        possible_paths.insert(0, std::path::PathBuf::from(env_ext));
    }

    let mut ext_path = possible_paths.into_iter()
        .find(|p| p.exists())
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "automa-webe/dist/cli-runner".to_string());

    if ext_path.starts_with(r"\\?\") {
        ext_path = ext_path[4..].to_string();
    }
    ext_path
}

pub async fn ensure_browser_worker(
    browser_id: &str,
    app_settings: &crate::core::models::settings::AppSettings,
    headless_opt: Option<bool>,
    default_browser_opt: Option<String>,
) {
    let _lock = get_browser_launcher_lock(browser_id).await;
    let _guard = _lock.lock().await;

    let is_connected = {
        let browsers = connected_browsers().read().await;
        browsers.contains(&browser_id.to_string())
    };

    if is_connected {
        return;
    }

    // Clean up any stale/dead PID in registry for this browser_id
    let old_pid = {
        let mut reg = crate::core::browser::manager::browser_registry().write().await;
        reg.remove(&browser_id.to_string())
    };
    if let Some(_pid) = old_pid {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = tokio::process::Command::new("taskkill");
            cmd.args(["/F", "/PID", &_pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .creation_flags(0x08000000);
            let _ = cmd.status().await;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = tokio::process::Command::new("kill")
                .args(["-9", &_pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }
    }

    use crate::core::browser::manager::{BrowserManager, BrowserManagerOptions};

    let ext_path = resolve_cli_runner_extension_path();
    tracing::info!("[submit_job] Resolved extension path: {}", ext_path);

    let headless = headless_opt.unwrap_or(app_settings.browser.headless);

    let default_browser = default_browser_opt.unwrap_or_else(|| {
        if app_settings.browser.default_type.is_empty() {
            "chromium".to_string()
        } else {
            app_settings.browser.default_type.clone()
        }
    });

    let mut custom_args = Vec::new();
    if app_settings.grid.enabled {
        let active_browsers_count = connected_browsers().read().await.len() as u32;
        let (pos_x, pos_y, slot_w, slot_h) = app_settings.grid.calculate_slot_bounds(active_browsers_count);
        custom_args.push(format!("--window-position={},{}", pos_x, pos_y));
        custom_args.push(format!("--window-size={},{}", slot_w, slot_h));
    }

    if let Some(ref ua) = app_settings.browser.default_user_agent {
        if !ua.is_empty() {
            custom_args.push(format!("--user-agent={}", ua));
        }
    }

    let mut manager = BrowserManager::new(BrowserManagerOptions {
        default_browser,
        browser_id: browser_id.to_string(),
        headless,
        extension_paths: vec![ext_path],
        custom_args,
        user_data_dir: None,
    });

    if let Err(e) = manager.launch().await {
        eprintln!("Failed to launch browser worker (maybe already running): {}", e);
    }

    for _ in 0..10 {
        let is_connected_now = {
            let browsers = connected_browsers().read().await;
            browsers.contains(&browser_id.to_string())
        };
        if is_connected_now {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}
