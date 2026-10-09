pub mod logs;
pub mod supervisor;
pub mod tools;
pub mod worker;

pub use logs::show_logs;
pub use supervisor::{
    check_status, get_log_file_path, get_pid_file_path, is_valid_runner_name, restart_daemon,
    run_server, stop_daemon,
};
pub use tools::{export_openapi, manage_config, print_probe_manifest};
pub use worker::run_cloud_worker;

use crate::cli::RunnerSubcommands;

pub async fn handle(command: RunnerSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RunnerSubcommands::Start {
            host,
            port,
            detach,
            data_dir,
            log_level,
            cloud,
        } => run_server(host, port, detach, data_dir, log_level, cloud).await,
        RunnerSubcommands::Worker {
            cloud_profile,
            workflow,
            headless,
            interval,
            once,
        } => run_cloud_worker(cloud_profile, workflow, headless, interval, once).await,
        RunnerSubcommands::Stop { force } => stop_daemon(force).await,
        RunnerSubcommands::Restart { detach } => restart_daemon(detach).await,
        RunnerSubcommands::Status { url, format } => {
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var(crate::constants::ENV_AUTOMA_HOST)
                    .unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
                let port = std::env::var(crate::constants::ENV_AUTOMA_PORT)
                    .unwrap_or_else(|_| crate::constants::DEFAULT_RUNNER_PORT.to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url, format.resolve()).await
        }
        RunnerSubcommands::Logs { follow, lines } => show_logs(follow, lines).await,
        RunnerSubcommands::Probe => print_probe_manifest(),
        RunnerSubcommands::ExportOpenapi { output } => export_openapi(&output),
        RunnerSubcommands::Config { edit, show, args } => manage_config(&args, edit, show),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_runner_name() {
        assert!(is_valid_runner_name("specter"));
        assert!(is_valid_runner_name("specter.exe"));
        assert!(is_valid_runner_name("SPECTER.EXE"));
        assert!(is_valid_runner_name("runner"));
        assert!(is_valid_runner_name("runner.exe"));

        // Innocent applications must be rejected
        assert!(!is_valid_runner_name("chrome.exe"));
        assert!(!is_valid_runner_name("notepad.exe"));
        assert!(!is_valid_runner_name("svchost.exe"));
        assert!(!is_valid_runner_name("cmd.exe"));
        assert!(!is_valid_runner_name("powershell.exe"));
    }

    #[tokio::test]
    async fn test_socket2_reuseaddr_and_listener() {
        let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)
            .expect("Failed to create socket2 socket");
        socket
            .set_reuse_address(true)
            .expect("Failed to set SO_REUSEADDR");
        socket
            .set_nonblocking(true)
            .expect("Failed to set nonblocking");
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("Failed to parse addr");
        socket.bind(&addr.into()).expect("Failed to bind socket");
        socket.listen(128).expect("Failed to listen on socket");

        let std_listener: std::net::TcpListener = socket.into();
        let listener = tokio::net::TcpListener::from_std(std_listener)
            .expect("Failed to convert std TcpListener to tokio TcpListener");
        assert!(listener.local_addr().is_ok());
    }
}
