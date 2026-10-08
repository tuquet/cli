//! Enterprise Constants for Specter CLI Ecosystem
//! Single Source of Truth (SSOT) Architecture

pub mod ports {
    /// Default port for Specter Automa / Runner HTTP and WebSocket API server
    pub const DEFAULT_RUNNER_PORT: u16 = 8765;
    /// Default SOCKS5 proxy port
    pub const DEFAULT_SOCKS5_PORT: u16 = 1080;
    /// Default HTTP bridge proxy port
    pub const DEFAULT_HTTP_BRIDGE_PORT: u16 = 8118;
    /// Default Chrome DevTools Protocol (CDP) port
    pub const DEFAULT_CDP_PORT: u16 = 9222;
    /// Default local SSH tunnel port
    pub const DEFAULT_SSH_TUNNEL_PORT: u16 = 2222;
    /// Default port for local Docker Supabase stack
    pub const DEFAULT_LOCAL_DOCKER_SUPABASE_PORT: u16 = 54321;
}

pub mod protocols {
    /// Protocol identifier for Automa workflow engine v1
    pub const PROTOCOL_AUTOMA_V1: &str = "specter.automa.v1";
    /// Protocol identifier for Specter dedicated browser runtime v1
    pub const PROTOCOL_BROWSER_V1: &str = "specter.browser.v1";
    /// Protocol identifier for Specter distributed execution agent v1
    pub const PROTOCOL_AGENT_V1: &str = "specter.agent.v1";
}

pub mod pillars {
    pub const PILLAR_BRIDGE: &str = "bridge";
    pub const PILLAR_FAKER: &str = "faker";
    pub const PILLAR_BROWSER: &str = "browser";
    pub const PILLAR_AUTOMA: &str = "automa";
    pub const PILLAR_RUNNER: &str = "runner";
    pub const PILLAR_CLOUD: &str = "cloud";
    pub const PILLAR_SYSTEM: &str = "system";

    pub const ALL_PILLARS: &[&str] = &[
        PILLAR_BRIDGE,
        PILLAR_FAKER,
        PILLAR_BROWSER,
        PILLAR_AUTOMA,
        PILLAR_RUNNER,
        PILLAR_CLOUD,
        PILLAR_SYSTEM,
    ];
}

pub mod envs {
    pub const ENV_SPECTER_HOME: &str = "SPECTER_HOME";
    pub const ENV_SPECTER_ENV: &str = "SPECTER_ENV";
    pub const ENV_SPECTER_CLOUD_URL: &str = "SPECTER_CLOUD_URL";
    pub const ENV_SPECTER_API_KEY: &str = "SPECTER_API_KEY";
    pub const ENV_SPECTER_ENROLLMENT_TOKEN: &str = "SPECTER_ENROLLMENT_TOKEN";
    pub const ENV_SPECTER_HOST: &str = "SPECTER_HOST";
    pub const ENV_SPECTER_PORT: &str = "SPECTER_PORT";
    pub const ENV_SPECTER_LOG_LEVEL: &str = "SPECTER_LOG_LEVEL";
    pub const ENV_SPECTER_DATA_DIR: &str = "SPECTER_DATA_DIR";
    pub const ENV_SPECTER_FORCE_NO_SANDBOX: &str = "SPECTER_FORCE_NO_SANDBOX";
    pub const ENV_SPECTER_EXTENSION_PATH: &str = "SPECTER_EXTENSION_PATH";

    pub const ENV_AUTOMA_HOST: &str = "AUTOMA_HOST";
    pub const ENV_AUTOMA_PORT: &str = "AUTOMA_PORT";
    pub const ENV_AUTOMA_DATA_DIR: &str = "AUTOMA_DATA_DIR";
    pub const ENV_AUTOMA_ENV: &str = "AUTOMA_ENV";
    pub const ENV_AUTOMA_LOG_LEVEL: &str = "AUTOMA_LOG_LEVEL";
    pub const ENV_AUTOMA_CLOUD_URL: &str = "AUTOMA_CLOUD_URL";
    pub const ENV_AUTOMA_ENROLLMENT_TOKEN: &str = "AUTOMA_ENROLLMENT_TOKEN";
    pub const ENV_AUTOMA_HEARTBEAT_INTERVAL: &str = "AUTOMA_HEARTBEAT_INTERVAL";

    pub const ENV_ALL_PROXY: &str = "ALL_PROXY";
    pub const ENV_HTTP_PROXY: &str = "HTTP_PROXY";
}

pub mod endpoints {
    pub const DEFAULT_HOST: &str = "127.0.0.1";
    pub const DEFAULT_CLOUD_URL: &str = "https://cloud.specter.dev";
    pub const DEFAULT_DEV_SUPABASE_URL: &str = "https://dswhacsoaxgpfnkaxnhz.supabase.co";
    pub const DEFAULT_AUTOMA_STUDIO_URL: &str = "https://automa-studio.vercel.app";
    pub const DEFAULT_DOCS_URL: &str = "https://tuquet.github.io";
    pub const DEFAULT_UPDATE_CHANNEL: &str = "stable";
    pub const GITHUB_REPO: &str = "tuquet/cli";
}

pub mod paths {
    pub const DEFAULT_SSOT_ROOT_DIR: &str = ".specter";

    pub const FILE_AUTOMA_SQLITE: &str = "automa.sqlite";
    pub const FILE_BRIDGE_JSON: &str = "bridge.json";
    pub const FILE_FAKER_JSON: &str = "faker.json";
    pub const FILE_BROWSER_JSON: &str = "browser.json";
    pub const FILE_AUTOMA_JSON: &str = "automa.json";
    pub const FILE_RUNNER_JSON: &str = "runner.json";
    pub const FILE_SYSTEM_JSON: &str = "system.json";
    pub const FILE_IDENTITY_JSON: &str = ".identity.json";
    pub const FILE_ENVIRONMENTS_JSON: &str = "environments.json";
    pub const FILE_RUNNER_PID: &str = "runner.pid";
    pub const FILE_RUNNER_LOG: &str = "runner.log";

    pub const DIR_RUNTIMES: &str = "runtimes";
    pub const DIR_PROFILES: &str = "profiles";
    pub const DIR_EXTENSIONS: &str = "extensions";
    pub const DIR_STEALTH: &str = "stealth";
    pub const DIR_PIDS: &str = "pids";
    pub const DIR_LOGS: &str = "logs";
    pub const DIR_CONFIG: &str = "config";
    pub const DIR_WORKFLOWS: &str = "workflows";
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

// Flat re-exports of common constants for ergonomics
pub use endpoints::*;
pub use envs::*;
pub use paths::*;
pub use pillars::*;
pub use ports::*;
pub use protocols::*;
pub use tools::*;
