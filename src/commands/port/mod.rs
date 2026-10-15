use std::collections::BTreeMap;
use std::process::Command;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::ui::{
    badge_online, clack_cancel, clack_intro, clack_outro, clack_select, colors,
    create_tabular_card, ClackOption, Column, Notify, OutputFormat, Table, TabularRow,
};

const DEV_PROCESS_KEYWORDS: &[&str] = &[
    "node",
    "java",
    "javaw",
    "cargo",
    "rustc",
    "specter",
    "spc",
    "bun",
    "deno",
    "python",
    "uvicorn",
    "gunicorn",
    "go",
    "docker",
    "com.docker",
    "postgres",
    "mysqld",
    "redis",
    "nginx",
    "caddy",
    "cloudflared",
    "ssh",
    "vite",
    "next",
    "esbuild",
    "ruby",
    "php",
    "dotnet",
];

const OS_SYSTEM_PROCESSES: &[&str] = &[
    "system",
    "svchost.exe",
    "lsass.exe",
    "wininit.exe",
    "services.exe",
    "spoolsv.exe",
    "smss.exe",
    "csrss.exe",
    "searchindexer.exe",
    "dashost.exe",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListeningPort {
    pub port: u16,
    pub local_address: String,
    pub pid: u32,
    pub process_name: String,
    pub memory_mb: f64,
    pub detail: String,
    pub is_dev_runtime: bool,
}

pub async fn handle(
    query: Option<String>,
    kill: bool,
    all: bool,
    list: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let ports = scan_listening_ports(all);

    let filtered: Vec<ListeningPort> = match query {
        Some(ref q) => filter_ports(&ports, q),
        None => ports,
    };

    if list {
        render_port_list(&filtered, all, format);
        return Ok(());
    }

    if filtered.is_empty() {
        match query {
            Some(ref q) => Notify::warn(format!(
                "No listening port or process matching '{}' found. (Use '--all' to include OS ports)",
                q
            )),
            None => Notify::info(
                "No active developer listening ports found. (Use 'spc port --all' to view all system ports)",
            ),
        }
        return Ok(());
    }

    // Direct non-interactive kill: `spc port 5173 --kill`
    if kill && query.is_some() {
        clack_intro(
            "spc port",
            &format!("Terminating {} matching port process(es)", filtered.len()),
        );
        for item in &filtered {
            kill_port_process(item)?;
        }
        return Ok(());
    }

    clack_intro(
        "spc port",
        &format!(
            "Local Port Scanner & Process Killer • {} listening port(s)",
            filtered.len()
        ),
    );

    let clack_opts: Vec<ClackOption> = filtered
        .iter()
        .map(|p| {
            let label = format!(":{}  •  {} (PID {})", p.port, p.process_name, p.pid);
            let hint = format!("{:.1} MB • {} • {}", p.memory_mb, p.local_address, p.detail);
            let summary = format!(":{} ({} • PID {})", p.port, p.process_name, p.pid);
            ClackOption::new(label, hint, summary)
        })
        .collect();

    match clack_select(
        "Select a listening port to terminate its process (type port or name to filter)",
        &clack_opts,
        0,
        10,
    )? {
        Some(idx) => {
            let target = &filtered[idx];
            kill_port_process(target)?;
        }
        None => {
            clack_cancel("Operation cancelled.");
        }
    }

    Ok(())
}

fn kill_port_process(item: &ListeningPort) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/PID", &item.pid.to_string()])
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to kill PID {} ({}) on port {}: {}",
                item.pid,
                item.process_name,
                item.port,
                stderr.trim()
            )
            .into());
        }
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("kill")
            .args(["-9", &item.pid.to_string()])
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to kill PID {} ({}) on port {}: {}",
                item.pid,
                item.process_name,
                item.port,
                stderr.trim()
            )
            .into());
        }
    }

    clack_outro(&format!(
        "Freed port {BOLD}{CYAN}:{port}{RESET} — terminated {BOLD_WHITE}{proc}{RESET} {MUTED}(PID {pid}){RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        port = item.port,
        proc = item.process_name,
        pid = item.pid,
    ));
    Ok(())
}

pub fn filter_ports(ports: &[ListeningPort], query: &str) -> Vec<ListeningPort> {
    let q = query.trim().trim_start_matches(':').to_lowercase();
    if q.is_empty() {
        return ports.to_vec();
    }

    // If query is an exact port number, prefer exact port match first
    if let Ok(port_num) = q.parse::<u16>() {
        let exact: Vec<ListeningPort> = ports
            .iter()
            .filter(|p| p.port == port_num)
            .cloned()
            .collect();
        if !exact.is_empty() {
            return exact;
        }
    }

    ports
        .iter()
        .filter(|p| {
            p.port.to_string().contains(&q)
                || p.process_name.to_lowercase().contains(&q)
                || p.pid.to_string() == q
                || p.detail.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

pub fn scan_listening_ports(include_all: bool) -> Vec<ListeningPort> {
    let raw_sockets = collect_raw_listening_sockets();
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut deduplicated: BTreeMap<(u16, u32), ListeningPort> = BTreeMap::new();

    for (port, local_addr, pid) in raw_sockets {
        if pid == 0 || pid == 4 {
            continue;
        }
        if deduplicated.contains_key(&(port, pid)) {
            continue;
        }

        let sys_pid = Pid::from_u32(pid);
        let (proc_name, mem_mb, detail) = if let Some(proc_info) = sys.process(sys_pid) {
            let name = proc_info.name().to_string_lossy().into_owned();
            let mem = proc_info.memory() as f64 / 1_048_576.0;
            let cwd_or_exe = proc_info
                .cwd()
                .map(|p| p.display().to_string())
                .or_else(|| proc_info.exe().map(|p| p.display().to_string()))
                .unwrap_or_else(|| "-".to_string());
            (name, mem, cwd_or_exe)
        } else {
            ("unknown".to_string(), 0.0, "-".to_string())
        };

        let name_lower = proc_name.to_lowercase();
        let is_os_sys = OS_SYSTEM_PROCESSES.iter().any(|s| *s == name_lower);
        let is_dev_runtime = DEV_PROCESS_KEYWORDS
            .iter()
            .any(|k| name_lower.contains(k));
        let is_common_dev_port = (1024..=49151).contains(&port);

        if !include_all {
            if is_os_sys {
                continue;
            }
            if !is_dev_runtime && !is_common_dev_port {
                continue;
            }
        }

        deduplicated.insert(
            (port, pid),
            ListeningPort {
                port,
                local_address: local_addr,
                pid,
                process_name: proc_name,
                memory_mb: (mem_mb * 10.0).round() / 10.0,
                detail,
                is_dev_runtime,
            },
        );
    }

    let mut list: Vec<ListeningPort> = deduplicated.into_values().collect();
    list.sort_by(|a, b| {
        let a_hot = (3000..=9999).contains(&a.port);
        let b_hot = (3000..=9999).contains(&b.port);
        b.is_dev_runtime
            .cmp(&a.is_dev_runtime)
            .then_with(|| b_hot.cmp(&a_hot))
            .then_with(|| a.port.cmp(&b.port))
    });
    list
}

#[cfg(windows)]
fn collect_raw_listening_sockets() -> Vec<(u16, String, u32)> {
    let Ok(output) = Command::new("netstat").args(["-ano", "-p", "tcp"]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    parse_netstat_output(&text)
}

#[cfg(not(windows))]
fn collect_raw_listening_sockets() -> Vec<(u16, String, u32)> {
    if let Ok(output) = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n"])
        .output()
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in text.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9
                && let Ok(pid) = parts[1].parse::<u32>()
            {
                let addr = parts[8];
                if let Some(colon) = addr.rfind(':')
                    && let Ok(port) = addr[colon + 1..].parse::<u16>()
                {
                    results.push((port, addr.to_string(), pid));
                }
            }
        }
        return results;
    }
    Vec::new()
}

pub fn parse_netstat_output(text: &str) -> Vec<(u16, String, u32)> {
    let mut results = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("TCP") {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 || !parts[3].eq_ignore_ascii_case("LISTENING") {
            continue;
        }
        let local_addr = parts[1];
        let Ok(pid) = parts[4].parse::<u32>() else {
            continue;
        };
        if let Some(colon_idx) = local_addr.rfind(':')
            && let Ok(port) = local_addr[colon_idx + 1..].parse::<u16>()
        {
            results.push((port, local_addr.to_string(), pid));
        }
    }
    results
}

fn render_port_list(ports: &[ListeningPort], include_all: bool, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(ports) {
                println!("{}", json);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new(vec![
                Column {
                    title: "PORT".to_string(),
                    min_width: 8,
                    align_right: false,
                },
                Column {
                    title: "PROCESS".to_string(),
                    min_width: 18,
                    align_right: false,
                },
                Column {
                    title: "PID".to_string(),
                    min_width: 8,
                    align_right: true,
                },
                Column {
                    title: "MEMORY".to_string(),
                    min_width: 10,
                    align_right: true,
                },
                Column {
                    title: "PATH / CWD".to_string(),
                    min_width: 28,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Found {} listening port(s){} • Run 'spc port [port]' to select & kill",
                ports.len(),
                if include_all { " (all system)" } else { " (developer)" }
            ));

            for p in ports {
                table.add_row(vec![
                    format!(":{}", p.port),
                    p.process_name.clone(),
                    p.pid.to_string(),
                    format!("{:.1} MB", p.memory_mb),
                    p.detail.clone(),
                ]);
            }
            println!();
            table.print();
            println!();
        }
        OutputFormat::Card => {
            let rows: Vec<TabularRow> = ports
                .iter()
                .map(|p| {
                    TabularRow::new(
                        format!(":{}", p.port),
                        &p.process_name,
                        format!("PID {} • {:.1} MB", p.pid, p.memory_mb),
                        badge_online(&p.local_address),
                    )
                })
                .collect();

            let footer = format!(
                "Found {} active port(s) • Run 'spc port <port> --kill' or 'spc port' to terminate",
                ports.len()
            );
            let card = create_tabular_card(
                "ACTIVE LISTENING PORTS",
                Some(badge_online(&format!("{} LISTENING", ports.len()))),
                ["PORT", "PROCESS NAME", "PID & MEMORY", "BIND ADDRESS"],
                &rows,
                Some(&footer),
                76,
            );
            println!();
            card.print();
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_netstat_output() {
        let sample = "\
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:5173           0.0.0.0:0              LISTENING       14200
  TCP    127.0.0.1:8080         0.0.0.0:0              LISTENING       9824
  TCP    127.0.0.1:5173         127.0.0.1:52100        ESTABLISHED     14200
  TCP    [::]:3000              [::]:0                 LISTENING       7712
";
        let parsed = parse_netstat_output(sample);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], (5173, "0.0.0.0:5173".to_string(), 14200));
        assert_eq!(parsed[1], (8080, "127.0.0.1:8080".to_string(), 9824));
        assert_eq!(parsed[2], (3000, "[::]:3000".to_string(), 7712));
    }
}
