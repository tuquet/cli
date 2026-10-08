use crate::cli::BridgeSubcommands;
use crate::infrastructure::bridge::{
    check_dependencies, probe_port, BridgeConfig, BridgeSupervisor, HttpToSocks5Bridge,
    DiagnosticLevel, ServerConfig, ServerSelector,
};
use crate::ui::{badge_offline, badge_online, badge_warn, Card, Column, Table};

pub async fn handle(subcmd: Option<BridgeSubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        Some(BridgeSubcommands::Status) | None => show_status().await?,
        Some(BridgeSubcommands::Start { server, tag, http, ssh, foreground }) => {
            start_bridge(BridgeStartOptions { server: server.as_deref(), tag: tag.as_deref(), http, ssh, foreground }).await?
        }
        Some(BridgeSubcommands::Stop { server }) => stop_bridge(server.as_deref()).await?,
        Some(BridgeSubcommands::Enable { server }) => toggle_server(&server, true)?,
        Some(BridgeSubcommands::Disable { server }) => toggle_server(&server, false)?,
        Some(BridgeSubcommands::Check) => check_config()?,
        Some(BridgeSubcommands::Config { edit, show }) => manage_config(edit, show)?,
    }
    Ok(())
}

pub async fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load().unwrap_or_else(|_| BridgeConfig::default_config());
    let workstation_name = config.workstation.as_ref().map(|w| w.name.as_str()).unwrap_or("WORKSTATION");
    let default_server = config.workstation.as_ref().and_then(|w| w.default_server.as_deref()).unwrap_or("my-vps");

    println!();
    let mut header_card = Card::new("NETWORK BRIDGE & MULTI-VPS MESH");
    header_card.with_badge(badge_online("CONTROLLER READY"));
    header_card.with_min_width(74);
    header_card.add_kv("Workstation", workstation_name);
    header_card.add_kv("Config Source", BridgeConfig::config_path().to_string_lossy().to_string());
    header_card.add_kv("Default Server", default_server);
    header_card.with_footer("Pure Rust Engine • Zero Shell Scripts • Embedded HTTP-to-SOCKS5 Adapter");
    header_card.print();
    println!();

    // 1. Servers & VPS Port Map Table
    let columns = vec![
        Column { title: "Server ID".to_string(), min_width: 12, align_right: false },
        Column { title: "Display Name".to_string(), min_width: 20, align_right: false },
        Column { title: "Target Endpoint".to_string(), min_width: 18, align_right: false },
        Column { title: "Local SSH".to_string(), min_width: 12, align_right: false },
        Column { title: "SOCKS5 Proxy".to_string(), min_width: 12, align_right: false },
        Column { title: "Tags".to_string(), min_width: 10, align_right: false },
        Column { title: "Status".to_string(), min_width: 12, align_right: false },
    ];

    let mut table = Table::new(columns);

    let mut sorted_keys: Vec<_> = config.servers.keys().cloned().collect();
    sorted_keys.sort();

    for id in &sorted_keys {
        let srv = &config.servers[id];
        let endpoint = if srv.server_type == "cloudflare" {
            srv.cf_hostname.as_deref().unwrap_or("CF Tunnel")
        } else {
            srv.host.as_deref().unwrap_or("Direct IP")
        };

        let local_port = srv.local_ssh_port.unwrap_or(2222);
        let socks_port = srv.socks_port.unwrap_or(1080);

        let local_active = probe_port(local_port);
        let socks_active = probe_port(socks_port);

        let local_str = format!("{}:{}", local_port, if local_active { " [UP]" } else { " [DOWN]" });
        let socks_str = format!("{}:{}", socks_port, if socks_active { " [UP]" } else { " [DOWN]" });

        let status_str = if !srv.is_enabled() {
            "⊘ DISABLED".to_string()
        } else if socks_active {
            "● ONLINE".to_string()
        } else if local_active {
            "◐ TUNNEL UP".to_string()
        } else {
            "○ OFFLINE".to_string()
        };

        let tags_str = if srv.tags.is_empty() {
            "-".to_string()
        } else {
            srv.tags.join(",")
        };

        table.add_row(vec![
            id.clone(),
            srv.name.clone(),
            endpoint.to_string(),
            local_str,
            socks_str,
            tags_str,
            status_str,
        ]);
    }

    println!("{}", table.render());

    // 2. Active Workload Status
    println!();
    let mut workload_card = Card::new("WORKLOAD STATUS");
    workload_card.with_min_width(74);

    let git_port = config.workloads.as_ref().and_then(|w| w.git.as_ref()).map(|g| g.port).unwrap_or(1080);
    let git_online = probe_port(git_port);
    workload_card.add_kv(
        "Git Operations (1080)",
        if git_online {
            format!("{} Active on 127.0.0.1:{} (git push ready)", badge_online("ONLINE"), git_port)
        } else {
            format!("{} Offline. Run 'specter bridge start' to activate", badge_offline("OFFLINE"))
        },
    );

    let http_port = config.workloads.as_ref().and_then(|w| w.supabase.as_ref()).map(|s| s.http_port).unwrap_or(8118);
    let http_online = probe_port(http_port);
    workload_card.add_kv(
        "Supabase HTTP Bridge (8118)",
        if http_online {
            format!("{} Pure Rust HTTP Adapter active on 127.0.0.1:{}", badge_online("ONLINE"), http_port)
        } else {
            format!("{} Offline (Run 'specter bridge start --http')", badge_offline("OFFLINE"))
        },
    );

    workload_card.print();

    // 3. System Tool Dependencies Check
    let deps = check_dependencies();
    let all_found = deps.iter().all(|d| d.found);
    if !all_found {
        println!();
        let mut missing_card = Card::new("MISSING PREREQUISITES");
        missing_card.with_badge(badge_warn("ACTION REQUIRED"));
        missing_card.with_min_width(74);
        for d in &deps {
            if !d.found {
                missing_card.add_line(format!("• {}: NOT FOUND in system PATH", d.name));
                missing_card.add_line(format!("  Purpose: {}", d.description));
                missing_card.add_line(format!("  Install: {}", d.install_hint));
            }
        }
        missing_card.print();
    }

    Ok(())
}

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
        return Err(format!("No servers found in configuration {}", BridgeConfig::config_path().display()).into());
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
                    format!("Server '{}' not found in configuration {}", target, BridgeConfig::config_path().display())
                })?;
                if !srv.is_enabled() {
                    eprintln!("{} Server '{}' is disabled in {}", badge_warn("DISABLED"), target, BridgeConfig::config_path().display());
                    eprintln!("  Enable it using: specter bridge enable {}", target);
                    return Err(format!("Server '{}' is disabled in bridge.json", target).into());
                }
                vec![(config.servers.get_key_value(target).unwrap().0, srv)]
            }
        }
        // Default: Start all enabled servers
        (None, None) => {
            ServerSelector::get_enabled(&config)
        }
    };

    // Report skipped disabled servers when starting all or by pattern
    let disabled = ServerSelector::get_disabled(&config);
    if opts.server.is_none() || opts.server == Some("all") || opts.server.map(|s| s.contains('*')).unwrap_or(false) {
        for (id, _) in &disabled {
            println!("  ○ SKIPPED Server '{}' (disabled in bridge.json)", id);
        }
    }

    if servers_to_start.is_empty() {
        println!("{} No enabled servers to start. (Run 'specter bridge enable <server>' to enable one)", badge_warn("WARNING"));
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
            if !d.found && (d.name == "cloudflared" && srv.server_type == "cloudflare" || d.name == "ssh") {
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
                eprintln!("  {} Failed to start server '{}': {}", badge_warn("ERROR"), srv_id, e);
            }
        }
    }

    if opts.http {
        let default_srv_socks = config.workstation.as_ref()
            .and_then(|w| w.default_server.as_deref())
            .and_then(|id| config.servers.get(id))
            .and_then(|s| s.socks_port)
            .unwrap_or(last_socks_port);

        let http_port = config.workloads.as_ref().and_then(|w| w.supabase.as_ref()).map(|s| s.http_port).unwrap_or(8118);
        
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
            let current_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("specter"));
            let mut cmd = std::process::Command::new(current_exe);
            cmd.arg("bridge").arg("start").arg("--http").arg("--foreground");
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
                eprintln!("{} Failed to spawn background daemon for HTTP bridge", badge_warn("ERROR"));
            }
        }
    }

    println!("\n{} Active servers: {}/{} online.", badge_online("READY"), started_count, config.servers.len());

    if opts.foreground {
        println!("\n{} Bridge supervisor running in foreground (Ctrl+C to stop)...", badge_online("SUPERVISOR"));
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

pub fn toggle_server(server_id: &str, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BridgeConfig::load()?;
    let action_str = if enabled { "enabled" } else { "disabled" };
    let badge = if enabled { badge_online("ENABLED") } else { badge_warn("DISABLED") };

    if config.set_server_enabled(server_id, enabled)? {
        println!("{} Server '{}' has been {} in {}", badge, server_id, action_str, BridgeConfig::config_path().display());
    } else {
        eprintln!("{} Server '{}' not found in configuration {}", badge_warn("NOT FOUND"), server_id, BridgeConfig::config_path().display());
    }

    Ok(())
}

pub async fn stop_bridge(server_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load()?;

    if let Some(target) = server_opt {
        if target.eq_ignore_ascii_case("all") {
            BridgeSupervisor::stop_all()?;
            println!("{} All bridge processes terminated cleanly.", badge_online("STOPPED"));
        } else if let Some(srv) = config.servers.get(target) {
            BridgeSupervisor::stop_server(target, srv)?;
            println!("{} Bridge for '{}' stopped.", badge_online("STOPPED"), target);
        } else {
            eprintln!("{} Unknown server '{}'", badge_warn("WARN"), target);
        }
    } else {
        // Default stop all
        BridgeSupervisor::stop_all()?;
        println!("{} All bridge tunnels and proxy daemons stopped cleanly.", badge_online("STOPPED"));
    }

    Ok(())
}

pub fn check_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = BridgeConfig::config_path();
    println!("Checking configuration at {}...", config_path.display());

    let config = BridgeConfig::load()?;
    let diagnostics = config.validate();

    if diagnostics.is_empty() {
        println!("{} Configuration is 100% valid! Zero conflicts or missing keys found.", badge_online("VALID"));
        println!("  Registered servers: {}", config.servers.keys().cloned().collect::<Vec<_>>().join(", "));
    } else {
        println!("{} Found {} issue(s):", badge_warn("WARNING"), diagnostics.len());
        for d in diagnostics {
            let badge = match d.level {
                DiagnosticLevel::Error => badge_warn("ERROR"),
                DiagnosticLevel::Warning => badge_warn("WARN"),
                DiagnosticLevel::Info => badge_online("INFO"),
            };
            println!("  {} {}", badge, d.message);
        }
    }

    Ok(())
}

pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = BridgeConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = BridgeConfig::load().unwrap_or_else(|_| BridgeConfig::default_config());
        println!();
        let mut card = Card::new("BRIDGE CONFIGURATION");
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        if let Some(ref w) = config.workstation {
            card.add_kv("Workstation", &w.name);
            card.add_kv("Default Server", w.default_server.as_deref().unwrap_or("none"));
        }
        card.add_kv("Configured VPS", format!("{} server(s)", config.servers.len()));
        card.with_footer("Tip: edit with 'specter bridge config --edit' or validate via 'specter bridge check'");
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}
