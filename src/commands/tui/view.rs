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

        // 2. Navigation Tabs (Flat SSOT matching --help)
        crossterm::queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
        let tabs = [
            ActiveTab::Workflow,
            ActiveTab::Browser,
            ActiveTab::Bridge,
            ActiveTab::Auth,
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
            ActiveTab::Workflow => Self::render_workflow_details(state, width, &mut stdout)?,
            ActiveTab::Browser => Self::render_browser_details(state, width, &mut stdout)?,
            ActiveTab::Bridge => Self::render_bridge_details(state, width, &mut stdout)?,
            ActiveTab::Auth => Self::render_auth_details(state, width, &mut stdout)?,
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

    fn render_workflow_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "● WORKFLOW AUTOMATION & EXECUTION ENGINE")?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Daemon State        : {}",
                if state.runner.daemon_online {
                    "[RUNNING] (Online)"
                } else {
                    "[STOPPED] (Offline)"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Runner RPC Endpoint : http://127.0.0.1:{}", state.runner.port),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  SQLite State Store  : {}", state.runner.db_size_str),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Vault Workflows     : {} workflow definitions",
                state.runner.workflow_count
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  Execution Mode      : Zero-port Extension Stealth / CDP DevTools",
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "─ COMMANDS & SHORTCUTS ────────────────────────────────────────────────────────",
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "  • 'r' / 'R'         : Refresh live runner & workflow state")?;
        count += 1;
        Self::print_box_line(stdout, width, "  • tq workflow run   : Execute automation workflow directly")?;
        count += 1;
        Self::print_box_line(stdout, width, "  • tq workflow list  : List all workflows stored in SQLite vault")?;
        count += 1;
        Self::print_box_line(stdout, width, "  • tq runner start   : Start background runner daemon worker")?;
        count += 1;
        Ok(count)
    }

    fn render_browser_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "● BROWSER RUNTIME & ANTI-DETECT FLEET")?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Runtime Status      : {}",
                if state.browser.installed {
                    "[READY] (Installed)"
                } else {
                    "[MISSING] (Run 'tq browser install')"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Chromium V8 Version : {}", state.browser.version),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Binary Executable   : {}", state.browser.executable_path),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Sandboxed Profiles  : {} isolated profile(s)",
                state.browser.profile_count
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Active Processes    : {} running instance(s)",
                state.browser.running_pids_count
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  Stealth Evasion     : C++ Native Fingerprint Masking Active",
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "─ COMMANDS & SHORTCUTS ────────────────────────────────────────────────────────",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • 'k' / 'K'         : Terminate all active browser processes",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq browser launch : Launch isolated browser sandbox profile",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq browser list   : Inspect registered browser profiles and PIDs",
        )?;
        count += 1;
        Ok(count)
    }

    fn render_bridge_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "● NETWORK BRIDGE & MULTI-VPS MESH")?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  SOCKS5 Dynamic Proxy  : 127.0.0.1:{} -> {}",
                state.bridge.socks5_port,
                if state.bridge.socks5_online {
                    "[ONLINE] (Operational)"
                } else {
                    "[OFFLINE] (Standby)"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Embedded HTTP Adapter : 127.0.0.1:{} -> {}",
                state.bridge.http_port,
                if state.bridge.http_online {
                    "[ONLINE] (Active)"
                } else {
                    "[OFFLINE] (Standby)"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Cloudflare Tunnel SSH : 127.0.0.1:{} -> {}",
                state.bridge.ssh_port,
                if state.bridge.ssh_online {
                    "[ONLINE] (Tunnel Up)"
                } else {
                    "[OFFLINE] (Tunnel Down)"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Default Egress Node   : {}", state.bridge.default_server),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Active Mesh Nodes     : {}/{} server(s) online",
                state.bridge.active_servers, state.bridge.total_servers
            ),
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "─ COMMANDS & SHORTCUTS ────────────────────────────────────────────────────────",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • 'b' / 'B'         : Start / Stop Specter Bridge tunnel daemons",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq bridge status  : Inspect multi-VPS mesh topology and latency",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq git <repo> push-bridge : Route git push operations through mesh",
        )?;
        count += 1;
        Ok(count)
    }

    fn render_auth_details(state: &TuiState, width: usize, stdout: &mut io::Stdout) -> io::Result<usize> {
        let mut count = 0;
        Self::print_box_line(stdout, width, "● CLOUD AUTHENTICATION & MULTI-TENANT IDENTITY")?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!(
                "  Enrollment Status   : {}",
                if state.cloud.enrolled {
                    "[ENROLLED] (Paired)"
                } else {
                    "[STANDBY] (Unpaired / Local)"
                }
            ),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Workstation Host    : {}", state.cloud.host_name),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Active Tenant Scope : {}", state.cloud.tenant_name),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            &format!("  Cloud Endpoint URL  : {}", state.cloud.cloud_url),
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  Security Protocol   : Ed25519 Device Keypair & JWT Session",
        )?;
        count += 1;
        Self::print_box_line(stdout, width, "")?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "─ COMMANDS & SHORTCUTS ────────────────────────────────────────────────────────",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq auth login     : Authenticate operator account and pair workstation",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq auth whoami    : Check active enrollment status and token claims",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq auth tenant    : Switch or list accessible multi-tenant workspaces",
        )?;
        count += 1;
        Self::print_box_line(
            stdout,
            width,
            "  • tq auth logout    : Revoke local token and disconnect workstation",
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
}
