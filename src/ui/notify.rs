//! Unified Notification & Log Dispatcher for Specter CLI
//! Standardizes console feedback, streams, and styling across commands.

use crate::ui::colors;

pub struct Notify;

impl Notify {
    /// Success notification (`[SUCCESS]` message)
    pub fn success(msg: impl std::fmt::Display) {
        println!("{}[SUCCESS]{} {}", colors::GREEN, colors::RESET, msg);
    }

    /// Success with detailed context
    pub fn success_with_detail(msg: impl std::fmt::Display, detail: impl std::fmt::Display) {
        println!("{}[SUCCESS]{} {}: {}", colors::GREEN, colors::RESET, msg, detail);
    }

    /// Informational notification (`[INFO]` message)
    pub fn info(msg: impl std::fmt::Display) {
        println!("{}[INFO]{} {}", colors::CYAN, colors::RESET, msg);
    }

    /// Warning notification (`[WARN]` message) to stderr
    pub fn warn(msg: impl std::fmt::Display) {
        eprintln!("{}[WARN]{} {}", colors::AMBER, colors::RESET, msg);
    }

    /// Error notification (`[ERROR]` message) strictly to stderr
    pub fn error(msg: impl std::fmt::Display) {
        eprintln!("{}[ERROR]{} {}", colors::RED, colors::RESET, msg);
    }

    /// Multi-step pipeline step (`[current/total]` message)
    pub fn step(current: usize, total: usize, msg: impl std::fmt::Display) {
        println!(
            "{}[{}/{}]{} {}",
            colors::PURPLE,
            current,
            total,
            colors::RESET,
            msg
        );
    }

    /// Shutdown lifecycle notification (`[SHUTDOWN]` message)
    pub fn shutdown(msg: impl std::fmt::Display) {
        println!("{}[SHUTDOWN]{} {}", colors::AMBER, colors::RESET, msg);
    }

    /// Clean shutdown completion notification (`[SHUTDOWN]` exited cleanly)
    pub fn shutdown_clean(msg: impl std::fmt::Display) {
        println!("{}[SHUTDOWN]{} {}", colors::GREEN, colors::RESET, msg);
    }

    /// Cancellation notification (`[CANCELLED]` message)
    pub fn cancelled(msg: impl std::fmt::Display) {
        eprintln!("{}[CANCELLED]{} {}", colors::AMBER, colors::RESET, msg);
    }

    /// Section header / divider
    pub fn header(title: impl std::fmt::Display) {
        println!(
            "{BOLD}============================================================{RESET}\n {BOLD}{}{RESET}\n{BOLD}============================================================{RESET}",
            title,
            BOLD = colors::BOLD,
            RESET = colors::RESET
        );
    }

    /// Sub-header divider
    pub fn divider() {
        println!(
            "{MUTED}------------------------------------------------------------{RESET}",
            MUTED = colors::MUTED,
            RESET = colors::RESET
        );
    }

    /// Key-value display line
    pub fn key_val(key: impl std::fmt::Display, val: impl std::fmt::Display) {
        println!(" {:<15} {}", format!("{}:", key), val);
    }
}
