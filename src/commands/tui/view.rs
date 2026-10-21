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
        let total_rows = rows as usize;

        // Position cursor at top-left
        crossterm::queue!(stdout, cursor::MoveTo(0, 0))?;

        // 1. Header Bar
        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
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
        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
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

        // Top separator border
        let sep_width = width.saturating_sub(2);
        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
        write!(stdout, "┌{}┐\r\n", "─".repeat(sep_width))?;

        // 3. Main Content Views according to ActiveTab
        let rendered_rows = match state.active_tab {
            ActiveTab::Overview => Self::render_overview(state, width, &mut stdout)?,
            ActiveTab::Bridge => Self::render_bridge_details(state, width, &mut stdout)?,
            ActiveTab::Browser => Self::render_browser_details(state, width, &mut stdout)?,
            ActiveTab::Automa => Self::render_automa_details(state, width, &mut stdout)?,
        };

        // Fill remaining height with blank box rows down to footer
        let footer_box_row = total_rows.saturating_sub(2);
        let current_row = 3 + rendered_rows;
        if current_row < footer_box_row {
            let empty_inner = " ".repeat(sep_width);
            for _ in current_row..footer_box_row {
                crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
                write!(stdout, "│{}│\r\n", empty_inner)?;
            }
        }

        // 4. Status & Hotkey Footer
        crossterm::queue!(
            stdout,
            cursor::MoveTo(0, footer_box_row as u16),
            terminal::Clear(terminal::ClearType::UntilNewLine)
        )?;
        write!(stdout, "└{}┘\r\n", "─".repeat(sep_width))?;

        let status_row = total_rows.saturating_sub(1) as u16;
        crossterm::queue!(
            stdout,
            cursor::MoveTo(0, status_row),
            terminal::Clear(terminal::ClearType::UntilNewLine),
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

    fn render_overview(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        // Row 1: Network Bridge & Browser Fleet
        let b_socks = if state.bridge.socks5_online { "[ONLINE]" } else { "[STANDBY]" };
        let b_http = if state.bridge.http_online { "[ONLINE]" } else { "[OFFLINE]" };
        let b_ssh = if state.bridge.ssh_online { "[ONLINE]" } else { "[STANDBY]" };

        let br_status = if state.browser.installed { "[READY]" } else { "[MISSING]" };

        Self::print_kv_split(
            stdout,
            width,
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

        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
        write!(stdout, "├{}┤\r\n", "─".repeat(width.saturating_sub(2)))?;

        // Row 2: Runner/Automa & Cloud Mesh
        let rn_status = if state.runner.daemon_online { "[READY]" } else { "[OFFLINE]" };
        let cl_status = if state.cloud.enrolled { "[ENROLLED]" } else { "[STANDBY]" };

        Self::print_kv_split(
            stdout,
            width,
            "● RUNNER RPC & WORKFLOW DAG",
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

        Ok(11)
    }

    fn render_bridge_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "[BRIDGE MESH TOPOLOGY & ENDPOINTS]")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- SOCKS5 Dynamic Proxy  : 127.0.0.1:{} -> {}",
                state.bridge.socks5_port,
                if state.bridge.socks5_online {
                    "ONLINE (Operational)"
                } else {
                    "OFFLINE"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Embedded HTTP Adapter : 127.0.0.1:{} -> {}",
                state.bridge.http_port,
                if state.bridge.http_online {
                    "ONLINE (Active)"
                } else {
                    "OFFLINE"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Cloudflare Access SSH : 127.0.0.1:{} -> {}",
                state.bridge.ssh_port,
                if state.bridge.ssh_online {
                    "ONLINE (Tunnel Up)"
                } else {
                    "OFFLINE"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(stdout, width, "Press 'b' to trigger bridge start/stop daemon.")?;
        count += 1;
        Ok(count)
    }

    fn render_browser_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "[STEALTH CHROMIUM RUNTIME & PROFILES]")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("- Version       : {}", state.browser.version),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("- Executable    : {}", state.browser.executable_path),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Total Profiles: {} sandboxed environments",
                state.browser.profile_count
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Active Sessions: {} instance(s) running",
                state.browser.running_pids_count
            ),
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "Press 'k' to terminate all active browser processes.",
        )?;
        count += 1;
        Ok(count)
    }

    fn render_automa_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "[WORKSTATION RUNNER & WORKFLOW DAG ENGINE]")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("- Daemon RPC Endpoint : http://127.0.0.1:{}", state.runner.port),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Daemon State        : {}",
                if state.runner.daemon_online {
                    "RUNNING"
                } else {
                    "STOPPED"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("- SQLite State Store  : {}", state.runner.db_size_str),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "- Vault Workflows     : {} workflow definitions",
                state.runner.workflow_count
            ),
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "Run 'tq workflow list' or 'tq runner start' to manage workers.",
        )?;
        count += 1;
        Ok(count)
    }

    fn print_box_line(stdout: &mut io::Stdout, width: usize, content: &str) -> io::Result<()> {
        let inner_width = width.saturating_sub(4); // 2 chars for "│ " + 2 chars for " │"
        let content_len = content.chars().count();
        let padded = if content_len < inner_width {
            format!("{}{}", content, " ".repeat(inner_width - content_len))
        } else if inner_width > 0 {
            content.chars().take(inner_width).collect::<String>()
        } else {
            String::new()
        };

        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
        write!(stdout, "│ {} │\r\n", padded)?;
        Ok(())
    }

    fn print_kv_split(
        stdout: &mut io::Stdout,
        width: usize,
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
        let total_avail = width.saturating_sub(7); // "│ " + col1 + " │ " + col2 + " │"
        let col_w1 = total_avail / 2;
        let col_w2 = total_avail.saturating_sub(col_w1);

        let pad = |s: &str, w: usize| {
            let count = s.chars().count();
            if count < w {
                format!("{}{}", s, " ".repeat(w - count))
            } else if w > 0 {
                s.chars().take(w).collect::<String>()
            } else {
                String::new()
            }
        };

        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
        crossterm::queue!(stdout, SetForegroundColor(Color::Cyan))?;
        write!(stdout, "│ {} │ {} │\r\n", pad(h1, col_w1), pad(h2, col_w2))?;
        crossterm::queue!(stdout, ResetColor)?;

        for (a, b) in [(r1_a, r1_b), (r2_a, r2_b), (r3_a, r3_b), (r4_a, r4_b)] {
            crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
            write!(stdout, "│ {} │ {} │\r\n", pad(a, col_w1), pad(b, col_w2))?;
        }
        Ok(())
    }
}
