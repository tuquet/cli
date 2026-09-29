use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use reedline::{
    Completer, CompletionResult, FileBackedHistory, Prompt, PromptEditMode, PromptHistorySearch,
    Reedline, Signal, Span, Suggestion,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellScope {
    Global,
    Automa,
    Runner,
    Cloud,
    Browser,
}

pub struct TuquetPrompt {
    pub scope: ShellScope,
}

impl Prompt for TuquetPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        match self.scope {
            ShellScope::Global => Cow::Borrowed("\x1b[1;36mtuquet\x1b[0m"),
            ShellScope::Automa => Cow::Borrowed("\x1b[1;36mtuquet\x1b[0m\x1b[33m(automa)\x1b[0m"),
            ShellScope::Runner => Cow::Borrowed("\x1b[1;36mtuquet\x1b[0m\x1b[32m(runner)\x1b[0m"),
            ShellScope::Cloud => Cow::Borrowed("\x1b[1;36mtuquet\x1b[0m\x1b[35m(cloud)\x1b[0m"),
            ShellScope::Browser => Cow::Borrowed("\x1b[1;36mtuquet\x1b[0m\x1b[34m(browser)\x1b[0m"),
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("(search)> ")
    }
}

fn make_suggestion(value: impl Into<String>, desc: impl Into<String>, span: Span) -> Suggestion {
    Suggestion {
        value: value.into(),
        display_override: None,
        description: Some(desc.into()),
        style: None,
        extra: None,
        span,
        append_whitespace: true,
        match_indices: None,
    }
}

pub struct TuquetCompleter {
    pub scope: ShellScope,
}

impl Completer for TuquetCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let prefix = if pos <= line.len() { &line[..pos] } else { line };
        let start = prefix.rfind(|c: char| c.is_whitespace()).map(|idx| idx + 1).unwrap_or(0);
        let current_word = &prefix[start..];
        let span = Span::new(start, pos);

        let words_before: Vec<&str> = prefix[..start].split_whitespace().collect();

        let mut suggestions = Vec::new();

        if words_before.is_empty() {
            // Completing command name based on current scope
            let commands: &[(&str, &str)] = match self.scope {
                ShellScope::Global => &[
                    ("use", "Switch active service scope (automa, runner, cloud, browser)"),
                    ("automa", "Enter browser automation scope"),
                    ("runner", "Enter distributed runner scope"),
                    ("cloud", "Enter cloud authentication scope"),
                    ("browser", "Enter browser runtime management scope"),
                    ("help", "Print help overview"),
                    ("clear", "Clear terminal screen"),
                    ("exit", "Exit Tuquet shell"),
                ],
                ShellScope::Automa => &[
                    ("run", "Execute a workflow (.json or stored ID)"),
                    ("list", "List stored workflows in vault and database"),
                    ("inspect", "Inspect and validate a workflow structure"),
                    ("import", "Import a workflow file into local storage"),
                    ("export", "Export a workflow to a JSON file"),
                    ("delete", "Delete a workflow from local storage"),
                    ("studio", "Launch Automa Web Studio"),
                    ("back", "Return to global scope"),
                    ("help", "Print Automa scope help"),
                    ("clear", "Clear screen"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Runner => &[
                    ("start", "Start the local runner daemon server"),
                    ("status", "Inspect local runner daemon health check"),
                    ("probe", "Probe runner driver manifest capabilities"),
                    ("export-openapi", "Export OpenAPI v3 spec to file"),
                    ("setup-ext", "Launch browser with extension loaded"),
                    ("back", "Return to global scope"),
                    ("help", "Print Runner scope help"),
                    ("clear", "Clear screen"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Cloud => &[
                    ("login", "Authenticate and pair device with Tuquet Cloud"),
                    ("logout", "Log out and remove local cloud credentials"),
                    ("whoami", "Check active cloud pairing and enrollment"),
                    ("back", "Return to global scope"),
                    ("help", "Print Cloud scope help"),
                    ("clear", "Clear screen"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Browser => &[
                    ("install", "Download and install Open-Source Chromium runtime"),
                    ("status", "Show installed browser path and disk usage"),
                    ("clean", "Delete installed browser runtime directory"),
                    ("path", "Print absolute path to browser executable"),
                    ("back", "Return to global scope"),
                    ("help", "Print Browser scope help"),
                    ("clear", "Clear screen"),
                    ("exit", "Return to global scope (or quit)"),
                ],
            };

            for (cmd, desc) in commands {
                if cmd.starts_with(current_word) {
                    suggestions.push(make_suggestion(*cmd, *desc, span));
                }
            }
        } else if words_before.len() == 1 && words_before[0] == "use" {
            // Completing scope target for 'use'
            let scopes = ["automa", "runner", "cloud", "browser", "global"];
            for s in &scopes {
                if s.starts_with(current_word) {
                    suggestions.push(make_suggestion(*s, "Target service scope", span));
                }
            }
        } else if self.scope == ShellScope::Automa && matches!(words_before[0], "run" | "inspect" | "export" | "delete") {
            // Suggest workflows from ~/.tuquet/workflows/
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                let vault_dir = PathBuf::from(home).join(".tuquet").join("workflows");
                if let Ok(entries) = std::fs::read_dir(vault_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file()
                            && path.extension().map(|e| e == "json").unwrap_or(false)
                            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                        {
                            let clean_id = stem.trim_end_matches(".workflow");
                            if clean_id.starts_with(current_word) {
                                suggestions.push(make_suggestion(clean_id, "Workflow from vault", span));
                            }
                        }
                    }
                }
            }

            // Flag completions
            if current_word.starts_with('-') {
                for flag in &["--headless", "--timeout", "--browser"] {
                    if flag.starts_with(current_word) {
                        suggestions.push(make_suggestion(*flag, "Execution option", span));
                    }
                }
            }
        }

        CompletionResult::fresh(suggestions)
    }
}

pub async fn run(initial_service: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = match initial_service.map(|s| s.to_lowercase()).as_deref() {
        Some("automa") => ShellScope::Automa,
        Some("runner") => ShellScope::Runner,
        Some("cloud") => ShellScope::Cloud,
        Some("browser") => ShellScope::Browser,
        _ => ShellScope::Global,
    };

    println!("\x1b[1;36m============================================================\x1b[0m");
    println!("\x1b[1;36m Tuquet Unified Interactive Shell (v{})\x1b[0m", env!("CARGO_PKG_VERSION"));
    println!("\x1b[1;36m============================================================\x1b[0m");
    println!("Type \x1b[1;33m'help'\x1b[0m for commands, \x1b[1;33m'use <service>'\x1b[0m to switch scope, \x1b[1;33m'exit'\x1b[0m to quit.");
    println!("Tip: Press \x1b[1;32m[Tab]\x1b[0m for smart autocomplete and workflow suggestions.");
    println!();

    if scope != ShellScope::Global {
        println!("Starting in \x1b[33m{:?}\x1b[0m service scope.", scope);
    }

    let history_path = {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".tuquet").join("history.txt")
    };
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let history = Box::new(FileBackedHistory::with_file(1000, history_path)?);

    let mut line_editor = Reedline::create()
        .with_history(history);

    loop {
        let prompt = TuquetPrompt { scope };
        let completer = Box::new(TuquetCompleter { scope });
        line_editor = line_editor.with_completer(completer);

        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let input = buffer.trim();
                if input.is_empty() {
                    continue;
                }
                match handle_command(input, &mut scope).await {
                    Ok(should_exit) => {
                        if should_exit {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[31m[ERROR] {}\x1b[0m", e);
                    }
                }
            }
            Ok(Signal::CtrlC) => {
                println!("^C");
            }
            Ok(Signal::CtrlD) => {
                println!("Bye!");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("\x1b[31m[ERROR] Readline error: {}\x1b[0m", e);
                break;
            }
        }
    }

    println!("Exiting Tuquet Interactive Shell. Goodbye!");
    Ok(())
}

async fn handle_command(input: &str, scope: &mut ShellScope) -> Result<bool, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(false);
    }

    let cmd = parts[0];
    let args = &parts[1..];

    match cmd {
        "exit" | "quit" | "q" => {
            if *scope != ShellScope::Global {
                println!("Returning to \x1b[1;36mGlobal\x1b[0m scope. Type 'exit' again to leave Tuquet.");
                *scope = ShellScope::Global;
                return Ok(false);
            } else {
                return Ok(true);
            }
        }
        "back" | "cd .." => {
            if *scope != ShellScope::Global {
                println!("Returned to \x1b[1;36mGlobal\x1b[0m scope.");
                *scope = ShellScope::Global;
            }
            return Ok(false);
        }
        "use" => {
            if args.is_empty() {
                println!("Usage: use <automa | runner | cloud | browser | global>");
            } else {
                match args[0].to_lowercase().as_str() {
                    "automa" => {
                        *scope = ShellScope::Automa;
                        println!("Switched to \x1b[33mAutoma\x1b[0m context (Browser automation engine).");
                    }
                    "runner" => {
                        *scope = ShellScope::Runner;
                        println!("Switched to \x1b[32mRunner\x1b[0m context (Distributed daemon & node).");
                    }
                    "cloud" => {
                        *scope = ShellScope::Cloud;
                        println!("Switched to \x1b[35mCloud\x1b[0m context (Authentication & pairing).");
                    }
                    "browser" => {
                        *scope = ShellScope::Browser;
                        println!("Switched to \x1b[34mBrowser\x1b[0m context (Isolated Chromium management).");
                    }
                    "global" | "root" => {
                        *scope = ShellScope::Global;
                        println!("Returned to \x1b[1;36mGlobal\x1b[0m context.");
                    }
                    other => {
                        println!("Unknown scope '{}'. Available: automa, runner, cloud, browser, global", other);
                    }
                }
            }
            return Ok(false);
        }
        "clear" | "cls" => {
            print!("\x1B[2J\x1B[1;1H");
            let _ = std::io::stdout().flush();
            return Ok(false);
        }
        "help" | "?" => {
            print_scope_help(*scope);
            return Ok(false);
        }
        _ => {}
    }

    // Direct scope switching shortcuts in Global: "automa", "runner", "cloud", "browser"
    if *scope == ShellScope::Global {
        match cmd {
            "automa" if args.is_empty() => {
                *scope = ShellScope::Automa;
                println!("Switched to \x1b[33mAutoma\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "runner" if args.is_empty() => {
                *scope = ShellScope::Runner;
                println!("Switched to \x1b[32mRunner\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "cloud" if args.is_empty() => {
                *scope = ShellScope::Cloud;
                println!("Switched to \x1b[35mCloud\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "browser" if args.is_empty() => {
                *scope = ShellScope::Browser;
                println!("Switched to \x1b[34mBrowser\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            _ => {}
        }
    }

    match *scope {
        ShellScope::Global => dispatch_global(cmd, args).await?,
        ShellScope::Automa => dispatch_automa(cmd, args).await?,
        ShellScope::Runner => dispatch_runner(cmd, args).await?,
        ShellScope::Cloud => dispatch_cloud(cmd, args).await?,
        ShellScope::Browser => dispatch_browser(cmd, args).await?,
    }

    Ok(false)
}

async fn dispatch_global(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "automa" => {
            if args.is_empty() {
                println!("Usage: automa <run | list | inspect | import | export | delete | studio>");
            } else {
                dispatch_automa(args[0], &args[1..]).await?;
            }
        }
        "runner" => {
            if args.is_empty() {
                println!("Usage: runner <status | probe | export-openapi | setup-ext | start>");
            } else {
                dispatch_runner(args[0], &args[1..]).await?;
            }
        }
        "cloud" => {
            if args.is_empty() {
                println!("Usage: cloud <login | logout | whoami>");
            } else {
                dispatch_cloud(args[0], &args[1..]).await?;
            }
        }
        "browser" => {
            if args.is_empty() {
                println!("Usage: browser <status | install | clean | path>");
            } else {
                dispatch_browser(args[0], &args[1..]).await?;
            }
        }
        other => {
            println!("Unknown global command: '{}'. Type 'help' or 'use <service>' to enter a scope.", other);
        }
    }
    Ok(())
}

async fn dispatch_automa(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "run" => {
            if args.is_empty() {
                println!("Usage: run <workflow_id_or_file> [--headless] [--timeout <secs>]");
                return Ok(());
            }
            let target = Some(args[0].to_string());
            let headless = args.contains(&"--headless");
            let timeout = args.iter().position(|&x| x == "--timeout")
                .and_then(|idx| args.get(idx + 1))
                .and_then(|val| val.parse::<u64>().ok());

            crate::commands::automa::run_workflow(target, None, headless, None, None, Vec::new(), timeout).await?;
        }
        "list" | "ls" => {
            let search = if !args.is_empty() && !args[0].starts_with('-') {
                Some(args[0].to_string())
            } else {
                None
            };
            let db_only = args.contains(&"--db-only");
            let vault_only = args.contains(&"--vault-only");
            crate::commands::automa::list_workflows(search, db_only, vault_only).await?;
        }
        "inspect" => {
            if args.is_empty() {
                println!("Usage: inspect <workflow_id_or_file>");
            } else {
                crate::commands::automa::inspect_workflow(args[0])?;
            }
        }
        "import" | "add" => {
            if args.is_empty() {
                println!("Usage: import <file.json> [id] [name]");
            } else {
                let file = PathBuf::from(args[0]);
                let id = args.get(1).map(|s| s.to_string());
                let name = args.get(2).map(|s| s.to_string());
                crate::commands::automa::import_workflow(file, id, name, None).await?;
            }
        }
        "export" => {
            if args.is_empty() {
                println!("Usage: export <workflow_id> [output_file]");
            } else {
                let id = args[0].to_string();
                let output = args.get(1).map(PathBuf::from);
                crate::commands::automa::export_workflow(id, output).await?;
            }
        }
        "delete" | "rm" => {
            if args.is_empty() {
                println!("Usage: delete <workflow_id> [--vault]");
            } else {
                let id = args[0].to_string();
                let from_vault = args.contains(&"--vault");
                crate::commands::automa::delete_workflow(id, from_vault).await?;
            }
        }
        "studio" => {
            crate::commands::automa::open_studio()?;
        }
        other => {
            println!("Unknown automa command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

async fn dispatch_runner(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            let target_url = args.first().copied().unwrap_or("http://127.0.0.1:8765");
            crate::commands::runner::check_status(target_url).await?;
        }
        "probe" => {
            crate::commands::runner::print_probe_manifest()?;
        }
        "export-openapi" => {
            let output = args.first().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("openapi.json"));
            crate::commands::runner::export_openapi(&output)?;
        }
        "setup-ext" => {
            let browser = args.first().copied().unwrap_or("chrome");
            crate::commands::runner::setup_extension(browser, None).await?;
        }
        "start" => {
            println!("Starting runner daemon in foreground (Ctrl+C to stop)...");
            crate::commands::runner::run_server(None, None, None, None).await?;
        }
        other => {
            println!("Unknown runner command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

async fn dispatch_cloud(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "login" => {
            let token = args.first().map(|s| s.to_string());
            crate::commands::cloud::login(None, token, None).await?;
        }
        "logout" => {
            crate::commands::cloud::logout().await?;
        }
        "whoami" => {
            crate::commands::cloud::whoami().await?;
        }
        other => {
            println!("Unknown cloud command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

async fn dispatch_browser(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Status).await?;
        }
        "install" => {
            let force = args.contains(&"--force");
            crate::commands::browser::handle(crate::cli::BrowserCommands::Install { force, revision: None }).await?;
        }
        "clean" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Clean).await?;
        }
        "path" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Path).await?;
        }
        other => {
            println!("Unknown browser command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

fn print_scope_help(scope: ShellScope) {
    match scope {
        ShellScope::Global => {
            println!("============================================================");
            println!(" Tuquet Interactive Shell - Global Scope");
            println!("============================================================");
            println!(" Scope Navigation:");
            println!("   use <service>      Switch context: automa, runner, cloud, browser");
            println!("   automa             Quick switch to automa scope");
            println!("   runner             Quick switch to runner scope");
            println!("   cloud              Quick switch to cloud scope");
            println!("   browser            Quick switch to browser scope");
            println!("   clear              Clear terminal display");
            println!("   exit               Quit interactive shell");
            println!(" Direct Execution:");
            println!("   automa run <id>    Execute workflow directly from global");
            println!("   browser status     Inspect dedicated browser runtime");
            println!("   runner status      Check local runner daemon health");
            println!("   cloud whoami       Check cloud pairing status");
            println!("============================================================");
        }
        ShellScope::Automa => {
            println!("============================================================");
            println!(" Tuquet Interactive Shell - Automa (Browser Automation)");
            println!("============================================================");
            println!(" Commands:");
            println!("   run <wf> [--headless]  Execute workflow directly");
            println!("   list [query]           List workflows in vault & database");
            println!("   inspect <wf>           Validate & inspect workflow graph");
            println!("   import <file.json>     Import workflow into local storage");
            println!("   export <id> [out.json] Export workflow to file");
            println!("   delete <id> [--vault]  Delete workflow");
            println!("   studio                 Launch Automa Web Studio in browser");
            println!(" Navigation:");
            println!("   back                   Return to global scope");
            println!("   exit                   Return to global scope (or quit)");
            println!("============================================================");
        }
        ShellScope::Runner => {
            println!("============================================================");
            println!(" Tuquet Interactive Shell - Runner (Daemon & Execution Node)");
            println!("============================================================");
            println!(" Commands:");
            println!("   status             Check local runner daemon health");
            println!("   probe              Inspect capability manifest");
            println!("   export-openapi     Export OpenAPI specification");
            println!("   setup-ext          Developer utility to load unpacked extension");
            println!("   start              Start local runner daemon in foreground");
            println!(" Navigation:");
            println!("   back               Return to global scope");
            println!("   exit               Return to global scope (or quit)");
            println!("============================================================");
        }
        ShellScope::Cloud => {
            println!("============================================================");
            println!(" Tuquet Interactive Shell - Cloud (Multi-Tenant Auth)");
            println!("============================================================");
            println!(" Commands:");
            println!("   login [token]      Authenticate workstation with Tuquet Cloud");
            println!("   logout             Log out and clear device pairing credentials");
            println!("   whoami             Check current device enrollment & tenant");
            println!(" Navigation:");
            println!("   back               Return to global scope");
            println!("   exit               Return to global scope (or quit)");
            println!("============================================================");
        }
        ShellScope::Browser => {
            println!("============================================================");
            println!(" Tuquet Interactive Shell - Browser (Runtime Management)");
            println!("============================================================");
            println!(" Commands:");
            println!("   status             Inspect dedicated Chromium runtime installation");
            println!("   install [--force]  Download and setup pure Open-Source Chromium");
            println!("   clean              Remove browser runtime to reclaim disk space");
            println!("   path               Print absolute browser binary path");
            println!(" Navigation:");
            println!("   back               Return to global scope");
            println!("   exit               Return to global scope (or quit)");
            println!("============================================================");
        }
    }
}
