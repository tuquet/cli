use super::colors::{self, visible_width};

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

        let footer_w = self
            .footer
            .as_ref()
            .map(|f| visible_width(f) + 7)
            .unwrap_or(0);
        let inner_width = self.min_width.max(header_w).max(max_row_w).max(footer_w);

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
        let badge_part_w = if self.badge.is_some() { badge_w + 3 } else { 0 };

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

    /// Render the card using `@clack/prompts` vertical rail (`┌ │ ◇ └`)
    pub fn render_clack(&self) -> String {
        let mut out = String::new();
        let badge_str = self.badge.as_deref().unwrap_or("");
        out.push_str(&format!(
            "  \x1b[90m┌\x1b[0m  \x1b[46;30;1m {} \x1b[0m {}\n",
            self.title, badge_str
        ));
        out.push_str("  \x1b[90m│\x1b[0m\n");

        if !self.rows.is_empty() {
            let max_key_len = self
                .rows
                .iter()
                .map(|(k, _)| visible_width(k))
                .max()
                .unwrap_or(12);
            let key_col_width = (max_key_len + 2).max(16);

            for (i, (k, v)) in self.rows.iter().enumerate() {
                let kw = visible_width(k);
                let pad_k = " ".repeat(key_col_width.saturating_sub(kw));
                let marker = if i == 0 {
                    "\x1b[36m◇\x1b[0m"
                } else {
                    "\x1b[90m│\x1b[0m"
                };
                out.push_str(&format!(
                    "  {marker}  {}{}{pad_k}{}{}{}\n",
                    colors::MUTED,
                    k,
                    colors::BOLD_WHITE,
                    v,
                    colors::RESET
                ));
            }
        }

        for line in &self.raw_lines {
            out.push_str(&format!("  \x1b[90m│\x1b[0m  {}\n", line));
        }

        if let Some(ref f) = self.footer {
            out.push_str("  \x1b[90m│\x1b[0m\n");
            out.push_str(&format!(
                "  \x1b[90m└\x1b[0m  {}{}{}\n",
                colors::MUTED,
                f,
                colors::RESET
            ));
        } else {
            out.push_str("  \x1b[90m└\x1b[0m\n");
        }

        out
    }

    /// Render ultra-compact output for Non-TTY / AI Agents (zero box-border token waste)
    pub fn render_compact(&self) -> String {
        use super::prompt::strip_ansi_codes;
        let mut out = String::new();
        let title_clean = strip_ansi_codes(&self.title);
        if let Some(ref b) = self.badge {
            out.push_str(&format!("[{}] {}\n", title_clean, strip_ansi_codes(b)));
        } else {
            out.push_str(&format!("[{}]\n", title_clean));
        }
        for (k, v) in &self.rows {
            out.push_str(&format!(
                "{}: {}\n",
                strip_ansi_codes(k).trim(),
                strip_ansi_codes(v).trim()
            ));
        }
        for line in &self.raw_lines {
            let clean = strip_ansi_codes(line);
            let trimmed = clean.trim();
            if !trimmed.is_empty() && !trimmed.chars().all(|c| c == '─' || c == '-') {
                out.push_str(&format!("{}\n", trimmed));
            }
        }
        if let Some(ref f) = self.footer {
            out.push_str(&format!("note: {}\n", strip_ansi_codes(f).trim()));
        }
        out
    }

    pub fn print(&self) {
        use std::io::IsTerminal;
        if std::io::stdout().is_terminal() {
            print!("{}", self.render_clack());
        } else {
            print!("{}", self.render_compact());
        }
    }
}
