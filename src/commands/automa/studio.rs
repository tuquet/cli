use crate::config::AppConfig;

pub fn open_studio() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let port = config.server_port;
    let url = std::env::var("AUTOMA_STUDIO_URL").unwrap_or_else(|_| {
        format!("{}?port={}", crate::constants::DEFAULT_AUTOMA_STUDIO_URL, port)
    });
    println!("Opening Automa Web Studio at: {}", url);
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    Ok(())
}
