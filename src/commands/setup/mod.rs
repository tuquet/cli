pub mod gateway;
pub mod browser;
pub mod cloud;
pub mod core;
pub mod mcp;

use crate::cli::{SetupArgs, SetupComponent};
use crate::ui::prompt::{
    clack_confirm, clack_intro, clack_outro, clack_step_skip, is_interactive_tty,
};

pub async fn run(args: SetupArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Direct granular subcommands if specified
    if let Some(ref comp) = args.component {
        match comp {
            SetupComponent::Core => {
                core::run(true)?;
                mcp::run(false)?;
                return Ok(());
            }
            SetupComponent::Mcp => {
                mcp::run(true)?;
                return Ok(());
            }
            SetupComponent::Gateway => {
                gateway::run(true).await?;
                return Ok(());
            }
            SetupComponent::Browser => {
                browser::run(args.force).await?;
                return Ok(());
            }
            SetupComponent::Cloud => {
                cloud::run(true).await?;
                return Ok(());
            }
        }
    }

    // 2. Full or Quick Guided Onboarding Wizard
    let is_interactive = is_interactive_tty() && !args.quick && !args.yes;

    clack_intro(
        "TQ SETUP",
        if args.quick {
            "Quick Core Provisioning"
        } else {
            "Workstation Onboarding & Environment Wizard"
        },
    );

    // ─────────────────────────────────────────────────────────────────────────────
    // STEP 1: Core Essentials (SSOT Root & SQLite DB)
    // ─────────────────────────────────────────────────────────────────────────────
    core::run(true)?;

    // ─────────────────────────────────────────────────────────────────────────────
    // STEP 2: AI Agent MCP Integration
    // ─────────────────────────────────────────────────────────────────────────────
    mcp::run(is_interactive)?;

    // If quick mode requested, finish early
    if args.quick {
        clack_outro("Quick setup complete! SSOT directories and MCP server ready.");
        return Ok(());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // OPTIONAL 1: Specter Gateway (VPS Mesh & SOCKS5 Proxy)
    // ─────────────────────────────────────────────────────────────────────────────
    let want_gateway = if is_interactive {
        clack_confirm(
            "Configure Specter Gateway (VPS SOCKS5 proxy via Cloudflare)?",
            false,
        )?
        .unwrap_or(false)
    } else {
        false
    };

    if want_gateway {
        gateway::run(is_interactive).await?;
    } else {
        clack_step_skip("Specter Gateway (configure anytime via 'tq setup gateway')");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // OPTIONAL 2: Antidetect Chromium Sandbox
    // ─────────────────────────────────────────────────────────────────────────────
    let b_status = crate::core::browser::resolver::get_runtime_status();
    let want_browser = if b_status.installed {
        false
    } else if is_interactive {
        clack_confirm(
            "Download Antidetect Chromium Golden LTS (~150MB)?",
            false,
        )?
        .unwrap_or(false)
    } else {
        false
    };

    if want_browser || (b_status.installed && args.force) {
        browser::run(args.force).await?;
    } else if !b_status.installed {
        clack_step_skip("Chromium download (install anytime via 'tq browser install')");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // OPTIONAL 3: Cloud Pairing
    // ─────────────────────────────────────────────────────────────────────────────
    let want_cloud = if is_interactive {
        clack_confirm("Pair workstation with TQ Cloud (Supabase)?", false)?
            .unwrap_or(false)
    } else {
        false
    };

    if want_cloud {
        cloud::run(is_interactive).await?;
    } else {
        clack_step_skip("Cloud pairing (pair anytime via 'tq login')");
    }

    clack_outro("Workstation setup complete! Run 'tq status' or 'tq tui' to get started.");
    Ok(())
}
