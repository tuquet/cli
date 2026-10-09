use std::io::Write;
use nu_ansi_term::Style;
use reedline::{
    default_emacs_keybindings, DefaultHinter, Emacs, FileBackedHistory, IdeMenu, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, Signal,
};

use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{badge_online, colors, status_pill, Card};

pub mod completer;
pub mod dispatch;
pub mod highlighter;
pub mod prompt;
pub mod scope;

pub use completer::SpecterCompleter;
pub use highlighter::SpecterHighlighter;
pub use prompt::SpecterPrompt;
pub use scope::ShellScope;

pub async fn run(initial_service: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = initial_service.and_then(ShellScope::from_name).unwrap_or(ShellScope::Global);

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

    if let Some(info) = crate::infrastructure::updater::get_cached_update()
        && info.has_update
    {
        println!("{}", crate::ui::render_update_banner(&info.current_version, &info.latest_version));
        println!();
    }

    if scope != ShellScope::Global {
        println!("Starting in \x1b[38;2;251;191;36m{:?}\x1b[0m service scope.", scope);
        println!();
    }

    let history_path = {
        let sys_dir = crate::config::canonical_ssot_dir().join("system");
        let _ = std::fs::create_dir_all(&sys_dir);
        sys_dir.join("history.txt")
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
        let prompt = SpecterPrompt {
            scope,
            cloud_env: cloud_env_badge.clone(),
        };
        let completer = Box::new(SpecterCompleter { scope });
        let highlighter = Box::new(SpecterHighlighter { scope });

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

    println!("Exiting Specter Interactive Shell. Goodbye!");
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
                println!("Returning to \x1b[1;38;2;56;189;248mGlobal\x1b[0m scope. Type 'exit' again to leave Specter.");
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
                println!("Usage: use <global | automa | runner | cloud | browser | bridge | faker>");
            } else if let Some(target) = ShellScope::from_name(args[0]) {
                *scope = target;
                match target {
                    ShellScope::Global => println!("Returned to \x1b[1;38;2;56;189;248mGlobal\x1b[0m context."),
                    ShellScope::Automa => println!("Switched to \x1b[38;2;251;191;36mAutoma\x1b[0m context (Browser automation engine)."),
                    ShellScope::Runner => println!("Switched to \x1b[38;2;74;222;128mRunner\x1b[0m context (Distributed daemon & node)."),
                    ShellScope::Cloud => println!("Switched to \x1b[38;2;168;85;247mCloud\x1b[0m context (Authentication & pairing)."),
                    ShellScope::Browser => println!("Switched to \x1b[38;2;96;165;250mBrowser\x1b[0m context (Isolated Chromium management)."),
                    ShellScope::Bridge => println!("Switched to \x1b[38;2;217;70;239mBridge\x1b[0m context (Network tunnels & SOCKS5 proxy)."),
                    ShellScope::Faker => println!("Switched to \x1b[38;2;56;189;248mFaker\x1b[0m context (Synthetic persona & CCCD generator)."),
                }
            } else {
                println!("Unknown scope '{}'. Available: global, automa, runner, cloud, browser, bridge, faker", args[0]);
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
        "doctor" => {
            crate::commands::doctor::run(false, crate::ui::OutputFormat::Card).await?;
            return Ok(false);
        }
        _ => {}
    }

    // Direct scope switching shortcuts in Global: "automa", "runner", "cloud", "browser", "bridge", "faker"
    if *scope == ShellScope::Global && args.is_empty()
        && let Some(target) = ShellScope::from_name(cmd)
            && target != ShellScope::Global {
                *scope = target;
                println!(
                    "Switched to {}{}{RESET} context. Type 'help' for commands, 'back' to return.",
                    target.color(),
                    target.as_str(),
                    RESET = colors::RESET
                );
                return Ok(false);
            }

    // Support scope prefix redundancy gracefully (e.g. typing "automa run" inside automa scope)
    if scope.is_scope_prefix(cmd) && !args.is_empty() {
        match *scope {
            ShellScope::Automa => return dispatch::dispatch_automa(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Runner => return dispatch::dispatch_runner(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Cloud => return dispatch::dispatch_cloud(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Browser => return dispatch::dispatch_browser(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Bridge => return dispatch::dispatch_bridge(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Faker => return dispatch::dispatch_faker(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Global => {}
        }
    }

    match *scope {
        ShellScope::Global => dispatch::dispatch_global(cmd, args).await?,
        ShellScope::Automa => dispatch::dispatch_automa(cmd, args).await?,
        ShellScope::Runner => dispatch::dispatch_runner(cmd, args).await?,
        ShellScope::Cloud => dispatch::dispatch_cloud(cmd, args).await?,
        ShellScope::Browser => dispatch::dispatch_browser(cmd, args).await?,
        ShellScope::Bridge => dispatch::dispatch_bridge(cmd, args).await?,
        ShellScope::Faker => dispatch::dispatch_faker(cmd, args).await?,
    }

    Ok(false)
}

pub fn print_scope_help(scope: ShellScope) {
    match scope {
        ShellScope::Global => {
            let mut card = Card::new("SPECTER SHELL");
            card.with_badge(badge_online("GLOBAL SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Scope Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  use <service>", "Switch active context: automa, runner, cloud, browser, bridge, faker");
            card.add_kv("  automa", "Enter browser automation scope directly");
            card.add_kv("  runner", "Enter worker daemon scope directly");
            card.add_kv("  cloud", "Enter cloud pairing scope directly");
            card.add_kv("  browser", "Enter browser runtime management scope");
            card.add_kv("  bridge", "Enter network bridge & tunnel mesh scope directly");
            card.add_kv("  faker", "Enter synthetic persona & CCCD generator scope");
            card.add_kv("  clear", "Clear terminal screen buffer");
            card.add_kv("  exit", "Quit interactive shell");
            card.add_line(format!("{BOLD}Direct Shortcuts:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  status", "Unified ecosystem health & subsystem status");
            card.add_kv("  doctor", "Diagnose environment dependencies (Scoop, Cloudflared, SSH, Browser)");
            card.add_kv("  upgrade", "Check and update Specter to latest release");
            card.add_kv("  whoami", "Inspect cloud enrollment identity & device ID");
            card.add_kv("  login [token]", "Authenticate workstation with Specter Cloud");
            card.add_kv("  card", "Display synthetic identity card (CCCD, credentials)");
            card.add_kv("  run <wf>", "Execute workflow (.json file or stored ID)");
            card.add_kv("  list [query]", "List stored workflows in vault & database");
            card.add_kv("  studio", "Launch Automa Web Studio in browser");
            card.add_kv("  install [ver]", "Download and install Antidetect Chromium (defaults to 148)");
            card.with_footer("Tip: Press [Tab] anytime for smart floating autocomplete");
            println!();
            card.print();
            println!();
        }
        ShellScope::Automa => {
            let mut card = Card::new("SPECTER SHELL");
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
            let mut card = Card::new("SPECTER SHELL");
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
            let mut card = Card::new("SPECTER SHELL");
            card.with_badge(badge_online("CLOUD SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Cloud Pairing Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  login [token]", "Authenticate and pair device with Specter Cloud");
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
            let mut card = Card::new("SPECTER SHELL");
            card.with_badge(badge_online("BROWSER SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Antidetect Browser Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  status", "Show installed Antidetect browser status, active version, and disk usage");
            card.add_kv("  launch [id]", "Launch browser profile with direct CDP DevTools bridge");
            card.add_kv("  verify [--url <url>]", "Verify stealth fingerprint against Cloudflare Turnstile");
            card.add_kv("  profile list", "List local browser profiles");
            card.add_kv("  profile create <name>", "Create isolated profile with deterministic fingerprint");
            card.add_kv("  profile inspect <id>", "Inspect profile hardware specs and disk size");
            card.add_kv("  profile pack <id>", "Pack profile into lightweight .tar.zst delta archive");
            card.add_kv("  profile unpack <path>", "Restore profile from .tar.zst archive");
            card.add_kv("  profile cloud list", "List remote cloud profiles and lease locks");
            card.add_kv("  profile cloud acquire <id>", "Acquire exclusive distributed lease lock");
            card.add_kv("  profile cloud release <id>", "Release distributed lease lock");
            card.add_kv("  list", "List installed Antidetect Chromium versions with active badge");
            card.add_kv("  search", "Search upstream releases in curated manifest");
            card.add_kv("  use <ver>", "Switch active version (e.g. 'use 148', 'use lts')");
            card.add_kv("  install [ver]", "Download and install Antidetect Chromium (defaults to 148)");
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
        ShellScope::Faker => {
            let mut card = Card::new("SPECTER SHELL");
            card.with_badge(badge_online("FAKER SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Synthetic Identity Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  generate [-n <count>] [-d <domain>]", "Generate synthetic profiles in terminal table");
            card.add_kv("  generate -f csv -o <path>", "Export profiles to CSV dataset");
            card.add_kv("  generate -f json", "Output profiles in formatted JSON");
            card.add_kv("  card [-d <domain>]", "Inspect single rich persona with verified CCCD");
            card.add_kv("  config [--edit] [-d <domain>]", "Display or configure email domains and templates");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
        ShellScope::Bridge => {
            let mut card = Card::new("SPECTER SHELL");
            card.with_badge(badge_online("BRIDGE SCOPE"));
            card.with_min_width(68);
            card.add_line(format!("{BOLD}Network Bridge & Tunnel Commands:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  status", "Show health check of all configured VPSs, ports, and workloads");
            card.add_kv("  start [server] [-t <tag>] [--http]", "Activate bridge connection(s) (default: all enabled)");
            card.add_kv("  stop [server | all]", "Stop running bridge tunnels and proxy daemons");
            card.add_kv("  enable <server>", "Enable a server in ~/.specter/bridge/bridge.json");
            card.add_kv("  disable <server>", "Disable a server in ~/.specter/bridge/bridge.json");
            card.add_kv("  check", "Validate ~/.specter/bridge/bridge.json schema, ports, and workloads");
            card.add_kv("  config [--edit]", "Display or open bridge.json in default editor");
            card.add_line(format!("{BOLD}Navigation:{RESET}", BOLD = colors::BOLD, RESET = colors::RESET));
            card.add_kv("  back / cd ..", "Return to global scope");
            card.add_kv("  exit", "Return to global scope (or quit)");
            println!();
            card.print();
            println!();
        }
    }
}
