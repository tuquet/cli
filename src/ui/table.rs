use super::badges::{badge_offline, badge_online};
use super::card::Card;
use super::colors::{self, visible_width};

/// Modern Rounded Table Component
pub struct Column {
    pub title: String,
    pub min_width: usize,
    pub align_right: bool,
}

pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
    footer: Option<String>,
}

impl Table {
    pub fn new(columns: Vec<Column>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            footer: None,
        }
    }

    pub fn add_row(&mut self, row: Vec<impl Into<String>>) -> &mut Self {
        self.rows.push(row.into_iter().map(|s| s.into()).collect());
        self
    }

    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn render(&self) -> String {
        // Compute column widths
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.title.len().max(c.min_width)).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let w = visible_width(cell);
                    if w > widths[i] {
                        widths[i] = w;
                    }
                }
            }
        }

        let mut out = String::new();
        let border = colors::BORDER;
        let reset = colors::RESET;
        let bold = colors::BOLD;
        let cyan = colors::CYAN;
        let muted = colors::MUTED;
        let white = colors::BOLD_WHITE;

        // Top Border: ╭──────┬──────╮
        out.push_str(border);
        out.push('╭');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < widths.len() {
                out.push('┬');
            } else {
                out.push('╮');
            }
        }
        out.push('\n');
        out.push_str(reset);

        // Header Row: │  ID  │ NAME │
        out.push_str(border);
        out.push('│');
        for (i, col) in self.columns.iter().enumerate() {
            let w = widths[i];
            let cw = visible_width(&col.title);
            let pad = " ".repeat(w.saturating_sub(cw));
            out.push(' ');
            out.push_str(reset);
            out.push_str(bold);
            out.push_str(cyan);
            if col.align_right {
                out.push_str(&pad);
                out.push_str(&col.title);
            } else {
                out.push_str(&col.title);
                out.push_str(&pad);
            }
            out.push_str(reset);
            out.push(' ');
            out.push_str(border);
            out.push('│');
        }
        out.push('\n');

        // Middle Divider: ├──────┼──────┤
        out.push('├');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < widths.len() {
                out.push('┼');
            } else {
                out.push('┤');
            }
        }
        out.push('\n');
        out.push_str(reset);

        // Data Rows
        if self.rows.is_empty() {
            let total_w: usize = widths.iter().sum::<usize>() + (widths.len() * 3) - 1;
            let empty_text = "(No entries found)";
            let pad = (total_w.saturating_sub(empty_text.len())) / 2;
            out.push_str(border);
            out.push('│');
            out.push_str(&" ".repeat(pad));
            out.push_str(reset);
            out.push_str(muted);
            out.push_str(empty_text);
            out.push_str(reset);
            let remaining = total_w.saturating_sub(pad + empty_text.len());
            out.push_str(&" ".repeat(remaining));
            out.push_str(border);
            out.push_str("│\n");
        } else {
            for row in &self.rows {
                out.push_str(border);
                out.push('│');
                for (i, col) in self.columns.iter().enumerate() {
                    let w = widths[i];
                    let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    let cw = visible_width(cell);
                    let pad = " ".repeat(w.saturating_sub(cw));
                    out.push(' ');
                    out.push_str(reset);
                    out.push_str(white);
                    if col.align_right {
                        out.push_str(&pad);
                        out.push_str(cell);
                    } else {
                        out.push_str(cell);
                        out.push_str(&pad);
                    }
                    out.push_str(reset);
                    out.push(' ');
                    out.push_str(border);
                    out.push('│');
                }
                out.push('\n');
            }
        }

        // Bottom Border: ╰──────┴──────╯
        out.push_str(border);
        out.push('╰');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < widths.len() {
                out.push('┴');
            } else {
                out.push('╯');
            }
        }
        out.push('\n');
        out.push_str(reset);

        // Footer Tip
        if let Some(ref f) = self.footer {
            out.push_str("  ");
            out.push_str(muted);
            out.push_str(f);
            out.push_str(reset);
            out.push('\n');
        }

        out
    }

    pub fn print(&self) {
        print!("{}", self.render());
    }
}

/// Represents an endpoint entry in the network topology table
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkEndpoint {
    pub protocol: String,
    pub endpoint: String,
    pub role: String,
    pub active: bool,
}

impl NetworkEndpoint {
    pub fn new(
        protocol: impl Into<String>,
        endpoint: impl Into<String>,
        role: impl Into<String>,
        active: bool,
    ) -> Self {
        Self {
            protocol: protocol.into(),
            endpoint: endpoint.into(),
            role: role.into(),
            active,
        }
    }
}

/// Helper to build standard workstation endpoints for Specter services
pub fn default_workstation_endpoints(
    socks5_port: u16,
    socks5_active: bool,
    http_port: u16,
    http_active: bool,
    ssh_port: u16,
    ssh_active: bool,
    runner: Option<(u16, bool)>,
) -> Vec<NetworkEndpoint> {
    let mut items = vec![
        NetworkEndpoint::new(
            "SOCKS5",
            format!("127.0.0.1:{socks5_port}"),
            "Dynamic Proxy (SSH)",
            socks5_active,
        ),
        NetworkEndpoint::new(
            "HTTP Relay",
            format!("127.0.0.1:{http_port}"),
            format!("HTTP-to-SOCKS5 (:{socks5_port})"),
            http_active,
        ),
        NetworkEndpoint::new(
            "TCP Tunnel",
            format!("127.0.0.1:{ssh_port}"),
            "Cloudflare Access (VPS)",
            ssh_active,
        ),
    ];
    if let Some((r_port, r_active)) = runner {
        items.push(NetworkEndpoint::new(
            "Daemon API",
            format!("127.0.0.1:{r_port}"),
            "Runner Worker (RPC/WS)",
            r_active,
        ));
    }
    items
}

/// Represents a row entry in a standardized 4-column tabular card
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabularRow {
    pub col1: String, // Component / Protocol (14 chars visible width)
    pub col2: String, // Endpoint / Resource / Version (20 chars visible width)
    pub col3: String, // Role / Details (26 chars visible width)
    pub status: String, // Formatted badge string
}

impl TabularRow {
    pub fn new(
        col1: impl Into<String>,
        col2: impl Into<String>,
        col3: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            col1: col1.into(),
            col2: col2.into(),
            col3: col3.into(),
            status: status.into(),
        }
    }
}

/// Generalized generator for 4-column Tabular Cards across all microservice pillars
pub fn create_tabular_card(
    title: &str,
    badge: Option<String>,
    headers: [&str; 4],
    rows: &[TabularRow],
    footer: Option<&str>,
    min_width: usize,
) -> Card {
    let mut card = Card::new(title);

    if let Some(b) = badge {
        card.with_badge(b);
    }

    // Dynamically calculate required column widths based on longest content + at least 2 spaces gap
    let mut w0 = visible_width(headers[0]);
    let mut w1 = visible_width(headers[1]);
    let mut w2 = visible_width(headers[2]);

    for row in rows {
        w0 = w0.max(visible_width(&row.col1));
        w1 = w1.max(visible_width(&row.col2));
        w2 = w2.max(visible_width(&row.col3));
    }

    let c0_width = w0.max(14) + 2;
    let c1_width = w1.max(18) + 2;
    let c2_width = w2.max(24) + 2;

    let table_inner_width = c0_width + c1_width + c2_width + 12;
    card.with_min_width(min_width.max(table_inner_width));

    let pad0_hdr = " ".repeat(c0_width.saturating_sub(visible_width(headers[0])));
    let pad1_hdr = " ".repeat(c1_width.saturating_sub(visible_width(headers[1])));
    let pad2_hdr = " ".repeat(c2_width.saturating_sub(visible_width(headers[2])));

    // Header line
    let header_line = format!(
        "{BOLD}{MUTED}{}{}{}{}{}{}{}{RESET}",
        headers[0],
        pad0_hdr,
        headers[1],
        pad1_hdr,
        headers[2],
        pad2_hdr,
        headers[3],
        BOLD = colors::BOLD,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
    );
    card.add_line(header_line);

    for row in rows {
        let pad0 = " ".repeat(c0_width.saturating_sub(visible_width(&row.col1)));
        let pad1 = " ".repeat(c1_width.saturating_sub(visible_width(&row.col2)));
        let pad2 = " ".repeat(c2_width.saturating_sub(visible_width(&row.col3)));

        let row_line = format!(
            "{BOLD_WHITE}{}{pad0}{RESET}{CYAN}{}{pad1}{RESET}{MUTED}{}{pad2}{RESET}{}",
            row.col1,
            row.col2,
            row.col3,
            row.status,
            BOLD_WHITE = colors::BOLD_WHITE,
            RESET = colors::RESET,
            CYAN = colors::CYAN,
            MUTED = colors::MUTED,
        );
        card.add_line(row_line);
    }

    if let Some(f) = footer {
        card.with_footer(f);
    }

    card
}

/// Create a standardized Network Topology & Listeners card (Option 1)
pub fn create_network_topology_card(
    title: &str,
    endpoints: &[NetworkEndpoint],
    custom_footer: Option<&str>,
) -> Card {
    let any_active = endpoints.iter().any(|e| e.active);
    let badge = if any_active {
        badge_online("ACTIVE WORKLOADS")
    } else {
        badge_offline("STANDBY")
    };

    let rows: Vec<TabularRow> = endpoints
        .iter()
        .map(|ep| {
            let status = if ep.active {
                badge_online("ACTIVE")
            } else {
                badge_offline("STANDBY")
            };
            TabularRow::new(&ep.protocol, &ep.endpoint, &ep.role, status)
        })
        .collect();

    let dynamic_footer = if let Some(footer) = custom_footer {
        footer.to_string()
    } else {
        let socks5_active = endpoints
            .iter()
            .find(|e| e.protocol == "SOCKS5")
            .map(|e| e.active)
            .unwrap_or(false);
        let http_active = endpoints
            .iter()
            .find(|e| e.protocol == "HTTP Relay")
            .map(|e| e.active)
            .unwrap_or(false);

        if socks5_active && http_active {
            "Egress ready for Git push, cURL, and Supabase CLI".to_string()
        } else if socks5_active {
            "Egress ready for Git & cURL. Run 'specter bridge start --http' for Supabase"
                .to_string()
        } else {
            "Tunnels run on demand. Start with 'specter bridge start'".to_string()
        }
    };

    create_tabular_card(
        title,
        Some(badge),
        ["PROTOCOL", "ENDPOINT", "ROLE / TARGET", "STATUS"],
        &rows,
        Some(&dynamic_footer),
        72,
    )
}
