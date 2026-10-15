use reedline::{Completer, CompletionResult, Span, Suggestion};
use super::scope::ShellScope;

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

fn suggest_workflows(current_word: &str, span: Span, suggestions: &mut Vec<Suggestion>) {
    let vault_dir = crate::config::canonical_ssot_dir().join("automa").join("workflows");
    if let Ok(entries) = std::fs::read_dir(vault_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().map(|e| e == "json").unwrap_or(false)
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                let clean_id = stem.trim_end_matches(".workflow");
                if clean_id.to_lowercase().starts_with(&current_word.to_lowercase()) {
                    suggestions.push(make_suggestion(clean_id, "Workflow from vault", span));
                }
            }
        }
    }
}

pub fn complete_dev_for_packages(
    packages: &[crate::commands::dev::scanner::DiscoveredPackage],
    args: &[&str],
    last_word: &str,
    current_word: &str,
    span: Span,
    suggestions: &mut Vec<Suggestion>,
) {
    let matches_filter = |item: &str, query: &str| -> bool {
        item.to_lowercase().starts_with(&query.to_lowercase())
    };

    if last_word == "--pm" {
        for pm in &["pnpm", "bun", "yarn", "npm"] {
            if matches_filter(pm, current_word) {
                suggestions.push(make_suggestion(*pm, "Package manager override", span));
            }
        }
        return;
    }

    if matches!(last_word, "-f" | "--format") {
        for fmt in &["card", "table", "json"] {
            if matches_filter(fmt, current_word) {
                suggestions.push(make_suggestion(*fmt, "Output format option", span));
            }
        }
        return;
    }

    // Extract positional arguments typed before `current_word`
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--pm" | "--path" | "-P" | "--format" | "-f" => {
                i += 1; // skip flag value
            }
            arg if !arg.starts_with('-') => {
                positional.push(arg);
            }
            _ => {}
        }
        i += 1;
    }

    if positional.is_empty() {
        // 1. If user is typing `<project>:<script>` shorthand
        if let Some(colon_idx) = current_word.find(':')
            && colon_idx > 0
        {
            let proj_part = &current_word[..colon_idx];
            let matched = crate::commands::dev::scanner::filter_packages(packages, proj_part);
            for pkg in matched {
                for (script_name, script_cmd) in &pkg.scripts {
                    let candidate = format!("{}:{}", proj_part, script_name);
                    if matches_filter(&candidate, current_word) {
                        let desc = format!(
                            "[{}] {} run {} → {}",
                            pkg.id, pkg.package_manager, script_name, script_cmd
                        );
                        suggestions.push(make_suggestion(candidate, desc, span));
                    }
                }
            }
            return;
        }

        // 2. Suggest discovered frontend projects + `<project>:dev` shorthand
        let mut seen = std::collections::HashSet::new();
        for pkg in packages {
            let fw = pkg.framework.as_deref().unwrap_or("Node");
            let scripts_preview: Vec<&str> =
                pkg.scripts.iter().take(4).map(|(k, _)| k.as_str()).collect();
            let desc = format!(
                "[{} • {}] {} (scripts: {})",
                pkg.package_manager,
                fw,
                pkg.name.as_deref().unwrap_or(&pkg.id),
                scripts_preview.join(", ")
            );

            if matches_filter(&pkg.id, current_word) && seen.insert(pkg.id.clone()) {
                suggestions.push(make_suggestion(&pkg.id, &desc, span));
            }

            let short = pkg.short_name();
            if short != pkg.id
                && matches_filter(short, current_word)
                && seen.insert(short.to_string())
            {
                suggestions.push(make_suggestion(short, &desc, span));
            }
        }

        // 3. Suggest flags
        for (flag, desc) in &[
            ("--list", "List all discovered package.json projects in workspace"),
            ("-l", "List all discovered package.json projects in workspace"),
            ("--pm", "Override package manager (pnpm, bun, yarn, npm)"),
            ("--path", "Custom workspace root directory to scan"),
        ] {
            if matches_filter(flag, current_word) {
                suggestions.push(make_suggestion(*flag, *desc, span));
            }
        }
    } else if positional.len() == 1 {
        // Completing 2nd positional arg: `dev <project> <script>`
        let proj_query = positional[0];
        let matched = crate::commands::dev::scanner::filter_packages(packages, proj_query);
        let mut seen = std::collections::HashSet::new();
        for pkg in matched {
            for (script_name, script_cmd) in &pkg.scripts {
                if matches_filter(script_name, current_word) && seen.insert(script_name.clone()) {
                    let desc = format!(
                        "{} run {} → {}",
                        pkg.package_manager, script_name, script_cmd
                    );
                    suggestions.push(make_suggestion(script_name, desc, span));
                }
            }
        }
    }
}

fn suggest_dev_completions(
    args: &[&str],
    last_word: &str,
    current_word: &str,
    span: Span,
    suggestions: &mut Vec<Suggestion>,
) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut custom_root: Option<std::path::PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        if matches!(args[i], "--path" | "-P")
            && let Some(val) = args.get(i + 1)
        {
            custom_root = Some(std::path::PathBuf::from(*val));
            break;
        }
        i += 1;
    }

    let packages = match custom_root {
        Some(ref root) => crate::commands::dev::scanner::scan_workspace_packages(root),
        None => cached_machine_packages(&cwd),
    };
    complete_dev_for_packages(&packages, args, last_word, current_word, span, suggestions);
}

fn cached_machine_packages(
    cwd: &std::path::Path,
) -> Vec<crate::commands::dev::scanner::DiscoveredPackage> {
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    type CacheEntry = (
        Instant,
        std::path::PathBuf,
        Vec<crate::commands::dev::scanner::DiscoveredPackage>,
    );
    static CACHE: OnceLock<Mutex<Option<CacheEntry>>> = OnceLock::new();

    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock()
        && let Some((ts, ref cached_cwd, ref pkgs)) = *guard
        && ts.elapsed() < Duration::from_secs(15)
        && cached_cwd == cwd
    {
        return pkgs.clone();
    }

    let pkgs = crate::commands::dev::scanner::scan_machine_packages(cwd);
    if let Ok(mut guard) = lock.lock() {
        *guard = Some((Instant::now(), cwd.to_path_buf(), pkgs.clone()));
    }
    pkgs
}

pub struct SpecterCompleter {
    pub scope: ShellScope,
}

impl Completer for SpecterCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let prefix = if pos <= line.len() { &line[..pos] } else { line };
        let start = prefix.rfind(|c: char| c.is_whitespace()).map(|idx| idx + 1).unwrap_or(0);
        let current_word = &prefix[start..];
        let span = Span::new(start, pos);

        let words_before: Vec<&str> = prefix[..start].split_whitespace().collect();
        let mut suggestions = Vec::new();

        let matches_filter = |item: &str, query: &str| -> bool {
            item.to_lowercase().starts_with(&query.to_lowercase())
        };

        if words_before.is_empty() {
            // Completing command name based on current scope
            // 1. Universal commands
            for (cmd, desc) in ShellScope::UNIVERSAL_COMMANDS {
                if matches_filter(cmd, current_word) {
                    suggestions.push(make_suggestion(*cmd, *desc, span));
                }
            }

            // 2. Subcommands of the active scope
            for (cmd, desc) in self.scope.subcommands() {
                if matches_filter(cmd, current_word) {
                    suggestions.push(make_suggestion(*cmd, *desc, span));
                }
            }
            return CompletionResult::fresh(suggestions);
        }

        // Scope navigation: `use <scope>`
        if words_before.len() == 1 && words_before[0].eq_ignore_ascii_case("use") {
            for (s, desc) in ShellScope::ALL_SCOPES {
                if matches_filter(s, current_word) {
                    suggestions.push(make_suggestion(*s, *desc, span));
                }
            }
            return CompletionResult::fresh(suggestions);
        }

        // Normalize scope and command context:
        // Handles:
        //   In Global: `bridge start ...`, `faker generate ...`, `run ...`
        //   In Sub-scope with prefix: `bridge start ...`, `faker config ...`
        //   In Sub-scope normal: `start ...`, `config ...`
        let (effective_scope, remaining_words) = if self.scope == ShellScope::Global {
            if let Some(target) = words_before.first().and_then(|w| ShellScope::from_name(w)) {
                (target, &words_before[1..])
            } else {
                (ShellScope::Global, &words_before[..])
            }
        } else if self.scope.is_scope_prefix(words_before[0]) {
            (self.scope, &words_before[1..])
        } else {
            (self.scope, &words_before[..])
        };

        // If at the boundary of a scope prefix (e.g. `bridge <Tab>`, `faker <Tab>`)
        if remaining_words.is_empty() {
            for (cmd, desc) in effective_scope.subcommands() {
                if matches_filter(cmd, current_word) {
                    suggestions.push(make_suggestion(*cmd, *desc, span));
                }
            }
            return CompletionResult::fresh(suggestions);
        }

        let cmd = remaining_words[0].to_lowercase();
        let args = &remaining_words[1..];
        let last_word = words_before.last().copied().unwrap_or("");

        match effective_scope {
            ShellScope::Bridge => {
                match cmd.as_str() {
                    "start" => {
                        let config = crate::infrastructure::bridge::BridgeConfig::load()
                            .unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
                        if matches!(last_word, "-t" | "--tag") {
                            let mut all_tags: std::collections::BTreeSet<String> = config
                                .servers
                                .values()
                                .flat_map(|s| s.tags.clone())
                                .collect();
                            if all_tags.is_empty() {
                                all_tags.insert("primary".to_string());
                                all_tags.insert("git".to_string());
                                all_tags.insert("mmo".to_string());
                            }
                            for tag in all_tags {
                                if matches_filter(&tag, current_word) {
                                    suggestions.push(make_suggestion(tag, "Bridge server tag", span));
                                }
                            }
                        } else {
                            for id in config.servers.keys() {
                                if matches_filter(id, current_word) {
                                    suggestions.push(make_suggestion(id, "Configured VPS server target", span));
                                }
                            }
                            if matches_filter("all", current_word) {
                                suggestions.push(make_suggestion("all", "All configured servers", span));
                            }
                            for flag in &["--http", "-t", "--tag"] {
                                if matches_filter(flag, current_word) {
                                    let desc = match *flag {
                                        "--http" => "Start embedded HTTP adapter (port 8118)",
                                        "-t" | "--tag" => "Filter servers by tag",
                                        _ => "",
                                    };
                                    suggestions.push(make_suggestion(*flag, desc, span));
                                }
                            }
                        }
                    }
                    "stop" => {
                        let config = crate::infrastructure::bridge::BridgeConfig::load()
                            .unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
                        for id in config.servers.keys() {
                            if matches_filter(id, current_word) {
                                suggestions.push(make_suggestion(id, "Configured VPS server target", span));
                            }
                        }
                        if matches_filter("all", current_word) {
                            suggestions.push(make_suggestion("all", "All configured servers", span));
                        }
                    }
                    "enable" | "disable" => {
                        let config = crate::infrastructure::bridge::BridgeConfig::load()
                            .unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
                        for id in config.servers.keys() {
                            if matches_filter(id, current_word) {
                                suggestions.push(make_suggestion(id, "Configured VPS server target", span));
                            }
                        }
                    }
                    "config" => {
                        for (flag, desc) in &[
                            ("--edit", "Open bridge.json in default editor"),
                            ("-e", "Open bridge.json in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Faker => {
                match cmd.as_str() {
                    "generate" | "gen" | "g" | "card" | "show" | "inspect" => {
                        // Value-specific completions based on previous flag:
                        if matches!(last_word, "-g" | "--gender") {
                            for g in &["male", "female", "all"] {
                                if matches_filter(g, current_word) {
                                    suggestions.push(make_suggestion(*g, "Gender filter option", span));
                                }
                            }
                        } else if last_word == "--nat" {
                            for n in &["VN", "US", "JP", "all"] {
                                if matches_filter(n, current_word) {
                                    suggestions.push(make_suggestion(*n, "Nationality filter option", span));
                                }
                            }
                        } else if last_word == "--avatar" {
                            for a in &["real", "svg"] {
                                if matches_filter(a, current_word) {
                                    suggestions.push(make_suggestion(*a, "Avatar style option", span));
                                }
                            }
                        } else if matches!(last_word, "-f" | "--format") {
                            for f in &["table", "card", "json", "csv"] {
                                if matches_filter(f, current_word) {
                                    suggestions.push(make_suggestion(*f, "Output format option", span));
                                }
                            }
                        } else if matches!(last_word, "-d" | "--domain") {
                            let faker_cfg = specter_faker::FakerConfig::load();
                            for d in &faker_cfg.email_domains {
                                if matches_filter(d, current_word) {
                                    suggestions.push(make_suggestion(d.as_str(), "Configured email domain", span));
                                }
                            }
                        } else {
                            // General flag suggestions:
                            let is_generate = matches!(cmd.as_str(), "generate" | "gen" | "g");
                            let mut flags = vec![
                                ("-d", "Custom email domain override (e.g. flowup.io.vn)"),
                                ("--domain", "Custom email domain override"),
                                ("-g", "Gender filter (male, female, all)"),
                                ("--gender", "Gender filter (male, female, all)"),
                                ("--nat", "Nationality filter (VN, US, JP, all)"),
                                ("--avatar", "Avatar style (real, svg)"),
                            ];
                            if is_generate {
                                flags.extend_from_slice(&[
                                    ("-n", "Number of profiles to generate"),
                                    ("--count", "Number of profiles to generate"),
                                    ("-f", "Format: table, card, json, csv"),
                                    ("--format", "Format: table, card, json, csv"),
                                    ("-o", "Export output to file path"),
                                    ("--output", "Export output to file path"),
                                ]);
                            }
                            for (flag, desc) in flags {
                                if matches_filter(flag, current_word) {
                                    suggestions.push(make_suggestion(flag, desc, span));
                                }
                            }
                        }
                    }
                    "config" => {
                        if matches!(last_word, "-d" | "--domain") {
                            let faker_cfg = specter_faker::FakerConfig::load();
                            for d in &faker_cfg.email_domains {
                                if matches_filter(d, current_word) {
                                    suggestions.push(make_suggestion(d.as_str(), "Configured email domain", span));
                                }
                            }
                        } else {
                            for (flag, desc) in &[
                                ("--edit", "Open faker.json in default editor"),
                                ("-e", "Open faker.json in default editor"),
                                ("--show", "Display structured configuration details and summary card"),
                                ("-s", "Display structured configuration details"),
                                ("--domain", "Set default email domain (e.g. flowup.io.vn)"),
                                ("-d", "Set default email domain"),
                            ] {
                                if matches_filter(flag, current_word) {
                                    suggestions.push(make_suggestion(*flag, *desc, span));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Automa => {
                match cmd.as_str() {
                    "run" | "inspect" | "export" | "delete" => {
                        // 1. Vault workflow files
                        suggest_workflows(current_word, span, &mut suggestions);

                        // 2. Flags
                        let mut flags = vec![
                            ("-h", "Headless browser mode"),
                            ("--headless", "Headless browser mode"),
                            ("-t", "Timeout in seconds"),
                            ("--timeout", "Timeout in seconds"),
                            ("-v", "Inject runtime variables (JSON)"),
                            ("--variables", "Inject runtime variables (JSON)"),
                            ("--browser", "Specific browser executable path or profile"),
                        ];
                        if cmd == "delete" {
                            flags.push(("--vault", "Delete also from local disk file vault"));
                        }
                        for (flag, desc) in flags {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(flag, desc, span));
                            }
                        }
                    }
                    "studio" => {
                        for (flag, desc) in &[
                            ("-p", "Studio HTTP port (default: 8765)"),
                            ("--port", "Studio HTTP port (default: 8765)"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "config" => {
                        for (flag, desc) in &[
                            ("--edit", "Open automa.json in default editor"),
                            ("-e", "Open automa.json in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Browser => {
                match cmd.as_str() {
                    "install" => {
                        for (flag, desc) in &[
                            ("-f", "Force reinstall Chromium runtime"),
                            ("--force", "Force reinstall Chromium runtime"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "ext" => {
                        if args.is_empty() {
                            let ext_subcmds = [
                                ("catalog", "Browse available extensions in catalog"),
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
                                if matches_filter(sc, current_word) {
                                    suggestions.push(make_suggestion(*sc, *desc, span));
                                }
                            }
                        } else if args.len() == 1 {
                            match args[0] {
                                "install" => {
                                    for ext in &["automa", "ublock-origin", "captcha-solver"] {
                                        if matches_filter(ext, current_word) {
                                            suggestions.push(make_suggestion(*ext, "Catalog extension ID", span));
                                        }
                                    }
                                }
                                "remove" | "enable" | "disable" | "info" | "path" | "launch" => {
                                    let ext = "automa";
                                    if matches_filter(ext, current_word) {
                                        suggestions.push(make_suggestion(ext, "Installed extension ID", span));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "config" => {
                        for (flag, desc) in &[
                            ("--edit", "Open browser.json in default editor"),
                            ("-e", "Open browser.json in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "profile" => {
                        if args.is_empty() {
                            let profile_subcmds = [
                                ("list", "List all local browser profiles"),
                                ("create", "Create new isolated profile with deterministic fingerprint"),
                                ("inspect", "Inspect profile hardware specs and disk size"),
                                ("delete", "Delete a browser profile and sandbox data"),
                                ("pack", "Pack profile into .tar.zst delta archive"),
                                ("unpack", "Restore profile from .tar.zst archive"),
                                ("test-proxy", "Probe and test proxy configured for profile"),
                                ("cloud", "Manage cloud-synchronized browser profiles and lease locks"),
                            ];
                            for (sc, desc) in &profile_subcmds {
                                if matches_filter(sc, current_word) {
                                    suggestions.push(make_suggestion(*sc, *desc, span));
                                }
                            }
                        } else if args.len() == 1 && args[0] == "cloud" {
                            let cloud_subcmds = [
                                ("list", "List remote cloud profiles and lease locks"),
                                ("acquire", "Acquire exclusive distributed lease lock"),
                                ("release", "Release distributed lease lock"),
                            ];
                            for (sc, desc) in &cloud_subcmds {
                                if matches_filter(sc, current_word) {
                                    suggestions.push(make_suggestion(*sc, *desc, span));
                                }
                            }
                        }
                    }
                    "launch" => {
                        for (flag, desc) in &[
                            ("--port", "CDP DevTools remote debugging port (default: 9222)"),
                            ("--headless", "Run browser in headless background mode"),
                            ("--url", "Initial URL to navigate to"),
                            ("--proxy", "Override proxy URL (e.g. socks5://127.0.0.1:1080)"),
                            ("--detach", "Run in background without keeping terminal attached"),
                            ("--no-cdp", "Ultra-stealth zero-port mode"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Runner => {
                match cmd.as_str() {
                    "start" | "restart" => {
                        for (flag, desc) in &[
                            ("-d", "Run daemon in background (detached)"),
                            ("--detach", "Run daemon in background (detached)"),
                            ("-p", "Daemon server port (default: 8765)"),
                            ("--port", "Daemon server port (default: 8765)"),
                            ("--host", "Daemon server host (default: 127.0.0.1)"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "stop" => {
                        for (flag, desc) in &[
                            ("-f", "Force kill daemon process"),
                            ("--force", "Force kill daemon process"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "logs" => {
                        for (flag, desc) in &[
                            ("-f", "Follow and stream daemon logs"),
                            ("--follow", "Follow and stream daemon logs"),
                            ("-n", "Number of lines to show (default: 50)"),
                            ("--lines", "Number of lines to show"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "config" => {
                        for (flag, desc) in &[
                            ("--edit", "Open runner.json in default editor"),
                            ("-e", "Open runner.json in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Cloud => {
                match cmd.as_str() {
                    "login" => {
                        for (flag, desc) in &[
                            ("-u", "Specter Cloud API endpoint URL"),
                            ("--url", "Specter Cloud API endpoint URL"),
                            ("-t", "Cloud access / pairing token"),
                            ("--token", "Cloud access / pairing token"),
                            ("-n", "Workstation display name"),
                            ("--name", "Workstation display name"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    "config" => {
                        for (flag, desc) in &[
                            ("--edit", "Open system.json in default editor"),
                            ("-e", "Open system.json in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ShellScope::Global => {
                // If in Global and user typed a direct shortcut like `dev`, `run`, `card`, `install`, `start`, etc.
                match cmd.as_str() {
                    "dev" | "fe" | "pkg" => {
                        suggest_dev_completions(args, last_word, current_word, span, &mut suggestions);
                    }
                    "run" | "inspect" | "export" | "delete" => {
                        suggest_workflows(current_word, span, &mut suggestions);
                        for flag in &["-h", "--headless", "-t", "--timeout", "-v", "--variables", "--browser"] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, "Execution option", span));
                            }
                        }
                    }
                    "card" => {
                        for flag in &["-d", "--domain", "-g", "--gender", "--nat", "--avatar"] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, "Identity option", span));
                            }
                        }
                    }
                    "start" | "stop" | "enable" | "disable" => {
                        let config = crate::infrastructure::bridge::BridgeConfig::load()
                            .unwrap_or_else(|_| crate::infrastructure::bridge::BridgeConfig::default_config());
                        if cmd == "start" && matches!(last_word, "-t" | "--tag") {
                            let mut all_tags: std::collections::BTreeSet<String> = config
                                .servers
                                .values()
                                .flat_map(|s| s.tags.clone())
                                .collect();
                            if all_tags.is_empty() {
                                all_tags.insert("primary".to_string());
                                all_tags.insert("git".to_string());
                                all_tags.insert("mmo".to_string());
                            }
                            for tag in all_tags {
                                if matches_filter(&tag, current_word) {
                                    suggestions.push(make_suggestion(tag, "Bridge server tag", span));
                                }
                            }
                        } else {
                            for id in config.servers.keys() {
                                if matches_filter(id, current_word) {
                                    suggestions.push(make_suggestion(id, "Configured VPS server target", span));
                                }
                            }
                            if (cmd == "start" || cmd == "stop") && matches_filter("all", current_word) {
                                suggestions.push(make_suggestion("all", "All configured servers", span));
                            }
                            if cmd == "start" {
                                for flag in &["--http", "-t", "--tag"] {
                                    if matches_filter(flag, current_word) {
                                        let desc = match *flag {
                                            "--http" => "Start embedded HTTP adapter (port 8118)",
                                            "-t" | "--tag" => "Filter servers by tag",
                                            _ => "",
                                        };
                                        suggestions.push(make_suggestion(*flag, desc, span));
                                    }
                                }
                            }
                        }
                    }
                    "install" => {
                        for flag in &["-f", "--force"] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, "Install option", span));
                            }
                        }
                    }
                    "logs" => {
                        for flag in &["-f", "--follow", "-n", "--lines"] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, "Log stream option", span));
                            }
                        }
                    }
                    "config" => {
                        for s in &["bridge", "faker", "browser", "automa", "runner", "cloud", "system"] {
                            if matches_filter(s, current_word) {
                                suggestions.push(make_suggestion(*s, "Target microservice configuration", span));
                            }
                        }
                        for (flag, desc) in &[
                            ("--edit", "Open configuration in default editor"),
                            ("-e", "Open configuration in default editor"),
                            ("--show", "Display structured configuration details and summary card"),
                            ("-s", "Display structured configuration details"),
                        ] {
                            if matches_filter(flag, current_word) {
                                suggestions.push(make_suggestion(*flag, *desc, span));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CompletionResult::fresh(suggestions)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::dev::scanner::DiscoveredPackage;
    use std::path::PathBuf;

    fn extract_values(res: CompletionResult) -> Vec<String> {
        match res {
            CompletionResult::Fresh { suggestions, .. } => {
                suggestions.iter().map(|s| s.value.clone()).collect()
            }
            _ => vec![],
        }
    }

    #[test]
    fn test_completer_dev_projects_and_scripts() {
        let packages = vec![
            DiscoveredPackage {
                id: "apps/studio".to_string(),
                name: Some("@tuquet/studio".to_string()),
                dir: PathBuf::from("apps/studio"),
                package_manager: "pnpm".to_string(),
                framework: Some("Vite".to_string()),
                scripts: vec![
                    ("dev".to_string(), "vite".to_string()),
                    ("build".to_string(), "vite build".to_string()),
                    ("preview".to_string(), "vite preview".to_string()),
                ],
            },
            DiscoveredPackage {
                id: "web-portal".to_string(),
                name: Some("web-portal".to_string()),
                dir: PathBuf::from("web-portal"),
                package_manager: "bun".to_string(),
                framework: Some("Next.js".to_string()),
                scripts: vec![
                    ("dev".to_string(), "next dev".to_string()),
                    ("lint".to_string(), "next lint".to_string()),
                ],
            },
        ];

        // 1. `dev ` -> suggests project IDs, short names, and flags
        let mut suggestions = Vec::new();
        complete_dev_for_packages(&packages, &[], "dev", "", Span::new(4, 4), &mut suggestions);
        let vals: Vec<String> = suggestions.iter().map(|s| s.value.clone()).collect();
        assert!(vals.contains(&"apps/studio".to_string()));
        assert!(vals.contains(&"studio".to_string()));
        assert!(vals.contains(&"web-portal".to_string()));
        assert!(vals.contains(&"--list".to_string()));

        // 2. `dev studio ` -> suggests scripts from studio's package.json
        let mut script_suggestions = Vec::new();
        complete_dev_for_packages(
            &packages,
            &["studio"],
            "studio",
            "",
            Span::new(11, 11),
            &mut script_suggestions,
        );
        let script_vals: Vec<String> = script_suggestions.iter().map(|s| s.value.clone()).collect();
        assert_eq!(script_vals, vec!["dev", "build", "preview"]);

        // 3. `dev studio:b` -> suggests `studio:build`
        let mut colon_suggestions = Vec::new();
        complete_dev_for_packages(
            &packages,
            &[],
            "dev",
            "studio:b",
            Span::new(4, 12),
            &mut colon_suggestions,
        );
        let colon_vals: Vec<String> = colon_suggestions.iter().map(|s| s.value.clone()).collect();
        assert_eq!(colon_vals, vec!["studio:build"]);
    }

    #[test]
    fn test_completer_bridge_start_and_multiword() {
        let mut completer = SpecterCompleter { scope: ShellScope::Bridge };

        // "start " -> suggestions should include "all" and server names
        let res = completer.complete("start ", 6);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "all"), "Should suggest 'all' for start");

        // "start my-vps -" -> suggestions should include "--http"
        let res = completer.complete("start my-vps -", 14);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "--http"), "Should suggest '--http' after server target");

        // "enable " -> suggestions should include configured servers
        let res = completer.complete("enable ", 7);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "my-vps"), "Should suggest 'my-vps' for enable");

        // "disable " -> suggestions should include configured servers
        let res = completer.complete("disable ", 8);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "my-vps"), "Should suggest 'my-vps' for disable");

        // "start -t " -> suggestions should include server tags
        let res = completer.complete("start -t ", 9);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "primary" || v == "git"), "Should suggest server tags for start -t");
    }

    #[test]
    fn test_completer_faker_flags_and_values() {
        let mut completer = SpecterCompleter { scope: ShellScope::Faker };

        // "generate -g " -> should suggest gender values
        let res = completer.complete("generate -g ", 12);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "male"));
        assert!(vals.iter().any(|v| v == "female"));

        // "generate -n 5 -" -> should suggest flags even after arguments
        let res = completer.complete("generate -n 5 -", 15);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "-d" || v == "--domain"));
    }

    #[test]
    fn test_completer_global_delegation_and_case_insensitive() {
        let mut completer = SpecterCompleter { scope: ShellScope::Global };

        // "bridge " -> should suggest bridge subcommands
        let res = completer.complete("bridge ", 7);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "status"));
        assert!(vals.iter().any(|v| v == "start"));

        // Case-insensitivity: "uSe " -> should suggest scopes
        let res = completer.complete("uSe ", 4);
        let vals = extract_values(res);
        assert!(vals.iter().any(|v| v == "bridge"));
        assert!(vals.iter().any(|v| v == "faker"));
    }
}


