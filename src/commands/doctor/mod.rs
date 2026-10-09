pub mod bootstrap;
pub mod checks;
pub mod types;

pub use bootstrap::bootstrap;
pub use types::DiagnosticItem;

use crate::ui::{colors, respond_with, OutputFormat};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorReport {
    pub ready: bool,
    pub ssh_ready: bool,
    pub browser_ready: bool,
    pub missing_actions: Vec<String>,
    pub ssot_root: String,
}

pub async fn run(fix: bool, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    if fix {
        if !format.is_json() {
            println!();
            println!(
                "  {BOLD}{CYAN}SPECTER DOCTOR (--fix){RESET} {MUTED}— Automatic Remediation & Environment Bootstrap{RESET}",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                RESET = colors::RESET,
                MUTED = colors::MUTED,
            );
        }
        let _ = bootstrap(false).await;
        if !format.is_json() {
            println!();
        }
    }

    let mut missing_actions: Vec<String> = Vec::new();

    // 1. Tool Dependencies Table
    let (tool_table, ssh_found) = checks::check_tools(&mut missing_actions);

    // 2. Antidetect Browser Engine
    let (browser_card, browser_ok) = checks::check_browser(&mut missing_actions);

    let all_ready = ssh_found && browser_ok && missing_actions.is_empty();

    let report = DoctorReport {
        ready: all_ready,
        ssh_ready: ssh_found,
        browser_ready: browser_ok,
        missing_actions,
        ssot_root: crate::config::canonical_specter_dir().display().to_string(),
    };

    respond_with(format, &report, |r| {
        println!();
        println!(
            "  {BOLD}{CYAN}SPECTER DOCTOR{RESET} {MUTED}— Ecosystem Environment & Dependency Diagnostics{RESET}",
            BOLD = colors::BOLD,
            CYAN = colors::CYAN,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
        );
        println!();

        println!("╭─ ENVIRONMENT & SYSTEM TOOL DEPENDENCIES ──────────────────────────╮");
        tool_table.print();
        println!();

        browser_card.print();
        println!();

        // 3. Canonical Microservice Storage Pillars (~/.specter/)
        let pillar_table = checks::check_storage_pillars();
        println!("╭─ CANONICAL MICROSERVICE STORAGE PILLARS (SSOT) ───────────────────╮");
        pillar_table.print();
        println!();

        // 4. Port Availability & Daemon Probes
        let port_card = checks::check_network_topology();
        port_card.print();
        println!();

        // 5. Final Diagnostic Verdict Card
        let verdict_card = checks::check_verdict(ssh_found, browser_ok, &r.missing_actions);
        verdict_card.print();
        println!();
    })
}
