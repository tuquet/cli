use super::state::TuiState;
use crate::infrastructure::gateway::BridgeSupervisor;

pub async fn toggle_bridge(state: &mut TuiState) {
    if state.gateway.socks5_online || state.gateway.http_online {
        state.status_message = "Stopping Specter Bridge tunnels...".to_string();
        let _ = BridgeSupervisor::stop_all();
        state.refresh().await;
        state.status_message = "Specter Bridge stopped.".to_string();
    } else {
        state.status_message = "Starting Specter Bridge (SOCKS5 + HTTP)...".to_string();
        let opts = crate::commands::gateway::start::BridgeStartOptions {
            server: None,
            tag: None,
            http: true,
            ssh: false,
            foreground: false,
        };
        let _ = crate::commands::gateway::start::start_bridge(opts).await;
        state.refresh().await;
        state.status_message = "Specter Bridge online.".to_string();
    }
}

pub async fn kill_browsers(state: &mut TuiState) {
    let base_dir = crate::core::browser::resolve_data_dir();
    let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);
    let count = sessions.len();
    if count == 0 {
        state.status_message = "No active browser sessions to kill.".to_string();
        return;
    }

    state.status_message = format!("Terminating {} browser session(s)...", count);
    for session in &sessions {
        let _ = crate::core::browser::pid_tracker::terminate_session(session, &base_dir, true).await;
    }
    state.refresh().await;
    state.status_message = format!("Terminated {} browser session(s).", count);
}
