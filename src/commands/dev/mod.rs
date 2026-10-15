pub mod runner;
pub mod scanner;

use std::path::PathBuf;
use crate::ui::{
    badge_online, clack_cancel, clack_intro, clack_select, create_tabular_card, ClackOption,
    Column, Notify, OutputFormat, Table, TabularRow,
};
use scanner::{
    filter_packages, scan_machine_packages, scan_workspace_packages, DiscoveredPackage,
};

/// Parse `<project>:<script>` shorthand if `script` was not passed as a separate positional arg.
/// Preserves Windows drive letters (e.g. `C:\path` where colon is at index 1).
pub fn split_project_and_script(
    project: Option<String>,
    script: Option<String>,
) -> (Option<String>, Option<String>) {
    if script.is_some() {
        return (project, script);
    }
    if let Some(ref p) = project
        && let Some(colon_idx) = p.find(':')
        && colon_idx > 1
    {
        let proj_part = p[..colon_idx].trim();
        let script_part = p[colon_idx + 1..].trim();
        let resolved_proj = if proj_part.is_empty() {
            None
        } else {
            Some(proj_part.to_string())
        };
        let resolved_script = if script_part.is_empty() {
            None
        } else {
            Some(script_part.to_string())
        };
        return (resolved_proj, resolved_script);
    }
    (project, script)
}

pub async fn handle(
    project: Option<String>,
    script: Option<String>,
    list: bool,
    path: Option<PathBuf>,
    pm_override: Option<String>,
    extra_args: Vec<String>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (proj_query, mut script_query) = split_project_and_script(project, script);
    let interactive_flow = !list && (proj_query.is_none() || script_query.is_none());

    if interactive_flow {
        use std::io::Write;
        eprint!("  \x1b[36m◆\x1b[0m  \x1b[90mScanning machine for package.json projects...\x1b[0m\r");
        let _ = std::io::stderr().flush();
    }

    let (scan_label, packages) = match path {
        Some(ref custom_root) => (
            custom_root.display().to_string(),
            scan_workspace_packages(custom_root),
        ),
        None => ("Full Machine".to_string(), scan_machine_packages(&cwd)),
    };

    if interactive_flow {
        use std::io::Write;
        eprint!("\r\x1b[2K");
        let _ = std::io::stderr().flush();
    }

    if list {
        render_package_list(&packages, &scan_label, format);
        return Ok(());
    }

    if packages.is_empty() {
        Notify::warn(format!(
            "No runnable package.json projects (with 'scripts') found ({}).",
            scan_label
        ));
        return Ok(());
    }

    if interactive_flow {
        clack_intro(
            "spc dev",
            &format!(
                "Workspace Frontend Runner • {} projects discovered ({})",
                packages.len(),
                scan_label
            ),
        );
    }

    // 1. Resolve Target Package
    let target_pkg: &DiscoveredPackage = match proj_query {
        None => {
            if packages.len() == 1 {
                &packages[0]
            } else {
                let clack_opts: Vec<ClackOption> =
                    packages.iter().map(build_package_clack_option).collect();
                match clack_select("Select a frontend project", &clack_opts, 0, 10)? {
                    Some(idx) => &packages[idx],
                    None => {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    }
                }
            }
        }
        Some(ref q) => {
            let matched = filter_packages(&packages, q);
            if matched.is_empty() {
                // Smart fallback: if there is only 1 package in workspace and `q` matches one of its scripts,
                // treat `q` as the script name!
                if script_query.is_none()
                    && packages.len() == 1
                    && packages[0].scripts.iter().any(|(k, _)| k.eq_ignore_ascii_case(q))
                {
                    script_query = Some(q.clone());
                    &packages[0]
                } else {
                    let available: Vec<&str> = packages.iter().take(10).map(|p| p.id.as_str()).collect();
                    return Err(format!(
                        "No frontend project matching '{}'. Examples: {}",
                        q,
                        available.join(", ")
                    )
                    .into());
                }
            } else if matched.len() == 1 {
                if interactive_flow {
                    let chosen = build_package_clack_option(matched[0]);
                    println!("  \x1b[32m◇\x1b[0m  \x1b[1mProject\x1b[0m");
                    println!("  \x1b[90m│\x1b[0m  \x1b[36m{}\x1b[0m", chosen.summary);
                    println!("  \x1b[90m│\x1b[0m");
                }
                matched[0]
            } else {
                let clack_opts: Vec<ClackOption> =
                    matched.iter().map(|p| build_package_clack_option(p)).collect();
                match clack_select(
                    &format!("Multiple projects match '{}'. Select target project", q),
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

    // 2. Resolve Target Script (Always prompt with selectable suggestions when not explicitly specified)
    let pm = pm_override.as_deref().unwrap_or(&target_pkg.package_manager);
    let selected_script: String = match script_query {
        Some(ref s) => {
            if let Some((exact_name, _)) = target_pkg
                .scripts
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(s))
            {
                exact_name.clone()
            } else {
                let matching_scripts: Vec<&(String, String)> = target_pkg
                    .scripts
                    .iter()
                    .filter(|(k, _)| k.to_lowercase().contains(&s.to_lowercase()))
                    .collect();

                if matching_scripts.len() == 1 {
                    matching_scripts[0].0.clone()
                } else {
                    match prompt_for_script_clack(target_pkg, pm)? {
                        Some(sc) => sc,
                        None => {
                            clack_cancel("Operation cancelled.");
                            return Ok(());
                        }
                    }
                }
            }
        }
        None => match prompt_for_script_clack(target_pkg, pm)? {
            Some(sc) => sc,
            None => {
                clack_cancel("Operation cancelled.");
                return Ok(());
            }
        },
    };

    // 3. Execute Script (Browser auto-open is intentionally disabled by default)
    runner::run_package_script(
        target_pkg,
        &selected_script,
        pm_override.as_deref(),
        &extra_args,
    )
}

fn build_package_clack_option(pkg: &DiscoveredPackage) -> ClackOption {
    let fw_part = pkg.framework.as_deref().unwrap_or("Node");
    let script_preview: Vec<&str> = pkg
        .scripts
        .iter()
        .take(5)
        .map(|(k, _)| k.as_str())
        .collect();

    let name_tag = match pkg.name {
        Some(ref n) if n != &pkg.id => format!("{} • ", n),
        _ => String::new(),
    };

    let hint = format!(
        "{}{} ({}) — {}",
        name_tag,
        fw_part,
        pkg.package_manager,
        script_preview.join(", ")
    );
    let summary = format!("{} ({} • {})", pkg.id, fw_part, pkg.package_manager);

    ClackOption::new(&pkg.id, hint, summary)
}

fn prompt_for_script_clack(
    pkg: &DiscoveredPackage,
    pm: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let default_idx = pkg.default_script_index();
    let clack_opts: Vec<ClackOption> = pkg
        .scripts
        .iter()
        .map(|(name, cmd)| {
            let hint = format!("{pm} run {name}  →  {cmd}");
            let summary = format!("{name} ({pm} run {name} → {cmd})");
            ClackOption::new(name, hint, summary)
        })
        .collect();

    let prompt_title = format!("Select script to run in {}", pkg.id);
    let chosen_idx = clack_select(&prompt_title, &clack_opts, default_idx, 10)?;
    Ok(chosen_idx.map(|idx| pkg.scripts[idx].0.clone()))
}

fn render_package_list(
    packages: &[DiscoveredPackage],
    scan_label: &str,
    format: OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(packages) {
                println!("{}", json);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new(vec![
                Column {
                    title: "PROJECT ID".to_string(),
                    min_width: 18,
                    align_right: false,
                },
                Column {
                    title: "PACKAGE NAME".to_string(),
                    min_width: 18,
                    align_right: false,
                },
                Column {
                    title: "STACK / PM".to_string(),
                    min_width: 14,
                    align_right: false,
                },
                Column {
                    title: "SCRIPTS".to_string(),
                    min_width: 24,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Scanned {} project(s) ({}) • Run 'specter dev <project> [script]'",
                packages.len(),
                scan_label
            ));

            for p in packages {
                let stack = match p.framework {
                    Some(ref fw) => format!("{} ({})", fw, p.package_manager),
                    None => p.package_manager.clone(),
                };
                let scripts_list: Vec<&str> =
                    p.scripts.iter().take(6).map(|(k, _)| k.as_str()).collect();
                table.add_row(vec![
                    p.id.clone(),
                    p.name.clone().unwrap_or_else(|| "-".to_string()),
                    stack,
                    scripts_list.join(", "),
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
                    let stack = match p.framework {
                        Some(ref fw) => format!("{} • {}", fw, p.package_manager),
                        None => p.package_manager.clone(),
                    };
                    let scripts_list: Vec<&str> =
                        p.scripts.iter().take(5).map(|(k, _)| k.as_str()).collect();
                    TabularRow::new(
                        &p.id,
                        p.name.as_deref().unwrap_or("-"),
                        scripts_list.join(", "),
                        badge_online(&stack),
                    )
                })
                .collect();

            let footer = format!(
                "Found {} package.json project(s) • Run 'specter dev <project>' to select & start",
                packages.len()
            );
            let card = create_tabular_card(
                "WORKSPACE FRONTEND PROJECTS",
                Some(badge_online(&format!("{} FOUND", packages.len()))),
                ["PROJECT PATH", "PACKAGE NAME", "AVAILABLE SCRIPTS", "STACK / PM"],
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
    fn test_split_project_and_script_shorthand() {
        assert_eq!(
            split_project_and_script(Some("studio:dev".to_string()), None),
            (Some("studio".to_string()), Some("dev".to_string()))
        );
        assert_eq!(
            split_project_and_script(Some("apps/web:build".to_string()), None),
            (Some("apps/web".to_string()), Some("build".to_string()))
        );
        assert_eq!(
            split_project_and_script(Some("studio".to_string()), Some("lint".to_string())),
            (Some("studio".to_string()), Some("lint".to_string()))
        );
        // Should not split Windows drive letter paths
        assert_eq!(
            split_project_and_script(Some("C:\\repo\\app".to_string()), None),
            (Some("C:\\repo\\app".to_string()), None)
        );
    }
}
