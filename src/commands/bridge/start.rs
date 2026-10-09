use crate::infrastructure::bridge::{
    check_dependencies, probe_port, BridgeConfig, BridgeSupervisor, HttpToSocks5Bridge,
    ServerConfig, ServerSelector,
};
use crate::ui::{badge_online, badge_warn};

pub struct BridgeStartOptions<'a> {
    pub server: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub http: bool,
    pub ssh: bool,
    pub foreground: bool,
}

pub async fn start_bridge(opts: BridgeStartOptions<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;

    if config.servers.is_empty() {
        return Err(format!(
            "No servers found in configuration {}",
            BridgeConfig::config_path().display()
        )
        .into());
    }

    // Determine target servers
    let servers_to_start: Vec<(&String, &ServerConfig)> = match (opts.server, opts.tag) {
        // Tag filtering
        (_, Some(tag)) => {
            let matches = ServerSelector::filter_by_tag(&config, tag);
            if matches.is_empty() {
                return Err(format!("No enabled servers found matching tag '{}'", tag).into());
            }
            matches
        }
        // Specific server name, pattern, or "all"
        (Some(target), None) => {
            if target.eq_ignore_ascii_case("all") {
                ServerSelector::get_enabled(&config)
            } else if target.contains('*') {
                let matches = ServerSelector::filter_by_pattern(&config, target);
                if matches.is_empty() {
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
                    eprintln!(
                        "{} Server '{}' is disabled in {}",
                        badge_warn("DISABLED"),
                        target,
                        BridgeConfig::config_path().display()
                    );
                    eprintln!("  Enable it using: specter bridge enable {}", target);
                    return Err(format!("Server '{}' is disabled in bridge.json", target).into());
                }
                vec![(config.servers.get_key_value(target).unwrap().0, srv)]
            }
        }
        // Default: Start all enabled servers
        (None, None) => ServerSelector::get_enabled(&config),
    };

    // Report skipped disabled servers when starting all or by pattern
    let disabled = ServerSelector::get_disabled(&config);
    if opts.server.is_none()
        || opts.server == Some("all")
        || opts.server.map(|s| s.contains('*')).unwrap_or(false)
    {
        for (id, _) in &disabled {
            println!("  ○ SKIPPED Server '{}' (disabled in bridge.json)", id);
        }
    }

    if servers_to_start.is_empty() {
        println!(
            "{} No enabled servers to start. (Run 'specter bridge enable <server>' to enable one)",
            badge_warn("WARNING")
        );
        return Ok(());
    }

    println!(
        "{} Activating bridge connection(s) for {} server(s)...",
        badge_online("STARTING"),
        servers_to_start.len()
    );

    let mut started_count = 0;
    let mut last_socks_port = 1080;

    let mut sorted_servers = servers_to_start;
    sorted_servers.sort_by_key(|(id, srv)| (srv.priority.unwrap_or(100), (*id).clone()));

    for (srv_id, srv) in &sorted_servers {
        println!("  Connecting to '{}' ({})...", srv_id, srv.name);

        // Check prerequisites
        let deps = check_dependencies();
        for d in &deps {
            if !d.found
                && (d.name == "cloudflared" && srv.server_type == "cloudflare" || d.name == "ssh")
            {
                eprintln!("\n{} Missing required tool: '{}'", badge_warn("ERROR"), d.name);
                eprintln!("  Install command: {}", d.install_hint);
                return Err(format!("Missing prerequisite tool: {}", d.name).into());
            }
        }

        match BridgeSupervisor::start_server(srv_id, srv, opts.ssh).await {
            Ok(result) => {
                let port = result.socks_port.unwrap_or(1080);
                println!(
                    "  {} Server '{}' is ready! SOCKS5 proxy active on 127.0.0.1:{}",
                    badge_online("SUCCESS"),
                    result.server_id,
                    port
                );
                last_socks_port = port;
                started_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to start server '{}': {}",
                    badge_warn("ERROR"),
                    srv_id,
                    e
                );
            }
        }
    }

    if opts.http {
        let default_srv_socks = config
            .workstation
            .as_ref()
            .and_then(|w| w.default_server.as_deref())
            .and_then(|id| config.servers.get(id))
            .and_then(|s| s.socks_port)
            .unwrap_or(last_socks_port);

        let http_port = config
            .workloads
            .as_ref()
            .and_then(|w| w.supabase.as_ref())
            .map(|s| s.http_port)
            .unwrap_or(8118);

        if opts.foreground {
            let bridge = HttpToSocks5Bridge::new(http_port, default_srv_socks);
            bridge.start().await?;
            println!(
                "{} Embedded HTTP-to-SOCKS5 adapter active on 127.0.0.1:{} -> 127.0.0.1:{}",
                badge_online("SUCCESS"),
                http_port,
                default_srv_socks
            );
        } else {
            let current_exe = std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("specter"));
            let mut cmd = std::process::Command::new(current_exe);
            cmd.arg("bridge")
                .arg("start")
                .arg("--http")
                .arg("--foreground");
            if let Some(srv) = opts.server {
                cmd.arg(srv);
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
                let pid_path = BridgeSupervisor::pids_dir().join("specter-http.json");
                let pids = crate::infrastructure::bridge::supervisor::ServerPids {
                    server_id: "specter-http".to_string(),
                    cf_pid: Some(child.id()),
                    ssh_pid: None,
                };
                if let Ok(json) = serde_json::to_string(&pids) {
                    let _ = std::fs::write(pid_path, json);
                }
                println!(
                    "{} Embedded HTTP-to-SOCKS5 adapter active on 127.0.0.1:{} (Background daemon PID {})",
                    badge_online("SUCCESS"),
                    http_port,
                    child.id()
                );
            } else {
                eprintln!(
                    "{} Failed to spawn background daemon for HTTP bridge",
                    badge_warn("ERROR")
                );
            }
        }
    }

    println!(
        "\n{} Active servers: {}/{} online.",
        badge_online("READY"),
        started_count,
        config.servers.len()
    );

    if opts.foreground {
        println!(
            "\n{} Bridge supervisor running in foreground (Ctrl+C to stop)...",
            badge_online("SUPERVISOR")
        );
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("\n{} Shutdown signal received. Terminating all bridge tunnels...", badge_warn("STOPPING"));
                    let _ = BridgeSupervisor::stop_all();
                    println!("{} All bridge tunnels stopped cleanly.", badge_online("STOPPED"));
                    break;
                }
                _ = interval.tick() => {
                    for (srv_id, srv) in &sorted_servers {
                        let socks_port = srv.socks_port.unwrap_or(1080);
                        if !probe_port(socks_port) {
                            eprintln!("  {} Tunnel for '{}' dropped on port {}. Reconnecting...", badge_warn("HEAL"), srv_id, socks_port);
                            let _ = BridgeSupervisor::start_server(srv_id, srv, opts.ssh).await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
