use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};

use crate::commands::dev::history::load_history;
use crate::commands::dev::scanner::{
    filter_packages, scan_machine_packages, scan_workspace_packages, DiscoveredPackage,
};
use crate::infrastructure::bridge::tools::find_executable;
use crate::ui::{
    badge_online, clack_cancel, clack_intro, clack_outro, clack_select, colors,
    create_tabular_card, ClackOption, ClackSpinner, Column, Notify, OutputFormat, Table,
    TabularRow,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEditor {
    pub id: String,
    pub label: String,
    pub binary: String,
}

pub fn detect_available_editors() -> Vec<DetectedEditor> {
    let mut editors = Vec::new();

    for (id, label, candidates) in &[
        ("code", "Visual Studio Code", &["code.cmd", "code"][..]),
        ("agy", "Antigravity IDE", &["agy.cmd", "agy"][..]),
        ("cursor", "Cursor IDE", &["cursor.cmd", "cursor"][..]),
    ] {
        for bin in *candidates {
            if find_executable(bin).is_some() {
                editors.push(DetectedEditor {
                    id: (*id).to_string(),
                    label: (*label).to_string(),
                    binary: (*bin).to_string(),
                });
                break;
            }
        }
    }

    #[cfg(windows)]
    editors.push(DetectedEditor {
        id: "explorer".to_string(),
        label: "Windows File Explorer".to_string(),
        binary: "explorer.exe".to_string(),
    });

    #[cfg(target_os = "macos")]
    editors.push(DetectedEditor {
        id: "explorer".to_string(),
        label: "macOS Finder".to_string(),
        binary: "open".to_string(),
    });

    #[cfg(all(unix, not(target_os = "macos")))]
    editors.push(DetectedEditor {
        id: "explorer".to_string(),
        label: "System File Manager".to_string(),
        binary: "xdg-open".to_string(),
    });

    editors
}

pub async fn handle(
    project: Option<String>,
    editor: Option<String>,
    list: bool,
    path: Option<PathBuf>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let interactive_flow = !list && (project.is_none() || editor.is_none());

    if interactive_flow {
        clack_intro("spc code", "Project Finder & IDE Launcher");
    }

    let sp = if interactive_flow {
        Some(ClackSpinner::start("Scanning machine for projects..."))
    } else {
        None
    };

    let (scan_label, mut packages) = match path {
        Some(ref custom_root) => (
            custom_root.display().to_string(),
            scan_workspace_packages(custom_root),
        ),
        None => ("Full Machine".to_string(), scan_machine_packages(&cwd)),
    };

    if let Some(spinner) = sp {
        spinner.finish(&format!(
            "Discovered {} projects ({})",
            packages.len(),
            scan_label
        ));
    }

    // Prioritize projects from recent history at the top
    let history = load_history();
    packages.sort_by(|a, b| {
        let a_hist = history.iter().position(|h| h.project_dir == a.dir);
        let b_hist = history.iter().position(|h| h.project_dir == b.dir);
        match (a_hist, b_hist) {
            (Some(ia), Some(ib)) => ia.cmp(&ib),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    let editors = detect_available_editors();

    if list {
        render_code_list(&packages, &editors, &scan_label, format);
        return Ok(());
    }

    if packages.is_empty() {
        Notify::warn(format!("No projects found ({}).", scan_label));
        return Ok(());
    }

    let target_pkg: &DiscoveredPackage = match project {
        None => {
            let clack_opts: Vec<ClackOption> = packages
                .iter()
                .map(|p| {
                    let is_recent = history.iter().any(|h| h.project_dir == p.dir);
                    let prefix = if is_recent { "★ " } else { "" };
                    let fw = p.framework.as_deref().unwrap_or("Project");
                    let label = format!("{}{}", prefix, p.id);
                    let hint = format!("{} ({}) — {}", fw, p.package_manager, p.dir.display());
                    let summary = format!("{} ({})", p.id, p.dir.display());
                    ClackOption::new(label, hint, summary)
                })
                .collect();

            match clack_select("Select a project to open", &clack_opts, 0, 10)? {
                Some(idx) => &packages[idx],
                None => {
                    clack_cancel("Operation cancelled.");
                    return Ok(());
                }
            }
        }
        Some(ref q) => {
            let matched = filter_packages(&packages, q);
            if matched.is_empty() {
                return Err(format!("No project matching '{}' found.", q).into());
            } else if matched.len() == 1 {
                if interactive_flow {
                    println!("  \x1b[32m◇\x1b[0m  \x1b[1mProject\x1b[0m");
                    println!(
                        "  \x1b[90m│\x1b[0m  \x1b[36m{} ({})\x1b[0m",
                        matched[0].id,
                        matched[0].dir.display()
                    );
                    println!("  \x1b[90m│\x1b[0m");
                }
                matched[0]
            } else {
                let clack_opts: Vec<ClackOption> = matched
                    .iter()
                    .map(|p| {
                        let fw = p.framework.as_deref().unwrap_or("Project");
                        let hint = format!("{} ({}) — {}", fw, p.package_manager, p.dir.display());
                        let summary = format!("{} ({})", p.id, p.dir.display());
                        ClackOption::new(&p.id, hint, summary)
                    })
                    .collect();
                match clack_select(
                    &format!("Multiple projects match '{}'. Select project", q),
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

    let selected_editor: &DetectedEditor = match editor {
        Some(ref ed_query) => editors
            .iter()
            .find(|e| e.id.eq_ignore_ascii_case(ed_query) || e.binary.eq_ignore_ascii_case(ed_query))
            .ok_or_else(|| {
                let avail: Vec<&str> = editors.iter().map(|e| e.id.as_str()).collect();
                format!(
                    "Editor '{}' not found. Available: {}",
                    ed_query,
                    avail.join(", ")
                )
            })?,
        None => {
            if editors.len() == 1 {
                &editors[0]
            } else {
                let ed_opts: Vec<ClackOption> = editors
                    .iter()
                    .map(|e| {
                        ClackOption::new(
                            &e.label,
                            format!("{} {}", e.id, target_pkg.dir.display()),
                            format!("{} ({})", e.label, e.id),
                        )
                    })
                    .collect();
                match clack_select("Select IDE or File Manager", &ed_opts, 0, 6)? {
                    Some(idx) => &editors[idx],
                    None => {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    }
                }
            }
        }
    };

    launch_editor(selected_editor, &target_pkg.dir, &target_pkg.id)
}

fn launch_editor(
    editor: &DetectedEditor,
    project_dir: &Path,
    project_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        if editor.id == "explorer" {
            Command::new("explorer.exe")
                .arg(project_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        } else {
            Command::new("cmd")
                .arg("/C")
                .arg(&editor.binary)
                .arg(project_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
    }

    #[cfg(not(windows))]
    {
        Command::new(&editor.binary)
            .arg(project_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    clack_outro(&format!(
        "Opened {BOLD}{CYAN}{id}{RESET} in {BOLD_WHITE}{ed}{RESET} {MUTED}({dir}){RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        id = project_id,
        ed = editor.label,
        dir = project_dir.display(),
    ));
    Ok(())
}

fn render_code_list(
    packages: &[DiscoveredPackage],
    editors: &[DetectedEditor],
    scan_label: &str,
    format: OutputFormat,
) {
    let ed_summary: Vec<String> = editors
        .iter()
        .map(|e| format!("{} ({})", e.label, e.id))
        .collect();

    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "editors": editors,
                "projects": packages,
            });
            if let Ok(json) = serde_json::to_string_pretty(&payload) {
                println!("{}", json);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new(vec![
                Column {
                    title: "PROJECT ID".to_string(),
                    min_width: 22,
                    align_right: false,
                },
                Column {
                    title: "STACK".to_string(),
                    min_width: 16,
                    align_right: false,
                },
                Column {
                    title: "DIRECTORY".to_string(),
                    min_width: 34,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Editors: {} • Scanned {} projects ({})",
                ed_summary.join(", "),
                packages.len(),
                scan_label
            ));

            for p in packages {
                let stack = p.framework.as_deref().unwrap_or(&p.package_manager);
                table.add_row(vec![
                    p.id.clone(),
                    stack.to_string(),
                    p.dir.display().to_string(),
                ]);
            }
            println!();
            table.print();
            println!();
        }
        OutputFormat::Card => {
            let rows: Vec<TabularRow> = packages
                .iter()
                .map(|p| {
                    let stack = p.framework.as_deref().unwrap_or(&p.package_manager);
                    TabularRow::new(
                        &p.id,
                        p.name.as_deref().unwrap_or("-"),
                        p.dir.display().to_string(),
                        badge_online(stack),
                    )
                })
                .collect();

            let footer = format!(
                "Detected Editors: {} • Run 'spc code <project>' to open",
                ed_summary.join(", ")
            );
            let card = create_tabular_card(
                "WORKSPACE IDE LAUNCHER (SPC CODE)",
                Some(badge_online(&format!("{} PROJECTS", packages.len()))),
                ["PROJECT ID", "NAME", "DIRECTORY", "STACK"],
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
    fn test_detect_available_editors_always_has_explorer() {
        let editors = detect_available_editors();
        assert!(
            editors.iter().any(|e| e.id == "explorer"),
            "File manager ('explorer') should always be available as a fallback launcher"
        );
    }
}
