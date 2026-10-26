//! Enterprise Constants for CLI Ecosystem
//! Single Source of Truth (SSOT) Architecture

pub mod ports {
    /// Default port for Automation / Runner HTTP and WebSocket API server
    pub const DEFAULT_RUNNER_PORT: u16 = 8765;
    /// Default SOCKS5 proxy port
    pub const DEFAULT_SOCKS5_PORT: u16 = 1080;
    /// Default HTTP gateway proxy port
    pub const DEFAULT_HTTP_GATEWAY_PORT: u16 = 8118;
    /// Default Chrome DevTools Protocol (CDP) port
    pub const DEFAULT_CDP_PORT: u16 = 9222;
    /// Default local SSH tunnel port
    pub const DEFAULT_SSH_TUNNEL_PORT: u16 = 2222;
    /// Default port for local Docker Supabase stack
    pub const DEFAULT_LOCAL_DOCKER_SUPABASE_PORT: u16 = 54321;
    /// Default port for Inbox webhook listener daemon
    pub const DEFAULT_INBOX_PORT: u16 = 9123;
}

pub mod protocols {
    /// Protocol identifier for Automation workflow engine v1
    pub const PROTOCOL_AUTOMATION_V1: &str = "tuquet.automation.v1";
    /// Protocol identifier for dedicated browser runtime v1
    pub const PROTOCOL_BROWSER_V1: &str = "tuquet.browser.v1";
    /// Protocol identifier for distributed execution agent v1
    pub const PROTOCOL_AGENT_V1: &str = "tuquet.agent.v1";
}

pub mod pillars {
    pub const PILLAR_GATEWAY: &str = "gateway";
    pub const PILLAR_FAKER: &str = "faker";
    pub const PILLAR_BROWSER: &str = "browser";
    pub const PILLAR_AUTOMATION: &str = "automation";
    pub const PILLAR_WORKER: &str = "worker";
    pub const PILLAR_CLOUD: &str = "cloud";
    pub const PILLAR_SYSTEM: &str = "system";
    pub const PILLAR_INBOX: &str = "inbox";

    pub const ALL_PILLARS: &[&str] = &[
        PILLAR_GATEWAY,
        PILLAR_FAKER,
        PILLAR_BROWSER,
        PILLAR_AUTOMATION,
        PILLAR_WORKER,
        PILLAR_CLOUD,
        PILLAR_SYSTEM,
        PILLAR_INBOX,
    ];
}

pub mod envs {
    // Canonical Tuquet Ecosystem Environment Variables (SSOT) — techstack naming (plan 011).
    // No brand prefixes (TQ_/TUQUET_/SPECTER_/AUTOMA_ are banned). One name per variable,
    // no fallback chains — missing required values fail fast at the call site.
    pub const ENV_DATA_DIR: &str = "DATA_DIR";
    pub const ENV_APP_ENV: &str = "APP_ENV";
    pub const ENV_SUPABASE_URL: &str = "SUPABASE_URL";
    pub const ENV_SUPABASE_ANON_KEY: &str = "SUPABASE_ANON_KEY";
    pub const ENV_ENROLLMENT_TOKEN: &str = "ENROLLMENT_TOKEN";
    pub const ENV_DAEMON_HOST: &str = "DAEMON_HOST";
    pub const ENV_DAEMON_PORT: &str = "DAEMON_PORT";
    pub const ENV_RUST_LOG: &str = "RUST_LOG";
    pub const ENV_CHROME_NO_SANDBOX: &str = "CHROME_NO_SANDBOX";
    pub const ENV_EXTENSION_PATH: &str = "EXTENSION_PATH";
    pub const ENV_SENTRY_DSN: &str = "SENTRY_DSN";
    pub const ENV_HEARTBEAT_INTERVAL: &str = "HEARTBEAT_INTERVAL";

    pub const ENV_ALL_PROXY: &str = "ALL_PROXY";
    pub const ENV_HTTP_PROXY: &str = "HTTP_PROXY";
}

pub mod endpoints {
    pub const DEFAULT_HOST: &str = "127.0.0.1";
    pub const DEFAULT_CLOUD_URL: &str = "https://cloud.tq.dev";
    pub const DEFAULT_AUTOMATION_STUDIO_URL: &str = "https://automation-studio.vercel.app";
    pub const DEFAULT_DOCS_URL: &str = "https://docs.tuquet.com/en/cli";
    pub const DEFAULT_UPDATE_CHANNEL: &str = "stable";
    pub const GITHUB_REPO: &str = "tuquet/cli";
}

pub mod paths {
    pub const DEFAULT_SSOT_ROOT_DIR: &str = ".tq";

    pub const FILE_AUTOMATION_SQLITE: &str = "automation.sqlite";
    pub const FILE_GATEWAY_JSON: &str = "gateway.json";
    pub const FILE_FAKER_JSON: &str = "faker.json";
    pub const FILE_BROWSER_JSON: &str = "browser.json";
    pub const FILE_AUTOMATION_JSON: &str = "automation.json";
    pub const FILE_WORKER_JSON: &str = "worker.json";
    pub const FILE_SYSTEM_JSON: &str = "system.json";
    pub const FILE_INBOX_JSON: &str = "inbox.json";
    pub const FILE_INBOX_SQLITE: &str = "inbox.sqlite";
    pub const FILE_IDENTITY_JSON: &str = ".identity.json";
    pub const FILE_ENVIRONMENTS_JSON: &str = "environments.json";
    pub const FILE_WORKER_PID: &str = "worker.pid";
    pub const FILE_WORKER_LOG: &str = "worker.log";

    pub const DIR_RUNTIMES: &str = "runtimes";
    pub const DIR_PROFILES: &str = "profiles";
    pub const DIR_EXTENSIONS: &str = "extensions";
    pub const DIR_STEALTH: &str = "stealth";
    pub const DIR_PIDS: &str = "pids";
    pub const DIR_LOGS: &str = "logs";
    pub const DIR_CONFIG: &str = "config";
    pub const DIR_SYSTEM: &str = "system";
}

pub mod tools {
    pub const DEFAULT_SSH_KEY_NAME: &str = "id_ed25519";
    pub const DEFAULT_SSH_ALIVE_INTERVAL: u32 = 15;
    pub const DEFAULT_SSH_ALIVE_COUNT_MAX: u32 = 3;
    pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;
    pub const DEFAULT_TIMEOUT_SECS: u64 = 300;
    pub const DEFAULT_BROWSER_NAME: &str = "chromium";
    pub const DEFAULT_VIEWPORT_WIDTH: u32 = 1280;
    pub const DEFAULT_VIEWPORT_HEIGHT: u32 = 800;
    pub const DEFAULT_MAX_CONCURRENT_JOBS: usize = 2;
    pub const PINNED_CHROMIUM_REVISION: &str = "134.0.6998.35";
}

pub mod messages;

// Flat re-exports of common constants for ergonomics
pub use endpoints::*;
pub use envs::*;
pub use messages::*;
pub use paths::*;
pub use pillars::*;
pub use ports::*;
pub use protocols::*;
pub use tools::*;
