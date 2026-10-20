use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Directories pruned during full recursive scan (build artifacts, dependencies, OS & cloud folders)
pub const EXCLUDED_DIRS: &[&str] = &[
    // Build & JS/Rust/Java/Tooling artifacts
    "node_modules",
    "local_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".next",
    ".nuxt",
    ".output",
    ".turbo",
    ".svelte-kit",
    ".vercel",
    "coverage",
    "out",
    "vendor",
    "bower_components",
    "src",
    "assets",
    "public",
    "static",
    "migrations",
    "jwork",
    "android",
    "ios",
    "third_party",
    // Package Manager & Runtime Caches
    "AppData",
    "scoop",
    "cargo",
    "rustup",
    "go",
    "npm-cache",
    "pnpm-cache",
    "nvm",
    // OS & System Directories
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "System Volume Information",
    "Recovery",
    "PerfLogs",
    "MSOCache",
    "Config.Msi",
    "Intel",
    "AMD",
    "NVIDIA",
    "Drivers",
    "LDPlayer",
    "XboxGames",
    "Quarantine",
    "Application Data",
    "Local Settings",
    "Cookies",
    "NetHood",
    "PrintHood",
    "Recent",
    "SendTo",
    "Start Menu",
    "Templates",
    "Contacts",
    "Links",
    "Searches",
    "Favorites",
    "Saved Games",
    "3D Objects",
    "Music",
    "Pictures",
    "Videos",
    "Library",
    "Applications",
];

const MAX_SCAN_DEPTH: usize = 10;

/// Priority order for sorting scripts in interactive prompts and completions
const SCRIPT_PRIORITY: &[&str] = &[
    "dev",
    "start",
    "serve",
    "preview",
    "build",
    "lint",
    "typecheck",
    "test",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredPackage {
    /// Short identifier for CLI selection & autocomplete (e.g. "web" or "tuquet/yak-map")
    pub id: String,
    /// Package name from package.json (if present)
    pub name: Option<String>,
    /// Directory containing package.json
    pub dir: PathBuf,
    /// Detected package manager (npm, pnpm, yarn, bun)
    pub package_manager: String,
    /// Detected frontend framework hint (e.g. "Vite", "Next.js", "Nuxt", "Astro", "Vue", "React")
    pub framework: Option<String>,
    /// Available script names and their raw commands from `scripts` object
    pub scripts: Vec<(String, String)>,
}

impl DiscoveredPackage {
    /// Returns the index of the best default script (prefers `dev`, then `start`, else 0)
    pub fn default_script_index(&self) -> usize {
        if let Some(idx) = self.scripts.iter().position(|(k, _)| k == "dev") {
            return idx;
        }
        if let Some(idx) = self.scripts.iter().position(|(k, _)| k == "start") {
            return idx;
        }
        0
    }

    /// Short folder basename (e.g. "studio" for "apps/studio")
    pub fn short_name(&self) -> &str {
        self.dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.id)
    }
}

/// Scan a specific workspace directory tree for `**/package.json` (used when `--path` is passed or in unit tests).
pub fn scan_workspace_packages(start_dir: &Path) -> Vec<DiscoveredPackage> {
    let mut results = scan_from_root_parallel(start_dir, start_dir);

    // Fallback: If inside a sub-repo with 0 package.json (e.g., `tuquet/cli`), scan parent workspace
    if results.is_empty()
        && let Some(parent) = start_dir.parent()
        && parent.parent().is_some()
    {
        results = scan_from_root_parallel(parent, parent);
    }

    sort_packages(&mut results, Some(start_dir));
    results
}

/// Scan the ENTIRE machine (all local fixed drives + user profile directories) in parallel for `**/package.json`.
/// Prioritizes packages located inside `current_dir` first, then packages with `dev`/`start` scripts.
pub fn scan_machine_packages(current_dir: &Path) -> Vec<DiscoveredPackage> {
    let roots = collect_machine_scan_roots();
    let home_dir = get_user_home_dir();

    let mut all_packages: Vec<DiscoveredPackage> = std::thread::scope(|s| {
        let handles: Vec<_> = roots
            .iter()
            .map(|(scan_dir, display_base)| {
                s.spawn(move || scan_from_root_parallel(scan_dir, display_base))
            })
            .collect();

        let mut merged = Vec::new();
        for h in handles {
            if let Ok(pkgs) = h.join() {
                merged.extend(pkgs);
            }
        }
        merged
    });

    // Normalize IDs to be clean and readable relative to ~/Repository, ~, or drive root
    for pkg in &mut all_packages {
        pkg.id = format_clean_package_id(&pkg.dir, current_dir, home_dir.as_deref());
    }

    // Deduplicate by canonical/exact directory path
    let mut seen_dirs = std::collections::HashSet::new();
    all_packages.retain(|p| seen_dirs.insert(p.dir.clone()));

    sort_packages(&mut all_packages, Some(current_dir));
    all_packages
}

pub(crate) fn format_clean_package_id(
    dir: &Path,
    current_dir: &Path,
    home_dir: Option<&Path>,
) -> String {
    // 1. If inside current_dir (and current_dir is not Home or Drive root), show relative to current_dir
    if current_dir.parent().is_some()
        && Some(current_dir) != home_dir
        && let Ok(rel) = dir.strip_prefix(current_dir)
    {
        let s = path_to_slash(rel);
        if !s.is_empty() {
            return s;
        }
    }

    // 2. If inside ~/Repository, show relative to ~/Repository
    if let Some(home) = home_dir {
        let repo_dir = home.join("Repository");
        if let Ok(rel) = dir.strip_prefix(&repo_dir) {
            let s = path_to_slash(rel);
            if !s.is_empty() {
                return s;
            }
        }
        if let Ok(rel) = dir.strip_prefix(home) {
            let s = path_to_slash(rel);
            if !s.is_empty() {
                return format!("~/{}", s);
            }
        }
    }

    path_to_slash(dir)
}

fn path_to_slash(p: &Path) -> String {
    p.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(os) => Some(os.to_string_lossy().into_owned()),
            std::path::Component::Prefix(pre) => Some(
                pre.as_os_str()
                    .to_string_lossy()
                    .trim_end_matches('\\')
                    .to_string(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn sort_packages(packages: &mut [DiscoveredPackage], current_dir: Option<&Path>) {
    let home = get_user_home_dir();
    let prioritize_cwd =
        current_dir.filter(|cwd| cwd.parent().is_some() && Some(*cwd) != home.as_deref());

    packages.sort_by(|a, b| {
        let a_in_cwd = prioritize_cwd
            .map(|cwd| a.dir.starts_with(cwd))
            .unwrap_or(false);
        let b_in_cwd = prioritize_cwd
            .map(|cwd| b.dir.starts_with(cwd))
            .unwrap_or(false);

        let a_has_dev = a.scripts.iter().any(|(k, _)| k == "dev" || k == "start");
        let b_has_dev = b.scripts.iter().any(|(k, _)| k == "dev" || k == "start");

        b_in_cwd
            .cmp(&a_in_cwd)
            .then_with(|| b_has_dev.cmp(&a_has_dev))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// Collect top-level directories across all fixed drives and the current user's home directory
/// so we can scan the entire machine in parallel while bypassing OS/system trees.
pub(crate) fn collect_machine_scan_roots() -> Vec<(PathBuf, PathBuf)> {
    let mut roots: Vec<(PathBuf, PathBuf)> = Vec::new();
    let home_dir = get_user_home_dir();

    // 1. Enumerate non-excluded directories inside the user's $HOME
    if let Some(ref home) = home_dir
        && let Ok(entries) = std::fs::read_dir(home)
    {
        for entry in entries.flatten() {
            if should_skip_dir_entry(&entry) {
                continue;
            }
            let path = entry.path();
            roots.push((path, home.clone()));
        }
    }

    // 2. Enumerate top-level directories on all fixed drives (e.g. C:\, D:\, E:\)
    for drive in list_fixed_drives() {
        let Ok(entries) = std::fs::read_dir(&drive) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            // `Users` is already handled via `$HOME` above (avoids scanning Default/Public/All Users junctions)
            if name.eq_ignore_ascii_case("Users") {
                continue;
            }
            if should_skip_dir_entry(&entry) {
                continue;
            }
            roots.push((entry.path(), drive.clone()));
        }
    }

    roots
}

#[cfg(windows)]
fn list_fixed_drives() -> Vec<PathBuf> {
    const DRIVE_FIXED: u32 = 3;
    unsafe extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    }

    let mut drives = Vec::new();
    let bitmask = unsafe { GetLogicalDrives() };
    for i in 0..26u8 {
        if (bitmask & (1 << i)) != 0 {
            let letter = (b'A' + i) as char;
            let root_str = format!("{}:\\", letter);
            let wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();
            let dtype = unsafe { GetDriveTypeW(wide.as_ptr()) };
            if dtype == DRIVE_FIXED {
                drives.push(PathBuf::from(root_str));
            }
        }
    }
    drives
}

#[cfg(not(windows))]
fn list_fixed_drives() -> Vec<PathBuf> {
    Vec::new()
}

pub(crate) fn get_user_home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

pub(crate) fn should_skip_dir_entry(entry: &std::fs::DirEntry) -> bool {
    let Ok(ft) = entry.file_type() else {
        return true;
    };
    if ft.is_symlink() || !ft.is_dir() {
        return true;
    }

    let file_name = entry.file_name();
    let name = file_name.to_string_lossy();
    if name.starts_with('.') || name.starts_with('$') {
        return true;
    }
    let name_lower = name.to_lowercase();
    if name_lower.starts_with("onedrive") || name_lower.starts_with("trellix") {
        return true;
    }
    if EXCLUDED_DIRS
        .iter()
        .any(|ex| ex.eq_ignore_ascii_case(&name))
    {
        return true;
    }

    #[cfg(windows)]
    if is_windows_special_or_cloud_dir(entry) {
        return true;
    }

    false
}

#[cfg(windows)]
fn is_windows_special_or_cloud_dir(entry: &std::fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

    if let Ok(meta) = entry.metadata() {
        let attrs = meta.file_attributes();
        let mask = FILE_ATTRIBUTE_HIDDEN
            | FILE_ATTRIBUTE_SYSTEM
            | FILE_ATTRIBUTE_REPARSE_POINT
            | FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
        if (attrs & mask) != 0 {
            return true;
        }
    }
    false
}

fn scan_from_root_parallel(root: &Path, display_base: &Path) -> Vec<DiscoveredPackage> {
    let mut packages = Vec::new();
    if let Some(pkg) = parse_directory_project(display_base, root) {
        packages.push(pkg);
    }

    let entries = match std::fs::read_dir(root) {
        Ok(read) => read,
        Err(_) => return packages,
    };

    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        if should_skip_dir_entry(&entry) {
            continue;
        }
        subdirs.push(entry.path());
    }

    subdirs.sort();

    let sub_packages: Vec<DiscoveredPackage> = std::thread::scope(|s| {
        let handles: Vec<_> = subdirs
            .iter()
            .map(|subdir| {
                s.spawn(move || {
                    let mut local = Vec::new();
                    walk_dir_recursive(display_base, subdir, 1, &mut local);
                    local
                })
            })
            .collect();

        let mut merged = Vec::new();
        for h in handles {
            if let Ok(pkgs) = h.join() {
                merged.extend(pkgs);
            }
        }
        merged
    });

    packages.extend(sub_packages);
    packages
}

fn walk_dir_recursive(
    root: &Path,
    current: &Path,
    depth: usize,
    packages: &mut Vec<DiscoveredPackage>,
) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }

    if let Some(pkg) = parse_directory_project(root, current) {
        packages.push(pkg);
    }

    let entries = match std::fs::read_dir(current) {
        Ok(read) => read,
        Err(_) => return,
    };

    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        if should_skip_dir_entry(&entry) {
            continue;
        }
        subdirs.push(entry.path());
    }

    subdirs.sort();
    for subdir in subdirs {
        walk_dir_recursive(root, &subdir, depth + 1, packages);
    }
}

fn parse_directory_project(root: &Path, dir: &Path) -> Option<DiscoveredPackage> {
    let pkg_json_path = dir.join("package.json");
    if pkg_json_path.is_file()
        && let Some(pkg) = parse_package_json(root, dir, &pkg_json_path)
    {
        return Some(pkg);
    }

    let cargo_toml_path = dir.join("Cargo.toml");
    if cargo_toml_path.is_file()
        && let Some(pkg) = parse_cargo_toml(root, dir, &cargo_toml_path)
    {
        return Some(pkg);
    }

    let pom_xml_path = dir.join("pom.xml");
    if pom_xml_path.is_file()
        && let Some(pkg) = parse_pom_xml(root, dir, &pom_xml_path)
    {
        return Some(pkg);
    }

    for gradle_file in &["build.gradle", "build.gradle.kts"] {
        let gradle_path = dir.join(gradle_file);
        if gradle_path.is_file()
            && let Some(pkg) = parse_gradle_build(root, dir, &gradle_path)
        {
            return Some(pkg);
        }
    }

    for compose_file in &[
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ] {
        let compose_path = dir.join(compose_file);
        if compose_path.is_file()
            && let Some(pkg) = parse_docker_compose(root, dir, compose_file)
        {
            return Some(pkg);
        }
    }

    None
}

fn compute_rel_id(root: &Path, dir: &Path) -> String {
    dir.strip_prefix(root)
        .ok()
        .and_then(|rel| {
            let s = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_else(|| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(".")
                .to_string()
        })
}

fn parse_cargo_toml(root: &Path, dir: &Path, cargo_path: &Path) -> Option<DiscoveredPackage> {
    let content = std::fs::read_to_string(cargo_path).ok()?;
    if !content.contains("[package]") {
        return None;
    }
    let has_bin = dir.join("src").join("main.rs").is_file() || content.contains("[[bin]]");
    if !has_bin {
        return None;
    }

    let name = content.lines().find_map(|line| {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(val) = rest.strip_prefix('=') {
                return Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        None
    });

    let scripts = vec![
        ("dev".to_string(), "cargo run".to_string()),
        ("check".to_string(), "cargo check".to_string()),
        ("test".to_string(), "cargo test".to_string()),
        ("build".to_string(), "cargo build --release".to_string()),
        ("clippy".to_string(), "cargo clippy".to_string()),
    ];

    Some(DiscoveredPackage {
        id: compute_rel_id(root, dir),
        name,
        dir: dir.to_path_buf(),
        package_manager: "cargo".to_string(),
        framework: Some("Rust".to_string()),
        scripts,
    })
}

pub fn extract_xml_tag_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let val = xml[start..end].trim();
    if val.is_empty() || val.starts_with("${") {
        None
    } else {
        Some(val.to_string())
    }
}

pub fn normalize_java_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed == "1.8" || trimmed == "8" {
        Some("8".to_string())
    } else if trimmed == "11" || trimmed == "17" || trimmed == "21" || trimmed == "22" {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn parse_pom_xml(root: &Path, dir: &Path, pom_path: &Path) -> Option<DiscoveredPackage> {
    let content = std::fs::read_to_string(pom_path).ok()?;
    let is_spring_boot = content.contains("spring-boot");
    let is_pom_packaging = content.contains("<packaging>pom</packaging>");
    if is_pom_packaging && !is_spring_boot {
        return None;
    }

    let name = extract_xml_tag_value(&content, "artifactId");
    let raw_java = extract_xml_tag_value(&content, "java.version")
        .or_else(|| extract_xml_tag_value(&content, "maven.compiler.release"))
        .or_else(|| extract_xml_tag_value(&content, "maven.compiler.source"));
    let java_ver = raw_java.as_deref().and_then(normalize_java_version);

    let fw = match (is_spring_boot, java_ver) {
        (true, Some(v)) => format!("Spring Boot (JDK {})", v),
        (true, None) => "Spring Boot".to_string(),
        (false, Some(v)) => format!("Maven (JDK {})", v),
        (false, None) => "Maven".to_string(),
    };

    let pm = if dir.join("mvnw.cmd").is_file() || dir.join("mvnw").is_file() {
        "mvnw".to_string()
    } else {
        "mvn".to_string()
    };

    let mut scripts = Vec::new();
    if is_spring_boot {
        scripts.push(("dev".to_string(), "spring-boot:run".to_string()));
    }
    scripts.push(("build".to_string(), "clean package -DskipTests".to_string()));
    scripts.push(("compile".to_string(), "test-compile".to_string()));
    scripts.push(("test".to_string(), "test".to_string()));

    Some(DiscoveredPackage {
        id: compute_rel_id(root, dir),
        name,
        dir: dir.to_path_buf(),
        package_manager: pm,
        framework: Some(fw),
        scripts,
    })
}

fn parse_gradle_build(root: &Path, dir: &Path, gradle_path: &Path) -> Option<DiscoveredPackage> {
    let content = std::fs::read_to_string(gradle_path).ok()?;
    let is_spring_boot =
        content.contains("org.springframework.boot") || content.contains("bootRun");
    let is_app = is_spring_boot || content.contains("application");
    if !is_app {
        return None;
    }

    let pm = if dir.join("gradlew.bat").is_file() || dir.join("gradlew").is_file() {
        "gradlew".to_string()
    } else {
        "gradle".to_string()
    };

    let fw = if is_spring_boot {
        "Spring Boot".to_string()
    } else {
        "Gradle".to_string()
    };

    let mut scripts = Vec::new();
    if is_spring_boot {
        scripts.push(("dev".to_string(), "bootRun".to_string()));
    } else {
        scripts.push(("dev".to_string(), "run".to_string()));
    }
    scripts.push(("build".to_string(), "assemble".to_string()));
    scripts.push(("test".to_string(), "test".to_string()));

    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    Some(DiscoveredPackage {
        id: compute_rel_id(root, dir),
        name,
        dir: dir.to_path_buf(),
        package_manager: pm,
        framework: Some(fw),
        scripts,
    })
}

fn parse_docker_compose(root: &Path, dir: &Path, compose_file: &str) -> Option<DiscoveredPackage> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    let scripts = vec![
        (
            "up".to_string(),
            format!("docker compose -f {} up", compose_file),
        ),
        (
            "up-d".to_string(),
            format!("docker compose -f {} up -d", compose_file),
        ),
        (
            "down".to_string(),
            format!("docker compose -f {} down", compose_file),
        ),
        (
            "logs".to_string(),
            format!("docker compose -f {} logs -f", compose_file),
        ),
        (
            "ps".to_string(),
            format!("docker compose -f {} ps", compose_file),
        ),
    ];

    Some(DiscoveredPackage {
        id: compute_rel_id(root, dir),
        name,
        dir: dir.to_path_buf(),
        package_manager: "docker".to_string(),
        framework: Some("Docker Compose".to_string()),
        scripts,
    })
}

fn parse_package_json(root: &Path, dir: &Path, pkg_json_path: &Path) -> Option<DiscoveredPackage> {
    let content = std::fs::read_to_string(pkg_json_path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;

    let scripts_obj = json.get("scripts")?.as_object()?;
    if scripts_obj.is_empty() {
        return None;
    }

    let mut scripts: Vec<(String, String)> = scripts_obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|cmd| (k.clone(), cmd.to_string())))
        .collect();

    if scripts.is_empty() {
        return None;
    }

    // Sort scripts by priority (`dev`, `start`, `serve`, `preview`, `build`, `lint`, `test`, then alphabetical)
    scripts.sort_by(|(a, _), (b, _)| {
        let pa = SCRIPT_PRIORITY
            .iter()
            .position(|p| *p == a.as_str())
            .unwrap_or(usize::MAX);
        let pb = SCRIPT_PRIORITY
            .iter()
            .position(|p| *p == b.as_str())
            .unwrap_or(usize::MAX);
        pa.cmp(&pb).then_with(|| a.cmp(b))
    });

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let rel_id = dir
        .strip_prefix(root)
        .ok()
        .and_then(|rel| {
            let s = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_else(|| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(".")
                .to_string()
        });

    let package_manager = detect_package_manager(&json, dir, root);
    let framework = detect_framework(&json, &scripts);

    Some(DiscoveredPackage {
        id: rel_id,
        name,
        dir: dir.to_path_buf(),
        package_manager,
        framework,
        scripts,
    })
}

fn detect_package_manager(json: &Value, dir: &Path, root: &Path) -> String {
    if let Some(pm_field) = json.get("packageManager").and_then(|v| v.as_str()) {
        let pm_lower = pm_field.to_lowercase();
        if pm_lower.starts_with("pnpm") {
            return "pnpm".to_string();
        }
        if pm_lower.starts_with("bun") {
            return "bun".to_string();
        }
        if pm_lower.starts_with("yarn") {
            return "yarn".to_string();
        }
        if pm_lower.starts_with("npm") {
            return "npm".to_string();
        }
    }

    let mut cursor = Some(dir);
    while let Some(curr) = cursor {
        if curr.join("pnpm-lock.yaml").is_file() || curr.join("pnpm-workspace.yaml").is_file() {
            return "pnpm".to_string();
        }
        if curr.join("bun.lockb").is_file() || curr.join("bun.lock").is_file() {
            return "bun".to_string();
        }
        if curr.join("yarn.lock").is_file() {
            return "yarn".to_string();
        }
        if curr.join("package-lock.json").is_file() {
            return "npm".to_string();
        }
        if curr == root {
            break;
        }
        cursor = curr.parent();
    }

    "npm".to_string()
}

fn detect_framework(json: &Value, scripts: &[(String, String)]) -> Option<String> {
    let has_dep = |dep_name: &str| -> bool {
        for section in &["dependencies", "devDependencies"] {
            if let Some(obj) = json.get(*section).and_then(|v| v.as_object())
                && obj.contains_key(dep_name)
            {
                return true;
            }
        }
        false
    };

    if has_dep("next") {
        return Some("Next.js".to_string());
    }
    if has_dep("nuxt") {
        return Some("Nuxt".to_string());
    }
    if has_dep("astro") {
        return Some("Astro".to_string());
    }
    if has_dep("@sveltejs/kit") || has_dep("svelte") {
        return Some("Svelte".to_string());
    }
    if has_dep("@remix-run/dev") || has_dep("@react-router/dev") {
        return Some("Remix".to_string());
    }
    if has_dep("wxt") || has_dep("plasmo") {
        return Some("Extension".to_string());
    }
    if has_dep("electron") {
        return Some("Electron".to_string());
    }
    if has_dep("vite") {
        return Some("Vite".to_string());
    }
    if has_dep("@angular/core") {
        return Some("Angular".to_string());
    }
    if has_dep("vue") || has_dep("@vue/cli-service") {
        return Some("Vue".to_string());
    }
    if has_dep("react-scripts") || has_dep("react") {
        return Some("React".to_string());
    }

    for (_, cmd) in scripts {
        let c = cmd.to_lowercase();
        if c.contains("next ") {
            return Some("Next.js".to_string());
        }
        if c.contains("nuxt ") {
            return Some("Nuxt".to_string());
        }
        if c.contains("astro ") {
            return Some("Astro".to_string());
        }
        if c.contains("vite") {
            return Some("Vite".to_string());
        }
    }

    None
}

/// Match a user query against discovered packages (exact ID, short folder name, package name, or substring)
pub fn filter_packages<'a>(
    packages: &'a [DiscoveredPackage],
    query: &str,
) -> Vec<&'a DiscoveredPackage> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return packages.iter().collect();
    }

    let exact: Vec<&'a DiscoveredPackage> = packages
        .iter()
        .filter(|p| {
            p.id.to_lowercase() == q
                || p.short_name().to_lowercase() == q
                || p.name
                    .as_deref()
                    .map(|n| n.to_lowercase() == q)
                    .unwrap_or(false)
        })
        .collect();
    if exact.len() == 1 {
        return exact;
    }

    packages
        .iter()
        .filter(|p| {
            p.id.to_lowercase().contains(&q)
                || p.short_name().to_lowercase().contains(&q)
                || p.name
                    .as_deref()
                    .map(|n| n.to_lowercase().contains(&q))
                    .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_temp_test_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dev_scan_test_{}_{}",
            std::process::id(),
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_scan_full_recursive_and_excludes_node_modules() {
        let root = make_temp_test_dir("recursive");

        let studio_dir = root.join("apps").join("studio");
        fs::create_dir_all(&studio_dir).unwrap();
        fs::write(studio_dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();
        fs::write(
            studio_dir.join("package.json"),
            r#"{
                "name": "@tuquet/studio",
                "scripts": {
                    "build": "tsc && vite build",
                    "dev": "vite --port 5173",
                    "preview": "vite preview"
                },
                "devDependencies": {
                    "vite": "^5.0.0"
                }
            }"#,
        )
        .unwrap();

        let web_dir = root
            .join("services")
            .join("portal")
            .join("packages")
            .join("web-dashboard");
        fs::create_dir_all(&web_dir).unwrap();
        fs::write(web_dir.join("bun.lock"), "").unwrap();
        fs::write(
            web_dir.join("package.json"),
            r#"{
                "name": "web-dashboard",
                "scripts": {
                    "dev": "next dev",
                    "build": "next build",
                    "lint": "next lint"
                },
                "dependencies": {
                    "next": "15.0.0",
                    "react": "19.0.0"
                }
            }"#,
        )
        .unwrap();

        let nm_pkg = studio_dir.join("node_modules").join("some-lib");
        fs::create_dir_all(&nm_pkg).unwrap();
        fs::write(
            nm_pkg.join("package.json"),
            r#"{"name": "some-lib", "scripts": {"dev": "echo should not be found"}}"#,
        )
        .unwrap();

        for excluded in &["target", "dist", ".next", ".git", "AppData", "scoop"] {
            let ex_dir = root.join(excluded).join("nested");
            fs::create_dir_all(&ex_dir).unwrap();
            fs::write(
                ex_dir.join("package.json"),
                r#"{"name": "ignored-pkg", "scripts": {"dev": "echo ignored"}}"#,
            )
            .unwrap();
        }

        let discovered = scan_workspace_packages(&root);
        let _ = fs::remove_dir_all(&root);

        assert_eq!(
            discovered.len(),
            2,
            "Expected exactly 2 discovered frontend packages"
        );

        let studio = discovered
            .iter()
            .find(|p| p.name.as_deref() == Some("@tuquet/studio"))
            .unwrap();
        assert_eq!(studio.id, "apps/studio");
        assert_eq!(studio.package_manager, "pnpm");
        assert_eq!(studio.framework.as_deref(), Some("Vite"));
        assert_eq!(
            studio.scripts[0].0, "dev",
            "dev script should be sorted first"
        );
        assert_eq!(studio.default_script_index(), 0);

        let web = discovered
            .iter()
            .find(|p| p.name.as_deref() == Some("web-dashboard"))
            .unwrap();
        assert_eq!(web.id, "services/portal/packages/web-dashboard");
        assert_eq!(web.package_manager, "bun");
        assert_eq!(web.framework.as_deref(), Some("Next.js"));
        assert!(web.scripts.iter().any(|(k, _)| k == "dev"));
    }

    #[test]
    fn test_smart_parent_workspace_fallback_when_in_rust_subrepo() {
        let workspace_root = make_temp_test_dir("parent_fallback");
        let cli_subrepo = workspace_root.join("cli");
        let fe_subrepo = workspace_root.join("web-app");

        fs::create_dir_all(&cli_subrepo).unwrap();
        fs::create_dir_all(&fe_subrepo).unwrap();

        fs::write(cli_subrepo.join("Cargo.toml"), "[package]\nname = \"cli\"").unwrap();
        fs::write(
            fe_subrepo.join("package.json"),
            r#"{
                "name": "@tuquet/web-app",
                "packageManager": "yarn@4.0.0",
                "scripts": {
                    "dev": "astro dev",
                    "build": "astro build"
                },
                "dependencies": {
                    "astro": "^4.0.0"
                }
            }"#,
        )
        .unwrap();

        let discovered = scan_workspace_packages(&cli_subrepo);
        let _ = fs::remove_dir_all(&workspace_root);

        assert_eq!(
            discovered.len(),
            1,
            "Should fall back to parent workspace and find sibling web-app"
        );
        assert_eq!(discovered[0].id, "web-app");
        assert_eq!(discovered[0].name.as_deref(), Some("@tuquet/web-app"));
        assert_eq!(discovered[0].package_manager, "yarn");
        assert_eq!(discovered[0].framework.as_deref(), Some("Astro"));
    }
}
