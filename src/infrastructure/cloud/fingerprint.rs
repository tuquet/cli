use sha2::{Digest, Sha256};

/// Deterministic hardware & operating system machine fingerprinting
pub struct MachineFingerprint;

impl MachineFingerprint {
    pub fn generate() -> String {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string());
        let os_name = sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
        let os_ver = sysinfo::System::os_version().unwrap_or_else(|| "unknown".to_string());
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "user".to_string());

        let raw = format!("{}:{}:{}:{}", hostname, os_name, os_ver, username);
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        format!("fp_{}", hex::encode(&hasher.finalize()[..16]))
    }
}
