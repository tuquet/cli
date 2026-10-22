use crate::ui::{Card, badge_offline};

pub async fn handle_stop(
    profile_query: Option<String>,
    all: bool,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = crate::core::browser::resolve_data_dir();

    if all {
        let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);
        if sessions.is_empty() {
            println!();
            let mut card = Card::new("STOP BROWSER PROFILES");
            card.with_badge(badge_offline("IDLE"));
            card.with_min_width(68);
            card.add_line("No active browser profiles are currently running.");
            card.print();
            println!();
            return Ok(());
        }

        let mut terminated = Vec::new();
        for s in &sessions {
            if crate::core::browser::pid_tracker::terminate_session(s, &base_dir, force)
                .await
                .is_ok()
            {
                terminated.push(format!("{} (PID: {})", s.profile_name, s.pid));
            }
        }

        for t in &terminated {
            println!("Stopped browser {}.", t);
        }
        return Ok(());
    }

    let target = match profile_query {
        Some(t) => t,
        None => {
            let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);
            if sessions.is_empty() {
                crate::ui::Notify::info("No browser profiles are currently running.");
                return Ok(());
            }
            eprintln!(
                "Specify a profile to stop: tq browser stop <profile_name|profile_id>, or stop all with '--all'."
            );
            eprintln!(
                "Active profiles: {}",
                sessions
                    .iter()
                    .map(|s| s.profile_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return Ok(());
        }
    };

    if let Some(session) =
        crate::core::browser::pid_tracker::find_active_session(&target, &base_dir)
    {
        let pid = session.pid;
                let p_id = session.profile_id.clone();

        crate::core::browser::pid_tracker::terminate_session(&session, &base_dir, force).await?;

        println!("Stopped browser [{}] (pid: {}).", p_id, pid);
    } else {
        crate::ui::Notify::error(format!(
            "No running browser profile found matching '{}'.",
            target
        ));
        eprintln!("Run 'tq browser ps' to view active profiles.\n");
        return Err(format!("Running profile '{}' not found", target).into());
    }

    Ok(())
}
