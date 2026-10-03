use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use nu_ansi_term::{Color, Style};
use reedline::{
    default_emacs_keybindings, Completer, CompletionResult, DefaultHinter, Emacs,
    FileBackedHistory, Highlighter, IdeMenu, KeyCode, KeyModifiers, MenuBuilder, Prompt,
    PromptEditMode, PromptHistorySearch, Reedline, ReedlineEvent, Signal, Span, StyledText,
    Suggestion,
};

use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{badge_online, colors, status_pill, Card};

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
    pub cloud_env: String,
}

impl Prompt for TuquetPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let border = colors::BORDER;
        let cyan = colors::CYAN;
        let bold = colors::BOLD;
        let reset = colors::RESET;
        let muted = colors::MUTED;
        let green = colors::GREEN;
        let yellow = colors::AMBER;
        let purple = colors::PURPLE;
        let blue = "\x1b[38;2;96;165;250m";

        let (scope_str, scope_color) = match self.scope {
            ShellScope::Global => ("global", cyan),
            ShellScope::Automa => ("automa", yellow),
            ShellScope::Runner => ("runner", green),
            ShellScope::Cloud => ("cloud", purple),
            ShellScope::Browser => ("browser", blue),
        };

        let env_badge = if self.cloud_env.is_empty() {
            format!("{muted}○ cloud:local{reset}")
        } else {
            format!("{green}● {}{reset}", self.cloud_env)
        };

        let line = format!(
            "{border}╭─{reset} {bold}{cyan}⚡ tuquet{reset}  {env_badge}  {border}[{reset}{scope_color}{scope_str}{reset}{border}]{reset}\n{border}╰─{reset}"
        );
        Cow::Owned(line)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[1;38;2;56;189;248m❯\x1b[0m ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[38;2;71;85;105m::: \x1b[0m")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[38;2;251;191;36m(search)\x1b[0m❯ ")
    }
}

pub struct TuquetHighlighter {
    pub scope: ShellScope,
}

impl Highlighter for TuquetHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        if line.is_empty() {
            return styled;
        }

        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 0 {
            styled.push((Style::new(), line[..leading_spaces].to_string()));
        }

        let valid_commands = match self.scope {
            ShellScope::Global => vec![
                "use", "automa", "runner", "cloud", "browser", "status", "whoami", "login",
                "logout", "run", "list", "studio", "inspect", "import", "export",
                "delete", "install", "clean", "path", "ext", "help", "clear", "cls", "exit", "quit", "q",
            ],
            ShellScope::Automa => vec![
                "run", "list", "inspect", "import", "export", "delete", "studio",
                "back", "help", "clear", "cls", "exit", "quit", "q",
            ],
            ShellScope::Runner => vec![
                "start", "stop", "restart", "status", "logs", "probe", "back", "help",
                "clear", "cls", "exit", "quit", "q",
            ],
            ShellScope::Cloud => vec![
                "login", "logout", "whoami", "back", "help", "clear", "cls", "exit", "quit", "q",
            ],
            ShellScope::Browser => vec![
                "install", "status", "clean", "path", "ext", "back", "help", "clear", "cls", "exit", "quit", "q",
            ],
        };

        let mut remaining = &line[leading_spaces..];
        let mut is_first_word = true;

        while !remaining.is_empty() {
            let next_word_end = remaining.find(char::is_whitespace).unwrap_or(remaining.len());
            let word = &remaining[..next_word_end];

            if is_first_word {
                is_first_word = false;
                if valid_commands.contains(&word) {
                    styled.push((Color::Cyan.bold(), word.to_string()));
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word.starts_with('-') {
                styled.push((Color::Yellow.normal(), word.to_string()));
            } else if word.starts_with('"') || word.starts_with('\'') {
                styled.push((Color::Green.normal(), word.to_string()));
            } else {
                styled.push((Color::White.normal(), word.to_string()));
            }

            remaining = &remaining[next_word_end..];
            let ws_len = remaining.len() - remaining.trim_start().len();
            if ws_len > 0 {
                styled.push((Style::new(), remaining[..ws_len].to_string()));
                remaining = &remaining[ws_len..];
            }
        }

        styled
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
                    ("status", "Inspect unified status across Cloud, Runner, and Browser"),
                    ("whoami", "Check active cloud enrollment identity & device ID"),
                    ("login", "Authenticate workstation with Tuquet Cloud"),
                    ("logout", "Disconnect and remove local cloud credentials"),
                    ("automa", "Enter browser automation scope"),
                    ("runner", "Enter distributed runner daemon scope"),
                    ("cloud", "Enter cloud authentication scope"),
                    ("browser", "Enter browser runtime management scope"),
                    ("run", "Execute a workflow (.json or stored ID)"),
                    ("list", "List stored workflows in vault and database"),
                    ("studio", "Launch Automa Web Studio"),
                    ("help", "Print help overview for current scope"),
                    ("clear", "Clear terminal screen"),
                    ("exit", "Exit Tuquet shell session"),
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
                    ("clear", "Clear terminal display"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Runner => &[
                    ("start", "Start the local runner daemon server"),
                    ("stop", "Gracefully terminate the runner daemon"),
                    ("restart", "Restart the local runner daemon"),
                    ("status", "Inspect local runner daemon health check"),
                    ("logs", "View background runner daemon logs"),
                    ("probe", "Probe runner driver manifest capabilities"),
                    ("back", "Return to global scope"),
                    ("help", "Print Runner scope help"),
                    ("clear", "Clear terminal display"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Cloud => &[
                    ("login", "Authenticate and pair device with Tuquet Cloud"),
                    ("logout", "Log out and remove local cloud credentials"),
                    ("whoami", "Check active cloud pairing and enrollment"),
                    ("back", "Return to global scope"),
                    ("help", "Print Cloud scope help"),
                    ("clear", "Clear terminal display"),
                    ("exit", "Return to global scope (or quit)"),
                ],
                ShellScope::Browser => &[
                    ("install", "Download and install Open-Source Chromium runtime"),
                    ("status", "Show installed browser path and disk usage"),
                    ("clean", "Delete installed browser runtime directory"),
                    ("path", "Print absolute path to browser executable"),
                    ("ext", "Inspect and configure Automa MV3 extension"),
                    ("back", "Return to global scope"),
                    ("help", "Print Browser scope help"),
                    ("clear", "Clear terminal display"),
                    ("exit", "Return to global scope (or quit)"),
                ],
            };

            for (cmd, desc) in commands {
                if cmd.starts_with(current_word) {
                    suggestions.push(make_suggestion(*cmd, *desc, span));
                }
            }
        } else if words_before.len() == 1 && words_before[0] == "use" {
            let scopes = ["automa", "runner", "cloud", "browser", "global"];
            for s in &scopes {
                if s.starts_with(current_word) {
                    suggestions.push(make_suggestion(*s, "Target service scope", span));
                }
            }
        } else if (self.scope == ShellScope::Automa || self.scope == ShellScope::Global) && matches!(words_before[0], "run" | "inspect" | "export" | "delete") {
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
        } else if words_before.len() == 1 && words_before[0] == "ext" {
            let ext_subcmds = [
                ("catalog", "Browse available extensions in tuquet-scoop-bucket"),
                ("install", "Download and install extension from catalog"),
                ("list", "List all registered extensions"),
                ("add", "Register custom extension from local directory"),
                ("remove", "Unregister extension by ID"),
                ("enable", "Enable extension for automated sessions"),
                ("disable", "Disable extension"),
                ("info", "Show extension details and manifest metadata"),
                ("path", "Print absolute path of extension"),
                ("launch", "Launch browser with loaded extensions"),
            ];
            for (sc, desc) in &ext_subcmds {
                if sc.starts_with(current_word) {
                    suggestions.push(make_suggestion(*sc, *desc, span));
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

    let config = AppConfig::load();
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let cloud_env_badge = if let Some(ref creds) = cloud_creds {
        if creds.cloud_url.as_deref().map(|u| u.contains("dswhacsoaxgpfnkaxnhz")).unwrap_or(false) {
            "prod:dswhacsoaxgpfnkaxnhz".to_string()
        } else {
            "enrolled".to_string()
        }
    } else {
        "".to_string()
    };

    // Probe runner daemon health
    let daemon_port = config.server_port;
    let daemon_online = {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(300))
            .build();
        if let Ok(c) = client {
            c.get(format!("http://127.0.0.1:{}/api/v1/health", daemon_port))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        } else {
            false
        }
    };

    let browser_status = crate::core::browser::resolver::get_runtime_status();

    let cloud_pill = if !cloud_env_badge.is_empty() {
        status_pill("cloud", &cloud_env_badge, true)
    } else {
        status_pill("cloud", "unpaired", false)
    };
    let daemon_pill = if daemon_online {
        status_pill("daemon", &daemon_port.to_string(), true)
    } else {
        status_pill("daemon", "offline", false)
    };
    let browser_pill = if browser_status.installed {
        status_pill("browser", "chromium", true)
    } else {
        status_pill("browser", "uninstalled", false)
    };

    // Print Hero Brand Banner
    println!();
    print!("{}", crate::ui::render_hero(env!("CARGO_PKG_VERSION"), &cloud_pill, &daemon_pill, &browser_pill));
    println!();

    if scope != ShellScope::Global {
        println!("Starting in \x1b[38;2;251;191;36m{:?}\x1b[0m service scope.", scope);
        println!();
    }

    let history_path = {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".tuquet").join("history.txt")
    };
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let history = Box::new(FileBackedHistory::with_file(1000, history_path)?);

    // Floating IDE completion popup menu
    let ide_menu = Box::new(
        IdeMenu::default()
            .with_name("completion_menu")
            .with_default_border()
            .with_min_completion_width(18)
            .with_max_completion_width(55)
            .with_min_description_width(20)
            .with_max_description_width(50)
            .with_max_completion_height(10),
    );

    // Custom keybindings for Tab autocomplete
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::MenuPrevious,
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let hinter = Box::new(DefaultHinter::default().with_style(Style::new().dimmed()));

    let mut line_editor = Reedline::create()
        .with_history(history)
        .with_edit_mode(edit_mode)
        .with_menu(reedline::ReedlineMenu::EngineCompleter(ide_menu))
        .with_hinter(hinter);

    loop {
        let prompt = TuquetPrompt {
            scope,
            cloud_env: cloud_env_badge.clone(),
        };
        let completer = Box::new(TuquetCompleter { scope });
        let highlighter = Box::new(TuquetHighlighter { scope });

        line_editor = line_editor
            .with_completer(completer)
            .with_highlighter(highlighter);

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
                        eprintln!("\x1b[38;2;248;113;113m[ERROR] {}\x1b[0m", e);
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
                eprintln!("\x1b[38;2;248;113;113m[ERROR] Readline error: {}\x1b[0m", e);
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
                println!("Returning to \x1b[1;38;2;56;189;248mGlobal\x1b[0m scope. Type 'exit' again to leave Tuquet.");
                *scope = ShellScope::Global;
                return Ok(false);
            } else {
                return Ok(true);
            }
        }
        "back" | "cd .." => {
            if *scope != ShellScope::Global {
                println!("Returned to \x1b[1;38;2;56;189;248mGlobal\x1b[0m scope.");
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
                        println!("Switched to \x1b[38;2;251;191;36mAutoma\x1b[0m context (Browser automation engine).");
                    }
                    "runner" => {
                        *scope = ShellScope::Runner;
                        println!("Switched to \x1b[38;2;74;222;128mRunner\x1b[0m context (Distributed daemon & node).");
                    }
                    "cloud" => {
                        *scope = ShellScope::Cloud;
                        println!("Switched to \x1b[38;2;168;85;247mCloud\x1b[0m context (Authentication & pairing).");
                    }
                    "browser" => {
                        *scope = ShellScope::Browser;
                        println!("Switched to \x1b[38;2;96;165;250mBrowser\x1b[0m context (Isolated Chromium management).");
                    }
                    "global" => {
                        *scope = ShellScope::Global;
                        println!("Returned to \x1b[1;38;2;56;189;248mGlobal\x1b[0m context.");
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
                println!("Switched to \x1b[38;2;251;191;36mAutoma\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "runner" if args.is_empty() => {
                *scope = ShellScope::Runner;
                println!("Switched to \x1b[38;2;74;222;128mRunner\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "cloud" if args.is_empty() => {
                *scope = ShellScope::Cloud;
                println!("Switched to \x1b[38;2;168;85;247mCloud\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            "browser" if args.is_empty() => {
                *scope = ShellScope::Browser;
                println!("Switched to \x1b[38;2;96;165;250mBrowser\x1b[0m context. Type 'help' for commands, 'back' to return.");
                return Ok(false);
            }
            _ => {}
        }
    }

    // Support scope prefix redundancy gracefully (e.g. typing "automa run" inside automa scope)
    match *scope {
        ShellScope::Automa if cmd == "automa" && !args.is_empty() => {
            return dispatch_automa(args[0], &args[1..]).await.map(|_| false);
        }
        ShellScope::Runner if cmd == "runner" && !args.is_empty() => {
            return dispatch_runner(args[0], &args[1..]).await.map(|_| false);
        }
        ShellScope::Cloud if cmd == "cloud" && !args.is_empty() => {
            return dispatch_cloud(args[0], &args[1..]).await.map(|_| false);
        }
        ShellScope::Browser if cmd == "browser" && !args.is_empty() => {
            return dispatch_browser(args[0], &args[1..]).await.map(|_| false);
        }
        _ => {}
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
        "status" => {
            crate::commands::status::show_dashboard().await?;
        }
        "whoami" => {
            crate::commands::cloud::whoami().await?;
        }
        "login" => {
            let token = args.first().map(|s| s.to_string());
            crate::commands::cloud::login(None, token, None).await?;
        }
        "logout" => {
            crate::commands::cloud::logout().await?;
        }
        "automa" => {
            if args.is_empty() {
                println!("Usage: automa <run | list | inspect | import | export | delete | studio>");
            } else {
                dispatch_automa(args[0], &args[1..]).await?;
            }
        }
        "runner" => {
            if args.is_empty() {
                println!("Usage: runner <start | stop | restart | status | logs | probe | export-openapi | setup-ext>");
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
                println!("Usage: browser <status | install | clean | path | ext>");
            } else {
                dispatch_browser(args[0], &args[1..]).await?;
            }
        }
        // Direct Global convenience commands
        "run" | "list" | "inspect" | "import" | "export" | "delete" | "studio" => {
            dispatch_automa(cmd, args).await?;
        }
        "install" | "clean" | "path" => {
            dispatch_browser(cmd, args).await?;
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
                println!("Usage: run <workflow.json | stored_id> [--headless] [--browser <name>] [--timeout <sec>]");
            } else {
                let wf = Some(args[0].to_string());
                let headless = args.contains(&"--headless");
                let mut browser = None;
                let mut timeout = None;

                let mut iter = args.iter().skip(1);
                while let Some(arg) = iter.next() {
                    if *arg == "--browser" {
                        browser = iter.next().map(|s| s.to_string());
                    } else if *arg == "--timeout" {
                        timeout = iter.next().and_then(|s| s.parse::<u64>().ok());
                    }
                }

                crate::commands::automa::run_workflow(
                    wf, None, headless, browser, None, Vec::new(), timeout,
                )
                .await?;
            }
        }
        "list" => {
            let search = args.first().map(|s| s.to_string());
            crate::commands::automa::list_workflows(search, false, false).await?;
        }
        "inspect" => {
            if args.is_empty() {
                println!("Usage: inspect <workflow.json | stored_id>");
            } else {
                crate::commands::automa::inspect_workflow(args[0])?;
            }
        }
        "import" => {
            if args.is_empty() {
                println!("Usage: import <file.json> [--id <id>] [--name <name>]");
            } else {
                let file = PathBuf::from(args[0]);
                let mut id = None;
                let mut name = None;
                let mut iter = args.iter().skip(1);
                while let Some(arg) = iter.next() {
                    if *arg == "--id" {
                        id = iter.next().map(|s| s.to_string());
                    } else if *arg == "--name" {
                        name = iter.next().map(|s| s.to_string());
                    }
                }
                crate::commands::automa::import_workflow(file, id, name, None).await?;
            }
        }
        "export" => {
            if args.is_empty() {
                println!("Usage: export <workflow_id> [output_file.json]");
            } else {
                let id = args[0].to_string();
                let output = args.get(1).map(PathBuf::from);
                crate::commands::automa::export_workflow(id, output).await?;
            }
        }
        "delete" => {
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
        "start" => {
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            if detach {
                println!("Starting runner daemon in background...");
            } else {
                println!("Starting runner daemon in foreground (Ctrl+C to stop)...");
            }
            crate::commands::runner::run_server(None, None, detach, None, None).await?;
        }
        "stop" => {
            let force = args.contains(&"-f") || args.contains(&"--force");
            crate::commands::runner::stop_daemon(force).await?;
        }
        "restart" => {
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            crate::commands::runner::restart_daemon(detach).await?;
        }
        "logs" => {
            let follow = args.contains(&"-f") || args.contains(&"--follow");
            crate::commands::runner::show_logs(follow, 50).await?;
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
            crate::commands::browser::setup_extension(browser, None).await?;
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
        "ext" => {
            let subcmd_str = args.first().copied();
            let ext_subcmd = match subcmd_str {
                Some("list") => Some(crate::cli::ExtCommands::List),
                Some("catalog") => {
                    let query = args.get(1).map(|s| s.to_string());
                    Some(crate::cli::ExtCommands::Catalog { query })
                }
                Some("install") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    let force = args.contains(&"-f") || args.contains(&"--force");
                    Some(crate::cli::ExtCommands::Install { id, force })
                }
                Some("add") => {
                    let path_str = args.get(1).copied().unwrap_or(".");
                    Some(crate::cli::ExtCommands::Add {
                        path: std::path::PathBuf::from(path_str),
                        id: args.get(2).map(|s| s.to_string()),
                    })
                }
                Some("remove") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Remove { id })
                }
                Some("enable") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Enable { id })
                }
                Some("disable") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Disable { id })
                }
                Some("info") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    Some(crate::cli::ExtCommands::Info { id })
                }
                Some("path") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    Some(crate::cli::ExtCommands::Path { id })
                }
                Some("launch") => {
                    Some(crate::cli::ExtCommands::Launch {
                        ext: args.get(1).map(|s| s.to_string()),
                        browser: "chrome".to_string(),
                    })
                }
                _ => None,
            };
            crate::commands::browser::handle_ext(ext_subcmd, "chrome", None).await?;
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
            let mut card = Card::new("TUQUET SHELL");
            card.with_badge(badge_online("GLOBAL SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Scope Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  use <service>", "Switch active context: automa, runner, cloud, browser");
            card.add_kv("  automa", "Enter browser automation scope directly");
            card.add_kv("  runner", "Enter worker daemon scope directly");
            card.add_kv("  cloud", "Enter cloud pairing scope directly");
            card.add_kv("  browser", "Enter browser runtime management scope");
            card.add_kv("  clear", "Clear terminal screen buffer");
            card.add_kv("  exit", "Quit interactive shell");
            card.add_line(format!("{BOLD}Direct Shortcuts:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  status", "Unified ecosystem health & subsystem status");
            card.add_kv("  whoami", "Inspect cloud enrollment identity & device ID");
            card.add_kv("  login [token]", "Authenticate workstation with Tuquet Cloud");
            card.add_kv("  run <wf>", "Execute workflow (.json file or stored ID)");
            card.add_kv("  list [query]", "List stored workflows in vault & database");
            card.add_kv("  studio", "Launch Automa Web Studio in browser");
            card.add_kv("  install", "Download and install isolated Chromium runtime");
            card.with_footer("Tip: Press [Tab] anytime for smart floating autocomplete");
            println!();
            card.print();
            println!();
        }
        ShellScope::Automa => {
            let mut card = Card::new("TUQUET SHELL");
            card.with_badge(badge_online("AUTOMA SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Workflow Automation Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  run <wf> [--headless]", "Execute workflow (.json file or saved ID)");
            card.add_kv("  list [query]", "List all workflows saved in database and vault");
            card.add_kv("  inspect <wf>", "Inspect block graph structure and triggers");
            card.add_kv("  import <file.json>", "Import workflow into local database & vault");
            card.add_kv("  export <id>", "Export saved workflow to a JSON file");
            card.add_kv("  delete <id>", "Delete workflow from storage");
            card.add_kv("  studio", "Launch Automa Web Studio in browser");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
        ShellScope::Runner => {
            let mut card = Card::new("TUQUET SHELL");
            card.with_badge(badge_online("RUNNER SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Daemon Worker Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  start [-d]", "Start runner daemon worker (foreground or detached)");
            card.add_kv("  stop [-f]", "Gracefully terminate running runner daemon");
            card.add_kv("  restart [-d]", "Restart local runner daemon worker");
            card.add_kv("  status", "Inspect local runner daemon health check");
            card.add_kv("  logs [-f]", "View or stream background runner daemon logs");
            card.add_kv("  probe", "Probe runner driver capabilities manifest");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
        ShellScope::Cloud => {
            let mut card = Card::new("TUQUET SHELL");
            card.with_badge(badge_online("CLOUD SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Cloud Pairing Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  login [token]", "Authenticate and pair device with Tuquet Cloud");
            card.add_kv("  logout", "Log out and delete local cloud pairing");
            card.add_kv("  whoami", "Inspect current workstation identity and tenant");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
        ShellScope::Browser => {
            let mut card = Card::new("TUQUET SHELL");
            card.with_badge(badge_online("BROWSER SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Browser Runtime Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  status", "Show installed browser status, path, and disk usage");
            card.add_kv("  install [--force]", "Download and install Open-Source Chromium");
            card.add_kv("  clean", "Delete browser runtime directory to reclaim disk");
            card.add_kv("  path", "Print absolute path to browser executable");
            card.add_kv("  ext [subcmd]", "Manage extensions (list, add, remove, enable, disable, info, launch)");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
    }
}
