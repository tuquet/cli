
use automa_core::api;
use automa_core::AppState;
use automa_core::cli::{
    AutomaSubcommands, BrowserCommands, Cli, CloudSubcommands, Commands, RunnerSubcommands, WorkflowCommands,
};
use automa_core::config::AppConfig;
use automa_core::infrastructure::db::AutomaDb;
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.probe || matches!(cli.command, Some(Commands::Probe)) {
        return print_probe_manifest();
    }

    if let Some(ref path_opt) = cli.export_openapi {
        let output_path = path_opt.clone().unwrap_or_else(|| std::path::PathBuf::from("openapi.json"));
        return export_openapi(&output_path);
    }

    match cli.command {
        // =========================================================
        // 1. Service: Automa (Browser Automation)
        // =========================================================
        Some(Commands::Automa { command }) => match command {
            AutomaSubcommands::Run {
                workflow_pos,
                workflow,
                workflow_json,
                headless,
                browser,
                browser_id,
                variables,
                timeout,
            } => {
                let target_workflow = workflow.or(workflow_pos);
                run_workflow(target_workflow, workflow_json, headless, browser, browser_id, variables, timeout).await
            }
            AutomaSubcommands::Workflow { command } => match command {
                WorkflowCommands::List { search, db_only, vault_only } => {
                    list_workflows(search, db_only, vault_only).await
                }
                WorkflowCommands::Import { file, id, name, description } => {
                    import_workflow(file, id, name, description).await
                }
                WorkflowCommands::Export { id, output } => {
                    export_workflow(id, output).await
                }
                WorkflowCommands::Info { id } => {
                    inspect_workflow(&id)
                }
                WorkflowCommands::Delete { id, vault } => {
                    delete_workflow(id, vault).await
                }
            },
            AutomaSubcommands::Inspect { workflow } => {
                inspect_workflow(&workflow)
            }
            AutomaSubcommands::Studio => {
                let config = AppConfig::load();
                let port = config.server_port;
                let url = std::env::var("AUTOMA_STUDIO_URL").unwrap_or_else(|_| {
                    format!("https://automa-studio.vercel.app?port={}", port)
                });
                println!("Opening Automa Web Studio at: {}", url);
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd").args(["/C", "start", &url]).spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&url).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                Ok(())
            }
            AutomaSubcommands::Probe => {
                print_probe_manifest()
            }
        },

        // =========================================================
        // 2. Service: Runner (Daemon & Cloud Node)
        // =========================================================
        Some(Commands::Runner { command }) => match command {
            RunnerSubcommands::Start { host, port, data_dir, log_level } => {
                run_server(host, port, data_dir, log_level).await
            }
            RunnerSubcommands::Status { url } => {
                let target_url = url.unwrap_or_else(|| {
                    let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| "8765".to_string());
                    format!("http://{}:{}", host, port)
                });
                check_status(&target_url).await
            }
            RunnerSubcommands::Probe => {
                print_probe_manifest()
            }
            RunnerSubcommands::ExportOpenapi { output } => {
                export_openapi(&output)
            }
            RunnerSubcommands::SetupExt { browser, extension_path } => {
                setup_extension(&browser, extension_path).await
            }
        },

        // =========================================================
        // 3. Service: Cloud (Multi-Tenant Auth & Pairing)
        // =========================================================
        Some(Commands::Cloud { command }) => match command {
            CloudSubcommands::Login { url, token, name } => {
                handle_cloud_login(url, token, name).await
            }
            CloudSubcommands::Logout => {
                handle_cloud_logout().await
            }
            CloudSubcommands::Whoami => {
                handle_cloud_whoami().await
            }
        },

        // =========================================================
        // 4. Service: Browser (Chromium Runtime Management)
        // =========================================================
        Some(Commands::Browser { command }) => {
            handle_browser_command(command).await
        }

        // =========================================================
        // Backward-Compatibility Root Shortcuts
        // =========================================================
        Some(Commands::Run {
            workflow_pos,
            workflow,
            workflow_json,
            headless,
            browser,
            browser_id,
            variables,
            timeout,
        }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet run' is now organized under 'tuquet automa run'\x1b[0m");
            let target_workflow = workflow.or(workflow_pos);
            run_workflow(target_workflow, workflow_json, headless, browser, browser_id, variables, timeout).await
        }
        Some(Commands::Workflow { command }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet workflow' is now organized under 'tuquet automa workflow'\x1b[0m");
            match command {
                WorkflowCommands::List { search, db_only, vault_only } => {
                    list_workflows(search, db_only, vault_only).await
                }
                WorkflowCommands::Import { file, id, name, description } => {
                    import_workflow(file, id, name, description).await
                }
                WorkflowCommands::Export { id, output } => {
                    export_workflow(id, output).await
                }
                WorkflowCommands::Info { id } => {
                    inspect_workflow(&id)
                }
                WorkflowCommands::Delete { id, vault } => {
                    delete_workflow(id, vault).await
                }
            }
        }
        Some(Commands::Inspect { workflow }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet inspect' is now organized under 'tuquet automa inspect'\x1b[0m");
            inspect_workflow(&workflow)
        }
        Some(Commands::Server { host, port, data_dir, log_level }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet server' is now organized under 'tuquet runner start'\x1b[0m");
            run_server(host, port, data_dir, log_level).await
        }
        Some(Commands::SetupExt { browser, extension_path }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet setup-ext' is now organized under 'tuquet runner setup-ext'\x1b[0m");
            setup_extension(&browser, extension_path).await
        }
        Some(Commands::ExportOpenapi { output }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet export-openapi' is now organized under 'tuquet runner export-openapi'\x1b[0m");
            export_openapi(&output)
        }
        Some(Commands::Status { url }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet status' is now organized under 'tuquet runner status'\x1b[0m");
            let target_url = url.unwrap_or_else(|| {
                let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| "8765".to_string());
                format!("http://{}:{}", host, port)
            });
            check_status(&target_url).await
        }
        Some(Commands::Login { url, token, name }) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet login' is now organized under 'tuquet cloud login'\x1b[0m");
            handle_cloud_login(url, token, name).await
        }
        Some(Commands::Logout) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet logout' is now organized under 'tuquet cloud logout'\x1b[0m");
            handle_cloud_logout().await
        }
        Some(Commands::Whoami) => {
            eprintln!("\x1b[33m💡 Hint: 'tuquet whoami' is now organized under 'tuquet cloud whoami'\x1b[0m");
            handle_cloud_whoami().await
        }
        Some(Commands::Probe) => {
            print_probe_manifest()
        }
        None => {
            run_server(None, None, None, None).await
        }
    }
}

async fn handle_cloud_login(url: Option<String>, token: Option<String>, name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let cloud_url = url.or(config.cloud_url).unwrap_or_else(|| "https://cloud.tuquet.com".to_string());
    let enrollment_token = token.as_deref().or(config.cloud_enrollment_token.as_deref());
    match automa_core::infrastructure::cloud_reporter::CloudReporter::login(&cloud_url, enrollment_token, name.as_deref(), &config.data_dir).await {
        Ok(creds) => {
            println!("\x1b[32m[SUCCESS] Workstation enrolled successfully!\x1b[0m");
            println!("Device ID:   {}", creds.device_id);
            println!("Device Name: {}", creds.name);
            println!("Tenant ID:   {}", creds.tenant_id.as_deref().unwrap_or("none"));
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERROR] Enrollment failed: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}

async fn handle_cloud_logout() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    match automa_core::infrastructure::cloud_reporter::CloudReporter::logout(&config.data_dir).await {
        Ok(true) => {
            println!("\x1b[32m[SUCCESS] Logged out and removed local cloud credentials.\x1b[0m");
            Ok(())
        }
        Ok(false) => {
            println!("No active cloud session found.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERROR] Logout failed: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}

async fn handle_cloud_whoami() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    if let Some(creds) = automa_core::infrastructure::cloud_reporter::CloudReporter::whoami(&config.data_dir).await {
        println!("Device ID:   {}", creds.device_id);
        println!("Device Name: {}", creds.name);
        println!("Tenant ID:   {}", creds.tenant_id.as_deref().unwrap_or("none"));
        println!("Cloud URL:   {}", creds.cloud_url.as_deref().unwrap_or("none"));
    } else {
        println!("Not logged in to Tuquet Cloud.");
    }
    Ok(())
}

async fn handle_browser_command(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Install { force, revision } => {
            match automa_core::core::browser::resolver::download_chromium_runtime(force, revision.as_deref()).await {
                Ok(path) => {
                    println!("\x1b[32m[SUCCESS] Dedicated Open-Source Chromium runtime ready at: {}\x1b[0m", path);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR] Failed to install Chromium runtime: {}\x1b[0m", e);
                    std::process::exit(1);
                }
            }
        }
        BrowserCommands::Status => {
            let status = automa_core::core::browser::resolver::get_runtime_status();
            println!("============================================================");
            println!(" Tuquet Ecosystem - Open-Source Chromium Runtime Status");
            println!("============================================================");
            println!(" Engine:          Chromium (Pure Open Source - BSD 3-Clause)");
            println!(" Platform:        {}", status.platform);
            println!(" Status:          {}", if status.installed { "\x1b[32mINSTALLED\x1b[0m" } else { "\x1b[33mNOT INSTALLED\x1b[0m" });
            println!(" Version/Rev:     {}", status.pinned_version);
            println!(" Executable Path: {}", status.executable_path);
            println!(" Directory:       {}", status.directory);
            if let Some(mb) = status.size_mb {
                println!(" Disk Usage:      {:.1} MB", mb);
            }
            println!("============================================================");
            if !status.installed {
                println!("👉 Run 'tuquet browser install' to download and setup.");
            }
            Ok(())
        }
        BrowserCommands::Clean => {
            match automa_core::core::browser::resolver::clean_runtime() {
                Ok(_) => {
                    println!("\x1b[32m[SUCCESS] Cleaned browser runtime directory.\x1b[0m");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR] Failed to clean runtime: {}\x1b[0m", e);
                    std::process::exit(1);
                }
            }
        }
        BrowserCommands::Path => {
            match automa_core::core::browser::resolver::resolve_executable_path("default").await {
                Ok(path) => {
                    println!("{}", path);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR] {}\x1b[0m", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn print_probe_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = serde_json::json!({
        "protocol": "tuquet.automa.v1",
        "name": "automa-runner",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "chromium-extension-worker",
        "status": "ready",
        "capabilities": [
            "browser:chromium",
            "mv3_extension_worker",
            "isolation:profile_sandbox",
            "headless",
            "automation:workflow_graph"
        ],
        "plugin_type": "runner_driver"
    });
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

fn export_openapi(output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use utoipa::OpenApi;
    let openapi = api::routes::ApiDoc::openapi();
    let json = openapi.to_pretty_json()?;
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(output_path, json)?;
    println!("OpenAPI spec successfully exported to {:?}", output_path);
    Ok(())
}

async fn check_status(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let target = format!("{}/api/v1/health", url.trim_end_matches('/'));
    println!("Querying Automa Core daemon status at {} ...", target);
    match reqwest::get(&target).await {
        Ok(res) if res.status().is_success() => {
            println!("Status: ONLINE [HTTP 200]");
            let text = res.text().await?;
            println!("Response: {}", text);
        }
        Ok(res) => {
            println!("Status: ERROR [HTTP {}]", res.status());
        }
        Err(e) => {
            println!("Status: OFFLINE or UNREACHABLE ({})", e);
        }
    }
    Ok(())
}

async fn setup_extension(browser: &str, extension_path: Option<std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let ext_dir = extension_path.unwrap_or_else(|| {
        std::path::PathBuf::from(automa_core::core::browser::worker_coordinator::resolve_cli_runner_extension_path())
    });

    println!("============================================================");
    println!(" Automa Web Extension Setup Utility");
    println!("============================================================");
    println!("Target Browser: {}", browser);
    println!("Extension Path: {}", ext_dir.display());

    if !ext_dir.exists() {
        println!("Status: Extension path does not exist yet.");
        println!("Hint: Build extension first with: pnpm --filter @automa/runner build");
    } else {
        println!("Status: Extension directory verified.");
        println!("To launch Chrome manually with extension loaded:");
        println!("  chrome.exe --load-extension=\"{}\"", ext_dir.display());
    }
    Ok(())
}

async fn list_workflows(
    search: Option<String>,
    db_only: bool,
    vault_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    struct WfInfo {
        id: String,
        name: String,
        version: String,
        source: String,
        blocks: usize,
        updated_at: String,
    }

    let config = AppConfig::load();
    let mut workflows: Vec<WfInfo> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. Fetch from SQLite Database if not vault_only
    if !vault_only {
        let db_path = std::path::PathBuf::from(&config.data_dir).join("automa.sqlite");
        if db_path.exists() {
            if let Ok(db) = AutomaDb::new(&db_path) {
                if let Ok(list) = db.workflows().get_workflows(None, None, search.as_deref()) {
                    for wf in list {
                        let parsed: Option<serde_json::Value> = serde_json::from_str(&wf.data).ok();
                        let blocks = parsed.as_ref().map(|v| {
                            if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
                                nodes.len()
                            } else if let Some(drawflow) = v.get("drawflow") {
                                if let Some(s) = drawflow.as_str() {
                                    serde_json::from_str::<serde_json::Value>(s).ok()
                                        .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).map(|a| a.len()))
                                        .unwrap_or(0)
                                } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
                                    nodes.len()
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        }).unwrap_or(0);

                        seen_ids.insert(wf.id.clone());
                        workflows.push(WfInfo {
                            id: wf.id,
                            name: wf.name,
                            version: wf.version,
                            source: "Database".to_string(),
                            blocks,
                            updated_at: wf.updated_at,
                        });
                    }
                }
            }
        }
    }

    // 2. Fetch from Vault if not db_only
    if !db_only {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = std::path::PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = std::path::PathBuf::from(&config.data_dir).join("workflows");

        let vault_dirs = [user_vault, config_vault];
        for vdir in &vault_dirs {
            if let Ok(mut entries) = tokio::fs::read_dir(vdir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() && (path.extension().map(|e| e == "json").unwrap_or(false)) {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wf");
                                let id = val.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                                    .unwrap_or_else(|| stem.trim_end_matches(".workflow").to_string());
                                let name = val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
                                    .unwrap_or_else(|| id.clone());
                                let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();
                                let desc = val.get("description").and_then(|v| v.as_str()).unwrap_or("");

                                if let Some(ref q) = search {
                                    let q_lower = q.to_lowercase();
                                    if !id.to_lowercase().contains(&q_lower)
                                        && !name.to_lowercase().contains(&q_lower)
                                        && !desc.to_lowercase().contains(&q_lower) {
                                        continue;
                                    }
                                }

                                if seen_ids.contains(&id) {
                                    if let Some(item) = workflows.iter_mut().find(|w| w.id == id) {
                                        item.source = "DB+Vault".to_string();
                                    }
                                    continue;
                                }

                                seen_ids.insert(id.clone());

                                let blocks = if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
                                    nodes.len()
                                } else if let Some(drawflow) = val.get("drawflow") {
                                    if let Some(s) = drawflow.as_str() {
                                        serde_json::from_str::<serde_json::Value>(s).ok()
                                            .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).map(|a| a.len()))
                                            .unwrap_or(0)
                                    } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
                                        nodes.len()
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };

                                let updated_at = "Vault File".to_string();

                                workflows.push(WfInfo {
                                    id,
                                    name,
                                    version,
                                    source: "Vault".to_string(),
                                    blocks,
                                    updated_at,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    println!("========================================================================================");
    println!(" 📂 AUTOMA SAVED WORKFLOWS ({} Workflows Found)", workflows.len());
    println!("========================================================================================");
    if workflows.is_empty() {
        println!(" (No workflows found)");
        println!("----------------------------------------------------------------------------------------");
        println!("💡 Import a workflow with:");
        println!("   tuquet workflow import <file.json> --id <workflow_id>");
    } else {
        println!(" {:<20} {:<24} {:<8} {:<10} {:<8} {}", "ID", "NAME", "VERSION", "SOURCE", "BLOCKS", "UPDATED AT");
        println!("----------------------------------------------------------------------------------------");
        for wf in &workflows {
            let id_display = if wf.id.len() > 19 { format!("{}...", &wf.id[..16]) } else { wf.id.clone() };
            let name_display = if wf.name.len() > 23 { format!("{}...", &wf.name[..20]) } else { wf.name.clone() };
            println!(" {:<20} {:<24} {:<8} {:<10} {:<8} {}", id_display, name_display, wf.version, wf.source, wf.blocks, wf.updated_at);
        }
        println!("========================================================================================");
        println!("💡 Run with:  tuquet run <ID>");
        if let Some(first) = workflows.first() {
            println!("   Example:   tuquet run {} --headless", first.id);
        }
    }
    println!("========================================================================================");

    Ok(())
}

async fn import_workflow(
    file: std::path::PathBuf,
    id_opt: Option<String>,
    name_opt: Option<String>,
    desc_opt: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() || !file.is_file() {
        return Err(format!("File does not exist or is not a file: {}", file.display()).into());
    }

    let content = tokio::fs::read_to_string(&file).await?;
    let mut val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file as JSON: {}", e))?;

    if !val.is_object() {
        return Err("Workflow file must contain a JSON object".into());
    }

    let derived_stem = file.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".workflow").replace(' ', "_"))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let id = id_opt
        .or_else(|| val.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or(derived_stem);

    let name = name_opt
        .or_else(|| val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| id.clone());

    let description = desc_opt
        .or_else(|| val.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()));

    let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();
    let icon = val.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Synchronize ID and Name into JSON data
    if let Some(obj) = val.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
        obj.insert("name".to_string(), serde_json::Value::String(name.clone()));
        if let Some(ref desc) = description {
            obj.insert("description".to_string(), serde_json::Value::String(desc.clone()));
        }
    }

    let serialized = serde_json::to_string_pretty(&val)?;

    let config = AppConfig::load();
    let db_dir = std::path::PathBuf::from(&config.data_dir);
    tokio::fs::create_dir_all(&db_dir).await?;
    let db_path = db_dir.join("automa.sqlite");

    let db = AutomaDb::new(&db_path)?;
    db.workflows().create_workflow(
        &id,
        &name,
        description.as_deref(),
        &serialized,
        Some(&version),
        icon.as_deref(),
    )?;

    // Also copy to Vault ~/.automa/workflows/
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let vault_dir = std::path::PathBuf::from(home).join(".automa").join("workflows");
    let _ = tokio::fs::create_dir_all(&vault_dir).await;
    let vault_file = vault_dir.join(format!("{}.workflow.json", id));
    tokio::fs::write(&vault_file, &serialized).await?;

    let block_count = val.get("nodes").and_then(|n| n.as_array()).map(|a| a.len())
        .or_else(|| val.pointer("/drawflow/nodes").and_then(|n| n.as_array()).map(|a| a.len()))
        .unwrap_or(0);

    println!("============================================================");
    println!(" ✔ Workflow Successfully Imported");
    println!("============================================================");
    println!(" ID:          {}", id);
    println!(" Name:        {}", name);
    println!(" Version:     {}", version);
    if let Some(ref d) = description {
        println!(" Description: {}", d);
    }
    println!(" Blocks:      {}", block_count);
    println!(" Vault File:  {}", vault_file.display());
    println!(" Database:    {}", db_path.display());
    println!("------------------------------------------------------------");
    println!(" 💡 Ready to execute:");
    println!("    tuquet run {} --headless", id);
    println!("============================================================");

    Ok(())
}

async fn export_workflow(
    id: String,
    output: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut workflow_json: Option<String> = None;

    // 1. Check persistent SQLite DB
    let db_path = std::path::PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists() {
        if let Ok(db) = AutomaDb::new(&db_path) {
            if let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(&id) {
                workflow_json = Some(wf.data);
            }
        }
    }

    // 2. Check Vault
    if workflow_json.is_none() {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = std::path::PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = std::path::PathBuf::from(&config.data_dir).join("workflows");

        let candidates = [
            user_vault.join(format!("{}.workflow.json", id)),
            user_vault.join(format!("{}.json", id)),
            config_vault.join(format!("{}.workflow.json", id)),
            config_vault.join(format!("{}.json", id)),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(c).await {
                    workflow_json = Some(content);
                    break;
                }
            }
        }
    }

    let raw_data = match workflow_json {
        Some(d) => d,
        None => {
            return Err(format!("Workflow '{}' not found in database or vault.", id).into());
        }
    };

    let final_content = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_data) {
        serde_json::to_string_pretty(&val)?
    } else {
        raw_data
    };

    let dest = output.unwrap_or_else(|| std::path::PathBuf::from(format!("{}.workflow.json", id)));
    if let Some(parent) = dest.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&dest, final_content).await?;

    println!("✔ Workflow '{}' exported successfully to: {:?}", id, dest);
    Ok(())
}

async fn delete_workflow(
    id: String,
    delete_from_vault: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let mut deleted_db = false;
    let mut deleted_vault = false;

    // 1. Delete from SQLite DB
    let db_path = std::path::PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists() {
        if let Ok(db) = AutomaDb::new(&db_path) {
            if let Ok(res) = db.workflows().delete_workflow(&id) {
                deleted_db = res;
            }
        }
    }

    // 2. Delete from Vault if requested
    if delete_from_vault {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let user_vault = std::path::PathBuf::from(&home).join(".automa").join("workflows");
        let config_vault = std::path::PathBuf::from(&config.data_dir).join("workflows");

        let candidates = [
            user_vault.join(format!("{}.workflow.json", id)),
            user_vault.join(format!("{}.json", id)),
            config_vault.join(format!("{}.workflow.json", id)),
            config_vault.join(format!("{}.json", id)),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                if tokio::fs::remove_file(c).await.is_ok() {
                    deleted_vault = true;
                }
            }
        }
    }

    if deleted_db || deleted_vault {
        println!("✔ Workflow '{}' deleted successfully (Database: {}, Vault: {}).", id, deleted_db, deleted_vault);
    } else {
        println!("Workflow '{}' was not found in database or vault.", id);
    }

    Ok(())
}

fn inspect_workflow(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!(" Automa Workflow Inspector");
    println!("============================================================");

    let (content, source_label) = {
        let target_path = std::path::Path::new(target);
        if target_path.exists() && target_path.is_file() {
            (std::fs::read_to_string(target_path)?, format!("File: {}", target_path.display()))
        } else {
            // Try resolving from DB or Vault
            let config = AppConfig::load();
            let mut resolved = None;

            // Check SQLite DB
            let db_path = std::path::PathBuf::from(&config.data_dir).join("automa.sqlite");
            if db_path.exists() {
                if let Ok(db) = AutomaDb::new(&db_path) {
                    if let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(target) {
                        resolved = Some((wf.data, format!("Database (ID: {})", wf.id)));
                    }
                }
            }

            // Check Vault
            if resolved.is_none() {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                let user_vault = std::path::PathBuf::from(&home).join(".automa").join("workflows");
                let config_vault = std::path::PathBuf::from(&config.data_dir).join("workflows");

                let candidates = [
                    user_vault.join(format!("{}.workflow.json", target)),
                    user_vault.join(format!("{}.json", target)),
                    config_vault.join(format!("{}.workflow.json", target)),
                    config_vault.join(format!("{}.json", target)),
                ];

                for c in &candidates {
                    if c.exists() && c.is_file() {
                        if let Ok(text) = std::fs::read_to_string(c) {
                            resolved = Some((text, format!("Vault File: {}", c.display())));
                            break;
                        }
                    }
                }
            }

            match resolved {
                Some(r) => r,
                None => {
                    eprintln!("Error: Workflow file or ID '{}' not found in file system, database, or vault.", target);
                    std::process::exit(1);
                }
            }
        }
    };

    println!("Target:      {}", target);
    println!("Source:      {}", source_label);

    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Failed to parse workflow file as valid JSON: {}", e);
            std::process::exit(1);
        }
    };

    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("(Unnamed Workflow)");
    let description = val.get("description").and_then(|v| v.as_str()).unwrap_or("(No description)");
    println!("Name:        {}", name);
    println!("Description: {}", description);

    let parsed_drawflow_opt = val.get("drawflow")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

    let nodes: Vec<serde_json::Value> = if let Some(arr) = val.pointer("/drawflow/nodes").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = parsed_drawflow_opt.as_ref().and_then(|d| d.get("nodes")).and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = val.get("nodes").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };

    let edge_count = if let Some(arr) = val.pointer("/drawflow/edges").and_then(|v| v.as_array()) {
        arr.len()
    } else if let Some(arr) = parsed_drawflow_opt.as_ref().and_then(|d| d.get("edges")).and_then(|v| v.as_array()) {
        arr.len()
    } else if let Some(arr) = val.get("edges").and_then(|v| v.as_array()) {
        arr.len()
    } else {
        0
    };

    println!("Structure:   {} nodes, {} connections", nodes.len(), edge_count);
    println!("------------------------------------------------------------");

    let mut triggers = Vec::new();
    let mut block_list = Vec::new();

    for node in &nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let label = node.get("label").or_else(|| node.get("type")).and_then(|v| v.as_str()).unwrap_or("unknown");
        let data = node.get("data");
        block_list.push((node_id, label, data));

        if label == "trigger" {
            let trigger_type = node.pointer("/data/type").and_then(|v| v.as_str()).unwrap_or("manual");
            triggers.push((node_id, trigger_type, node.pointer("/data/parameters")));
        }
    }

    if triggers.is_empty() {
        println!("Triggers: [!] NO TRIGGER NODE FOUND");
    } else {
        println!("Triggers ({}):", triggers.len());
        for (id, t_type, params) in &triggers {
            println!("  - [{}] Type: {}", id, t_type);
            if let Some(param_arr) = params.and_then(|p| p.as_array()) {
                if !param_arr.is_empty() {
                    println!("    Parameters:");
                    for p in param_arr {
                        let pname = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let pval = p.get("defaultValue").map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
                        println!("      * {} (default: {})", pname, pval);
                    }
                }
            }
        }
    }

    println!("\nBlocks Sequence ({}):", block_list.len());
    for (i, (id, label, data)) in block_list.iter().enumerate() {
        let mut detail = String::new();
        if let Some(d) = data {
            if let Some(url) = d.get("url").and_then(|v| v.as_str()) {
                detail = format!("url: {}", url);
            } else if let Some(time) = d.get("time").and_then(|v| v.as_i64()) {
                detail = format!("delay: {}ms", time);
            } else if let Some(sel) = d.get("selector").and_then(|v| v.as_str()) {
                detail = format!("selector: {}", sel);
            } else if let Some(code) = d.get("code").and_then(|v| v.as_str()) {
                let first_line = code.lines().next().unwrap_or("").chars().take(40).collect::<String>();
                detail = format!("js: {}...", first_line);
            }
        }
        if detail.is_empty() {
            println!("  {:2}. [{:<16}] id: {}", i + 1, label, id);
        } else {
            println!("  {:2}. [{:<16}] id: {} ({})", i + 1, label, id, detail);
        }
    }

    println!("\nPredefined Variables:");
    if let Some(vars) = val.get("variables") {
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                println!("  - {}: {}", k, v);
            }
        } else if let Some(arr) = vars.as_array() {
            for item in arr {
                println!("  - {}", item);
            }
        }
    } else {
        println!("  (None)");
    }

    // Dynamic expressions scan
    let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
    let mut expressions = std::collections::BTreeSet::new();
    for cap in re.captures_iter(&content) {
        if let Some(matched) = cap.get(1) {
            expressions.insert(matched.as_str().trim().to_string());
        }
    }

    if !expressions.is_empty() {
        println!("\nDynamic Expressions Detected:");
        for expr in &expressions {
            println!("  - {{{{ {} }}}}", expr);
        }
    }

    println!("============================================================");
    println!("Status: VALID WORKFLOW");
    println!("Execution command:");
    println!("  tuquet run \"{}\"", target);
    println!("============================================================");

    Ok(())
}

async fn run_workflow(
    workflow_path_opt: Option<String>,
    workflow_json_opt: Option<String>,
    headless: bool,
    browser_opt: Option<String>,
    browser_id_opt: Option<String>,
    variables: Vec<String>,
    timeout_opt: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout_duration = timeout_opt
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(300));
    let mut config = AppConfig::load();
    let data_dir = std::env::temp_dir().join(format!("automa_run_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&data_dir);
    config.data_dir = data_dir.to_string_lossy().to_string();

    let db_path = data_dir.join("automa_run.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path)?));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let bound_port = listener.local_addr()?.port();
    config.server_port = bound_port;
    
    // Set environment variable so browser extension connects to this ephemeral bridge port
    unsafe {
        std::env::set_var("AUTOMA_PORT", bound_port.to_string());
    }

    let (tx, mut rx) = tokio::sync::broadcast::channel(1000);
    let (worker_tx, _) = tokio::sync::broadcast::channel(1000);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx: tx.clone(),
        worker_tx,
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = api::routes::create_router(state.clone());

    let _server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let workflow_json_val: Option<serde_json::Value> = if let Some(ref raw_json) = workflow_json_opt {
        serde_json::from_str(raw_json).ok()
    } else {
        None
    };

    let vars_map: Option<serde_json::Value> = if !variables.is_empty() {
        let mut map = serde_json::Map::new();
        for var_str in variables {
            if let Some((k, v)) = var_str.split_once('=') {
                let key = k.trim().to_string();
                let val_trimmed = v.trim();
                let parsed_val = if let Ok(val) = serde_json::from_str::<serde_json::Value>(val_trimmed) {
                    val
                } else {
                    serde_json::Value::String(val_trimmed.to_string())
                };
                map.insert(key, parsed_val);
            } else {
                eprintln!("Warning: Invalid variable format '{}'. Expected KEY=VALUE", var_str);
            }
        }
        Some(serde_json::Value::Object(map))
    } else {
        None
    };

    let submit_options = automa_core::api::handlers::jobs::SubmitJobOptions {
        browser_id: browser_id_opt.or_else(|| Some("run_worker".to_string())),
        headless: Some(headless),
        default_browser: browser_opt,
        variables: vars_map,
        debug: Some(true),
        close_browser_on_finish: Some(true),
    };

    println!(">> Submitting workflow to browser worker (Bridge Port: {})...", bound_port);

    use automa_core::core::engine::job_coordinator::JobCoordinator;
    let job_id = match JobCoordinator::submit(
        &state,
        None,
        workflow_path_opt.as_deref(),
        workflow_json_val.as_ref(),
        Some(submit_options),
    ).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error submitting workflow: {}", e);
            std::process::exit(1);
        }
    };

    println!(">> Workflow dispatched [Job ID: {}]. Waiting for worker execution...", job_id);

    // Stream logs to console until job finishes or timeout/ctrl-c occurs
    let execution_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\n>> Execution interrupted by user (Ctrl+C). Cleaning up...");
            Err("Interrupted by user (SIGINT)")
        }
        _ = tokio::time::sleep(timeout_duration) => {
            eprintln!("\n>> Execution timed out after {:?}.", timeout_duration);
            Err("Workflow execution timed out")
        }
        res = async {
            let mut failed = false;
            while let Ok(msg) = rx.recv().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if let Some(event_type) = val.get("type").and_then(|v| v.as_str()) {
                        if event_type == "job_failed" {
                            failed = true;
                            println!(">> Job finished with event: {}", event_type);
                            break;
                        } else if event_type == "job_finish" || event_type == "job_completed" || event_type == "workflow_finished" {
                            println!(">> Job finished with event: {}", event_type);
                            break;
                        }
                    }
                    if let Some(log_msg) = val.get("message").and_then(|v| v.as_str()) {
                        println!("[worker] {}", log_msg);
                    } else if let Some(data) = val.get("data") {
                        println!("[worker] {}", data);
                    }
                }
            }
            if failed {
                Err("Workflow job failed during execution")
            } else {
                Ok(())
            }
        } => res
    };

    // Clean up server and browser child processes
    _server_handle.abort();
    automa_core::core::browser::manager::BrowserManager::destroy_all().await;

    // Explicitly drop state to release SQLite file handles
    drop(state);

    #[cfg(target_os = "windows")]
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Cleanup ephemeral data directory with retry
    for _ in 0..5 {
        if std::fs::remove_dir_all(&data_dir).is_ok() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    match execution_result {
        Ok(_) => {
            println!(">> Run completed successfully.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!(">> Run failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_server(
    host_override: Option<String>,
    port_override: Option<u16>,
    data_dir_override: Option<std::path::PathBuf>,
    log_level_override: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::load();
    if let Some(host) = host_override {
        config.server_host = host;
    }
    if let Some(port) = port_override {
        config.server_port = port;
    }
    if let Some(dir) = data_dir_override {
        config.data_dir = dir.to_string_lossy().to_string();
    }
    if let Some(level) = log_level_override {
        config.log_level = level;
    }

    let log_level = match config.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Tuquet Automa Core Bridge starting in Native Launcher mode...");
    info!("Environment: {}", config.environment);
    info!("Data Directory: {}", config.data_dir);
    
    std::fs::create_dir_all(&config.data_dir)?;

    let db_path = std::path::Path::new(&config.data_dir).join("automa.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path)?));

    let (tx, _) = tokio::sync::broadcast::channel(10000);
    let (worker_tx, _) = tokio::sync::broadcast::channel(10000);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx,
        worker_tx,
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = api::routes::create_router(state.clone());

    // Start background Cloud Telemetry & Inventory Reporter (runs if TUQUET_CLOUD_URL is configured)
    let _reporter_handle = automa_core::infrastructure::cloud_reporter::CloudReporter::start_background_loop(state.clone());

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", listener.local_addr()?);
    let panic_log_path = std::path::PathBuf::from(&config.data_dir).join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[AUTOMA-CORE PANIC] {:?}\n", info);
        eprintln!("{}", msg);
        let _ = std::fs::write(&panic_log_path, &msg);
    }));

    let shutdown_signal = async {
        #[cfg(windows)]
        {
            if std::env::var("AUTOMA_NO_CTRLC_SHUTDOWN").is_ok() {
                std::future::pending::<()>().await;
            } else {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
        #[cfg(not(windows))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        eprintln!("[AUTOMA-CORE SHUTDOWN TRIGGERED] SIGINT/Ctrl-C received!");
        info!("Shutdown signal received. Cleaning up child processes...");
        automa_core::core::browser::manager::BrowserManager::destroy_all().await;
        info!("All browser processes cleaned up.");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await {
        eprintln!("[AUTOMA-CORE SERVER ERROR] {:?}", e);
        return Err(e.into());
    }
    
    eprintln!("[AUTOMA-CORE EXITED MAIN OK]");
    Ok(())
}
