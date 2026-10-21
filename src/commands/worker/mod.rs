pub mod logs;
pub mod supervisor;
pub mod tools;
pub mod worker;

pub use logs::show_logs;
pub use supervisor::{
    check_status, get_log_file_path, get_pid_file_path, is_valid_worker_name, restart_daemon,
    run_server, stop_daemon,
};
pub use tools::{export_openapi, manage_config, print_probe_manifest};
pub use worker::run_cloud_worker;

use crate::cli::WorkerSubcommands;

pub async fn handle(command: WorkerSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        WorkerSubcommands::Start {
            host,
            port,
            detach,
            data_dir,
            log_level,
            cloud,
        } => run_server(host, port, detach, data_dir, log_level, cloud).await,
        WorkerSubcommands::Worker {
            cloud_profile,
            workflow,
            headless,
            interval,
            once,
        } => run_cloud_worker(cloud_profile, workflow, headless, interval, once).await,
        WorkerSubcommands::Stop { force } => stop_daemon(force).await,
        WorkerSubcommands::Restart { detach } => restart_daemon(detach).await,
        WorkerSubcommands::Status { url, format } => {
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var(crate::constants::ENV_AUTOMA_HOST)
                    .unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
                let port = std::env::var(crate::constants::ENV_AUTOMA_PORT)
                    .unwrap_or_else(|_| crate::constants::DEFAULT_RUNNER_PORT.to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url, format.resolve()).await
        }
        WorkerSubcommands::Logs { follow, lines } => show_logs(follow, lines).await,
        WorkerSubcommands::Probe => print_probe_manifest(),
        WorkerSubcommands::ExportOpenapi { output } => export_openapi(&output),
        WorkerSubcommands::Config { edit, show, args } => manage_config(&args, edit, show),
    }
}

pub async fn handle_interactive() -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::prompt::{ClackOption, clack_cancel, clack_intro, clack_select};

    if !crate::ui::is_interactive_tty() {
        return handle(WorkerSubcommands::Status {
            url: None,
            format: crate::ui::FormatArgs {
                json: true,
                ..Default::default()
            },
        })
        .await;
    }

    clack_intro("tq runner", "Runner Daemon Supervisor");

    let options = vec![
        ClackOption::new(
            "Runner Status",
            "Check HTTP health and active daemon status",
            "status",
        ),
        ClackOption::new(
            "Start Runner Daemon",
            "Launch background runner service on port 3030",
            "start",
        ),
        ClackOption::new(
            "Stop Runner Daemon",
            "Gracefully stop running daemon instance",
            "stop",
        ),
        ClackOption::new(
            "Restart Runner Daemon",
            "Restart background runner daemon",
            "restart",
        ),
        ClackOption::new("View Runner Logs", "Show recent daemon log output", "logs"),
        ClackOption::new(
            "Runner Configuration",
            "Configure host, port, and log level",
            "config",
        ),
    ];

    let Some(idx) = clack_select("Select Runner action:", &options, 0, 6)? else {
        clack_cancel("Runner hub closed.");
        return Ok(());
    };

    match idx {
        0 => {
            handle(WorkerSubcommands::Status {
                url: None,
                format: crate::ui::FormatArgs::default(),
            })
            .await
        }
        1 => {
            handle(WorkerSubcommands::Start {
                host: Some(crate::constants::DEFAULT_HOST.to_string()),
                port: Some(crate::constants::DEFAULT_RUNNER_PORT),
                detach: true,
                data_dir: None,
                log_level: Some("info".to_string()),
                cloud: false,
            })
            .await
        }
        2 => handle(WorkerSubcommands::Stop { force: false }).await,
        3 => handle(WorkerSubcommands::Restart { detach: true }).await,
        4 => {
            handle(WorkerSubcommands::Logs {
                follow: false,
                lines: 50,
            })
            .await
        }
        5 => manage_config(&[], false, false),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_worker_name() {
        // Valid worker binary is strictly tq / tq.exe (case-insensitive)
        assert!(is_valid_worker_name("tq"));
        assert!(is_valid_worker_name("tq.exe"));
        assert!(is_valid_worker_name("TQ.EXE"));
        assert!(is_valid_worker_name("  tq.exe  "));

        // Obsolete legacy names & non-binaries rejected per KISS & YAGNI
        assert!(!is_valid_worker_name("tuquet"));
        assert!(!is_valid_worker_name("tuquet.exe"));
        assert!(!is_valid_worker_name("spc"));
        assert!(!is_valid_worker_name("SPC.EXE"));
        assert!(!is_valid_worker_name("specter"));
        assert!(!is_valid_worker_name("SPECTER.EXE"));
        assert!(!is_valid_worker_name("runner"));
        assert!(!is_valid_worker_name("runner.exe"));

        // Innocent applications must be rejected
        assert!(!is_valid_worker_name("chrome.exe"));
        assert!(!is_valid_worker_name("notepad.exe"));
        assert!(!is_valid_worker_name("svchost.exe"));
        assert!(!is_valid_worker_name("cmd.exe"));
        assert!(!is_valid_worker_name("powershell.exe"));
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
