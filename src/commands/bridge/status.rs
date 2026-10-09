use crate::infrastructure::bridge::{
    check_dependencies, probe_port_async, BridgeConfig,
};
use crate::ui::{
    badge_online, badge_warn, create_network_topology_card, default_workstation_endpoints,
    respond_with, Card, Column, Table,
};

pub async fn show_status(format: crate::ui::OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::load().unwrap_or_else(|_| BridgeConfig::default_config());
    let workstation_name = config
        .workstation
        .as_ref()
        .map(|w| w.name.as_str())
        .unwrap_or("WORKSTATION");
    let default_server = config
        .workstation
        .as_ref()
        .and_then(|w| w.default_server.as_deref())
        .unwrap_or("my-vps");

    let mut sorted_keys: Vec<_> = config.servers.keys().cloned().collect();
    sorted_keys.sort();

    let mut server_json_entries = Vec::new();
    let mut server_table_rows = Vec::new();

    // Concurrent asynchronous probing across all servers in parallel
    let mut probe_futures = Vec::new();
    for id in &sorted_keys {
        let srv = &config.servers[id];
        let local_port = srv.local_ssh_port.unwrap_or(2222);
        let socks_port = srv.socks_port.unwrap_or(1080);
        probe_futures.push(async move {
            tokio::join!(
                crate::infrastructure::bridge::probe_port_async(local_port),
                crate::infrastructure::bridge::probe_port_async(socks_port),
            )
        });
    }
    let probe_results = futures::future::join_all(probe_futures).await;

    for (i, id) in sorted_keys.iter().enumerate() {
        let srv = &config.servers[id];
        let endpoint = if srv.server_type == "cloudflare" {
            srv.cf_hostname.as_deref().unwrap_or("CF Tunnel")
        } else {
            srv.host.as_deref().unwrap_or("Direct IP")
        };

        let local_port = srv.local_ssh_port.unwrap_or(2222);
        let socks_port = srv.socks_port.unwrap_or(1080);

        let (local_active, socks_active) = probe_results[i];

        let status_code = if !srv.is_enabled() {
            "DISABLED"
        } else if socks_active {
            "ONLINE"
        } else if local_active {
            "TUNNEL_UP"
        } else {
            "OFFLINE"
        };

        server_json_entries.push(serde_json::json!({
            "id": id,
            "name": srv.name,
            "endpoint": endpoint,
            "server_type": srv.server_type,
            "local_ssh_port": local_port,
            "socks_port": socks_port,
            "local_ssh_online": local_active,
            "socks_proxy_online": socks_active,
            "status": status_code,
            "tags": srv.tags,
        }));

        let local_str = format!(
            "{}:{}",
            local_port,
            if local_active {
                " [UP]"
            } else {
                " [DOWN]"
            }
        );
        let socks_str = format!(
            "{}:{}",
            socks_port,
            if socks_active {
                " [UP]"
            } else {
                " [DOWN]"
            }
        );

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

        server_table_rows.push(vec![
            id.clone(),
            srv.name.clone(),
            endpoint.to_string(),
            local_str,
            socks_str,
            tags_str,
            status_str,
        ]);
    }

    let ssh_port = config
        .servers
        .get(default_server)
        .or_else(|| config.servers.get("my-vps"))
        .and_then(|s| s.local_ssh_port)
        .unwrap_or(crate::constants::DEFAULT_SSH_TUNNEL_PORT);
    let git_port = config
        .workloads
        .as_ref()
        .and_then(|w| w.git.as_ref())
        .map(|g| g.port)
        .unwrap_or(crate::constants::DEFAULT_SOCKS5_PORT);
    let http_port = config
        .workloads
        .as_ref()
        .and_then(|w| w.supabase.as_ref())
        .map(|s| s.http_port)
        .unwrap_or(crate::constants::DEFAULT_HTTP_BRIDGE_PORT);

    let (ssh_online, git_online, http_online) = tokio::join!(
        probe_port_async(ssh_port),
        probe_port_async(git_port),
        probe_port_async(http_port),
    );

    let deps = check_dependencies();
    let all_found = deps.iter().all(|d| d.found);

    let payload = serde_json::json!({
        "workstation": workstation_name,
        "config_source": BridgeConfig::config_path().to_string_lossy(),
        "default_server": default_server,
        "servers": server_json_entries,
        "workloads": {
            "ssh": { "port": ssh_port, "online": ssh_online },
            "git": { "port": git_port, "online": git_online },
            "supabase_http": { "port": http_port, "online": http_online }
        },
        "dependencies": deps.iter().map(|d| serde_json::json!({
            "name": d.name,
            "found": d.found,
            "description": d.description,
            "install_hint": d.install_hint
        })).collect::<Vec<_>>(),
        "ready": all_found
    });

    respond_with(format, &payload, |_| {

    println!();
    let mut header_card = Card::new("NETWORK BRIDGE & MULTI-VPS MESH");
    header_card.with_badge(badge_online("CONTROLLER READY"));
    header_card.with_min_width(74);
    header_card.add_kv("Workstation", workstation_name);
    header_card.add_kv(
        "Config Source",
        BridgeConfig::config_path().to_string_lossy().to_string(),
    );
    header_card.add_kv("Default Server", default_server);
    header_card.with_footer(
        "Pure Rust Engine • Zero Shell Scripts • Embedded HTTP-to-SOCKS5 Adapter",
    );
    header_card.print();
    println!();

    // 1. Servers & VPS Port Map Table
    let columns = vec![
        Column {
            title: "Server ID".to_string(),
            min_width: 12,
            align_right: false,
        },
        Column {
            title: "Display Name".to_string(),
            min_width: 20,
            align_right: false,
        },
        Column {
            title: "Target Endpoint".to_string(),
            min_width: 18,
            align_right: false,
        },
        Column {
            title: "Local SSH".to_string(),
            min_width: 12,
            align_right: false,
        },
        Column {
            title: "SOCKS5 Proxy".to_string(),
            min_width: 12,
            align_right: false,
        },
        Column {
            title: "Tags".to_string(),
            min_width: 10,
            align_right: false,
        },
        Column {
            title: "Status".to_string(),
            min_width: 12,
            align_right: false,
        },
    ];

    let mut table = Table::new(columns);

    for row in server_table_rows {
        table.add_row(row);
    }

    println!("{}", table.render());

    // 2. Active Workload Status
    println!();

    let endpoints = default_workstation_endpoints(
        git_port,
        git_online,
        http_port,
        http_online,
        ssh_port,
        ssh_online,
        None,
    );
    let mut workload_card = create_network_topology_card(
        "WORKLOAD TOPOLOGY & LISTENERS",
        &endpoints,
        Some("Start workloads with 'specter bridge start' (add --http for Supabase)"),
    );
    workload_card.with_min_width(74);
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
})
}
