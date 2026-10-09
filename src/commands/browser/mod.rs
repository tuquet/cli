pub mod runtime;
pub mod extension;
pub mod launcher;
pub mod profile;
pub mod cloud;
pub mod proxy;
pub mod verify;

pub use runtime::{handle_runtime, manage_config};
pub use extension::{handle_ext, setup_extension};
pub use launcher::launch_browser;
pub use profile::handle_profile;
pub use cloud::{handle_profile_cloud, handle_profile_push, handle_profile_pull};
pub use proxy::handle_proxy;
pub use verify::verify_stealth_presentation;

use crate::cli::BrowserCommands;

pub async fn handle(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Ext { command: ext_subcmd, browser, extension_path } => {
            handle_ext(ext_subcmd, &browser, extension_path).await
        }
        BrowserCommands::Launch { profile, port, headless, url, detach, proxy, mode, no_cdp, skip_proxy_check } => {
            launch_browser(profile, port, headless, url, detach, proxy, mode, no_cdp, skip_proxy_check).await
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
