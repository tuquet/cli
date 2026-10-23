use crate::infrastructure::bridge::{
    BridgeConfig, BridgeSupervisor, ServerConfig, ServerSelector,
    check_dependencies, probe_port,
};
use crate::ui::prompt::{
    clack_intro, clack_outro, clack_select, clack_step_err, clack_step_ok, clack_step_warn,
    ClackOption, ClackSpinner, is_interactive_tty,
};

pub struct BridgeStartOptions<'a> {
    pub server: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub http: bool,
    pub ssh: bool,
    pub foreground: bool,
}

pub async fn start_bridge(mut opts: BridgeStartOptions<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;

    if config.servers.is_empty() {
        return Err(format!(
            "No servers found in configuration {}",
            BridgeConfig::config_path().display()
        )
        .into());
    }

    let enabled_servers = ServerSelector::get_enabled(&config);

    // Interactive prompt only when on a TTY and no explicit server/tag/http options provided
    let is_interactive = is_interactive_tty() && opts.server.is_none() && opts.tag.is_none() && !opts.http && !opts.ssh;

    if is_interactive {
        clack_intro("tq gateway start", "Autonomous SOCKS5 & HTTP Forwarder");

        // 1. Select gateway mode (Zero icons, plain technical)
        let mode_options = vec![
            ClackOption::new(
                "full",
                "full",
                "SOCKS5 (:1080) and HTTP forwarder (:8118) (default)",
            ),
            ClackOption::new(
                "socks5",
                "socks5",
                "SOCKS5 dynamic proxy only (:1080)",
            ),
            ClackOption::new(
                "http",
                "http",
                "HTTP-to-SOCKS5 forwarder only (:8118)",
            ),
        ];

        if let Ok(Some(idx)) = clack_select("Select gateway mode:", &mode_options, 0, 5) {
            match idx {
                0 => {
                    opts.http = true;
                }
                1 => {
                    opts.http = false;
                }
                2 => {
                    opts.http = true;
                }
                _ => {}
            }
        }

        // 2. Select target server: ONLY ask if more than 1 server is enabled!
        if enabled_servers.len() > 1 {
            let mut srv_options = Vec::new();
            for (id, srv) in &enabled_servers {
                let desc = srv.cf_hostname.as_deref().or(srv.host.as_deref()).unwrap_or("active");
                srv_options.push(ClackOption::new(id.as_str(), id.as_str(), desc));
            }
            srv_options.push(ClackOption::new("all", "all", "all enabled servers"));

            if let Ok(Some(idx)) = clack_select("Select target server:", &srv_options, 0, 7) {
                if idx < srv_options.len() - 1 {
                    let chosen = srv_options[idx].label.clone();
                    return Box::pin(start_bridge_resolved(opts, chosen)).await;
                }
            }
        } else if let Some((single_id, single_srv)) = enabled_servers.first() {
            let desc = single_srv.cf_hostname.as_deref().or(single_srv.host.as_deref()).unwrap_or("active");
            eprintln!("Target server: {} ({})", single_id, desc);
        }
    }

    start_bridge_core(opts, &config, enabled_servers).await
}

async fn start_bridge_resolved(
    opts: BridgeStartOptions<'_>,
    server_target: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;
    let mut resolved_opts = opts;
    resolved_opts.server = Some(&server_target);
    let enabled_servers = ServerSelector::get_enabled(&config);
    start_bridge_core(resolved_opts, &config, enabled_servers).await
}

async fn start_bridge_core(
    opts: BridgeStartOptions<'_>,
    config: &BridgeConfig,
    enabled_servers: Vec<(&String, &ServerConfig)>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine target servers
    let servers_to_start: Vec<(&String, &ServerConfig)> = match (opts.server, opts.tag) {
        // Tag filtering
        (_, Some(tag)) => {
            let matches = ServerSelector::filter_by_tag(config, tag);
            if matches.is_empty() {
                clack_step_err(format!("No enabled servers found matching tag '{}'", tag));
                return Err(format!("No enabled servers found matching tag '{}'", tag).into());
            }
            matches
        }
        // Specific server name, pattern, or "all"
        (Some(target), None) => {
            if target.eq_ignore_ascii_case("all") {
                enabled_servers
            } else if target.contains('*') {
                let matches = ServerSelector::filter_by_pattern(config, target);
                if matches.is_empty() {
                    clack_step_err(format!("No enabled servers matching pattern '{}'", target));
                    return Err(format!("No enabled servers matching pattern '{}'", target).into());
                }
                matches
            } else {
                let srv = config.servers.get(target).ok_or_else(|| {
                    format!(
                        "Server '{}' not found in configuration {}",
                        target,
                        BridgeConfig::config_path().display()
                    )
                })?;
                if !srv.is_enabled() {
                    clack_step_warn(format!(
                        "Server '{}' is disabled in {} (Enable with: tq bridge enable {})",
                        target,
                        BridgeConfig::config_path().display(),
                        target
                    ));
                    return Err(format!("Server '{}' is disabled in bridge.json", target).into());
                }
                vec![(config.servers.get_key_value(target).unwrap().0, srv)]
            }
        }
        // Default: Start all enabled servers
        (None, None) => enabled_servers,
    };

    if servers_to_start.is_empty() {
        clack_step_warn("No enabled servers to start. (Run 'tq bridge enable' to enable one)");
        clack_outro("0 servers activated.");
        return Ok(());
    }

    let mut _started_count = 0;
    let mut last_socks_port = 1080;

    let mut sorted_servers = servers_to_start;
    sorted_servers.sort_by_key(|(id, srv)| (srv.priority.unwrap_or(100), (*id).clone()));

    for (srv_id, srv) in &sorted_servers {
        // Check prerequisites
        let deps = check_dependencies();
        for d in &deps {
            if !d.found
                && (d.name == "cloudflared" && srv.server_type == "cloudflare" || d.name == "ssh")
            {
                clack_step_err(format!(
                    "Missing required tool: '{}' (Install: {})",
                    d.name, d.install_hint
                ));
                return Err(format!("Missing prerequisite tool: {}", d.name).into());
            }
        }

        let sp = ClackSpinner::start(format!("Connecting to '{}' ({})...", srv_id, srv.name));
        match BridgeSupervisor::start_server(srv_id, srv, opts.ssh).await {
            Ok(result) => {
                let port = result.socks_port.unwrap_or(1080);
                sp.finish(format!(
                    "Server '{}' is ready. SOCKS5 proxy active on 127.0.0.1:{}",
                    result.server_id, port
                ));
                last_socks_port = port;
                _started_count += 1;
            }
            Err(e) => {
                sp.fail(format!("Failed to start server '{}': {}", srv_id, e));
            }
        }
    }

    let mut http_active = false;
    let mut http_port = 8118;
    if opts.http {
        let configured_http_port = config
            .workloads
            .as_ref()
            .and_then(|w| w.supabase.as_ref())
            .map(|s| s.http_port)
            .unwrap_or(8118);

        http_port = configured_http_port;

        if probe_port(configured_http_port) {
            clack_step_ok(format!(
                "HTTP-to-SOCKS5 forwarder already active on 127.0.0.1:{}",
                configured_http_port
            ));
            http_active = true;
        } else {
            let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("tq"));
            let mut cmd = std::process::Command::new(exe);
            cmd.arg("gateway")
                .arg("start")
                .arg("--http")
                .arg("--foreground");
            if let Some(srv) = opts.server {
                cmd.arg("-s").arg(srv);
            }
            if let Some(tag) = opts.tag {
                cmd.arg("-t").arg(tag);
            }
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000008);
            }

            if let Ok(child) = cmd.spawn() {
                let pid_path = BridgeSupervisor::pids_dir().join("http.json");
                let pids = crate::infrastructure::bridge::supervisor::ServerPids {
                    server_id: "http".to_string(),
                    cf_pid: Some(child.id()),
                    ssh_pid: None,
                };
                if let Ok(json) = serde_json::to_string(&pids) {
                    let _ = std::fs::write(pid_path, json);
                }
                clack_step_ok(format!(
                    "HTTP-to-SOCKS5 forwarder active on 127.0.0.1:{} (pid: {})",
                    http_port,
                    child.id()
                ));
                http_active = true;
            } else {
                clack_step_err("Failed to spawn background daemon for HTTP bridge");
            }
        }
    }

    eprintln!("\nGateway online:");
    eprintln!("  SOCKS5 proxy   : 127.0.0.1:{}", last_socks_port);
    if http_active {
        eprintln!("  HTTP forwarder : 127.0.0.1:{}", http_port);
    }
    eprintln!("  Supervisor     : active (probe: 5s, auto-heal: enabled)");
    eprintln!("\n[watchdog] Press Ctrl+C to detach (daemon keeps running). Run 'tq gateway stop' to terminate.\n");

    // Unified Watchdog Loop: Streams live health and detaches cleanly on Ctrl+C without killing tunnels
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\nDetached. Gateway watchdog continues running in background.");
                eprintln!("Use 'tq gateway status' to inspect or 'tq gateway stop' to terminate.");
                break;
            }
            _ = interval.tick() => {
                for (srv_id, srv) in &sorted_servers {
                    let socks_port = srv.socks_port.unwrap_or(1080);
                    if !probe_port(socks_port) {
                        eprintln!("[watchdog] WARN: Port {} dropped. Reconnecting to '{}'...", socks_port, srv_id);
                        let _ = BridgeSupervisor::start_server(srv_id, srv, opts.ssh).await;
                    }
                }
            }
        }
    }

    Ok(())
}
