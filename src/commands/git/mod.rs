use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::commands::bridge::start::{BridgeStartOptions, start_bridge};
use crate::commands::dev::scanner::{
    collect_machine_scan_roots, format_clean_package_id, get_user_home_dir, should_skip_dir_entry,
};
use crate::infrastructure::bridge::probe_port;
use crate::ui::{
    ClackOption, ClackSpinner, Column, Notify, OutputFormat, Table, TabularRow, badge_online,
    badge_warn, clack_cancel, clack_intro, clack_outro, clack_select, colors, create_tabular_card,
};

const MAX_GIT_SCAN_DEPTH: usize = 6;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredGitRepo {
    pub id: String,
    pub dir: PathBuf,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
}

impl DiscoveredGitRepo {
    pub fn needs_attention(&self) -> bool {
        self.modified > 0
            || self.staged > 0
            || self.untracked > 0
            || self.ahead > 0
            || self.behind > 0
    }

    pub fn status_summary(&self) -> String {
        let mut parts = Vec::new();
        if self.ahead > 0 {
            parts.push(format!("↑{} ahead", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("↓{} behind", self.behind));
        }
        if self.staged > 0 {
            parts.push(format!("+{} staged", self.staged));
        }
        if self.modified > 0 {
            parts.push(format!("~{} modified", self.modified));
        }
        if self.untracked > 0 {
            parts.push(format!("?{} untracked", self.untracked));
        }
        if parts.is_empty() {
            "clean".to_string()
        } else {
            parts.join(", ")
        }
    }
}

pub async fn handle(
    repo_query: Option<String>,
    action_query: Option<String>,
    all: bool,
    list: bool,
    path: Option<PathBuf>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let interactive_flow = !list && (repo_query.is_none() || action_query.is_none());

    if interactive_flow {
        clack_intro("tq git", "Multi-Repo Git Radar");
    }

    let sp = if interactive_flow {
        Some(ClackSpinner::start(
            "Scanning machine for Git repositories...",
        ))
    } else {
        None
    };

    let (scan_label, all_repos) = match path {
        Some(ref custom_root) => (
            custom_root.display().to_string(),
            scan_git_repositories(Some(custom_root), &cwd),
        ),
        None => (
            "Full Machine".to_string(),
            scan_git_repositories(None, &cwd),
        ),
    };

    let repos: Vec<DiscoveredGitRepo> = if all || repo_query.is_some() {
        all_repos
    } else {
        let dirty: Vec<DiscoveredGitRepo> = all_repos
            .iter()
            .filter(|r| r.needs_attention())
            .cloned()
            .collect();
        if dirty.is_empty() { all_repos } else { dirty }
    };

    if let Some(spinner) = sp {
        let dirty_count = repos.iter().filter(|r| r.needs_attention()).count();
        spinner.finish(format!(
            "Discovered {} repos ({} needing attention • {})",
            repos.len(),
            dirty_count,
            scan_label
        ));
    }

    if list {
        render_git_list(&repos, &scan_label, format);
        return Ok(());
    }

    if repos.is_empty() {
        Notify::info(format!("No Git repositories found ({}).", scan_label));
        return Ok(());
    }

    let target_repo: &DiscoveredGitRepo = match repo_query {
        None => {
            if repos.len() == 1 {
                &repos[0]
            } else {
                let clack_opts: Vec<ClackOption> = repos
                    .iter()
                    .map(|r| {
                        let marker = if r.needs_attention() { "● " } else { "○ " };
                        let label = format!("{}{} [{}]", marker, r.id, r.branch);
                        let hint = format!("{} — {}", r.status_summary(), r.dir.display());
                        let summary = format!("{} [{}] ({})", r.id, r.branch, r.status_summary());
                        ClackOption::new(label, hint, summary)
                    })
                    .collect();

                match clack_select("Select a Git repository", &clack_opts, 0, 10)? {
                    Some(idx) => &repos[idx],
                    None => {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    }
                }
            }
        }
        Some(ref q) => {
            let q_lower = q.trim().to_lowercase();
            let matched: Vec<&DiscoveredGitRepo> = repos
                .iter()
                .filter(|r| r.id.to_lowercase().contains(&q_lower))
                .collect();

            if matched.is_empty() {
                return Err(format!("No Git repository matching '{}' found.", q).into());
            } else if matched.len() == 1 {
                if interactive_flow {
                    println!("  \x1b[32m◇\x1b[0m  \x1b[1mRepository\x1b[0m");
                    println!(
                        "  \x1b[90m│\x1b[0m  \x1b[36m{} [{}] ({})\x1b[0m",
                        matched[0].id,
                        matched[0].branch,
                        matched[0].status_summary()
                    );
                    println!("  \x1b[90m│\x1b[0m");
                }
                matched[0]
            } else {
                let clack_opts: Vec<ClackOption> = matched
                    .iter()
                    .map(|r| {
                        let label = format!("{} [{}]", r.id, r.branch);
                        let hint = format!("{} — {}", r.status_summary(), r.dir.display());
                        let summary = format!("{} [{}] ({})", r.id, r.branch, r.status_summary());
                        ClackOption::new(label, hint, summary)
                    })
                    .collect();
                match clack_select(
                    &format!("Multiple repositories match '{}'. Select repository", q),
                    &clack_opts,
                    0,
                    10,
                )? {
                    Some(idx) => matched[idx],
                    None => {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    }
                }
            }
        }
    };

    let actions = [
        (
            "push-bridge",
            "Push via Bridge (SOCKS5 127.0.0.1:1080) — Bypasses corporate HTTP 403",
        ),
        ("status", "Inspect detailed git status & diff summary"),
        ("pull", "Pull latest changes (git pull --rebase)"),
        ("push", "Standard git push"),
        ("stash", "Stash uncommitted changes (git stash push -u)"),
    ];

    let selected_action: &str = match action_query {
        Some(ref act) => actions
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(act))
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                format!(
                    "Unknown git action '{}'. Available: push-bridge, status, pull, push, stash",
                    act
                )
            })?,
        None => {
            let act_opts: Vec<ClackOption> = actions
                .iter()
                .map(|(id, desc)| {
                    ClackOption::new(*id, *desc, format!("{} ({})", id, target_repo.id))
                })
                .collect();
            match clack_select(
                &format!(
                    "Select action for {} [{}]",
                    target_repo.id, target_repo.branch
                ),
                &act_opts,
                0,
                6,
            )? {
                Some(idx) => actions[idx].0,
                None => {
                    clack_cancel("Operation cancelled.");
                    return Ok(());
                }
            }
        }
    };

    execute_git_action(target_repo, selected_action).await
}

async fn execute_git_action(
    repo: &DiscoveredGitRepo,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        "push-bridge" => {
            if !probe_port(1080) {
                println!(
                    "  \x1b[36m◆\x1b[0m  Starting Bridge SOCKS5 tunnel (127.0.0.1:1080)..."
                );
                start_bridge(BridgeStartOptions {
                    server: None,
                    tag: Some("git"),
                    http: false,
                    ssh: false,
                    foreground: false,
                })
                .await?;
            }

            clack_outro(&format!(
                "Pushing {BOLD}{CYAN}{id}{RESET} [{branch}] via Bridge {MUTED}(socks5://127.0.0.1:1080){RESET}",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                MUTED = colors::MUTED,
                RESET = colors::RESET,
                id = repo.id,
                branch = repo.branch,
            ));

            run_git_subprocess(&repo.dir, &["push"], true)?;
        }
        "push" => {
            let use_bridge = probe_port(1080);
            clack_outro(&format!(
                "Pushing {BOLD}{CYAN}{id}{RESET} [{branch}]{proxy}",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                RESET = colors::RESET,
                id = repo.id,
                branch = repo.branch,
                proxy = if use_bridge {
                    " (via Bridge :1080)"
                } else {
                    ""
                }
            ));
            run_git_subprocess(&repo.dir, &["push"], use_bridge)?;
        }
        "pull" => {
            let use_bridge = probe_port(1080);
            clack_outro(&format!(
                "Pulling {BOLD}{CYAN}{id}{RESET} [{branch}] (--rebase)",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                RESET = colors::RESET,
                id = repo.id,
                branch = repo.branch,
            ));
            run_git_subprocess(&repo.dir, &["pull", "--rebase"], use_bridge)?;
        }
        "stash" => {
            clack_outro(&format!(
                "Stashing working tree in {BOLD}{CYAN}{id}{RESET}",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                RESET = colors::RESET,
                id = repo.id,
            ));
            run_git_subprocess(&repo.dir, &["stash", "push", "-u"], false)?;
        }
        _ => {
            clack_outro(&format!(
                "Inspecting status of {BOLD}{CYAN}{id}{RESET} {MUTED}({dir}){RESET}",
                BOLD = colors::BOLD,
                CYAN = colors::CYAN,
                MUTED = colors::MUTED,
                RESET = colors::RESET,
                id = repo.id,
                dir = repo.dir.display(),
            ));
            run_git_subprocess(&repo.dir, &["status", "-sb"], false)?;
            println!();
            run_git_subprocess(&repo.dir, &["diff", "--stat"], false)?;
        }
    }

    Ok(())
}

fn run_git_subprocess(
    dir: &Path,
    args: &[&str],
    use_socks_bridge: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if use_socks_bridge {
        cmd.env("ALL_PROXY", "socks5://127.0.0.1:1080")
            .env("HTTPS_PROXY", "socks5://127.0.0.1:1080")
            .env("HTTP_PROXY", "socks5://127.0.0.1:1080");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("git {} exited with status {}", args.join(" "), status).into());
    }
    Ok(())
}

pub fn scan_git_repositories(
    custom_root: Option<&Path>,
    current_dir: &Path,
) -> Vec<DiscoveredGitRepo> {
    let home_dir = get_user_home_dir();
    let repo_dirs: Vec<PathBuf> = match custom_root {
        Some(root) => collect_git_dirs_from_root(root),
        None => {
            let roots = collect_machine_scan_roots();
            std::thread::scope(|s| {
                let handles: Vec<_> = roots
                    .iter()
                    .map(|(scan_dir, _)| s.spawn(move || collect_git_dirs_from_root(scan_dir)))
                    .collect();

                let mut merged = Vec::new();
                for h in handles {
                    if let Ok(dirs) = h.join() {
                        merged.extend(dirs);
                    }
                }
                merged
            })
        }
    };

    let mut seen = std::collections::HashSet::new();
    let unique_dirs: Vec<PathBuf> = repo_dirs
        .into_iter()
        .filter(|d| seen.insert(d.clone()))
        .collect();

    // Inspect git status in parallel across all discovered repositories
    let mut repos: Vec<DiscoveredGitRepo> = std::thread::scope(|s| {
        let handles: Vec<_> = unique_dirs
            .iter()
            .map(|dir| {
                let id = format_clean_package_id(dir, current_dir, home_dir.as_deref());
                s.spawn(move || inspect_git_repo(dir, id))
            })
            .collect();

        let mut results = Vec::new();
        for h in handles {
            if let Ok(Some(repo)) = h.join() {
                results.push(repo);
            }
        }
        results
    });

    repos.sort_by(|a, b| {
        b.needs_attention()
            .cmp(&a.needs_attention())
            .then_with(|| b.ahead.cmp(&a.ahead))
            .then_with(|| {
                (b.modified + b.staged + b.untracked).cmp(&(a.modified + a.staged + a.untracked))
            })
            .then_with(|| a.id.cmp(&b.id))
    });

    repos
}

fn collect_git_dirs_from_root(root: &Path) -> Vec<PathBuf> {
    let mut git_dirs = Vec::new();
    walk_for_git_dirs(root, 0, &mut git_dirs);
    git_dirs
}

fn walk_for_git_dirs(current: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_GIT_SCAN_DEPTH {
        return;
    }

    if current.join(".git").exists() {
        out.push(current.to_path_buf());
    }

    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        if should_skip_dir_entry(&entry) {
            continue;
        }
        walk_for_git_dirs(&entry.path(), depth + 1, out);
    }
}

fn inspect_git_repo(dir: &Path, id: String) -> Option<DiscoveredGitRepo> {
    let output = Command::new("git")
        .args([
            "-C",
            &dir.to_string_lossy(),
            "status",
            "--porcelain=v1",
            "-b",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Some(parse_git_status_porcelain(&text, id, dir.to_path_buf()))
}

pub fn parse_git_status_porcelain(text: &str, id: String, dir: PathBuf) -> DiscoveredGitRepo {
    let mut branch = "HEAD".to_string();
    let mut ahead = 0usize;
    let mut behind = 0usize;
    let mut modified = 0usize;
    let mut staged = 0usize;
    let mut untracked = 0usize;

    for line in text.lines() {
        if let Some(header) = line.strip_prefix("## ") {
            // e.g. `main...origin/main [ahead 2, behind 1]` or `No commits yet on main`
            let branch_part = header.split("...").next().unwrap_or(header);
            let branch_clean = branch_part
                .split_whitespace()
                .next()
                .unwrap_or("HEAD")
                .to_string();
            branch = branch_clean;

            if let Some(bracket_start) = header.find('[')
                && let Some(bracket_end) = header[bracket_start..].find(']')
            {
                let inside = &header[bracket_start + 1..bracket_start + bracket_end];
                for part in inside.split(',') {
                    let p = part.trim();
                    if let Some(n) = p.strip_prefix("ahead ")
                        && let Ok(val) = n.trim().parse::<usize>()
                    {
                        ahead = val;
                    } else if let Some(n) = p.strip_prefix("behind ")
                        && let Ok(val) = n.trim().parse::<usize>()
                    {
                        behind = val;
                    }
                }
            }
            continue;
        }

        if line.len() < 2 {
            continue;
        }
        let bytes = line.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;

        if x == '?' && y == '?' {
            untracked += 1;
        } else {
            if x != ' ' && x != '?' {
                staged += 1;
            }
            if y != ' ' && y != '?' {
                modified += 1;
            }
        }
    }

    DiscoveredGitRepo {
        id,
        dir,
        branch,
        ahead,
        behind,
        modified,
        staged,
        untracked,
    }
}

fn render_git_list(repos: &[DiscoveredGitRepo], scan_label: &str, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(repos) {
                println!("{}", json);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new(vec![
                Column {
                    title: "REPOSITORY".to_string(),
                    min_width: 24,
                    align_right: false,
                },
                Column {
                    title: "BRANCH".to_string(),
                    min_width: 14,
                    align_right: false,
                },
                Column {
                    title: "SYNC".to_string(),
                    min_width: 12,
                    align_right: false,
                },
                Column {
                    title: "WORKING TREE".to_string(),
                    min_width: 24,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Scanned {} repository(s) ({}) • Run 'tq git [repo]' for actions",
                repos.len(),
                scan_label
            ));

            for r in repos {
                let sync = if r.ahead == 0 && r.behind == 0 {
                    "synced".to_string()
                } else {
                    format!("↑{} ↓{}", r.ahead, r.behind)
                };
                table.add_row(vec![
                    r.id.clone(),
                    r.branch.clone(),
                    sync,
                    r.status_summary(),
                ]);
            }
            println!();
            table.print();
            println!();
        }
        OutputFormat::Card => {
            let rows: Vec<TabularRow> = repos
                .iter()
                .map(|r| {
                    let badge = if r.needs_attention() {
                        badge_warn(&r.status_summary())
                    } else {
                        badge_online("CLEAN")
                    };
                    TabularRow::new(&r.id, &r.branch, r.dir.display().to_string(), badge)
                })
                .collect();

            let footer = format!(
                "Found {} repo(s) • Run 'tq git' to push via Bridge or inspect",
                repos.len()
            );
            let card = create_tabular_card(
                "MULTI-REPO GIT RADAR (SPC GIT)",
                Some(badge_online(&format!("{} REPOS", repos.len()))),
                ["REPOSITORY", "BRANCH", "DIRECTORY", "STATUS"],
                &rows,
                Some(&footer),
                76,
            );
            println!();
            card.print();
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_status_porcelain() {
        let sample = "\
## main...origin/main [ahead 2, behind 1]
M  src/lib.rs
 M src/main.rs
MM Cargo.toml
?? new_file.rs
";
        let repo = parse_git_status_porcelain(
            sample,
            "tuquet/cli".to_string(),
            PathBuf::from("C:/repo/cli"),
        );
        assert_eq!(repo.branch, "main");
        assert_eq!(repo.ahead, 2);
        assert_eq!(repo.behind, 1);
        assert_eq!(repo.staged, 2);
        assert_eq!(repo.modified, 2);
        assert_eq!(repo.untracked, 1);
        assert!(repo.needs_attention());
    }
}
