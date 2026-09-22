use anyhow::{anyhow, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Host system browser discovery record
pub struct DetectedHostBrowser {
    /// Browser engine type (always chromium in phase 1)
    pub browser_type: String,
    /// Human-friendly display label
    pub name: String,
    /// Absolute filesystem path to browser executable
    pub executable_path: String,
}

fn get_runtime_exe_path() -> std::path::PathBuf {
    let config = crate::config::AppConfig::load();
    let cache_dir = std::path::PathBuf::from(&config.data_dir).join("runtimes");
    let exe_name = if cfg!(target_os = "windows") { "chrome.exe" } else { "chrome" };
    let platform_key = if cfg!(target_os = "windows") {
        "win64"
    } else if cfg!(target_os = "macos") {
        "mac-arm64"
    } else {
        "linux64"
    };
    let target_dir_name = format!("chrome-{}", platform_key);
    cache_dir.join(&target_dir_name).join(exe_name)
}

/// Returns the dedicated standalone Chromium runtime descriptor (Zero Host Scanning)
pub fn detect_host_browsers() -> Vec<DetectedHostBrowser> {
    let exe_path = get_runtime_exe_path();
    vec![DetectedHostBrowser {
        browser_type: "chromium".to_string(),
        name: "Default Chromium".to_string(),
        executable_path: exe_path.to_string_lossy().to_string(),
    }]
}

async fn download_chromium_runtime() -> Result<String> {
    let config = crate::config::AppConfig::load();
    let cache_dir = std::path::PathBuf::from(&config.data_dir).join("runtimes");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    let exe_name = if cfg!(target_os = "windows") { "chrome.exe" } else { "chrome" };
    let platform_key = if cfg!(target_os = "windows") {
        "win64"
    } else if cfg!(target_os = "macos") {
        "mac-arm64"
    } else {
        "linux64"
    };

    let target_dir_name = format!("chrome-{}", platform_key);
    let cached_exe = cache_dir.join(&target_dir_name).join(exe_name);

    if cached_exe.exists() {
        let abs_path = if !cached_exe.is_absolute() {
            std::env::current_dir().map(|cwd| cwd.join(&cached_exe)).unwrap_or_else(|_| cached_exe.clone())
        } else {
            cached_exe.clone()
        };
        return Ok(abs_path.to_string_lossy().to_string());
    }

    println!("Chromium not found locally. Downloading latest Chromium from Google Storage...");
    
    let latest_url = if platform_key == "win64" {
        "https://storage.googleapis.com/chromium-browser-snapshots/Win_x64/LAST_CHANGE"
    } else if platform_key == "mac-arm64" {
        "https://storage.googleapis.com/chromium-browser-snapshots/Mac_Arm/LAST_CHANGE"
    } else {
        "https://storage.googleapis.com/chromium-browser-snapshots/Linux_x64/LAST_CHANGE"
    };
    
    let revision = reqwest::get(latest_url).await?.text().await?.trim().to_string();
    
    let download_url = if platform_key == "win64" {
        format!("https://storage.googleapis.com/chromium-browser-snapshots/Win_x64/{}/chrome-win.zip", revision)
    } else if platform_key == "mac-arm64" {
        format!("https://storage.googleapis.com/chromium-browser-snapshots/Mac_Arm/{}/chrome-mac.zip", revision)
    } else {
        format!("https://storage.googleapis.com/chromium-browser-snapshots/Linux_x64/{}/chrome-linux.zip", revision)
    };

    println!("Downloading from: {}", download_url);
    let zip_bytes = reqwest::get(&download_url).await?.bytes().await?;

    println!("Extracting browser to {:?}...", cache_dir);
    let extract_dir = cache_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        use std::io::Cursor;
        let reader = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader)?;
        archive.extract(&extract_dir)?;
        Ok(())
    })
    .await??;
    
    let extracted_dir_name = if platform_key == "win64" {
        "chrome-win"
    } else if platform_key == "mac-arm64" {
        "chrome-mac"
    } else {
        "chrome-linux"
    };
    
    let new_exe_path = cache_dir.join(extracted_dir_name).join(exe_name);
    if new_exe_path.exists() {
        if let Err(e) = std::fs::rename(cache_dir.join(extracted_dir_name), cache_dir.join(&target_dir_name)) {
            eprintln!("Warning: Failed to rename Chromium directory: {}", e);
        }
    }

    if cached_exe.exists() {
        println!("Browser successfully installed!");
        let abs_path = if !cached_exe.is_absolute() {
            std::env::current_dir().map(|cwd| cwd.join(&cached_exe)).unwrap_or_else(|_| cached_exe.clone())
        } else {
            cached_exe.clone()
        };
        Ok(abs_path.to_string_lossy().to_string())
    } else {
        Err(anyhow!("Failed to locate downloaded executable at {:?}", cached_exe))
    }
}

/// Resolves the executable path of the desired browser.
/// Always falls back to the isolated downloaded Chromium runtime.
pub async fn resolve_executable_path(_default_browser: &str) -> Result<String> {
    if let Ok(path) = env::var("AUTOMA_BROWSER_PATH") {
        if !path.is_empty() {
            return Ok(path);
        }
    }
    if let Ok(path) = env::var("CHROME_EXECUTABLE_PATH") {
        if !path.is_empty() {
            return Ok(path);
        }
    }

    let cached_exe = get_runtime_exe_path();
    if cached_exe.exists() {
        let abs_path = if !cached_exe.is_absolute() {
            std::env::current_dir().map(|cwd| cwd.join(&cached_exe)).unwrap_or_else(|_| cached_exe.clone())
        } else {
            cached_exe.clone()
        };
        return Ok(abs_path.to_string_lossy().to_string());
    }

    download_chromium_runtime().await
}
