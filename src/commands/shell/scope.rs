use crate::ui::colors;

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
                ("profile", "Manage isolated browser profiles (create, list, inspect, pack, unpack, cloud)"),
                ("launch", "Launch an antidetect browser profile with direct CDP DevTools bridge"),
                ("verify", "Live visual stealth verification (Cloudflare Turnstile demo)"),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
