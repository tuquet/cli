//! Clean Key-Value Presentation for Inspect / Describe / Details commands
//! Follows Google Cloud SDK (`gcloud compute instances describe`) standards.
//! Zero boxes, zero border frames, indentation-based hierarchy, 100% terminal responsive.

use crate::ui::colors;

/// A field in a key-value description list
pub struct KvItem {
    pub key: String,
    pub val: String,
}

impl KvItem {
    pub fn new(key: impl Into<String>, val: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            val: val.into(),
        }
    }
}

/// Print flat or hierarchically indented key-value pairs
pub fn print_key_values(items: &[(&str, &str)]) {
    if items.is_empty() {
        return;
    }

    // Determine max key width for clean alignment (cap at 24 to prevent blowup)
    let max_key_len = items
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(12)
        .min(24);

    for (k, v) in items {
        if v.is_empty() {
            continue;
        }
        println!(
            "{BOLD}{key:<width$}{RESET}  {val}",
            BOLD = colors::BOLD,
            RESET = colors::RESET,
            key = format!("{}:", k),
            width = max_key_len + 1,
            val = v
        );
    }
}

/// Print a section header with optional indented key-value pairs
pub fn print_section(title: &str, items: &[(&str, &str)]) {
    println!("{BOLD}{CYAN}{title}{RESET}", BOLD = colors::BOLD, CYAN = colors::CYAN, RESET = colors::RESET);
    if items.is_empty() {
        return;
    }

    let max_key_len = items
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(12)
        .min(24);

    for (k, v) in items {
        if v.is_empty() {
            continue;
        }
        println!(
            "  {MUTED}{key:<width$}{RESET} {val}",
            MUTED = colors::MUTED,
            RESET = colors::RESET,
            key = format!("{}:", k),
            width = max_key_len + 1,
            val = v
        );
    }
}
