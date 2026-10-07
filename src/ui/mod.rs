
pub mod colors {
    pub const CYAN: &str = "\x1b[38;2;56;189;248m";
    pub const PURPLE: &str = "\x1b[38;2;168;85;247m";
    pub const GREEN: &str = "\x1b[38;2;74;222;128m";
    pub const AMBER: &str = "\x1b[38;2;251;191;36m";
    pub const RED: &str = "\x1b[38;2;248;113;113m";
    pub const MUTED: &str = "\x1b[38;2;148;163;184m";
    pub const BORDER: &str = "\x1b[38;2;71;85;105m";
    pub const BOLD: &str = "\x1b[1m";
    pub const BOLD_WHITE: &str = "\x1b[1;37m";
    pub const DIM: &str = "\x1b[2m";
    pub const RESET: &str = "\x1b[0m";
}

/// Calculate the visible display width of a string, ignoring ANSI escape codes.
pub fn visible_width(s: &str) -> usize {
    let mut in_escape = false;
    let mut width = 0;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            // Count characters (approx width)
            width += 1;
        }
    }
    width
}

/// Status Badge Helpers
pub fn badge_online(text: &str) -> String {
    format!(
        "{GREEN}●{RESET} {BOLD_WHITE}{}{RESET}",
        text,
        GREEN = colors::GREEN,
        RESET = colors::RESET,
        BOLD_WHITE = colors::BOLD_WHITE,
    )
}

pub fn badge_offline(text: &str) -> String {
    format!(
        "{MUTED}○{RESET} {MUTED}{}{RESET}",
        text,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
    )
}

pub fn badge_warn(text: &str) -> String {
    format!(
        "{AMBER}●{RESET} {AMBER}{}{RESET}",
        text,
        AMBER = colors::AMBER,
        RESET = colors::RESET,
    )
}

pub fn badge_error(text: &str) -> String {
    format!(
        "{RED}●{RESET} {RED}{}{RESET}",
        text,
        RED = colors::RED,
        RESET = colors::RESET,
    )
}

pub fn badge_step(text: &str) -> String {
    format!(
        "{CYAN}[+]{RESET} {BOLD_WHITE}{}{RESET}",
        text,
        CYAN = colors::CYAN,
        RESET = colors::RESET,
        BOLD_WHITE = colors::BOLD_WHITE,
    )
}

pub fn status_pill(label: &str, value: &str, is_online: bool) -> String {
    if is_online {
        format!(
            "{BORDER}[{RESET}{MUTED}{}:{RESET}{GREEN}● {}{RESET}{BORDER}]{RESET}",
            label,
            value,
            BORDER = colors::BORDER,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
            GREEN = colors::GREEN,
        )
    } else {
        format!(
            "{BORDER}[{RESET}{MUTED}{}:{RESET}{MUTED}○ {}{RESET}{BORDER}]{RESET}",
            label,
            value,
            BORDER = colors::BORDER,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
        )
    }
}

/// Modern Rounded Card Component
#[derive(Clone, Debug)]
pub struct Card {
    title: String,
    badge: Option<String>,
    rows: Vec<(String, String)>,
    raw_lines: Vec<String>,
    footer: Option<String>,
    min_width: usize,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            badge: None,
            rows: Vec::new(),
            raw_lines: Vec::new(),
            footer: None,
            min_width: 66,
        }
    }

    pub fn with_badge(&mut self, badge: impl Into<String>) -> &mut Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn with_min_width(&mut self, width: usize) -> &mut Self {
        self.min_width = width;
        self
    }

    pub fn add_kv(&mut self, key: impl Into<String>, val: impl Into<String>) -> &mut Self {
        self.rows.push((key.into(), val.into()));
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) -> &mut Self {
        self.raw_lines.push(line.into());
        self
    }

    pub fn with_footer(&mut self, footer: impl Into<String>) -> &mut Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn render(&self) -> String {
        let mut max_key_len = 0;
        for (k, _) in &self.rows {
            let w = visible_width(k);
            if w > max_key_len {
                max_key_len = w;
            }
        }
        let key_col_width = (max_key_len + 2).max(14);

        // Compute required total inner width
        let title_w = visible_width(&self.title);
        let badge_w = self.badge.as_ref().map(|b| visible_width(b)).unwrap_or(0);
        let header_w = title_w + badge_w + 6;

        let mut max_row_w = 0;
        for (_k, v) in &self.rows {
            let row_w = key_col_width + visible_width(v) + 4;
            if row_w > max_row_w {
                max_row_w = row_w;
            }
        }
        for line in &self.raw_lines {
            let lw = visible_width(line) + 4;
            if lw > max_row_w {
                max_row_w = lw;
            }
        }

        let inner_width = self.min_width.max(header_w).max(max_row_w);

        let mut out = String::new();
        let border = colors::BORDER;
        let reset = colors::RESET;
        let cyan = colors::CYAN;
        let bold = colors::BOLD;
        let muted = colors::MUTED;
        let white = colors::BOLD_WHITE;

        // Top Border: ╭─ Title ─────── Badge ─╮
        out.push_str(border);
        out.push_str("╭─ ");
        out.push_str(reset);
        out.push_str(bold);
        out.push_str(cyan);
        out.push_str(&self.title);
        out.push_str(reset);
        out.push(' ');

        let title_part_w = title_w + 4;
        let badge_part_w = if self.badge.is_some() {
            badge_w + 3
        } else {
            0
        };

        let dashes = if inner_width > (title_part_w + badge_part_w) {
            inner_width - title_part_w - badge_part_w
        } else {
            2
        };

        out.push_str(border);
        out.push_str(&"─".repeat(dashes));
        out.push_str(reset);

        if let Some(ref b) = self.badge {
            out.push(' ');
            out.push_str(b);
            out.push(' ');
            out.push_str(border);
            out.push('─');
        }
        out.push_str(border);
        out.push_str("╮\n");
        out.push_str(reset);

        // Key-Value Rows
        for (k, v) in &self.rows {
            let kw = visible_width(k);
            let pad_k = if key_col_width > kw {
                " ".repeat(key_col_width - kw)
            } else {
                " ".to_string()
            };
            let vw = visible_width(v);
            let row_content_w = key_col_width + vw + 2;
            let pad_r = if inner_width > row_content_w {
                " ".repeat(inner_width - row_content_w)
            } else {
                "".to_string()
            };

            out.push_str(border);
            out.push_str("│  ");
            out.push_str(reset);
            out.push_str(muted);
            out.push_str(k);
            out.push_str(&pad_k);
            out.push_str(reset);
            out.push_str(white);
            out.push_str(v);
            out.push_str(reset);
            out.push_str(&pad_r);
            out.push_str(border);
            out.push_str("│\n");
            out.push_str(reset);
        }

        // Raw Lines
        for line in &self.raw_lines {
            let lw = visible_width(line);
            let pad_r = if inner_width > (lw + 2) {
                " ".repeat(inner_width - lw - 2)
            } else {
                "".to_string()
            };
            out.push_str(border);
            out.push_str("│  ");
            out.push_str(reset);
            out.push_str(line);
            out.push_str(&pad_r);
            out.push_str(border);
            out.push_str("│\n");
            out.push_str(reset);
        }

        // Bottom Border: ╰─ Footer ───╯
        out.push_str(border);
        if let Some(ref f) = self.footer {
            let fw = visible_width(f);
            let footer_dashes = if inner_width > (fw + 5) {
                inner_width - fw - 5
            } else {
                2
            };
            out.push_str("╰─ ");
            out.push_str(reset);
            out.push_str(muted);
            out.push_str(f);
            out.push_str(reset);
            out.push(' ');
            out.push_str(border);
            out.push_str(&"─".repeat(footer_dashes));
            out.push_str("╯\n");
        } else {
            out.push('╰');
            out.push_str(&"─".repeat(inner_width));
            out.push_str("╯\n");
        }
        out.push_str(reset);

        out
    }

    pub fn print(&self) {
        print!("{}", self.render());
    }
}

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

/// Brand Hero Banner with Styled ASCII Art Logo
pub fn render_hero(
    version: &str,
    cloud_status: &str,
    daemon_status: &str,
    browser_status: &str,
) -> String {
    let reset = colors::RESET;
    let bold_white = colors::BOLD_WHITE;
    let muted = colors::MUTED;
    let cyan = colors::CYAN;

    let c1 = "\x1b[38;2;56;189;248m";
    let c2 = "\x1b[38;2;96;165;250m";
    let c3 = "\x1b[38;2;129;140;248m";
    let c4 = "\x1b[38;2;168;85;247m";
    let c5 = "\x1b[38;2;192;132;252m";

    format!(
"
{c1}  ______          ____                  __ {reset}
{c2} /_  __/_  __    / __ \\__  __  ___     / /_{reset}
{c3}  / /  / / / /  / / / // / / // _ \\   / __/{reset}   {cyan}v{version}{reset}
{c4} / /  / /_/ /  / /_/ // /_/ //  __/  / /_  {reset}
{c5}/_/   \\__,_/   \\___\\_\\\\__,_/ \\___/   \\__/  {reset}

  {bold_white}Autonomous Browser Automation & Distributed Mesh Runtime{reset}
  {cloud_status}  {daemon_status}  {browser_status}
  {muted}Press [Tab] for autocomplete, 'help' for commands, 'exit' to quit{reset}
"
    )
}

/// Render a sleek notification banner when a newer CLI release is available
pub fn render_update_banner(current: &str, latest: &str) -> String {
    let amber = colors::AMBER;
    let bold = colors::BOLD;
    let white = colors::BOLD_WHITE;
    let reset = colors::RESET;
    let muted = colors::MUTED;
    let cyan = colors::CYAN;

    format!(
"  {amber}╭──────────────────────────────────────────────────────────────────────────╮{reset}
  {amber}│{reset}  🔔 {bold}{white}Update available:{reset} {muted}v{current}{reset} → {cyan}v{latest}{reset}  {muted}(Run 'specter upgrade' or 'scoop update'){reset} {amber}│{reset}
  {amber}╰──────────────────────────────────────────────────────────────────────────╯{reset}"
    )
}
