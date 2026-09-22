use anyhow::{anyhow, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};

#[derive(Clone)]
pub struct BrowserLauncherOptions {
    pub executable_path: String,
    pub user_data_dir: String,
    pub debugging_port: u16,
    pub extension_paths: Vec<String>,
    pub custom_args: Vec<String>,
}

pub struct BrowserLauncher {
    options: BrowserLauncherOptions,
    process: Option<Child>,
    ws_url: Option<String>,
}

impl BrowserLauncher {
    pub fn new(options: BrowserLauncherOptions) -> Self {
        Self {
            options,
            process: None,
            ws_url: None,
        }
    }

    /// Khởi chạy trình duyệt dưới dạng tiến trình độc lập
    pub async fn launch(&mut self) -> Result<String> {
        let mut args = vec![
            format!("--remote-debugging-port={}", self.options.debugging_port),
            format!("--user-data-dir={}", self.options.user_data_dir),
            "--no-first-run".to_string(),
            "--password-store=basic".to_string(),
            "--restore-last-session".to_string(),
            "--disable-gpu".to_string(),
            "--disable-software-rasterizer".to_string(),
            "--window-size=1280,720".to_string(),
            "--no-sandbox".to_string(),
            "--disable-setuid-sandbox".to_string(),
            "--log-level=3".to_string(),
            "--test-type".to_string(),
        ];

        if !self.options.extension_paths.is_empty() {
            let exts = self.options.extension_paths.join(",");
            args.push(format!("--load-extension={}", exts));
            args.push(format!("--disable-extensions-except={}", exts));
        }

        args.extend(self.options.custom_args.clone());
        args.push("about:blank".to_string());

        println!("[BrowserLauncher] Launching: {} {}", self.options.executable_path, args.join(" "));

        #[allow(unused_mut)]
        let mut cmd = Command::new(&self.options.executable_path);
        cmd.args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(target_os = "windows")]
        {
            // CREATE_NO_WINDOW (0x08000000)
            cmd.creation_flags(0x08000000);
        }

        let child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn browser process: {}", e))?;

        println!("[BrowserLauncher] Trình duyệt đang chạy tại PID: {:?}", child.id());
        
        self.process = Some(child);

        let ws_url = self.wait_for_ws_url().await?;
        self.ws_url = Some(ws_url.clone());
        Ok(ws_url)
    }

    /// Lấy WebSocket Debugger URL bằng cách polling
    async fn wait_for_ws_url(&mut self) -> Result<String> {
        let url = format!("http://127.0.0.1:{}/json/version", self.options.debugging_port);
        let client = reqwest::Client::new();

        for _ in 0..60 {
            if let Some(child) = self.process.as_mut() {
                if let Ok(Some(_status)) = child.try_wait() {
                    return Err(anyhow!("Chrome process exited unexpectedly before opening debugger port. Browser might be locked."));
                }
            }
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(ws) = json.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                        return Ok(ws.to_string());
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(anyhow!("Timeout: Không thể kết nối tới Chrome CDP Endpoint."))
    }

    /// Đóng trình duyệt (Kill tiến trình chính)
    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.process.as_ref().and_then(|p| p.id())
    }
}
