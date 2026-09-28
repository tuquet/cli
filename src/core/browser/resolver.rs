use anyhow::{anyhow, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Write;
use utoipa::ToSchema;

/// Pinned stable Long-Term-Support (LTS) release of official Google Chrome for Testing (CfT)
pub const PINNED_CHROME_VERSION: &str = "131.0.6778.85";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Dedicated browser runtime descriptor for Automa Core (Zero Host Scanning)
pub struct DetectedHostBrowser {
    /// Browser engine type (always chromium in phase 1)
    pub browser_type: String,
    /// Human-friendly display label
    pub name: String,
    /// Absolute filesystem path to browser executable
    pub executable_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub installed: bool,
    pub platform: String,
    pub executable_path: String,
    pub directory: String,
    pub pinned_version: String,
    pub size_mb: Option<f64>,
}

/// Target platform identifier matching Chrome for Testing release assets
pub fn get_platform_key() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "win64"
    }
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            "mac-arm64"
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            "mac-x64"
        }
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        "linux64"
    }
}

/// Relative path to executable within the unpacked platform archive
pub fn get_platform_exe_rel_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::path::PathBuf::from("chrome.exe")
    }
    #[cfg(target_os = "macos")]
    {
        std::path::PathBuf::from("Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::path::PathBuf::from("chrome")
    }
}

/// Dedicated runtime directory path: <data_dir>/runtimes/chrome-<platform>
pub fn get_runtime_dir() -> std::path::PathBuf {
    let config = crate::config::AppConfig::load();
    let platform = get_platform_key();
    std::path::PathBuf::from(&config.data_dir)
        .join("runtimes")
        .join(format!("chrome-{}", platform))
}

/// Absolute filesystem path to the isolated Chrome for Testing binary
pub fn get_runtime_exe_path() -> std::path::PathBuf {
    get_runtime_dir().join(get_platform_exe_rel_path())
}

/// Constructs the official Google Cloud storage download URL for Chrome for Testing
pub fn get_download_url(version: &str) -> String {
    let platform = get_platform_key();
    let zip_name = format!("chrome-{}.zip", platform);
    format!(
        "https://storage.googleapis.com/chrome-for-testing-public/{}/{}/{}",
        version, platform, zip_name
    )
}

/// Returns the dedicated standalone Chromium runtime descriptor (Zero Host Scanning)
pub fn detect_host_browsers() -> Vec<DetectedHostBrowser> {
    let exe_path = get_runtime_exe_path();
    vec![DetectedHostBrowser {
        browser_type: "chromium".to_string(),
        name: format!("Chrome for Testing ({})", PINNED_CHROME_VERSION),
        executable_path: exe_path.to_string_lossy().to_string(),
    }]
}

/// Inspects current installation status and disk consumption of dedicated runtime
pub fn get_runtime_status() -> RuntimeStatus {
    let exe_path = get_runtime_exe_path();
    let runtime_dir = get_runtime_dir();
    let installed = exe_path.exists();
    let size_mb = if installed {
        calculate_dir_size(&runtime_dir).map(|bytes| bytes as f64 / 1_048_576.0)
    } else {
        None
    };

    RuntimeStatus {
        installed,
        platform: get_platform_key().to_string(),
        executable_path: exe_path.to_string_lossy().to_string(),
        directory: runtime_dir.to_string_lossy().to_string(),
        pinned_version: PINNED_CHROME_VERSION.to_string(),
        size_mb,
    }
}

/// Recursively calculates directory size in bytes
fn calculate_dir_size(dir: &std::path::Path) -> Option<u64> {
    if !dir.exists() {
        return None;
    }
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            } else if path.is_dir() {
                if let Some(sub_total) = calculate_dir_size(&path) {
                    total += sub_total;
                }
            }
        }
    }
    Some(total)
}

/// Cleans and removes installed browser runtime to free up storage
pub fn clean_runtime() -> Result<()> {
    let runtime_dir = get_runtime_dir();
    if runtime_dir.exists() {
        std::fs::remove_dir_all(&runtime_dir)?;
        println!("Successfully removed dedicated runtime directory: {:?}", runtime_dir);
    } else {
        println!("No dedicated runtime found at {:?}", runtime_dir);
    }
    Ok(())
}

/// Downloads and installs official Google Chrome for Testing into <data_dir>/runtimes/
pub async fn download_chromium_runtime(force: bool, custom_version: Option<&str>) -> Result<String> {
    let exe_path = get_runtime_exe_path();
    if !force && exe_path.exists() {
        let abs_path = if !exe_path.is_absolute() {
            std::env::current_dir().map(|cwd| cwd.join(&exe_path)).unwrap_or_else(|_| exe_path.clone())
        } else {
            exe_path.clone()
        };
        return Ok(abs_path.to_string_lossy().to_string());
    }

    let version = custom_version.unwrap_or(PINNED_CHROME_VERSION);
    let download_url = get_download_url(version);
    let platform = get_platform_key();

    println!("============================================================");
    println!(" Automa Core - Dedicated Browser Provisioning");
    println!("============================================================");
    println!(" Engine:     Google Chrome for Testing (Zero Host Scanning)");
    println!(" Version:    {}", version);
    println!(" Platform:   {}", platform);
    println!(" URL:        {}", download_url);
    println!(" Target:     {}", exe_path.display());
    println!("------------------------------------------------------------");

    // Configure proxy-aware HTTP client
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600));

    if let Ok(proxy_url) = env::var("ALL_PROXY")
        .or_else(|_| env::var("HTTP_PROXY"))
        .or_else(|_| env::var("all_proxy"))
        .or_else(|_| env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    let client = builder.build()?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to initiate download from {}: {}", download_url, e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with HTTP status code: {}",
            response.status()
        ));
    }

    let total_bytes = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();

    let temp_zip_path = std::env::temp_dir().join(format!(
        "chrome_cft_{}_{}_{}.zip",
        platform,
        version,
        std::process::id()
    ));

    let mut file = tokio::fs::File::create(&temp_zip_path)
        .await
        .map_err(|e| anyhow!("Failed to create temporary archive {:?}: {}", temp_zip_path, e))?;

    let mut downloaded_bytes: u64 = 0;
    let mut last_reported = std::time::Instant::now();

    use tokio::io::AsyncWriteExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| anyhow!("Network error while streaming browser binary: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| anyhow!("Failed to write chunk to disk: {}", e))?;
        downloaded_bytes += chunk.len() as u64;

        if last_reported.elapsed().as_millis() >= 350 || (total_bytes > 0 && downloaded_bytes == total_bytes) {
            if total_bytes > 0 {
                let percent = (downloaded_bytes as f64 / total_bytes as f64) * 100.0;
                let mb_down = downloaded_bytes as f64 / 1_048_576.0;
                let mb_tot = total_bytes as f64 / 1_048_576.0;
                print!(
                    "\r[BrowserDownloader] {:>5.1}% ({:.1} MB / {:.1} MB)...",
                    percent, mb_down, mb_tot
                );
                let _ = std::io::stdout().flush();
            } else {
                let mb_down = downloaded_bytes as f64 / 1_048_576.0;
                print!("\r[BrowserDownloader] Downloaded {:.1} MB...", mb_down);
                let _ = std::io::stdout().flush();
            }
            last_reported = std::time::Instant::now();
        }
    }
    file.flush().await?;
    drop(file);

    println!(
        "\n[BrowserDownloader] Download completed ({:.1} MB). Extracting archive...",
        downloaded_bytes as f64 / 1_048_576.0
    );

    let config = crate::config::AppConfig::load();
    let runtimes_dir = std::path::PathBuf::from(&config.data_dir).join("runtimes");
    if !runtimes_dir.exists() {
        std::fs::create_dir_all(&runtimes_dir)?;
    }

    let extract_dir = runtimes_dir.clone();
    let zip_clone = temp_zip_path.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        use std::fs::File;
        let archive_file = File::open(&zip_clone)?;
        let mut archive = zip::ZipArchive::new(archive_file)?;
        archive.extract(&extract_dir)?;
        Ok(())
    })
    .await??;

    let _ = tokio::fs::remove_file(&temp_zip_path).await;

    let target_exe = get_runtime_exe_path();
    if !target_exe.exists() {
        return Err(anyhow!(
            "Extraction completed but executable not found at {:?}",
            target_exe
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&target_exe) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&target_exe, perms);
        }
    }

    println!(
        "[BrowserDownloader] Dedicated browser successfully provisioned at: {:?}",
        target_exe
    );
    println!("============================================================");

    let abs_path = if !target_exe.is_absolute() {
        std::env::current_dir()
            .map(|cwd| cwd.join(&target_exe))
            .unwrap_or_else(|_| target_exe)
    } else {
        target_exe
    };
    Ok(abs_path.to_string_lossy().to_string())
}

/// Resolves the executable path of the automation browser.
///
/// Priority (Zero Host Scanning Standard):
/// 1. AUTOMA_BROWSER_PATH / CHROME_EXECUTABLE_PATH environment variable override.
/// 2. Dedicated standalone Chrome for Testing in <data_dir>/runtimes/chrome-<platform>/
/// 3. Auto-provision dedicated runtime on first execution.
pub async fn resolve_executable_path(_default_browser: &str) -> Result<String> {
    if let Ok(path) = env::var("AUTOMA_BROWSER_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    if let Ok(path) = env::var("CHROME_EXECUTABLE_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let cached_exe = get_runtime_exe_path();
    if cached_exe.exists() {
        let abs_path = if !cached_exe.is_absolute() {
            std::env::current_dir()
                .map(|cwd| cwd.join(&cached_exe))
                .unwrap_or_else(|_| cached_exe.clone())
        } else {
            cached_exe.clone()
        };
        return Ok(abs_path.to_string_lossy().to_string());
    }

    // Auto-provision dedicated runtime on first run
    println!("Dedicated automation browser not found. Auto-provisioning Chrome for Testing...");
    download_chromium_runtime(false, None).await
}
