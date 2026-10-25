use crate::infrastructure::cloud::credential_store::CredentialStore;
use crate::ui::prompt::{clack_confirm, clack_step_ok, is_interactive_tty};

pub async fn run(interactive: bool) -> Result<(), Box<dyn std::error::Error>> {
    let creds = CredentialStore::load("").await;
    if let Some(c) = creds {
        clack_step_ok(format!(
            "Workstation already paired with Cloud (Device: {})",
            c.device_id
        ));
        return Ok(());
    }

    if interactive && is_interactive_tty() {
        let want_login = clack_confirm(
            "Would you like to authenticate and pair workstation with TQ Cloud now?",
            false,
        )?;
        if want_login.unwrap_or(false) {
            crate::commands::cloud::handle_interactive().await?;
        } else {
            clack_step_ok("Cloud pairing deferred. You can pair anytime using 'tq login'.");
        }
    } else {
        clack_step_ok("Cloud pairing skipped. You can pair anytime using 'tq login'.");
    }

    Ok(())
}
