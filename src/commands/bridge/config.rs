use crate::infrastructure::bridge::{BridgeConfig, DiagnosticLevel};
use crate::ui::prompt::{
    clack_intro, clack_outro, clack_step_err, clack_step_info, clack_step_ok, clack_step_warn,
};

pub fn check_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = BridgeConfig::config_path();
    clack_intro("spc bridge", "Configuration Validator");
    clack_step_info(format!(
        "Checking configuration at {}...",
        config_path.display()
    ));

    let config = BridgeConfig::load()?;
    let diagnostics = config.validate();

    if diagnostics.is_empty() {
        clack_step_ok("Configuration is 100% valid! Zero conflicts or missing keys found.");
        clack_outro(&format!(
            "Registered servers: {}",
            config
                .servers
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        clack_step_warn(format!("Found {} issue(s):", diagnostics.len()));
        for d in diagnostics {
            match d.level {
                DiagnosticLevel::Error => clack_step_err(&d.message),
                DiagnosticLevel::Warning => clack_step_warn(&d.message),
                DiagnosticLevel::Info => clack_step_info(&d.message),
            }
        }
    }

    Ok(())
}

pub fn manage_config(
    args: &[String],
    edit: bool,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("bridge", args, edit, show)
}
