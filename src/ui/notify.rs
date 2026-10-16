//! Unified Notification & Log Dispatcher for Specter CLI
//! Uses `@clack/prompts` visual grammar (`◇`, `◆`, `▲`, `■`, `┌`, `│`, `└`) on TTY
//! and single-line token-compact output on Non-TTY (AI Agent / CI pipes).

use std::io::{self, IsTerminal};
use crate::ui::colors;
use crate::ui::prompt::{
    clack_cancel, clack_intro, clack_outro, clack_step_err, clack_step_info, clack_step_ok,
    clack_step_warn, strip_ansi_codes, S_BAR,
};

pub struct Notify;

impl Notify {
    /// Success notification (`◇  message` on TTY, `[OK] message` on Non-TTY)
    pub fn success(msg: impl std::fmt::Display) {
        clack_step_ok(msg);
    }

    /// Success with detailed context
    pub fn success_with_detail(msg: impl std::fmt::Display, detail: impl std::fmt::Display) {
        clack_step_ok(format!(
            "{} {MUTED}({}){RESET}",
            msg,
            detail,
            MUTED = colors::MUTED,
            RESET = colors::RESET
        ));
    }

    /// Informational notification (`◆  message` on TTY, `[INFO] message` on Non-TTY)
    pub fn info(msg: impl std::fmt::Display) {
        clack_step_info(msg);
    }

    /// Warning notification (`▲  message` on TTY, `[WARN] message` on Non-TTY)
    pub fn warn(msg: impl std::fmt::Display) {
        clack_step_warn(msg);
    }

    /// Error notification (`■  message` on TTY, `[ERROR] message` on Non-TTY)
    pub fn error(msg: impl std::fmt::Display) {
        clack_step_err(msg);
    }

    /// Multi-step pipeline step (`◆  [current/total] message`)
    pub fn step(current: usize, total: usize, msg: impl std::fmt::Display) {
        clack_step_info(format!(
            "{PURPLE}[{current}/{total}]{RESET} {msg}",
            PURPLE = colors::PURPLE,
            RESET = colors::RESET
        ));
    }

    /// Shutdown lifecycle notification
    pub fn shutdown(msg: impl std::fmt::Display) {
        clack_step_warn(msg);
    }

    /// Clean shutdown completion notification
    pub fn shutdown_clean(msg: impl std::fmt::Display) {
        clack_outro(&msg.to_string());
    }

    /// Cancellation notification
    pub fn cancelled(msg: impl std::fmt::Display) {
        clack_cancel(&msg.to_string());
    }

    /// Clack-styled section header (`┌  specter  title`)
    pub fn header(title: impl std::fmt::Display) {
        let clean = strip_ansi_codes(&title.to_string());
        clack_intro("specter", &clean);
    }

    /// Clack vertical rail connector / divider
    pub fn divider() {
        if io::stdout().is_terminal() {
            println!("  \x1b[90m{S_BAR}\x1b[0m");
        }
    }

    /// Key-value display line along the Clack rail (`│  Key: Value`)
    pub fn key_val(key: impl std::fmt::Display, val: impl std::fmt::Display) {
        let k = key.to_string();
        let v = val.to_string();
        if io::stdout().is_terminal() {
            println!(
                "  \x1b[90m{S_BAR}\x1b[0m  {MUTED}{:<15}{RESET} {BOLD_WHITE}{v}{RESET}",
                format!("{k}:"),
                MUTED = colors::MUTED,
                BOLD_WHITE = colors::BOLD_WHITE,
                RESET = colors::RESET
            );
        } else {
            println!("{}: {}", strip_ansi_codes(&k), strip_ansi_codes(&v));
        }
    }
}
