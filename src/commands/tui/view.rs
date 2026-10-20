use super::state::{ActiveTab, TuiState};
use crossterm::{
    cursor,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};
use std::io::{self, Write};

pub struct TuiView;

impl TuiView {
    pub fn render(state: &TuiState) -> io::Result<()> {
        let mut stdout = io::stdout();
        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let width = cols as usize;

        // Position cursor at top-left
        crossterm::queue!(stdout, cursor::MoveTo(0, 0))?;

        // 1. Header Bar
        let title = format!(" [tq tui] TUQUET MISSION CONTROL v{} ", env!("CARGO_PKG_VERSION"));
        let tick_indicator = if state.tick_count % 2 == 0 { "● LIVE" } else { "○ LIVE" };
        let top_right = format!(" [{} 1s] [q: Quit] ", tick_indicator);
        
        crossterm::queue!(
            stdout,
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White)
        )?;
        
        let header_fill = if width > title.len() + top_right.len() {
            width - title.len() - top_right.len()
        } else {
            0
        };
        write!(stdout, "{}{}{}", title, " ".repeat(header_fill), top_right)?;
        crossterm::queue!(stdout, ResetColor)?;
        write!(stdout, "\r\n")?;

        // 2. Navigation Tabs
        let tabs = [
            ActiveTab::Overview,
            ActiveTab::Bridge,
            ActiveTab::Browser,
            ActiveTab::Automa,
        ];
        write!(stdout, " ")?;
        for tab in tabs {
            if tab == state.active_tab {
                crossterm::queue!(
                    stdout,
                    SetBackgroundColor(Color::Cyan),
                    SetForegroundColor(Color::Black)
                )?;
                write!(stdout, " {} ", tab.label())?;
                crossterm::queue!(stdout, ResetColor)?;
            } else {
                crossterm::queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
                write!(stdout, " {} ", tab.label())?;
                crossterm::queue!(stdout, ResetColor)?;
            }
            write!(stdout, "  ")?;
        }
        write!(stdout, "\r\n")?;

        // Separator line
        let sep_width = width.saturating_sub(2);
        write!(stdout, "┌{}┐\r\n", "─".repeat(sep_width))?;

        // 3. Main Content Views according to ActiveTab
        match state.active_tab {
            ActiveTab::Overview => Self::render_overview(state, width, &mut stdout)?,
            ActiveTab::Bridge => Self::render_bridge_details(state, width, &mut stdout)?,
            ActiveTab::Browser => Self::render_browser_details(state, width, &mut stdout)?,
            ActiveTab::Automa => Self::render_automa_details(state, width, &mut stdout)?,
        }

        // Fill remaining height with blank rows if terminal is tall
        // 4. Status & Hotkey Footer
        crossterm::queue!(stdout, cursor::MoveTo(0, rows.saturating_sub(2)))?;
        write!(stdout, "└{}┘\r\n", "─".repeat(sep_width))?;

        // Status bar line
        crossterm::queue!(
            stdout,
            SetBackgroundColor(Color::Rgb { r: 24, g: 24, b: 27 }),
            SetForegroundColor(Color::Yellow)
        )?;
        let msg = format!(" STATUS: {}", state.status_message);
        let msg_padded = if msg.len() < width {
            format!("{}{}", msg, " ".repeat(width - msg.len()))
        } else {
            msg[..width].to_string()
        };
        write!(stdout, "{}", msg_padded)?;
        crossterm::queue!(stdout, ResetColor)?;

        stdout.flush()?;
        Ok(())
    }

    fn render_overview(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<()> {
        let col_width = (width.saturating_sub(6)) / 2;

        // Row 1: Network Bridge & Browser Fleet
        let b_socks = if state.bridge.socks5_online { "[ONLINE]" } else { "[STANDBY]" };
        let b_http = if state.bridge.http_online { "[ONLINE]" } else { "[OFFLINE]" };
        let b_ssh = if state.bridge.ssh_online { "[ONLINE]" } else { "[STANDBY]" };

        let br_status = if state.browser.installed { "[READY]" } else { "[MISSING]" };

        Self::print_kv_split(
            stdout,
            col_width,
            "● NETWORK BRIDGE (SOCKS5/HTTP)",
            "● BROWSER FLEET RUNTIME",
            &format!("  SOCKS5 Proxy (1080): {}", b_socks),
            &format!("  Chromium V8  : {}", state.browser.version),
            &format!("  HTTP Proxy   (8118): {}", b_http),
            &format!("  Status       : {}", br_status),
            &format!("  SSH Tunnel   (2222): {}", b_ssh),
            &format!("  Active PIDs  : {} running", state.browser.running_pids_count),
            &format!("  Default Node : {}", state.bridge.default_server),
            &format!("  Sandboxes    : {} profiles", state.browser.profile_count),
        )?;

        write!(stdout, "├{}┤\r\n", "─".repeat(width.saturating_sub(2)))?;

        // Row 2: Runner/Automa & Cloud Mesh
        let rn_status = if state.runner.daemon_online { "[READY]" } else { "[OFFLINE]" };
        let cl_status = if state.cloud.enrolled { "[ENROLLED]" } else { "[STANDBY]" };

        Self::print_kv_split(
            stdout,
            col_width,
            "● RUNNER RPC & AUTOMA DAG",
            "● CLOUD MESH & IDENTITY",
            &format!("  Runner RPC   : 127.0.0.1:{}", state.runner.port),
            &format!("  Host Name    : {}", state.cloud.host_name),
            &format!("  Daemon State : {}", rn_status),
            &format!("  Tenant Scope : {}", state.cloud.tenant_name),
            &format!("  SQLite Vault : {}", state.runner.db_size_str),
            &format!("  Cloud Node   : {}", state.cloud.cloud_url),
            &format!("  Stored Flows : {} flows", state.runner.workflow_count),
            &format!("  Identity     : {}", cl_status),
        )?;

        Ok(())
    }

    fn render_bridge_details(state: &TuiState, _width: usize, stdout: &mut io::Stdout) -> io::Result<()> {
        write!(stdout, "│  [BRIDGE MESH TOPOLOGY & ENDPOINTS]\r\n")?;
        write!(stdout, "│  - SOCKS5 Dynamic Proxy  : 127.0.0.1:{} -> {}\r\n", 
            state.bridge.socks5_port, 
            if state.bridge.socks5_online { "ONLINE (Operational)" } else { "OFFLINE" }
        )?;
        write!(stdout, "│  - Embedded HTTP Adapter : 127.0.0.1:{} -> {}\r\n", 
            state.bridge.http_port, 
            if state.bridge.http_online { "ONLINE (Active)" } else { "OFFLINE" }
        )?;
        write!(stdout, "│  - Cloudflare Access SSH : 127.0.0.1:{} -> {}\r\n", 
            state.bridge.ssh_port, 
            if state.bridge.ssh_online { "ONLINE (Tunnel Up)" } else { "OFFLINE" }
        )?;
        write!(stdout, "│\r\n")?;
        write!(stdout, "│  Press 'b' to trigger bridge start/stop daemon.\r\n")?;
        Ok(())
    }

    fn render_browser_details(state: &TuiState, _width: usize, stdout: &mut io::Stdout) -> io::Result<()> {
        write!(stdout, "│  [STEALTH CHROMIUM RUNTIME & PROFILES]\r\n")?;
        write!(stdout, "│  - Version       : {}\r\n", state.browser.version)?;
        write!(stdout, "│  - Executable    : {}\r\n", state.browser.executable_path)?;
        write!(stdout, "│  - Total Profiles: {} sandboxed environments\r\n", state.browser.profile_count)?;
        write!(stdout, "│  - Active Sessions: {} instance(s) running\r\n", state.browser.running_pids_count)?;
        write!(stdout, "│\r\n")?;
        write!(stdout, "│  Press 'k' to terminate all active browser processes.\r\n")?;
        Ok(())
    }

    fn render_automa_details(state: &TuiState, _width: usize, stdout: &mut io::Stdout) -> io::Result<()> {
        write!(stdout, "│  [WORKSTATION RUNNER & AUTOMA DAG ENGINE]\r\n")?;
        write!(stdout, "│  - Daemon RPC Endpoint : http://127.0.0.1:{}\r\n", state.runner.port)?;
        write!(stdout, "│  - Daemon State        : {}\r\n", if state.runner.daemon_online { "RUNNING" } else { "STOPPED" })?;
        write!(stdout, "│  - SQLite State Store  : {}\r\n", state.runner.db_size_str)?;
        write!(stdout, "│  - Vault Workflows     : {} workflow definitions\r\n", state.runner.workflow_count)?;
        write!(stdout, "│\r\n")?;
        write!(stdout, "│  Run 'tq automa list' or 'tq runner start' to manage workers.\r\n")?;
        Ok(())
    }

    fn print_kv_split(
        stdout: &mut io::Stdout,
        col_w: usize,
        h1: &str,
        h2: &str,
        r1_a: &str,
        r1_b: &str,
        r2_a: &str,
        r2_b: &str,
        r3_a: &str,
        r3_b: &str,
        r4_a: &str,
        r4_b: &str,
    ) -> io::Result<()> {
        let pad = |s: &str, w: usize| {
            if s.len() < w {
                format!("{}{}", s, " ".repeat(w - s.len()))
            } else {
                s[..w].to_string()
            }
        };

        crossterm::queue!(stdout, SetForegroundColor(Color::Cyan))?;
        write!(stdout, "│ {} │ {} │\r\n", pad(h1, col_w), pad(h2, col_w))?;
        crossterm::queue!(stdout, ResetColor)?;

        write!(stdout, "│ {} │ {} │\r\n", pad(r1_a, col_w), pad(r1_b, col_w))?;
        write!(stdout, "│ {} │ {} │\r\n", pad(r2_a, col_w), pad(r2_b, col_w))?;
        write!(stdout, "│ {} │ {} │\r\n", pad(r3_a, col_w), pad(r3_b, col_w))?;
        write!(stdout, "│ {} │ {} │\r\n", pad(r4_a, col_w), pad(r4_b, col_w))?;
        Ok(())
    }
}
