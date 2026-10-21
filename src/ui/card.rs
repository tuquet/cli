use super::colors::{self, visible_width};

/// Clean Minimal Card / Group Component (No heavy ASCII boxes)
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
            min_width: 50,
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
        let mut out = String::new();
        let reset = colors::RESET;
        let cyan = colors::CYAN;
        let bold = colors::BOLD;
        let muted = colors::MUTED;
        let white = colors::BOLD_WHITE;

        // Title Header: [TITLE] [BADGE]
        out.push_str("  ");
        out.push_str(bold);
        out.push_str(cyan);
        out.push('[');
        out.push_str(&self.title);
        out.push(']');
        out.push_str(reset);

        if let Some(ref b) = self.badge {
            out.push(' ');
            out.push_str(b);
        }
        out.push('\n');

        let key_col_width = self
            .rows
            .iter()
            .map(|(k, _)| visible_width(k))
            .max()
            .unwrap_or(15)
            .max(12);

        // Key-Value Rows
        for (k, v) in &self.rows {
            let kw = visible_width(k);
            let pad = " ".repeat(key_col_width.saturating_sub(kw));
            out.push_str("    ");
            out.push_str(muted);
            out.push_str(k);
            out.push_str(&pad);
            out.push_str(" : ");
            out.push_str(reset);
            out.push_str(white);
            out.push_str(v);
            out.push_str(reset);
            out.push('\n');
        }

        // Raw Lines
        for line in &self.raw_lines {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }

        // Footer
        if let Some(ref f) = self.footer {
            out.push_str("    ");
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
