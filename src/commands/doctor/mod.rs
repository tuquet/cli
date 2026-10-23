pub mod bootstrap;
pub mod checks;
pub mod types;

pub use bootstrap::bootstrap;
pub use types::DiagnosticItem;

use crate::ui::{OutputFormat, clack_intro, clack_outro, clack_step_info, respond_with};

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
            clack_intro(
                "tq doctor --fix",
                "Automationtic Remediation & Environment Bootstrap",
            );
        }
        let _ = bootstrap(false).await;
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
        ssot_root: crate::config::canonical_dir().display().to_string(),
    };

    respond_with(format, &report, |r| {
        if !fix {
            clack_intro(
                "tq doctor",
                "Ecosystem Environment & Dependency Diagnostics",
            );
        }

        clack_step_info("ENVIRONMENT & SYSTEM TOOL DEPENDENCIES");
        tool_table.print();

        browser_card.print();

        // 3. Canonical Microservice Storage Pillars (~/.tq/)
        let pillar_table = checks::check_storage_pillars();
        clack_step_info("CANONICAL MICROSERVICE STORAGE PILLARS (SSOT)");
        pillar_table.print();

        // 4. Port Availability & Daemon Probes
        let port_card = checks::check_network_topology();
        port_card.print();

        // 5. Final Diagnostic Verdict Card
        let verdict_card = checks::check_verdict(ssh_found, browser_ok, &r.missing_actions);
        verdict_card.print();

        clack_outro(if r.ready {
            "Workstation is 100% healthy and ready."
        } else {
            "Diagnostics completed with action items above."
        });
    })
}
