use std::io::IsTerminal;
use crate::cli::CloudSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud::{
    CloudApiClient, CredentialStore, SupabaseAuthClient,
    TenancyManager, TenantInfo, DEFAULT_SUPABASE_ANON_KEY,
};
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{
    badge_offline, badge_online, prompt_input, prompt_password, prompt_select, respond_with,
    Card, OutputFormat,
};

pub async fn handle(command: CloudSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CloudSubcommands::Login {
            url,
            email,
            password,
            otp,
            code,
            token,
            tenant,
            name,
            api_key,
            format,
        } => login(url, email, password, otp, code, token, tenant, name, api_key, format.resolve()).await.map_err(|e| e as Box<dyn std::error::Error>),
        CloudSubcommands::Logout => logout().await,
        CloudSubcommands::Whoami { format } => whoami(format.resolve()).await,
        CloudSubcommands::Tenant { command } => crate::commands::tenant::handle(command).await,
        CloudSubcommands::Config { edit, show, args } => manage_config(&args, edit, show),
    }
}

pub async fn login(
    url: Option<String>,
    email: Option<String>,
    password: Option<String>,
    otp: bool,
    code: Option<String>,
    token: Option<String>,
    tenant: Option<String>,
    name: Option<String>,
    api_key: Option<String>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = AppConfig::load();
    let cloud_url = url
        .or(config.cloud_url)
        .unwrap_or_else(|| "https://dswhacsoaxgpfnkaxnhz.supabase.co".to_string());
    
    let active_anon_key = api_key
        .or_else(|| std::env::var("SPECTER_API_KEY").ok())
        .or_else(|| std::env::var("SUPABASE_ANON_KEY").ok())
        .unwrap_or_else(|| DEFAULT_SUPABASE_ANON_KEY.to_string());

    let client = CloudApiClient::build_http_client();
    let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

    // 1. Interactive login method resolution if no explicit auth params given
    let (mut auth_email, mut auth_password, mut is_otp, auth_code, mut auth_token) = (
        email, password, otp, code, token
    );

    if auth_email.is_none() && auth_token.is_none() && !is_otp && is_tty {
        println!();
        println!("  \x1b[1;36mSPECTER CLOUD\x1b[0m • Authentication & Fleet Pairing");
        println!("  Endpoint: \x1b[90m{}\x1b[0m", cloud_url);

        let choices = [
            "Email & Password (Standard Operator Login)",
            "One-Time Passcode / OTP (Email Verification Code)",
            "Access Token / JWT (Personal Access Token)",
            "Anonymous Device Enrollment (Runner Token)",
        ];
        let choice = prompt_select("Choose authentication method", &choices, 0)
            .map_err(|e| format!("Failed to read user choice: {}", e))?;

        match choice {
            0 => {
                let e = prompt_input("Operator Email", None)
                    .map_err(|e| format!("Failed to read email: {}", e))?;
                let p = prompt_password("Operator Password")
                    .map_err(|e| format!("Failed to read password: {}", e))?;
                auth_email = Some(e);
                auth_password = Some(p);
            }
            1 => {
                let e = prompt_input("Operator Email", None)
                    .map_err(|e| format!("Failed to read email: {}", e))?;
                auth_email = Some(e);
                is_otp = true;
            }
            2 => {
                let t = prompt_input("Supabase Access Token (JWT)", None)
                    .map_err(|e| format!("Failed to read token: {}", e))?;
                auth_token = Some(t);
            }
            3 => {
                // Anonymous device enrollment fallback
            }
            _ => {}
        }
    }

    // 2. Perform Supabase GoTrue Authentication if credentials provided
    let mut maybe_session = None;
    let mut maybe_user_id = None;
    let mut maybe_user_email = None;

    if let Some(tok) = auth_token.as_ref() {
        if tok.contains('.') {
            // Validate JWT with Supabase Auth
            match SupabaseAuthClient::validate_token(&client, &cloud_url, &active_anon_key, tok).await {
                Ok(user) => {
                    maybe_user_id = Some(user.id);
                    maybe_user_email = user.email;
                    maybe_session = Some((tok.clone(), None, None));
                }
                Err(e) => {
                    // If validation fails and not a JWT, treat as enrollment token
                    if tok.len() < 40 {
                        // Likely an enrollment token
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    } else if is_otp || auth_code.is_some() {
        let email_str = match auth_email.as_deref() {
            Some(e) if !e.trim().is_empty() => e.trim().to_string(),
            _ => {
                if is_tty {
                    prompt_input("Operator Email for OTP", None)
                        .map_err(|e| format!("Email is required for OTP login: {}", e))?
                } else {
                    return Err("Email is required for OTP login (use --email <email>)".into());
                }
            }
        };

        let otp_code = match auth_code.as_deref() {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => {
                println!();
                println!("  Requesting One-Time Passcode (OTP) from Supabase...");
                SupabaseAuthClient::request_otp(&client, &cloud_url, &active_anon_key, &email_str).await?;
                println!("  \x1b[32m[OK]\x1b[0m Verification code sent to: \x1b[1m{}\x1b[0m", email_str);
                
                prompt_input("Enter 6-digit Verification Code", None)
                    .map_err(|e| format!("Failed to read OTP code: {}", e))?
            }
        };

        let sess = SupabaseAuthClient::verify_otp(&client, &cloud_url, &active_anon_key, &email_str, &otp_code).await?;
        maybe_user_id = Some(sess.user.id.clone());
        maybe_user_email = sess.user.email.clone().or(Some(email_str));
        maybe_session = Some((sess.access_token, sess.refresh_token, sess.expires_at));
    } else if let Some(email_str) = auth_email.as_deref() {
        let pass_str = match auth_password.as_deref() {
            Some(p) => p.to_string(),
            None => {
                if is_tty {
                    prompt_password("Operator Password")
                        .map_err(|e| format!("Password required for login: {}", e))?
                } else {
                    return Err("Password required for login (use --password <password>)".into());
                }
            }
        };

        let sess = SupabaseAuthClient::login_with_password(&client, &cloud_url, &active_anon_key, email_str, &pass_str).await?;
        maybe_user_id = Some(sess.user.id.clone());
        maybe_user_email = sess.user.email.clone().or_else(|| Some(email_str.to_string()));
        maybe_session = Some((sess.access_token, sess.refresh_token, sess.expires_at));
    }

    // 3. Multi-Tenant Workspace Resolution
    let mut resolved_tenant_id: Option<String> = None;
    let mut resolved_tenant_slug: Option<String> = None;
    let mut resolved_tenant_name: Option<String> = None;
    let mut resolved_tenant_role: Option<String> = None;
    let mut available_tenants: Option<Vec<TenantInfo>> = None;

    if let (Some(u_id), Some((access_tok, _, _))) = (&maybe_user_id, &maybe_session) {
        println!();
        println!("  Discovering accessible workspaces for operator: \x1b[1;32m{}\x1b[0m", maybe_user_email.as_deref().unwrap_or(u_id));

        match TenancyManager::fetch_user_tenants(&client, &cloud_url, &active_anon_key, access_tok, u_id).await {
            Ok(mut tenants) => {
                if !tenants.is_empty() {
                    let active_tenant = if let Some(target) = tenant.as_deref() {
                        TenancyManager::resolve_active_tenant(&mut tenants, Some(target), None)?
                    } else if tenants.len() == 1 {
                        &mut tenants[0]
                    } else if is_tty {
                        let names: Vec<String> = tenants.iter().map(|t| {
                            let role_str = t.role.as_deref().unwrap_or("member");
                            format!("{} ({}) [Role: {}]", t.name, t.slug, role_str)
                        }).collect();
                        let names_ref: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

                        let selected_idx = prompt_select("Select active workspace for this workstation", &names_ref, 0)
                            .unwrap_or(0);
                        &mut tenants[selected_idx]
                    } else {
                        &mut tenants[0]
                    };

                    active_tenant.is_active = true;
                    resolved_tenant_id = Some(active_tenant.id.clone());
                    resolved_tenant_slug = Some(active_tenant.slug.clone());
                    resolved_tenant_name = Some(active_tenant.name.clone());
                    resolved_tenant_role = active_tenant.role.clone();
                    available_tenants = Some(tenants);
                }
            }
            Err(e) => {
                println!("  \x1b[33m[!]\x1b[0m Could not introspect tenants: {}. Continuing with default workspace.", e);
            }
        }
    }

    // 4. Enroll Workstation Device with Specter Cloud Fleet
    let enrollment_token = resolved_tenant_slug.as_deref()
        .or(resolved_tenant_id.as_deref())
        .or(tenant.as_deref())
        .or(auth_token.as_deref())
        .or(config.cloud_enrollment_token.as_deref());

    let bearer_jwt = maybe_session.as_ref().map(|(tok, _, _)| tok.as_str());

    let mut device_creds = match CloudReporter::login(
        &cloud_url,
        enrollment_token,
        name.as_deref(),
        &config.data_dir,
        bearer_jwt,
    ).await {
        Ok(creds) => creds,
        Err(e) => return Err(format!("Device pairing failed: {}", e).into()),
    };

    // 5. Update and persist unified session in ~/.specter/system/.identity.json
    device_creds.api_key = Some(active_anon_key.clone());
    device_creds.user_id = maybe_user_id.clone();
    device_creds.email = maybe_user_email.clone();
    if let Some((access_tok, refresh_tok, exp_at)) = maybe_session {
        device_creds.access_token = Some(access_tok);
        device_creds.refresh_token = refresh_tok;
        device_creds.token_expires_at = exp_at;
    }
    if resolved_tenant_id.is_some() {
        device_creds.tenant_id = resolved_tenant_id;
    }
    device_creds.tenant_slug = resolved_tenant_slug;
    device_creds.tenant_name = resolved_tenant_name;
    device_creds.tenant_role = resolved_tenant_role;
    device_creds.available_tenants = available_tenants;

    CredentialStore::save(&config.data_dir, &device_creds).await?;

    let is_authenticated = device_creds.has_user_session();
    let badge_status = if is_authenticated { "AUTHENTICATED & PAIRED" } else { "ENROLLED" };

    // 6. Delegate presentation rendering (SRP)
    present_login_success(&device_creds, &cloud_url, is_authenticated, badge_status, format)
}

pub async fn logout() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    match CloudReporter::logout(&config.data_dir).await {
        Ok(logged_out) => {
            present_logout(logged_out);
            Ok(())
        }
        Err(e) => Err(format!("Logout failed: {}", e).into()),
    }
}

pub async fn whoami(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let maybe_creds = CloudReporter::whoami(&config.data_dir).await;
    present_whoami(maybe_creds, format)
}

// ----------------------------------------------------------------------------
// Presenters: Dedicated UI & Output Rendering Functions (Single Responsibility)
// ----------------------------------------------------------------------------

fn present_login_success(
    device_creds: &crate::infrastructure::cloud::DeviceCredentials,
    cloud_url: &str,
    is_authenticated: bool,
    badge_status: &str,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload = serde_json::json!({
        "success": true,
        "authenticated": is_authenticated,
        "device_id": device_creds.device_id,
        "device_name": device_creds.name,
        "operator_email": device_creds.email,
        "operator_id": device_creds.user_id,
        "active_workspace": {
            "id": device_creds.tenant_id,
            "name": device_creds.tenant_name.as_deref().unwrap_or("Personal Workspace"),
            "slug": device_creds.tenant_slug.as_deref().unwrap_or("default"),
            "role": device_creds.tenant_role.as_deref().unwrap_or("owner")
        },
        "endpoint": device_creds.cloud_url.as_deref().unwrap_or(cloud_url)
    });

    respond_with(format, &payload, |_| {
        let mut card = Card::new("SPECTER CLOUD");
        card.with_badge(badge_online(badge_status));
        card.with_min_width(68);

        if let Some(email_val) = device_creds.email.as_deref() {
            card.add_kv("Operator Account", email_val);
        }
        if let Some(user_val) = device_creds.user_id.as_deref() {
            card.add_kv("Operator ID", user_val);
        }
        card.add_kv("Active Workspace", device_creds.tenant_name.as_deref().unwrap_or("Personal Workspace"));
        if let Some(slug_val) = device_creds.tenant_slug.as_deref() {
            card.add_kv("Workspace Slug", slug_val);
        }
        if let Some(role_val) = device_creds.tenant_role.as_deref() {
            card.add_kv("Workspace Role", role_val);
        }
        card.add_kv("Device ID", &device_creds.device_id);
        card.add_kv("Device Name", &device_creds.name);
        card.add_kv("Endpoint", device_creds.cloud_url.as_deref().unwrap_or(cloud_url));
        card.with_footer("Workstation successfully paired with Specter Cloud fleet");

        println!();
        card.print();
        println!();
    })
    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
}

fn present_logout(logged_out: bool) {
    if logged_out {
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_offline("LOGGED OUT"));
        card.with_min_width(64);
        card.add_line("Removed local cloud pairing credentials and session identity.");
        card.with_footer("Run 'specter login' to authenticate with Specter Cloud");
        println!();
        card.print();
        println!();
    } else {
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_offline("DISCONNECTED"));
        card.with_min_width(64);
        card.add_line("No active cloud session found on this machine.");
        println!();
        card.print();
        println!();
    }
}

fn present_whoami(
    maybe_creds: Option<crate::infrastructure::cloud::DeviceCredentials>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(creds) = maybe_creds {
        let payload = serde_json::json!({
            "enrolled": true,
            "authenticated": creds.has_user_session(),
            "device_id": creds.device_id,
            "device_name": creds.name,
            "operator_email": creds.email,
            "operator_id": creds.user_id,
            "tenant_id": creds.tenant_id.as_deref().unwrap_or("Personal Workspace"),
            "tenant_name": creds.tenant_name,
            "tenant_slug": creds.tenant_slug,
            "tenant_role": creds.tenant_role,
            "endpoint": creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co"),
            "token_expires_at": creds.token_expires_at,
            "enrolled_at": creds.registered_at
        });

        respond_with(format, &payload, |_| {
            let is_prod = creds.cloud_url.as_deref().map(|u| u.contains("dswhacsoaxgpfnkaxnhz") || u.contains("supabase")).unwrap_or(false);
            let badge_text = if creds.has_user_session() {
                if is_prod { "AUTHENTICATED (PROD)" } else { "AUTHENTICATED" }
            } else if is_prod {
                "ENROLLED (PROD)"
            } else {
                "ENROLLED"
            };

            let mut card = Card::new("CLOUD IDENTITY");
            card.with_badge(badge_online(badge_text));
            card.with_min_width(68);

            if let Some(email) = creds.email.as_deref() {
                card.add_kv("Operator Account", email);
            }
            if let Some(user_id) = creds.user_id.as_deref() {
                card.add_kv("Operator ID", user_id);
            }
            if let Some(tenant_name) = creds.tenant_name.as_deref() {
                card.add_kv("Active Workspace", tenant_name);
            }
            if let Some(slug) = creds.tenant_slug.as_deref() {
                card.add_kv("Workspace Slug", slug);
            }
            if let Some(role) = creds.tenant_role.as_deref() {
                card.add_kv("Workspace Role", role);
            }
            card.add_kv("Device ID", &creds.device_id);
            card.add_kv("Device Name", &creds.name);
            card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal Workspace"));
            card.add_kv("Endpoint", creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co"));

            if let Some(exp) = creds.token_expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let remaining_secs = exp - now;
                if remaining_secs > 0 {
                    let mins = remaining_secs / 60;
                    card.add_kv("Session Status", format!("Valid ({}m remaining)", mins));
                } else {
                    card.add_kv("Session Status", "Expired (auto-refreshed on demand)");
                }
            }

            card.with_footer("Run 'specter tenant list' to view available multi-tenant workspaces");
            println!();
            card.print();
            println!();
        })
    } else {
        let payload = serde_json::json!({
            "enrolled": false,
            "authenticated": false,
            "message": "Workstation not enrolled with cloud fleet. Run 'specter login' to authenticate."
        });

        respond_with(format, &payload, |_| {
            let mut card = Card::new("CLOUD IDENTITY");
            card.with_badge(badge_offline("DISCONNECTED"));
            card.with_min_width(64);
            card.add_line("Workstation not enrolled with cloud fleet.");
            card.with_footer("Run 'specter login' to authenticate with Specter Cloud");
            println!();
            card.print();
            println!();
        })
    }
}

pub fn manage_config(args: &[String], edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("system", args, edit, show)
        .map_err(|e| format!("{}", e).into())
}
