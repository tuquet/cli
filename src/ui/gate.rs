use crate::core::readiness::{ReadinessState, WorkstationReadiness};
use std::error::Error;

/// Renders minimal single-line status notice without ASCII box frames or interactive menus
pub async fn handle_readiness_gate(readiness: &WorkstationReadiness) -> Result<(), Box<dyn Error>> {
    match readiness.state {
        ReadinessState::Fresh => {
            eprintln!("⚠ workstation not onboarded — run `tq setup` to initialize");
            std::process::exit(1);
        }
        ReadinessState::Degraded => {
            let problems: Vec<String> = readiness
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.component.to_lowercase(), i.title.to_lowercase()))
                .collect();
            let remedy = readiness
                .issues
                .first()
                .map(|i| i.remedy.as_str())
                .unwrap_or("run `tq doctor`");
            eprintln!("⚠ runtime out of sync ({}) — {}", problems.join("; "), remedy);
            std::process::exit(1);
        }
        ReadinessState::Pristine => Ok(()),
    }
}
