//! Standard Message Catalog for Specter CLI Ecosystem
//! Single Source of Truth for system feedback, errors, and lifecycle events.

/// Daemon & Server lifecycle messages
pub const MSG_SERVER_STARTING: &str = "Starting Specter API & Runner Daemon...";
pub const MSG_SERVER_STOPPING: &str = "Stopping existing runner daemon...";
pub const MSG_SERVER_STOPPED: &str = "Runner worker daemon has been gracefully terminated.";
pub const MSG_SERVER_RESTARTING: &str = "Restarting runner daemon...";
pub const MSG_SHUTDOWN_SIGNAL: &str = "Received termination signal (Ctrl+C). Initiating graceful shutdown...";
pub const MSG_SHUTDOWN_CLEAN: &str = "Runner worker daemon exited cleanly.";
pub const MSG_DAEMON_NO_PROCESS: &str = "No active runner daemon process found on this machine.";

/// Job execution messages
pub const MSG_JOB_RUNNING: &str = "Executing job specification...";
pub const MSG_JOB_COMPLETED: &str = "Job completed successfully.";
pub const MSG_JOB_FAILED: &str = "Job execution failed.";
pub const MSG_JOB_CANCELLED: &str = "Job execution cancelled by signal.";

/// Cloud enrollment & authentication messages
pub const MSG_ENROLLMENT_SUCCESS: &str = "Workstation successfully enrolled with Specter Cloud!";
pub const MSG_ENROLLMENT_FAILED: &str = "Cloud device enrollment failed.";
pub const MSG_NOT_ENROLLED: &str = "Workstation not enrolled with Specter Cloud.";
pub const MSG_IDENTITY_PURGED: &str = "Local device identity purged.";
pub const MSG_REVOCATION_DETECTED: &str = "Cloud device identity revocation detected! Initiating self-healing...";
pub const MSG_SELF_HEALING_SUCCESS: &str = "Fresh Device ID assigned via self-healing recovery.";

/// Browser & runtime provisioning messages
pub const MSG_BROWSER_READY: &str = "Antidetect Chromium runtime is operational and ready.";
pub const MSG_BROWSER_NOT_FOUND: &str = "Dedicated Chromium binary not found in storage pillar.";
pub const MSG_BROWSER_DOWNLOADING: &str = "Downloading & provisioning Antidetect Chromium runtime...";
pub const MSG_BROWSER_CHECKSUM_OK: &str = "SHA256 checksum verified successfully.";
pub const MSG_BROWSER_CLEANUP_OK: &str = "All browser child processes cleanly terminated.";

/// Storage & Workflow messages
pub const MSG_DB_INITIALIZED: &str = "Automa SQLite database initialized successfully.";
pub const MSG_PILLARS_INITIALIZED: &str = "Canonical storage pillars initialized under ~/.specter/";
pub const MSG_WORKFLOW_EXPORTED: &str = "Workflow exported successfully.";
pub const MSG_WORKFLOW_NOT_FOUND: &str = "Workflow file or ID not found in database or vault.";
