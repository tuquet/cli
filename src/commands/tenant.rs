use crate::cli::TenantSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud::{CloudApiClient, CredentialStore, TenancyManager};
use crate::ui::{
    Card, ClackOption, ClackSpinner, Column, OutputFormat, Table, badge_offline, badge_online,
    clack_cancel, clack_intro, clack_outro, clack_select, respond_with,
};

pub async fn handle(command: Option<TenantSubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Some(TenantSubcommands::List { format }) => list_tenants(format.resolve()).await,
        Some(TenantSubcommands::Switch { target, format }) => {
            switch_tenant(target.as_deref(), format.resolve()).await
        }
        Some(TenantSubcommands::Current { format }) => show_current_tenant(format.resolve()).await,
        None => {
            if !crate::ui::is_interactive_tty() {
                return show_current_tenant(OutputFormat::Json).await;
            }
            clack_intro("tq auth tenant", "Multi-Tenant Workspace Controller");
            let opts = vec![
                ClackOption::new(
                    "Current Active Workspace",
                    "Inspect the currently bound tenant & RLS scope",
                    "current",
                ),
                ClackOption::new(
                    "Switch Active Workspace",
                    "Select and bind to another accessible workspace",
                    "switch",
                ),
                ClackOption::new(
                    "List Accessible Workspaces",
                    "Display table of all organizations your account belongs to",
                    "list",
                ),
            ];
            let Some(idx) = clack_select("Select workspace action:", &opts, 0, 6)? else {
                clack_cancel("Tenant hub closed.");
                return Ok(());
            };
            match idx {
                0 => show_current_tenant(OutputFormat::Card).await,
                1 => switch_tenant(None, OutputFormat::Card).await,
                2 => list_tenants(OutputFormat::Table).await,
                _ => Ok(()),
            }
        }
    }
}

pub async fn list_tenants(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let creds = match CredentialStore::load(&config.data_dir).await {
        Some(c) => c,
        None => {
            return Err(
                "No active cloud session found on this machine. Run 'tq auth login' first.".into(),
            );
        }
    };

    let cloud_url =
        crate::infrastructure::cloud::resolve_supabase_url(None, creds.cloud_url.as_deref())?;
    let api_key =
        crate::infrastructure::cloud::resolve_supabase_anon_key(creds.api_key.as_deref())?;

    let client = CloudApiClient::build_http_client();

    let user_id = creds.user_id.clone().unwrap_or_default();
    let access_token = match creds.access_token.as_ref() {
        Some(t) => t.clone(),
        None => {
            if let Some(tid) = creds.tenant_id.as_deref() {
                let out = serde_json::json!({
                    "enrolled": true,
                    "standalone_tenant_id": tid,
                    "multi_tenant": false,
                    "message": "Workstation enrolled under standalone tenant. Log in with an operator account to manage multi-tenant workspaces."
                });
                return respond_with(format, &out, |_| {
                    clack_intro("tq auth tenant", "Standalone Workstation Enrollment");
                    crate::ui::clack_step_info(format!(
                        "Workstation enrolled under standalone tenant: \x1b[1;97m{}\x1b[0m",
                        tid
                    ));
                    clack_outro(
                        "Log in with an operator account via 'tq auth login' to manage multi-tenant workspaces.",
                    );
                });
            }
            return Err("Workstation is not logged in. Run 'tq auth login' first.".into());
        }
    };

    let sp = if !format.is_json() {
        Some(ClackSpinner::start(
            "Fetching accessible multi-tenant workspaces...",
        ))
    } else {
        None
    };

    let mut tenants =
        TenancyManager::fetch_user_tenants(&client, &cloud_url, &api_key, &access_token, &user_id)
            .await
            .unwrap_or_else(|_| creds.available_tenants.clone().unwrap_or_default());

    if let Some(s) = sp {
        s.clear();
    }

    let active_id = creds.tenant_id.as_deref().unwrap_or("");
    let active_slug = creds.tenant_slug.as_deref().unwrap_or("");

    for t in tenants.iter_mut() {
        t.is_active = t.id == active_id || t.slug.eq_ignore_ascii_case(active_slug);
    }

    respond_with(format, &tenants, |t_list| {
        if format.is_card() {
            println!();
            for t in t_list {
                let mut card = Card::new("WORKSPACE");
                if t.is_active {
                    card.with_badge(badge_online("ACTIVE"));
                } else {
                    card.with_badge(badge_offline("STANDBY"));
                }
                card.with_min_width(56);
                card.add_kv("Workspace Name", &t.name);
                card.add_kv("Slug", &t.slug);
                card.add_kv("Role", t.role.as_deref().unwrap_or("member"));
                card.add_kv("Tenant ID", &t.id);
                card.print();
                println!();
            }
            return;
        }

        let columns = vec![
            Column {
                title: "WORKSPACE".to_string(),
                min_width: 22,
                align_right: false,
            },
            Column {
                title: "SLUG".to_string(),
                min_width: 18,
                align_right: false,
            },
            Column {
                title: "ROLE".to_string(),
                min_width: 10,
                align_right: false,
            },
            Column {
                title: "STATUS".to_string(),
                min_width: 10,
                align_right: false,
            },
            Column {
                title: "ACTIVE".to_string(),
                min_width: 12,
                align_right: false,
            },
        ];
        let mut table = Table::new(columns);

        for t in t_list {
            let active_badge = if t.is_active {
                badge_online("ACTIVE")
            } else {
                badge_offline("STANDBY")
            };
            let status_str = t.status.as_deref().unwrap_or("active");
            let role_str = t.role.as_deref().unwrap_or("member");

            table.add_row(vec![
                t.name.clone(),
                t.slug.clone(),
                role_str.to_string(),
                status_str.to_string(),
                active_badge,
            ]);
        }

        clack_intro("tq auth tenant", "Accessible Multi-Tenant Workspaces");
        table.print();
        clack_outro("Switch active workspace: tq auth tenant switch [slug]");
    })
}

pub async fn switch_tenant(
    target_opt: Option<&str>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();

    let resolved_target: String = match target_opt {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            // Zero-Arg Interactive: load accessible tenants and let user select via Clack
            let creds = CredentialStore::load(&config.data_dir)
                .await
                .ok_or("No active cloud session found. Run 'tq auth login' first.")?;
            let tenants = creds.available_tenants.unwrap_or_default();
            if tenants.is_empty() {
                return Err("No multi-tenant workspaces cached. Run 'tq auth tenant' or 'tq auth login' first.".into());
            }

            clack_intro("tq auth tenant switch", "Interactive Workspace Selector");
            let active_slug = creds.tenant_slug.as_deref().unwrap_or("");
            let mut default_idx = 0usize;
            let opts: Vec<ClackOption> = tenants
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    if t.slug.eq_ignore_ascii_case(active_slug) {
                        default_idx = i;
                    }
                    let role = t.role.as_deref().unwrap_or("member");
                    let active_tag = if t.slug.eq_ignore_ascii_case(active_slug) {
                        " • active"
                    } else {
                        ""
                    };
                    ClackOption::new(
                        &t.name,
                        format!("{} (role: {}{})", t.slug, role, active_tag),
                        format!("{} ({})", t.name, t.slug),
                    )
                })
                .collect();

            match clack_select(
                "Select target workspace to activate",
                &opts,
                default_idx,
                10,
            )? {
                Some(idx) => tenants[idx].slug.clone(),
                None => {
                    clack_cancel("Workspace switch cancelled.");
                    return Ok(());
                }
            }
        }
    };

    match TenancyManager::switch_tenant(&config.data_dir, &resolved_target).await {
        Ok(tenant) => {
            let out = serde_json::json!({
                "switched": true,
                "workspace": {
                    "id": tenant.id,
                    "slug": tenant.slug,
                    "name": tenant.name,
                    "role": tenant.role,
                    "status": tenant.status
                }
            });

            respond_with(format, &out, |_| {
                let mut card = Card::new("WORKSPACE");
                card.with_badge(badge_online("SWITCHED"));
                card.with_min_width(64);
                card.add_kv("Active Workspace", &tenant.name);
                card.add_kv("Workspace Slug", &tenant.slug);
                card.add_kv("Tenant ID", &tenant.id);
                if let Some(r) = &tenant.role {
                    card.add_kv("Your Role", r);
                }
                card.with_footer("Workstation runner successfully rebound to target workspace");
                println!();
                card.print();
                println!();
            })
        }
        Err(e) => Err(format!("Failed to switch workspace: {}", e).into()),
    }
}

pub async fn show_current_tenant(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let creds = match CredentialStore::load(&config.data_dir).await {
        Some(c) => c,
        None => {
            let out = serde_json::json!({
                "authenticated": false,
                "message": "No active cloud session found on this machine. Run 'tq auth login' to authenticate."
            });
            return respond_with(format, &out, |_| {
                let mut card = Card::new("WORKSPACE");
                card.with_badge(badge_offline("NOT AUTHENTICATED"));
                card.with_min_width(64);
                card.add_line("No active cloud session found on this machine.");
                card.with_footer("Run 'tq auth login' to authenticate with Cloud");
                println!();
                card.print();
                println!();
            });
        }
    };

    let current_json = serde_json::json!({
        "active": true,
        "tenant_id": creds.tenant_id,
        "tenant_slug": creds.tenant_slug.as_deref().unwrap_or("default"),
        "tenant_name": creds.tenant_name.as_deref().unwrap_or("Personal Workspace"),
        "role": creds.tenant_role.as_deref().unwrap_or("owner"),
        "user_email": creds.email,
        "device_id": creds.device_id,
        "runner_device": creds.name
    });

    respond_with(format, &current_json, |_| {
        let mut card = Card::new("WORKSPACE");
        card.with_badge(badge_online("ACTIVE"));
        card.with_min_width(64);
        card.add_kv(
            "Workspace Name",
            creds.tenant_name.as_deref().unwrap_or("Personal Workspace"),
        );
        card.add_kv(
            "Workspace Slug",
            creds.tenant_slug.as_deref().unwrap_or("default"),
        );
        card.add_kv(
            "Tenant ID",
            creds.tenant_id.as_deref().unwrap_or("standalone"),
        );
        if let Some(role) = creds.tenant_role.as_deref() {
            card.add_kv("Assigned Role", role);
        }
        if let Some(email) = creds.email.as_deref() {
            card.add_kv("Logged In As", email);
        }
        card.add_kv("Runner Device", &creds.name);
        card.with_footer("Run 'tq auth tenant' to view all available workspaces");
        println!();
        card.print();
        println!();
    })
}
