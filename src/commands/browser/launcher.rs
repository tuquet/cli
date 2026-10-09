use crate::ui::{badge_error, badge_online, Card};
use crate::core::browser::ExtensionRegistry;

pub async fn launch_browser(
    profile_arg: String,
    port: u16,
    headless: bool,
    url_opt: Option<String>,
    detach: bool,
    proxy_override: Option<String>,
    mode: String,
    no_cdp: bool,
    skip_proxy_check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = crate::core::browser::resolve_data_dir();
    let profiles_dir = base_dir.join(crate::constants::DIR_PROFILES);
    let _ = std::fs::create_dir_all(&profiles_dir);

    let is_extension_mode = no_cdp || mode.eq_ignore_ascii_case("extension");
    let effective_port = if is_extension_mode { 0 } else { port };

    // 1. Resolve or auto-generate profile
    let mut profile = if profile_arg == "default" {
        match crate::core::browser::BrowserProfile::load("default", &base_dir) {
            Ok(p) => p,
            Err(_) => {
                let default_p = crate::core::browser::BrowserProfile::generate(
                    "default",
                    Some(133742),
                    Some("windows".to_string()),
                    Some(8),
                    Some(16),
                    None,
                    Some("Asia/Ho_Chi_Minh".to_string()),
                    Some("vi-VN".to_string()),
                );
                let _ = default_p.save(&base_dir);
                default_p
            }
        }
    } else {
        match crate::core::browser::BrowserProfile::load(&profile_arg, &base_dir) {
            Ok(p) => p,
            Err(e) => {
                crate::ui::Notify::error(&e);
                eprintln!("Run 'specter browser profile list' to view available profiles, or 'profile create <name>'.\n");
                return Err(e.into());
            }
        }
    };

    if let Some(pxy) = proxy_override {
        profile.proxy = Some(pxy);
    }

    // 2. Pre-flight Proxy Healthcheck (Fail-Safe Gate)
    let proxy_display = if let Some(ref pxy) = profile.proxy {
        if !skip_proxy_check {
            let probe = crate::core::browser::ProxyProbe::probe(pxy, 4).await;
            if !probe.alive {
                println!();
                let mut card = Card::new("PRE-FLIGHT CHECK FAILED: PROXY UNREACHABLE");
                card.with_badge(badge_error("FAIL-SAFE ABORT"));
                card.with_min_width(74);
                card.add_kv("Profile", format!("{} [{}]", profile.name, profile.id));
                card.add_kv("Target Proxy", pxy);
                if let Some(ref err) = probe.error {
                    card.add_kv("Probe Error", err);
                }
                card.add_line("");
                card.add_line("Safety Guardrail Triggered:");
                card.add_line("  • The configured proxy server is offline or unreachable.");
                card.add_line("  • Launch aborted to prevent session failure or real IP leakage.");
                card.add_line("  • To bypass this guardrail, launch with '--skip-proxy-check'.");
                card.with_footer("Test proxy directly with: specter proxy probe <url>");
                card.print();
                println!();
                return Err("Proxy unreachable (pre-flight check failed)".into());
            }

            let egress = probe.egress_ip.as_deref().unwrap_or("Verified");
            let loc = probe.country.as_deref().unwrap_or("ISO");
            format!("{} (● ONLINE - {}ms, IP: {} [{}])", pxy, probe.rtt_ms, egress, loc)
        } else {
            format!("{} (○ PRE-FLIGHT SKIPPED)", pxy)
        }
    } else {
        "Direct Connection (No Proxy)".to_string()
    };

    // 3. Resolve executable
    let exe_path = crate::core::browser::resolve_executable_path("default").await?;
    if !std::path::Path::new(&exe_path).exists() {
        crate::ui::Notify::error(format!("Dedicated Antidetect Chromium runtime not found at: {}", exe_path));
        eprintln!("Run 'specter browser install' to download and set up Golden LTS v148.\n");
        return Err("Missing browser binary".into());
    }

    // 4. Check port availability if running in Driver mode
    if !is_extension_mode && tokio::net::TcpListener::bind(format!("127.0.0.1:{}", effective_port)).await.is_err() {
        crate::ui::Notify::error(format!("Port {} is already occupied by another process.", effective_port));
        eprintln!("Specify a different port using '--port <PORT>', or run with '--mode extension' / '--no-cdp' for zero-port stealth.\n");
        return Err(format!("Port {} occupied", effective_port).into());
    }

    let mut custom_args = profile.build_cli_args(&base_dir);
    if headless && !custom_args.iter().any(|a| a.starts_with("--headless")) {
        custom_args.push("--headless=new".to_string());
    }
    if let Some(url) = url_opt {
        custom_args.push(url);
    } else {
        custom_args.push("https://bot.sannysoft.com".to_string());
    }

    let user_data_dir = profile.get_sandbox_dir(&base_dir).display().to_string();

    // 4. Auto-load registered extensions
    let ext_registry = ExtensionRegistry::load();
    let extension_paths = ext_registry
        .get_enabled_paths()
        .into_iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();

    let mut launcher = crate::core::browser::BrowserLauncher::new(crate::core::browser::BrowserLauncherOptions {
        executable_path: exe_path.clone(),
        user_data_dir: user_data_dir.clone(),
        debugging_port: effective_port,
        extension_paths: extension_paths.clone(),
        custom_args: custom_args.clone(),
    });

    if detach {
        let args = launcher.build_args();

        #[allow(unused_mut)]
        let mut cmd = std::process::Command::new(&exe_path);
        cmd.args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            if headless {
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                const DETACHED_PROCESS: u32 = 0x00000008;
                cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
            } else {
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
            }
        }

        let child = cmd.spawn()?;
        let pid = child.id();

        let mut ws_url = String::new();
        if effective_port > 0 {
            let client = reqwest::Client::new();
            let url = format!("http://127.0.0.1:{}/json/version", effective_port);
            for _ in 0..40 {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if let Ok(resp) = client.get(&url).send().await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(ws) = json.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                            ws_url = ws.to_string();
                            break;
                        }
                    }
                }
            }
        }

        let title = if is_extension_mode {
            "ANTIDETECT BROWSER (EXTENSION MODE - DETACHED)"
        } else {
            "ANTIDETECT BROWSER CDP BRIDGE (DETACHED)"
        };
        let badge = if is_extension_mode {
            badge_online("ZERO-PORT STEALTH")
        } else {
            badge_online("DETACHED")
        };

        let mut card = Card::new(title);
        card.with_badge(badge);
        card.with_min_width(74);
        card.add_kv("Engine", format!("adryfish/fingerprint-chromium (v{})", crate::core::browser::resolver::get_active_version()));
        card.add_kv("Profile", format!("{} [{}]", profile.name, profile.id));
        if let Some(seed) = profile.fingerprint_seed {
            card.add_kv("Seed (PRNG)", format!("{} (Deterministic Hardware)", seed));
        }
        if profile.proxy.is_some() {
            card.add_kv("Proxy", &proxy_display);
        }
        if is_extension_mode {
            card.add_kv("CDP Port", "Disabled (Zero-Port Protection against localhost port scanning)");
            card.add_kv("Extensions", format!("{} active extension(s)", extension_paths.len()));
        } else {
            card.add_kv("CDP Endpoint", format!("http://127.0.0.1:{}", effective_port));
            if !ws_url.is_empty() {
                card.add_kv("WebSocket URL", &ws_url);
            }
        }
        card.add_kv("PID", pid.to_string());
        card.with_footer("Browser running in background. Connect your agent or automation script.");
        println!();
        card.print();
        println!();
        return Ok(());
    }

    // Foreground Interactive Mode (Win32 Job Object Clean Supervision)
    println!();
    let ws_url = launcher.launch().await?;
    let pid_str = launcher.get_pid().map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());
    let active_ver = crate::core::browser::resolver::get_active_version();

    let title = if is_extension_mode {
        "ANTIDETECT BROWSER (EXTENSION MODE)"
    } else {
        "ANTIDETECT BROWSER CDP BRIDGE"
    };
    let badge = if is_extension_mode {
        badge_online("ZERO-PORT STEALTH")
    } else {
        badge_online("RUNNING")
    };

    let mut card = Card::new(title);
    card.with_badge(badge);
    card.with_min_width(74);
    card.add_kv("Engine", format!("adryfish/fingerprint-chromium (v{})", active_ver));
    card.add_kv("Profile", format!("{} [{}]", profile.name, profile.id));
    if let Some(seed) = profile.fingerprint_seed {
        card.add_kv("Seed (PRNG)", format!("{} (Deterministic Hardware Spoofing)", seed));
    }
    card.add_kv("Hardware", format!("{} cores / {} GB RAM", profile.hardware_concurrency.unwrap_or(8), profile.device_memory_gb.unwrap_or(16)));
    if profile.proxy.is_some() {
        card.add_kv("Proxy", &proxy_display);
    }

    if is_extension_mode {
        card.add_kv("CDP Port", "Disabled (Zero-Port Protection against localhost port scanning)");
        card.add_kv("Extensions", format!("{} active extension(s)", extension_paths.len()));
        card.add_line("");
        card.add_line("Ultra-Stealth Mode Active:");
        card.add_line("  • No listening CDP socket on localhost (anti-port scanning)");
        card.add_line("  • Automa MV3 background service worker / isolated world active");
        card.add_line("  • Native chrome.tabs and chrome.cookies execution");
    } else {
        card.add_kv("CDP Endpoint", format!("http://127.0.0.1:{}", effective_port));
        card.add_kv("WebSocket URL", &ws_url);
        card.add_line("");
        card.add_line("Playwright Connection Snippet (Python):");
        card.add_line(format!("  browser = await playwright.chromium.connect_over_cdp('http://localhost:{}')", effective_port));
        card.add_line("Playwright Connection Snippet (Node.js):");
        card.add_line(format!("  const browser = await chromium.connectOverCDP('http://localhost:{}');", effective_port));
    }
    card.add_kv("PID", pid_str);

    card.with_footer("Press Ctrl+C to terminate browser session cleanly");
    card.print();
    println!();

    tokio::signal::ctrl_c().await?;
    println!("\nReceived Ctrl+C, terminating browser process tree...");
    launcher.close().await?;
    println!("Browser session closed cleanly.");
    Ok(())
}
