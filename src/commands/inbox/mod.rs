pub mod clipboard;
pub mod cloud_client;
pub mod parser;
pub mod server;
pub mod storage;
pub mod test_runner;
pub mod worker_template;

use std::path::PathBuf;
use crate::cli::InboxSubcommands;
use crate::commands::inbox::cloud_client::InboxCloudClient;
use crate::commands::inbox::server::{read_pid, remove_pid, run_server};
use crate::commands::inbox::storage::InboxStorage;
use crate::commands::inbox::worker_template::{
    generate_cloudflare_worker_script, WorkerTemplateOptions,
};
use crate::config::inbox::InboxConfig;
use crate::ui::{badge_offline, badge_online, respond_with, Card, Column, Table};

pub async fn handle(subcmd: InboxSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        InboxSubcommands::Otp {
            recipient,
            wait,
            timeout,
            raw,
            no_consume,
            copy,
            format,
        } => handle_otp(&recipient, wait, timeout, raw, no_consume, copy, format.resolve()).await?,
        InboxSubcommands::Link {
            recipient,
            link_type,
            open,
            raw,
            format,
        } => handle_link(&recipient, link_type.as_deref(), open, raw, format.resolve()).await?,
        InboxSubcommands::List { recipient, limit, format } => {
            handle_list(recipient.as_deref(), limit, format.resolve()).await?
        }
        InboxSubcommands::Start {
            port,
            host,
            foreground,
        } => handle_start(port, host, foreground).await?,
        InboxSubcommands::Stop => handle_stop().await?,
        InboxSubcommands::Status { format } => handle_status(format.resolve()).await?,
        InboxSubcommands::Test { simulate, remote, json } => {
            test_runner::run_acceptance_test(simulate, remote, json).await?
        }
        InboxSubcommands::SetupWorker { output, secret } => {
            handle_setup_worker(output.as_deref(), secret.as_deref()).await?
        }
        InboxSubcommands::Sync { limit } => handle_sync(limit).await?,
        InboxSubcommands::Config { edit, show, args } => manage_config(&args, edit, show)?,
    }
    Ok(())
}

pub async fn handle_otp(
    recipient: &str,
    wait: bool,
    timeout: u64,
    raw: bool,
    no_consume: bool,
    copy: bool,
    format: crate::ui::OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())?;
    let cloud = InboxCloudClient::new(&config);

    let mark_consumed = !no_consume;

    let otp_record = if wait {
        if !raw && format.is_card() {
            println!(
                "⏳ Waiting for OTP for '{}' (timeout: {}s)...",
                recipient, timeout
            );
        }
        cloud
            .poll_otp(&storage, recipient, timeout, mark_consumed)
            .await?
    } else {
        // Quick local lookup
        match storage.get_latest_otp(recipient, mark_consumed)? {
            Some(otp) => {
                if mark_consumed {
                    let dev_id = crate::infrastructure::cloud::MachineFingerprint::generate();
                    let _ = storage.consume_otp(&otp.id, &dev_id);
                    if cloud.is_configured() {
                        let _ = cloud.consume_otp_remote(&otp.id, &dev_id).await;
                    }
                }
                Some(otp)
            }
            None => {
                // If not in local cache, do a fast 3-second cloud check if cloud is enabled
                if cloud.is_configured() {
                    cloud.poll_otp(&storage, recipient, 3, mark_consumed).await?
                } else {
                    None
                }
            }
        }
    };

    match otp_record {
        Some(otp) => {
            let copied = if copy {
                clipboard::copy_to_clipboard(&otp.otp_code)
            } else {
                false
            };

            if raw {
                println!("{}", otp.otp_code);
            } else if format.is_card() {
                println!();
                let mut card = Card::new("SPECTER INBOX - OTP INTERCEPTOR");
                card.with_badge(badge_online("OTP RECEIVED"));
                card.add_kv("Recipient", &otp.recipient);
                card.add_kv("OTP Code", format!("🔑 {}", otp.otp_code));
                card.add_kv(
                    "Service",
                    otp.service_name.as_deref().unwrap_or("Generic / Unknown"),
                );
                card.add_kv("Received At", &otp.created_at);
                card.add_kv(
                    "Status",
                    if no_consume {
                        "Active (Unconsumed)"
                    } else {
                        "Consumed (Device Marked)"
                    },
                );
                if copy {
                    card.add_kv("Clipboard", "Copied to system clipboard (Ctrl+V)");
                }
                card.print();
                println!();
            } else {
                let out = serde_json::json!({
                    "recipient": otp.recipient,
                    "otp_code": otp.otp_code,
                    "service": otp.service_name.as_deref().unwrap_or("Generic / Unknown"),
                    "received_at": otp.created_at,
                    "consumed": !no_consume,
                    "copied": copied,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            Ok(())
        }
        None => {
            if raw {
                std::process::exit(1);
            } else {
                Err(format!(
                    "No active OTP found for '{}'. Tip: use '--wait' to hold connection until email arrives.",
                    recipient
                )
                .into())
            }
        }
    }
}

pub async fn handle_link(
    recipient: &str,
    link_type: Option<&str>,
    open: bool,
    raw: bool,
    format: crate::ui::OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())?;
    let cloud = InboxCloudClient::new(&config);

    let mut link_opt = storage.get_latest_link(recipient, link_type)?;

    // If missing and cloud configured, sync once
    if link_opt.is_none() && cloud.is_configured() {
        let _ = cloud.sync_recent(&storage, 15).await;
        link_opt = storage.get_latest_link(recipient, link_type)?;
    }

    match link_opt {
        Some(link) => {
            if open {
                println!("🌐 Opening link in browser: {}", link.url);
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &link.url])
                    .spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&link.url).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&link.url).spawn();
            }

            if raw {
                println!("{}", link.url);
            } else if format.is_card() {
                println!();
                let mut card = Card::new("SPECTER INBOX - VERIFICATION LINK");
                card.with_badge(badge_online("LINK EXTRACTED"));
                card.add_kv("Recipient", &link.recipient);
                card.add_kv("Link Type", &link.link_type);
                card.add_kv("URL", &link.url);
                card.add_kv("Received At", &link.created_at);
                card.print();
                println!();
            } else {
                let out = serde_json::json!({
                    "recipient": link.recipient,
                    "url": link.url,
                    "link_type": link.link_type,
                    "received_at": link.created_at,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            Ok(())
        }
        None => Err(format!(
            "No verification or action link found for '{}'.",
            recipient
        )
        .into()),
    }
}

pub async fn handle_list(
    recipient: Option<&str>,
    limit: usize,
    format: crate::ui::OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InboxStorage::open(InboxConfig::sqlite_path())?;
    let emails = storage.list_emails(limit, 0, recipient)?;
    let otps = storage.list_otps(recipient, limit)?;

    let payload = serde_json::json!({
        "emails": emails,
        "otps": otps
    });

    respond_with(format, &payload, |_| {
        println!();
    println!("📬 RECENT EMAILS (showing max {})", limit);
    if emails.is_empty() {
        println!("  (No emails found in local cache)");
    } else {
        let columns = vec![
            Column {
                title: "ID".to_string(),
                min_width: 8,
                align_right: false,
            },
            Column {
                title: "FROM".to_string(),
                min_width: 20,
                align_right: false,
            },
            Column {
                title: "TO".to_string(),
                min_width: 20,
                align_right: false,
            },
            Column {
                title: "SUBJECT".to_string(),
                min_width: 25,
                align_right: false,
            },
            Column {
                title: "RECEIVED AT".to_string(),
                min_width: 18,
                align_right: false,
            },
        ];
        let mut table = Table::new(columns);
        for e in &emails {
            let short_id = if e.id.len() > 8 { &e.id[..8] } else { &e.id };
            let subj = e.subject.as_deref().unwrap_or("<No Subject>");
            let short_subj = if subj.chars().count() > 30 {
                format!("{}...", subj.chars().take(27).collect::<String>())
            } else {
                subj.to_string()
            };
            table.add_row(vec![
                short_id.to_string(),
                e.sender.clone(),
                e.recipient.clone(),
                short_subj,
                e.received_at.clone(),
            ]);
        }
        table.print();
    }

    println!();
    println!("🔑 RECENT OTPs");
    if otps.is_empty() {
        println!("  (No OTPs found in local cache)");
    } else {
        let columns = vec![
            Column {
                title: "CODE".to_string(),
                min_width: 10,
                align_right: false,
            },
            Column {
                title: "RECIPIENT".to_string(),
                min_width: 22,
                align_right: false,
            },
            Column {
                title: "SERVICE".to_string(),
                min_width: 12,
                align_right: false,
            },
            Column {
                title: "STATUS".to_string(),
                min_width: 10,
                align_right: false,
            },
            Column {
                title: "CREATED AT".to_string(),
                min_width: 18,
                align_right: false,
            },
        ];
        let mut table = Table::new(columns);
        for o in &otps {
            let status = if o.consumed_at.is_some() {
                "Consumed"
            } else {
                "Active"
            };
            let svc = o.service_name.as_deref().unwrap_or("Unknown");
            table.add_row(vec![
                o.otp_code.clone(),
                o.recipient.clone(),
                svc.to_string(),
                status.to_string(),
                o.created_at.clone(),
            ]);
        }
        table.print();
    }
        println!();
    })
}

pub async fn handle_start(
    port: Option<u16>,
    host: Option<String>,
    foreground: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InboxConfig::load();
    if let Some(p) = port {
        config.port = p;
    }
    if let Some(h) = host {
        config.host = h;
    }

    if let Some(existing_pid) = read_pid() {
        println!(
            "⚠️ Specter Inbox daemon is already running (PID: {}). Use 'specter inbox stop' to stop it.",
            existing_pid
        );
        return Ok(());
    }

    if foreground {
        run_server(config).await?;
    } else {
        // Spawn detached daemon process
        let current_exe = std::env::current_exe()?;
        let mut cmd = std::process::Command::new(current_exe);
        cmd.args(["inbox", "start", "-f"]);

        if let Some(p) = port {
            cmd.args(["-p", &p.to_string()]);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x00000008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        }

        let child = cmd.spawn()?;
        println!(
            "{} Specter Inbox daemon started in background (PID: {}).",
            badge_online("STARTED"),
            child.id()
        );
        println!("  Listening on: http://{}:{}", config.host, config.port);
        println!("  Local DB:     {}", InboxConfig::sqlite_path().display());
    }

    Ok(())
}

pub async fn handle_stop() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(pid) = read_pid() {
        println!("Stopping Specter Inbox daemon (PID: {})...", pid);
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
        #[cfg(not(windows))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
        }
        remove_pid();
        println!("{} Inbox daemon stopped.", badge_offline("STOPPED"));
    } else {
        println!("Specter Inbox daemon is not currently running.");
    }
    Ok(())
}

pub async fn handle_status(format: crate::ui::OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())?;
    let stats = storage.stats().unwrap_or_default();
    let cloud = InboxCloudClient::new(&config);

    let pid = read_pid();
    let is_running = pid.is_some();

    let payload = serde_json::json!({
        "daemon": {
            "running": is_running,
            "pid": pid,
            "host": &config.host,
            "port": config.port
        },
        "stats": {
            "total_emails": stats.total_emails,
            "total_otps": stats.total_otps,
            "unconsumed_otps": stats.unconsumed_otps,
            "total_links": stats.total_links
        },
        "cloud_sync": cloud.is_configured(),
        "database_path": InboxConfig::sqlite_path().display().to_string()
    });

    respond_with(format, &payload, |_| {
        println!();
        let mut card = Card::new("SPECTER INBOX CATCH-ALL & OTP SUBSYSTEM");
        if is_running {
            card.with_badge(badge_online("DAEMON ONLINE"));
        } else {
            card.with_badge(badge_offline("DAEMON OFFLINE"));
        }

        if let Some(p) = pid {
            card.add_kv("Daemon PID", format!("{}", p));
            card.add_kv(
                "Listener URL",
                format!("http://{}:{}", config.host, config.port),
            );
        } else {
            card.add_kv("Daemon Status", "Not running (Start with 'specter inbox start')");
        }

        card.add_kv("Local SQLite DB", InboxConfig::sqlite_path().display().to_string());
        card.add_kv("Total Cached Emails", format!("{}", stats.total_emails));
        card.add_kv("Total Intercepted OTPs", format!("{}", stats.total_otps));
        card.add_kv("Active Unconsumed OTPs", format!("{}", stats.unconsumed_otps));
        card.add_kv("Extracted Links", format!("{}", stats.total_links));

        if cloud.is_configured() {
            let dev_info = cloud.device_id().map(|d| format!(" [Device: {}]", d)).unwrap_or_default();
            card.add_kv("Cloud Sync", format!("Enabled (SSOT Device Identity Connected{})", dev_info));
        } else {
            card.add_kv(
                "Cloud Sync",
                "Standalone / Local-only (Pair with 'specter login')",
            );
        }

        card.print();
        println!();
    })
}

pub async fn handle_setup_worker(
    output: Option<&str>,
    secret: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = InboxConfig::load();

    let opts = WorkerTemplateOptions {
        supabase_url: config.supabase_url.as_deref().filter(|u| !u.contains('<')),
        supabase_service_role_key: None,
        webhook_secret: secret.or(config.webhook_secret.as_deref()),
        local_webhook_url: None,
    };

    let script = generate_cloudflare_worker_script(&opts);

    if let Some(out_path) = output {
        let path = PathBuf::from(out_path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, &script)?;
        println!(
            "{} Cloudflare Worker script written to: {}",
            badge_online("GENERATED"),
            path.display()
        );
    } else {
        println!("{}", script);
    }

    Ok(())
}

pub async fn handle_sync(limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = InboxConfig::load();
    let storage = InboxStorage::open(InboxConfig::sqlite_path())?;
    let cloud = InboxCloudClient::new(&config);

    if !cloud.is_configured() {
        return Err("Supabase cloud not configured. Please set SUPABASE_URL and SUPABASE_ANON_KEY in ~/.specter/inbox/inbox.json".into());
    }

    println!("🔄 Pulling latest emails and OTPs from Supabase Cloud...");
    let count = cloud.sync_recent(&storage, limit).await?;
    println!(
        "{} Successfully synced {} emails into local cache.",
        badge_online("SYNCED"),
        count
    );
    Ok(())
}

pub fn manage_config(args: &[String], edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("inbox", args, edit, show)
}
