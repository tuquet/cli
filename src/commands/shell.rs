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
    Faker,
    Bridge,
}

impl ShellScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShellScope::Global => "global",
            ShellScope::Automa => "automa",
            ShellScope::Runner => "runner",
            ShellScope::Cloud => "cloud",
            ShellScope::Browser => "browser",
            ShellScope::Faker => "faker",
            ShellScope::Bridge => "bridge",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            ShellScope::Global => colors::CYAN,
            ShellScope::Automa => colors::AMBER,
            ShellScope::Runner => colors::GREEN,
            ShellScope::Cloud => colors::PURPLE,
            ShellScope::Browser => "\x1b[38;2;96;165;250m", // Blue
            ShellScope::Faker => colors::CYAN,
            ShellScope::Bridge => "\x1b[38;2;217;70;239m", // Magenta
        }
    }

    /// All scope names and descriptions for `use <scope>`
    pub const ALL_SCOPES: &'static [(&'static str, &'static str)] = &[
        ("global", "Ecosystem dashboard & universal orchestrator"),
        ("automa", "Browser workflow automation engine"),
        ("runner", "Distributed daemon worker node"),
        ("cloud", "Specter Cloud (Supabase) authentication"),
        ("browser", "Dedicated isolated Chromium LTS runtime"),
        ("bridge", "Network bridge & multi-VPS proxy mesh"),
        ("faker", "Synthetic persona & CCCD test data generator"),
    ];

    /// Universal commands available across ALL scopes
    pub const UNIVERSAL_COMMANDS: &'static [(&'static str, &'static str)] = &[
        ("doctor", "Inspect ecosystem dependencies, required tools & environment health"),
        ("use", "Switch active service context"),
        ("back", "Return to global scope"),
        ("help", "Print scope help manual"),
        ("?", "Alias for help"),
        ("clear", "Clear terminal screen"),
        ("cls", "Alias for clear"),
        ("exit", "Return to global scope (or quit)"),
        ("quit", "Exit Specter shell session"),
        ("q", "Short alias for quit"),
    ];

    /// Subcommands specifically belonging to this scope
    pub fn subcommands(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            ShellScope::Global => &[
                ("status", "Inspect unified status across Cloud, Runner, Browser, and Bridge"),
                ("doctor", "Inspect ecosystem dependencies, required tools & environment health"),
                ("config", "Inspect or edit configuration across all microservice pillars"),
                ("whoami", "Check active cloud enrollment identity & device ID"),
                ("login", "Authenticate workstation with Specter Cloud"),
                ("logout", "Disconnect and remove local cloud credentials"),
                ("upgrade", "Check and update Specter to latest release"),
                ("update", "Alias for upgrade"),
                ("automa", "Enter browser automation scope directly"),
                ("runner", "Enter distributed runner daemon scope directly"),
                ("cloud", "Enter cloud authentication scope directly"),
                ("browser", "Enter browser runtime management scope directly"),
                ("bridge", "Enter network bridge & tunnel mesh scope directly"),
                ("faker", "Synthetic persona & CCCD test data generator"),
                ("user", "Alias for faker synthetic identity generator"),
                ("run", "Execute a workflow (.json or stored ID)"),
                ("list", "List stored workflows in vault and database"),
                ("inspect", "Inspect workflow node-graph structure"),
                ("import", "Import a workflow file into local storage"),
                ("export", "Export a workflow to a JSON file"),
                ("delete", "Delete a workflow from local storage"),
                ("studio", "Launch Automa Web Studio"),
                ("install", "Download and install Antidetect Chromium (defaults to 148)"),
                ("clean", "Purge installed browser runtime"),
                ("path", "Print absolute path to browser executable"),
                ("card", "Display synthetic identity card (CCCD, credentials)"),
            ],
            ShellScope::Automa => &[
                ("run", "Execute a workflow (.json file or stored ID)"),
                ("list", "List stored workflows in vault and database"),
                ("inspect", "Inspect and validate a workflow structure"),
                ("import", "Import a workflow file into local storage"),
                ("export", "Export a workflow to a JSON file"),
                ("delete", "Delete a workflow from local storage"),
                ("studio", "Launch Automa Web Studio"),
                ("config", "Display path, inspect (--show), or edit (--edit) automa.json"),
            ],
            ShellScope::Runner => &[
                ("start", "Start the local runner daemon server"),
                ("stop", "Gracefully terminate the runner daemon"),
                ("restart", "Restart the local runner daemon"),
                ("status", "Inspect local runner daemon health check"),
                ("logs", "View background runner daemon logs"),
                ("probe", "Probe runner driver manifest capabilities"),
                ("config", "Display path, inspect (--show), or edit (--edit) runner.json"),
            ],
            ShellScope::Cloud => &[
                ("login", "Authenticate and pair device with Specter Cloud"),
                ("logout", "Log out and remove local cloud credentials"),
                ("whoami", "Check active cloud pairing and enrollment"),
                ("config", "Display path, inspect (--show), or edit (--edit) system.json"),
            ],
            ShellScope::Browser => &[
                ("status", "Show Antidetect browser status, active version, and disk usage"),
                ("list", "List installed Antidetect Chromium versions"),
                ("search", "Search upstream releases in curated manifest"),
                ("use", "Switch active Antidetect Chromium version (e.g. use 148, use lts)"),
                ("install", "Download and install Antidetect Chromium (defaults to Golden LTS)"),
                ("clean", "Delete browser runtime directory to reclaim disk"),
                ("path", "Print absolute path to browser executable"),
                ("ext", "Inspect and configure Automa MV3 extension"),
                ("config", "Display path, inspect (--show), or edit (--edit) browser.json"),
            ],
            ShellScope::Faker => &[
                ("generate", "Generate synthetic persona profiles (CCCD, address, email)"),
                ("gen", "Short alias for generate"),
                ("g", "Short alias for generate"),
                ("card", "Display rich synthetic identity card"),
                ("show", "Alias for card"),
                ("inspect", "Alias for card"),
                ("config", "Display path, inspect (--show), or edit (--edit) faker.json"),
            ],
            ShellScope::Bridge => &[
                ("status", "Show health check of all configured VPSs and ports"),
                ("start", "Start bridge connection (default: all enabled)"),
                ("stop", "Stop running bridge tunnels and proxy daemons"),
                ("enable", "Enable a server in bridge configuration"),
                ("disable", "Disable a server in bridge configuration"),
                ("check", "Validate ~/.specter/bridge/bridge.json configuration"),
                ("config", "Display path, inspect (--show), or edit (--edit) bridge.json"),
            ],
        }
    }

    /// Check if word matches this scope name or its alias
    pub fn is_scope_prefix(&self, word: &str) -> bool {
        if *self == ShellScope::Global {
            false
        } else {
            Self::from_name(word) == Some(*self)
        }
    }

    /// Check if word is valid as the first command in this scope
    pub fn is_valid_first_word(&self, word: &str) -> bool {
        if Self::UNIVERSAL_COMMANDS.iter().any(|(cmd, _)| cmd.eq_ignore_ascii_case(word)) {
            return true;
        }
        if self.is_scope_prefix(word) {
            return true;
        }
        if *self == ShellScope::Global && Self::ALL_SCOPES.iter().any(|(s, _)| s.eq_ignore_ascii_case(word) || (*s == "faker" && word.eq_ignore_ascii_case("user"))) {
            return true;
        }
        self.subcommands().iter().any(|(cmd, _)| cmd.eq_ignore_ascii_case(word))
    }

    /// Check if word is a valid subcommand of this scope
    pub fn is_valid_subcommand(&self, word: &str) -> bool {
        self.subcommands().iter().any(|(cmd, _)| cmd.eq_ignore_ascii_case(word))
    }

    /// Resolve a scope by name or alias
    pub fn from_name(name: &str) -> Option<ShellScope> {
        if name.eq_ignore_ascii_case("global") {
            return Some(ShellScope::Global);
        }
        match crate::config::ConfigRegistry::canonical_service(name) {
            Some("automa") => Some(ShellScope::Automa),
            Some("runner") => Some(ShellScope::Runner),
            Some("system") => Some(ShellScope::Cloud),
            Some("browser") => Some(ShellScope::Browser),
            Some("bridge") => Some(ShellScope::Bridge),
            Some("faker") => Some(ShellScope::Faker),
            _ => None,
        }
    }
}

pub struct SpecterPrompt {
    pub scope: ShellScope,
    pub cloud_env: String,
}

impl Prompt for SpecterPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let border = colors::BORDER;
        let cyan = colors::CYAN;
        let bold = colors::BOLD;
        let reset = colors::RESET;
        let muted = colors::MUTED;
        let green = colors::GREEN;

        let scope_str = self.scope.as_str();
        let scope_color = self.scope.color();

        let env_badge = if self.cloud_env.is_empty() {
            format!("{muted}○ cloud:local{reset}")
        } else {
            format!("{green}● {}{reset}", self.cloud_env)
        };

        let line = format!(
            "{border}╭─{reset} {bold}{cyan}⚡ specter{reset}  {env_badge}  {border}[{reset}{scope_color}{scope_str}{reset}{border}]{reset}\n{border}╰─{reset}"
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

pub struct SpecterHighlighter {
    pub scope: ShellScope,
}

impl Highlighter for SpecterHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        if line.is_empty() {
            return styled;
        }

        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 0 {
            styled.push((Style::new(), line[..leading_spaces].to_string()));
        }

        let mut remaining = &line[leading_spaces..];
        let mut first_word: Option<&str> = None;
        let mut word_index = 0;

        while !remaining.is_empty() {
            let next_word_end = remaining.find(char::is_whitespace).unwrap_or(remaining.len());
            let word = &remaining[..next_word_end];

            if word_index == 0 {
                first_word = Some(word);
                if self.scope.is_valid_first_word(word) {
                    if word.eq_ignore_ascii_case("use") {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    } else if ShellScope::from_name(word).is_some() {
                        styled.push((Color::Magenta.bold(), word.to_string()));
                    } else {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    }
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && first_word.map(|w| w.eq_ignore_ascii_case("use")).unwrap_or(false) {
                // Second word after `use <scope>`
                if ShellScope::from_name(word).is_some() {
                    styled.push((Color::Magenta.bold(), word.to_string()));
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && self.scope.is_scope_prefix(first_word.unwrap_or("")) {
                // Second word after scope prefix redundancy: e.g. "bridge status", "automa run"
                if self.scope.is_valid_subcommand(word) {
                    styled.push((Color::Cyan.bold(), word.to_string()));
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && self.scope == ShellScope::Global && ShellScope::from_name(first_word.unwrap_or("")).is_some() {
                // Second word after scope name in Global: e.g. "bridge status", "automa run"
                if let Some(target_scope) = ShellScope::from_name(first_word.unwrap_or("")) {
                    if target_scope.is_valid_subcommand(word) {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    } else {
                        styled.push((Color::Red.normal(), word.to_string()));
                    }
                } else {
                    styled.push((Color::White.normal(), word.to_string()));
                }
            } else if word.starts_with('-') {
                styled.push((Color::Yellow.normal(), word.to_string()));
            } else if word.starts_with('@') {
                styled.push((Color::Magenta.normal(), word.to_string()));
            } else if word.starts_with('"') || word.starts_with('\'') {
                styled.push((Color::Green.normal(), word.to_string()));
            } else if word.chars().all(|c| c.is_ascii_digit()) {
                styled.push((Color::Cyan.normal(), word.to_string()));
            } else if word.eq_ignore_ascii_case("all") {
                styled.push((Color::Green.bold(), word.to_string()));
            } else {
                styled.push((Color::White.normal(), word.to_string()));
            }

            word_index += 1;
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
                            let faker_cfg = tuquet_faker::FakerConfig::load();
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
                            let faker_cfg = tuquet_faker::FakerConfig::load();
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
                // If in Global and user typed a direct shortcut like `run`, `card`, `install`, `start`, etc.
                match cmd.as_str() {
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
            crate::commands::doctor::run(false).await?;
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
            ShellScope::Automa => return dispatch_automa(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Runner => return dispatch_runner(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Cloud => return dispatch_cloud(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Browser => return dispatch_browser(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Bridge => return dispatch_bridge(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Faker => return dispatch_faker(args[0], &args[1..]).await.map(|_| false),
            ShellScope::Global => {}
        }
    }

    match *scope {
        ShellScope::Global => dispatch_global(cmd, args).await?,
        ShellScope::Automa => dispatch_automa(cmd, args).await?,
        ShellScope::Runner => dispatch_runner(cmd, args).await?,
        ShellScope::Cloud => dispatch_cloud(cmd, args).await?,
        ShellScope::Browser => dispatch_browser(cmd, args).await?,
        ShellScope::Bridge => dispatch_bridge(cmd, args).await?,
        ShellScope::Faker => dispatch_faker(cmd, args).await?,
    }

    Ok(false)
}

async fn dispatch_global(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            crate::commands::status::show_dashboard(false).await?;
        }
        "doctor" => {
            crate::commands::doctor::run(false).await?;
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
        "upgrade" | "update" => {
            crate::infrastructure::updater::run_upgrade().await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let service_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            match service_opt {
                None => {
                    if edit {
                        println!("Please specify a service to edit: config <service> --edit");
                        println!("Available: bridge, faker, browser, automa, runner, cloud, system");
                    } else {
                        crate::config::ConfigRegistry::render_overview_card();
                    }
                }
                Some("bridge" | "tunnel" | "vps") => {
                    crate::commands::bridge::manage_config(edit, show)?;
                }
                Some("faker" | "user" | "persona") => {
                    crate::commands::faker::manage_config(edit, show, None)?;
                }
                Some("browser" | "chromium" | "chrome") => {
                    crate::commands::browser::manage_config(edit, show)?;
                }
                Some("automa" | "workflow") => {
                    crate::commands::automa::manage_config(edit, show)?;
                }
                Some("runner" | "daemon") => {
                    crate::commands::runner::manage_config(edit, show)?;
                }
                Some("cloud" | "system") => {
                    crate::commands::cloud::manage_config(edit, show)?;
                }
                Some(unknown) => {
                    println!("Unknown service '{}'. Available: bridge, faker, browser, automa, runner, cloud, system", unknown);
                }
            }
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
        "bridge" => {
            if args.is_empty() {
                dispatch_bridge("status", &[]).await?;
            } else {
                dispatch_bridge(args[0], &args[1..]).await?;
            }
        }
        "faker" | "user" => {
            if args.is_empty() {
                dispatch_faker("card", &[]).await?;
            } else {
                dispatch_faker(args[0], &args[1..]).await?;
            }
        }
        // Direct Global convenience commands
        "run" | "list" | "inspect" | "import" | "export" | "delete" | "studio" => {
            dispatch_automa(cmd, args).await?;
        }
        "install" | "clean" | "path" => {
            dispatch_browser(cmd, args).await?;
        }
        "enable" | "disable" => {
            dispatch_bridge(cmd, args).await?;
        }
        other => {
            println!("Unknown global command: '{}'. Type 'help' or 'use <service>' to enter a scope.", other);
        }
    }
    Ok(())
}

fn get_flag_value<'a>(args: &'a [&'a str], short: &str, long: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == short || w[0] == long).map(|w| w[1])
}

async fn dispatch_faker(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let count = get_flag_value(args, "-n", "--count")
        .and_then(|v| v.parse().ok())
        .or_else(|| {
            let mut iter = args.iter();
            while let Some(&arg) = iter.next() {
                if arg.starts_with('-') {
                    iter.next();
                } else if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()) {
                    return arg.parse().ok();
                }
            }
            None
        })
        .unwrap_or(1);
    let gender = get_flag_value(args, "-g", "--gender");
    let nat = get_flag_value(args, "--nat", "--nat").or(Some("VN"));
    let avatar = get_flag_value(args, "--avatar", "--avatar").or(Some("real"));
    let domain = get_flag_value(args, "-d", "--domain");
    let format = get_flag_value(args, "-f", "--format").unwrap_or("table");
    let output = get_flag_value(args, "-o", "--output").map(std::path::PathBuf::from);

    match cmd {
        "card" | "show" | "inspect" => {
            crate::commands::faker::run_card(gender, nat, avatar, domain).await?;
        }
        "generate" | "gen" | "g" => {
            crate::commands::faker::run_generate(
                count,
                gender,
                nat,
                avatar,
                domain,
                format,
                output,
            ).await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::faker::manage_config(edit, show, domain)?;
        }
        "help" => {
            print_scope_help(ShellScope::Faker);
        }
        _ => {
            let effective_count = if cmd.chars().all(|c| c.is_ascii_digit()) {
                cmd.parse().unwrap_or(1)
            } else {
                count
            };
            crate::commands::faker::run_generate(
                effective_count,
                gender,
                nat,
                avatar,
                domain,
                format,
                output,
            ).await?;
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
                let browser = get_flag_value(args, "-b", "--browser").map(|s| s.to_string());
                let timeout = get_flag_value(args, "-t", "--timeout").and_then(|s| s.parse::<u64>().ok());

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
                let id = get_flag_value(args, "-i", "--id").map(|s| s.to_string());
                let name = get_flag_value(args, "-n", "--name").map(|s| s.to_string());
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
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::automa::manage_config(edit, show)?;
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
            crate::commands::runner::check_status(target_url, false).await?;
        }
        "start" => {
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            if detach {
                println!("Starting runner daemon in background...");
            } else {
                println!("Starting runner daemon in foreground (Ctrl+C to stop)...");
            }
            crate::commands::runner::run_server(None, None, detach, None, None, false).await?;
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
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::runner::manage_config(edit, show)?;
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
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::cloud::manage_config(edit, show)?;
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
            let ver = args.iter().find(|&&a| !a.starts_with('-')).map(|s| s.to_string());
            crate::commands::browser::handle(crate::cli::BrowserCommands::Install { force, version: ver }).await?;
        }
        "search" | "releases" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Search { remote: false }).await?;
        }
        "list" | "ls" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::List).await?;
        }
        "use" => {
            if let Some(target) = args.first() {
                crate::commands::browser::handle(crate::cli::BrowserCommands::Use { version: target.to_string() }).await?;
            } else {
                eprintln!("Usage: use <version> (e.g. '148', 'v148', '144', 'lts')");
            }
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
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::browser::manage_config(edit, show)?;
        }
        other => {
            println!("Unknown browser command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

async fn dispatch_bridge(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            crate::commands::bridge::show_status().await?;
        }
        "start" => {
            let start_http = args.contains(&"--http");
            let enable_ssh = args.contains(&"--ssh");
            let foreground = args.contains(&"-f") || args.contains(&"--foreground");
            let tag_opt = get_flag_value(args, "-t", "--tag");
            let server_opt = {
                let mut iter = args.iter();
                let mut srv = None;
                while let Some(&arg) = iter.next() {
                    if arg.starts_with('-') {
                        iter.next();
                    } else {
                        srv = Some(arg);
                        break;
                    }
                }
                srv
            };
            crate::commands::bridge::start_bridge(crate::commands::bridge::BridgeStartOptions {
                server: server_opt,
                tag: tag_opt,
                http: start_http,
                ssh: enable_ssh,
                foreground,
            }).await?;
        }
        "stop" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            crate::commands::bridge::stop_bridge(server_opt).await?;
        }
        "enable" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            if let Some(srv) = server_opt {
                crate::commands::bridge::toggle_server(srv, true)?;
            } else {
                eprintln!("Usage: enable <server_id>");
            }
        }
        "disable" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            if let Some(srv) = server_opt {
                crate::commands::bridge::toggle_server(srv, false)?;
            } else {
                eprintln!("Usage: disable <server_id>");
            }
        }
        "check" => {
            crate::commands::bridge::check_config()?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::bridge::manage_config(edit, show)?;
        }
        "help" => {
            print_scope_help(ShellScope::Bridge);
        }
        other => {
            println!("Unknown bridge command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}

fn print_scope_help(scope: ShellScope) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_ansi_term::Color;

    #[test]
    fn test_shell_scope_from_name() {
        assert_eq!(ShellScope::from_name("global"), Some(ShellScope::Global));
        assert_eq!(ShellScope::from_name("automa"), Some(ShellScope::Automa));
        assert_eq!(ShellScope::from_name("runner"), Some(ShellScope::Runner));
        assert_eq!(ShellScope::from_name("cloud"), Some(ShellScope::Cloud));
        assert_eq!(ShellScope::from_name("browser"), Some(ShellScope::Browser));
        assert_eq!(ShellScope::from_name("faker"), Some(ShellScope::Faker));
        assert_eq!(ShellScope::from_name("bridge"), Some(ShellScope::Bridge));

        // Case insensitivity
        assert_eq!(ShellScope::from_name("BRIDGE"), Some(ShellScope::Bridge));
        assert_eq!(ShellScope::from_name("Cloud"), Some(ShellScope::Cloud));

        // Unknown scope
        assert_eq!(ShellScope::from_name("unknown"), None);
    }

    #[test]
    fn test_shell_scope_validation() {
        let bridge = ShellScope::Bridge;
        assert!(bridge.is_valid_first_word("status"));
        assert!(bridge.is_valid_first_word("start"));
        assert!(bridge.is_valid_first_word("stop"));
        assert!(bridge.is_valid_first_word("bridge")); // prefix redundancy allowed
        assert!(bridge.is_valid_first_word("use"));
        assert!(bridge.is_valid_first_word("help"));
        assert!(bridge.is_valid_first_word("back"));
        assert!(bridge.is_valid_first_word("exit"));
        assert!(!bridge.is_valid_first_word("nonexistent"));

        let faker = ShellScope::Faker;
        assert!(faker.is_valid_first_word("generate"));
        assert!(faker.is_valid_first_word("card"));
        assert!(faker.is_valid_first_word("config"));
        assert!(faker.is_valid_first_word("faker"));

        let global = ShellScope::Global;
        assert!(global.is_valid_first_word("bridge"));
        assert!(global.is_valid_first_word("automa"));
        assert!(global.is_valid_first_word("faker"));
        assert!(global.is_valid_first_word("use"));
        assert!(global.is_valid_first_word("clear"));
        assert!(!global.is_valid_first_word("bogus"));
    }

    #[test]
    fn test_highlighter_valid_and_invalid_first_words() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // Valid subcommand "status"
        let styled = hl.highlight("status", 0);
        assert_eq!(styled.buffer.len(), 1);
        assert_eq!(styled.buffer[0].1, "status");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Cyan));

        // Invalid command "badcmd"
        let styled = hl.highlight("badcmd", 0);
        assert_eq!(styled.buffer.len(), 1);
        assert_eq!(styled.buffer[0].1, "badcmd");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Red));
    }

    #[test]
    fn test_highlighter_use_scope_arguments() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // "use automa" -> 'use' (cyan), ' ' (default), 'automa' (magenta)
        let styled = hl.highlight("use automa", 0);
        assert_eq!(styled.buffer.len(), 3);
        assert_eq!(styled.buffer[0].1, "use");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Cyan));
        assert_eq!(styled.buffer[2].1, "automa");
        assert_eq!(styled.buffer[2].0.foreground, Some(Color::Magenta));

        // "use invalid_scope" -> 'use' (cyan), ' ' (default), 'invalid_scope' (red)
        let styled_err = hl.highlight("use invalid_scope", 0);
        assert_eq!(styled_err.buffer.len(), 3);
        assert_eq!(styled_err.buffer[0].1, "use");
        assert_eq!(styled_err.buffer[2].1, "invalid_scope");
        assert_eq!(styled_err.buffer[2].0.foreground, Some(Color::Red));
    }

    #[test]
    fn test_highlighter_prefix_redundancy() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // "bridge status" in Bridge scope -> 'bridge' (magenta), ' ' (default), 'status' (cyan)
        let styled = hl.highlight("bridge status", 0);
        assert_eq!(styled.buffer.len(), 3);
        assert_eq!(styled.buffer[0].1, "bridge");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Magenta));
        assert_eq!(styled.buffer[2].1, "status");
        assert_eq!(styled.buffer[2].0.foreground, Some(Color::Cyan));

        // "bridge badsubcmd" in Bridge scope -> 'bridge' (magenta), ' ' (default), 'badsubcmd' (red)
        let styled_err = hl.highlight("bridge badsubcmd", 0);
        assert_eq!(styled_err.buffer.len(), 3);
        assert_eq!(styled_err.buffer[2].1, "badsubcmd");
        assert_eq!(styled_err.buffer[2].0.foreground, Some(Color::Red));
    }

    fn extract_values(res: CompletionResult) -> Vec<String> {
        match res {
            CompletionResult::Fresh { suggestions, .. } => {
                suggestions.iter().map(|s| s.value.clone()).collect()
            }
            _ => vec![],
        }
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

