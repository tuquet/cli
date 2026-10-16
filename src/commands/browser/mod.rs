pub mod runtime;
pub mod extension;
pub mod launcher;
pub mod profile;
pub mod cloud;
pub mod proxy;
pub mod ps;
pub mod stop;
pub mod verify;

pub use runtime::{handle_runtime, manage_config};
pub use extension::{handle_ext, setup_extension};
pub use launcher::launch_browser;
pub use profile::handle_profile;
pub use cloud::{handle_profile_cloud, handle_profile_push, handle_profile_pull};
pub use proxy::handle_proxy;
pub use ps::handle_ps;
pub use stop::handle_stop;
pub use verify::verify_stealth_presentation;

use crate::cli::BrowserCommands;

pub async fn handle(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Ext { command: ext_subcmd, browser, extension_path } => {
            handle_ext(ext_subcmd, &browser, extension_path).await
        }
        BrowserCommands::Launch {
            target,
            url,
            cdp,
            port,
            foreground,
            headless,
            detach,
            proxy,
            mode,
            no_cdp,
            force,
            skip_proxy_check,
        } => {
            let (effective_profile, effective_url) = match (target, url) {
                (Some(t), Some(u)) => (t, Some(u)),
                (Some(t), None) => {
                    if t.starts_with("http://")
                        || t.starts_with("https://")
                        || t.starts_with("about:")
                        || t.starts_with("chrome://")
                        || t.starts_with("file://")
                    {
                        ("default".to_string(), Some(t))
                    } else {
                        (t, None)
                    }
                }
                (None, Some(u)) => ("default".to_string(), Some(u)),
                (None, None) => ("default".to_string(), None),
            };

            launch_browser(
                effective_profile,
                cdp,
                port,
                foreground,
                headless,
                effective_url,
                detach,
                proxy,
                mode,
                no_cdp,
                force,
                skip_proxy_check,
            ).await
        }
        BrowserCommands::Ps { format } => {
            handle_ps(format.resolve()).await
        }
        BrowserCommands::Stop { profile, all, force } => {
            handle_stop(profile, all, force).await
        }
        BrowserCommands::Verify { url, headless, timeout } => {
            verify_stealth_presentation(url, headless, timeout).await
        }
        BrowserCommands::Profile { command } => {
            handle_profile(command).await
        }
        cmd => runtime::handle_runtime(cmd).await,
    }
}

pub async fn handle_interactive() -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::prompt::{clack_cancel, clack_intro, clack_select, ClackOption};

    if !crate::ui::is_interactive_tty() {
        return handle(BrowserCommands::Status {
            format: crate::ui::FormatArgs {
                json: true,
                ..Default::default()
            },
        })
        .await;
    }

    clack_intro("spc browser", "Specter Antidetect Browser Hub");

    let options = vec![
        ClackOption::new(
            "Browser Engine Status",
            "Inspect active Chromium runtime & profile count",
            "status",
        ),
        ClackOption::new(
            "Manage Browser Profiles",
            "List, create, or inspect antidetect profiles",
            "profile",
        ),
        ClackOption::new(
            "Launch Stealth Browser",
            "Start Chromium with an antidetect profile",
            "launch",
        ),
        ClackOption::new(
            "Running Browser Processes",
            "View active browser instances & CDP ports",
            "ps",
        ),
        ClackOption::new(
            "Installed Chromium Runtimes",
            "List downloaded Chromium versions in ~/.specter/browser/runtimes/",
            "list",
        ),
    ];

    let Some(idx) = clack_select("Select browser operation:", &options, 0, 6)? else {
        clack_cancel("Browser hub closed.");
        return Ok(());
    };

    match idx {
        0 => {
            handle(BrowserCommands::Status {
                format: crate::ui::FormatArgs::default(),
            })
            .await
        }
        1 => handle_profile(None).await,
        2 => {
            handle(BrowserCommands::Launch {
                target: None,
                url: None,
                cdp: false,
                port: 9222,
                foreground: false,
                headless: false,
                detach: false,
                proxy: None,
                mode: "stealth".to_string(),
                no_cdp: false,
                force: false,
                skip_proxy_check: false,
            })
            .await
        }
        3 => {
            handle(BrowserCommands::Ps {
                format: crate::ui::FormatArgs::default(),
            })
            .await
        }
        4 => {
            handle(BrowserCommands::List {
                format: crate::ui::FormatArgs::default(),
            })
            .await
        }
        _ => Ok(()),
    }
}

