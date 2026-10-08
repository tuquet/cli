use std::path::{Path, PathBuf};
use std::env;

#[derive(Debug, Clone)]
pub struct ToolCheckResult {
    pub name: &'static str,
    pub found: bool,
    pub path: Option<PathBuf>,
    pub description: &'static str,
    pub install_hint: &'static str,
}

/// Cross-platform tool discovery by inspecting system PATH.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    let paths = env::split_paths(&path_var);

    let candidates: Vec<String> = if cfg!(windows) {
        vec![
            format!("{}.exe", name),
            format!("{}.cmd", name),
            format!("{}.bat", name),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    };

    for dir in paths {
        for candidate in &candidates {
            let full_path = dir.join(candidate);
            if full_path.is_file() {
                return Some(unwrap_scoop_shim(&full_path));
            }
        }
    }

    // Common fallback locations on Windows
    if cfg!(windows) {
        if name == "ssh" {
            let win_ssh = Path::new(r"C:\Windows\System32\OpenSSH\ssh.exe");
            if win_ssh.is_file() {
                return Some(win_ssh.to_path_buf());
            }
        }
        if name == "cloudflared" {
            let cf = crate::config::canonical_ssot_dir()
                .join("bridge")
                .join("bin")
                .join("cloudflared.exe");
            if cf.is_file() {
                return Some(cf);
            }
        }
        if name == "specter" {
            let bin = crate::config::canonical_ssot_dir()
                .join("bin")
                .join(format!("{}.exe", name));
            if bin.is_file() {
                return Some(bin);
            }
        }
        if let Ok(userprofile) = env::var("USERPROFILE") {
            let scoop_shim = PathBuf::from(&userprofile)
                .join("scoop")
                .join("shims")
                .join(format!("{}.exe", name));
            if scoop_shim.is_file() {
                return Some(unwrap_scoop_shim(&scoop_shim));
            }
        }
    }

    // Common fallback locations on Unix (Linux & macOS)
    if cfg!(unix) {
        if name == "ssh" {
            for p in &["/usr/bin/ssh", "/usr/local/bin/ssh", "/opt/homebrew/bin/ssh"] {
                let path = Path::new(p);
                if path.is_file() {
                    return Some(path.to_path_buf());
                }
            }
        }
        if name == "cloudflared" {
            let cf = crate::config::canonical_ssot_dir().join("bridge").join("bin").join("cloudflared");
            if cf.is_file() {
                return Some(cf);
            }
            for p in &["/usr/local/bin/cloudflared", "/opt/homebrew/bin/cloudflared"] {
                let path = Path::new(p);
                if path.is_file() {
                    return Some(path.to_path_buf());
                }
            }
        }
        if name == "specter" {
            let bin = crate::config::canonical_ssot_dir().join("bin").join(name);
            if bin.is_file() {
                return Some(bin);
            }
        }
        if name == "brew" {
            for p in &["/opt/homebrew/bin/brew", "/usr/local/bin/brew", "/home/linuxbrew/.linuxbrew/bin/brew"] {
                let path = Path::new(p);
                if path.is_file() {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    None
}

/// If the executable is a Scoop shim (e.g. ~/.scoop/shims/foo.exe),
/// read the corresponding foo.shim file to resolve the canonical target executable.
pub fn unwrap_scoop_shim(path: &Path) -> PathBuf {
    if cfg!(windows) {
        let shim_file = path.with_extension("shim");
        if shim_file.is_file() {
            if let Ok(content) = std::fs::read_to_string(&shim_file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some(target) = trimmed.strip_prefix("path = \"").and_then(|s| s.strip_suffix('\"')) {
                        let target_path = PathBuf::from(target);
                        if target_path.is_file() {
                            return target_path;
                        }
                    }
                }
            }
        }
    }
    path.to_path_buf()
}

/// Check status of all required external bridge dependencies.
pub fn check_dependencies() -> Vec<ToolCheckResult> {
    let mut results = Vec::new();

    // 1. cloudflared
    let cf_path = find_executable("cloudflared");
    results.push(ToolCheckResult {
        name: "cloudflared",
        found: cf_path.is_some(),
        path: cf_path,
        description: "Cloudflare Access WebSocket TCP Tunnel client",
        install_hint: if cfg!(windows) {
            "scoop install cloudflared"
        } else if cfg!(target_os = "macos") {
            "brew install cloudflared"
        } else {
            "sudo apt install cloudflared (or curl from https://github.com/cloudflare/cloudflared/releases)"
        },
    });

    // 2. OpenSSH client
    let ssh_path = find_executable("ssh");
    results.push(ToolCheckResult {
        name: "ssh",
        found: ssh_path.is_some(),
        path: ssh_path,
        description: "OpenSSH dynamic SOCKS5 client and secure tunnel daemon",
        install_hint: if cfg!(windows) {
            "Add-WindowsCapability -Online -Name OpenSSH.Client~~~~0.0.1.0 or scoop install openssh"
        } else if cfg!(target_os = "macos") {
            "Built-in on macOS"
        } else {
            "sudo apt install openssh-client"
        },
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_scoop_shim_non_existent() {
        let fake = PathBuf::from("non_existent_binary.exe");
        assert_eq!(unwrap_scoop_shim(&fake), fake);
    }

    #[test]
    fn test_unwrap_scoop_shim_with_actual_file() {
        let temp_dir = std::env::temp_dir();
        let target_exe = temp_dir.join("test_app_real.exe");
        let shim_exe = temp_dir.join("test_app.exe");
        let shim_file = temp_dir.join("test_app.shim");

        let _ = std::fs::write(&target_exe, "MZ");
        let _ = std::fs::write(&shim_exe, "MZ");
        let _ = std::fs::write(
            &shim_file,
            format!("path = \"{}\"\nargs = \"\"\n", target_exe.display()),
        );

        let unwrapped = unwrap_scoop_shim(&shim_exe);
        if cfg!(windows) {
            assert_eq!(unwrapped, target_exe);
        } else {
            assert_eq!(unwrapped, shim_exe);
        }

        let _ = std::fs::remove_file(target_exe);
        let _ = std::fs::remove_file(shim_exe);
        let _ = std::fs::remove_file(shim_file);
    }
}

