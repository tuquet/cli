use crate::cli::SchemaSubcommands;
use crate::infrastructure::manifest::{CommandDef, manifest, raw_json};
use crate::ui::{
    Card, ClackOption, Column, Table, badge_online, clack_cancel, clack_intro, clack_outro,
    clack_select, clack_step_info, colors, is_interactive_tty,
};
use serde_json::json;

pub async fn handle(
    command: Option<SchemaSubcommands>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        None => {
            if json_output || !is_interactive_tty() {
                let pretty = serde_json::to_string_pretty(&raw_json())?;
                println!("{}", pretty);
            } else {
                run_interactive_schema()?;
            }
        }
        Some(SchemaSubcommands::Commands { pillar, format }) => {
            let m = manifest();
            let filtered: Vec<&CommandDef> = if let Some(ref p) = pillar {
                m.commands
                    .iter()
                    .filter(|c| c.pillar.eq_ignore_ascii_case(p))
                    .collect()
            } else {
                m.commands.iter().collect()
            };

            let fmt = format.resolve();
            if !json_output && !fmt.is_json() {
                clack_intro("tq schema commands", "CLI Command Manifest Catalog");
                show_commands_table(&filtered, pillar.as_deref());
                clack_outro(&format!(
                    "Listed {} command(s). Use '--json' for machine export.",
                    filtered.len()
                ));
            } else {
                let pretty = serde_json::to_string_pretty(&filtered)?;
                println!("{}", pretty);
            }
        }
        Some(SchemaSubcommands::Config { service }) => match service {
            Some(srv) => {
                if let Some(schema) = get_config_schema(&srv) {
                    println!("{}", schema.trim());
                } else {
                    return Err(format!(
                        "Unknown service '{}'. Available services: bridge, browser, automa, runner, faker, system, inbox",
                        srv
                    )
                    .into());
                }
            }
            None => {
                if is_interactive_tty() && !json_output {
                    clack_intro("tq schema config", "Microservice JSON Schema Definitions");
                    let services = [
                        (
                            "bridge",
                            "Network Bridge & Multi-VPS Mesh (bridge.schema.json)",
                        ),
                        ("faker", "Synthetic Persona Generator (faker.schema.json)"),
                        (
                            "browser",
                            "Antidetect Chromium Runtime (browser.schema.json)",
                        ),
                        ("automa", "Workflow Automation Engine (automa.schema.json)"),
                        ("runner", "Distributed Worker Daemon (runner.schema.json)"),
                        (
                            "system",
                            "Machine Identity & Cloud Fleet (system.schema.json)",
                        ),
                        (
                            "inbox",
                            "Catch-All Email & OTP Interceptor (inbox.schema.json)",
                        ),
                    ];
                    let opts: Vec<ClackOption> = services
                        .iter()
                        .map(|(id, desc)| ClackOption::new(*id, *desc, *id))
                        .collect();
                    match clack_select("Select a microservice schema to inspect", &opts, 0, 8)? {
                        Some(idx) => {
                            let (srv, _) = services[idx];
                            if let Some(schema) = get_config_schema(srv) {
                                println!("{}", schema.trim());
                            }
                            clack_outro(&format!("Exported {}.schema.json.", srv));
                        }
                        None => {
                            clack_cancel("Operation cancelled.");
                        }
                    }
                } else {
                    show_config_overview();
                }
            }
        },
        Some(SchemaSubcommands::Mcp) => {
            let mcp_tools = export_mcp_tools();
            if json_output || !is_interactive_tty() {
                let pretty = serde_json::to_string_pretty(&mcp_tools)?;
                println!("{}", pretty);
            } else {
                clack_intro(
                    "tq schema mcp",
                    "Model Context Protocol (MCP) Tool Definitions",
                );
                let m = manifest();
                let mcp_cmds: Vec<&CommandDef> = m
                    .commands
                    .iter()
                    .filter(|c| c.mcp.as_ref().is_some_and(|m| m.exported))
                    .collect();
                show_commands_table(&mcp_cmds, None);
                clack_outro(&format!(
                    "Listed {} MCP-exported tool(s). Pass 'tq schema --json mcp' for raw JSON Schema.",
                    mcp_cmds.len()
                ));
            }
        }
    }

    Ok(())
}

fn run_interactive_schema() -> Result<(), Box<dyn std::error::Error>> {
    clack_intro(
        "tq schema",
        "CLI Manifest & Microservice JSON Schema Inspector",
    );

    let opts = vec![
        ClackOption::new(
            "Show CLI Manifest Overview Card",
            "Summary of specification version, SSOT root & command counts by pillar",
            "Show CLI Manifest Overview Card",
        ),
        ClackOption::new(
            "Browse Commands Catalog (All Pillars)",
            "Render full table of registered CLI commands & MCP export status",
            "Browse Commands Catalog (All Pillars)",
        ),
        ClackOption::new(
            "Filter Commands by Microservice Pillar",
            "Filter commands by system, automa, browser, bridge, faker, runner, cloud",
            "Filter Commands by Microservice Pillar",
        ),
        ClackOption::new(
            "Inspect Microservice Config JSON Schema",
            "View Draft 2020-12 JSON Schema for bridge, faker, browser, automa, runner, system, inbox",
            "Inspect Microservice Config JSON Schema",
        ),
        ClackOption::new(
            "Export AI Agent MCP Tool Schemas (JSON)",
            "Dump Model Context Protocol tool definitions generated from manifest",
            "Export AI Agent MCP Tool Schemas",
        ),
    ];

    let Some(idx) = clack_select("Select a Schema action", &opts, 0, 6)? else {
        clack_cancel("Operation cancelled.");
        return Ok(());
    };

    let m = manifest();
    match idx {
        0 => {
            show_overview();
            clack_outro("Manifest overview displayed.");
        }
        1 => {
            let all_cmds: Vec<&CommandDef> = m.commands.iter().collect();
            show_commands_table(&all_cmds, None);
            clack_outro(&format!("Listed {} commands.", all_cmds.len()));
        }
        2 => {
            let pillar_opts: Vec<ClackOption> = m
                .pillars
                .iter()
                .map(|p| {
                    let count = m
                        .commands
                        .iter()
                        .filter(|c| c.pillar.eq_ignore_ascii_case(p))
                        .count();
                    ClackOption::new(p, format!("{} commands", count), p)
                })
                .collect();
            if let Some(p_idx) = clack_select("Select target pillar", &pillar_opts, 0, 8)? {
                let target_pillar = &m.pillars[p_idx];
                let filtered: Vec<&CommandDef> = m
                    .commands
                    .iter()
                    .filter(|c| c.pillar.eq_ignore_ascii_case(target_pillar))
                    .collect();
                show_commands_table(&filtered, Some(target_pillar));
                clack_outro(&format!(
                    "Listed {} commands in pillar '{}'.",
                    filtered.len(),
                    target_pillar
                ));
            } else {
                clack_cancel("Operation cancelled.");
            }
        }
        3 => {
            show_config_overview();
            clack_outro("Pass 'tq schema config <service>' to export raw JSON Schema.");
        }
        4 => {
            let mcp_tools = export_mcp_tools();
            let pretty = serde_json::to_string_pretty(&mcp_tools)?;
            println!("{}", pretty);
            clack_outro("Exported MCP tool definitions.");
        }
        _ => {}
    }

    Ok(())
}

fn show_overview() {
    let m = manifest();
    let root = crate::config::canonical_dir();

    let mut card = Card::new("TQ CLI SCHEMA MANIFEST");
    card.with_badge(badge_online(&format!("v{}", m.version)));
    card.with_min_width(70);

    card.add_kv(
        "Specification",
        "VS Code Contributes Standard (Draft 2020-12)",
    );
    card.add_kv(
        "Schema ID",
        m.schema.as_deref().unwrap_or("cli.manifest.schema.json"),
    );
    card.add_kv("Ecosystem Root", root.display().to_string());
    card.add_kv("Total Commands", m.commands.len().to_string());

    for p in &m.pillars {
        let count = m
            .commands
            .iter()
            .filter(|c| c.pillar.eq_ignore_ascii_case(p))
            .count();
        card.add_kv(
            format!("Pillar: {}", p),
            format!("{} command{}", count, if count == 1 { "" } else { "s" }),
        );
    }

    card.with_footer("Inspect: 'tq schema commands' | Export: 'tq schema --json'");
    card.print();
}

fn show_commands_table(commands: &[&CommandDef], pillar_filter: Option<&str>) {
    let title = if let Some(p) = pillar_filter {
        format!("MANIFEST COMMANDS (PILLAR: {})", p.to_uppercase())
    } else {
        "MANIFEST COMMANDS CATALOG (VS CODE CONTRIBUTES)".to_string()
    };
    clack_step_info(&title);

    let cols = vec![
        Column {
            title: "ROUTE / COMMAND".to_string(),
            min_width: 26,
            align_right: false,
        },
        Column {
            title: "PILLAR".to_string(),
            min_width: 10,
            align_right: false,
        },
        Column {
            title: "TITLE & DESCRIPTION".to_string(),
            min_width: 44,
            align_right: false,
        },
        Column {
            title: "MCP".to_string(),
            min_width: 5,
            align_right: false,
        },
    ];

    let mut table = Table::new(cols);

    for cmd in commands {
        let route_str = format!(
            "{CYAN}tq {}{RESET}",
            cmd.route.join(" "),
            CYAN = colors::CYAN,
            RESET = colors::RESET,
        );

        let pillar_badge = format!(
            "{PURPLE}{}{RESET}",
            cmd.pillar,
            PURPLE = colors::PURPLE,
            RESET = colors::RESET,
        );

        let desc_str = if cmd.description.len() > 50 {
            format!("{}...", &cmd.description[..47])
        } else {
            cmd.description.clone()
        };

        let title_and_desc = format!(
            "{BOLD_WHITE}{}{RESET} {MUTED}— {}{RESET}",
            cmd.title,
            desc_str,
            BOLD_WHITE = colors::BOLD_WHITE,
            MUTED = colors::MUTED,
            RESET = colors::RESET,
        );

        let mcp_indicator = if cmd.mcp.as_ref().is_some_and(|m| m.exported) {
            format!(
                "{GREEN}✓{RESET}",
                GREEN = colors::GREEN,
                RESET = colors::RESET
            )
        } else {
            format!(
                "{MUTED}—{RESET}",
                MUTED = colors::MUTED,
                RESET = colors::RESET
            )
        };

        table.add_row(vec![route_str, pillar_badge, title_and_desc, mcp_indicator]);
    }

    let footer_text = format!(
        "Total: {} commands matching filter. Use --json for machine export.",
        commands.len()
    );
    table.with_footer(footer_text).print();
}

fn show_config_overview() {
    let mut card = Card::new("MICROSERVICE CONFIGURATION SCHEMAS");
    card.with_badge(badge_online("JSON SCHEMA DRAFT 2020-12"));
    card.with_min_width(70);

    card.add_kv("Standard", "JSON Schema Specification (Draft 2020-12)");
    card.add_kv(
        "Supported Services",
        "bridge, browser, automa, runner, faker, system, inbox",
    );
    card.add_line("");
    card.add_line(format!(
        "  {BOLD_WHITE}Usage Example:{RESET}",
        BOLD_WHITE = colors::BOLD_WHITE,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}tq schema config bridge{RESET}   Output bridge.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}tq schema config browser{RESET}  Output browser.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}tq schema config automa{RESET}   Output automa.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));

    card.with_footer("Pass service name argument to export raw JSON Schema definition.");
    card.print();
}

const BRIDGE_SCHEMA: &str = include_str!("../../schema/config/bridge.schema.json");
const BROWSER_SCHEMA: &str = include_str!("../../schema/config/browser.schema.json");
const AUTOMA_SCHEMA: &str = include_str!("../../schema/config/automa.schema.json");
const RUNNER_SCHEMA: &str = include_str!("../../schema/config/runner.schema.json");
const FAKER_SCHEMA: &str = include_str!("../../schema/config/faker.schema.json");
const SYSTEM_SCHEMA: &str = include_str!("../../schema/config/system.schema.json");
const INBOX_SCHEMA: &str = include_str!("../../schema/config/inbox.schema.json");

pub fn get_config_schema(service: &str) -> Option<&'static str> {
    match crate::config::ConfigRegistry::canonical_service(service)? {
        "bridge" => Some(BRIDGE_SCHEMA),
        "browser" => Some(BROWSER_SCHEMA),
        "automa" => Some(AUTOMA_SCHEMA),
        "runner" => Some(RUNNER_SCHEMA),
        "faker" => Some(FAKER_SCHEMA),
        "system" => Some(SYSTEM_SCHEMA),
        "inbox" => Some(INBOX_SCHEMA),
        _ => None,
    }
}

fn export_mcp_tools() -> serde_json::Value {
    let m = manifest();
    let tools: Vec<serde_json::Value> = m
        .commands
        .iter()
        .filter_map(|cmd| {
            let mcp = cmd.mcp.as_ref().filter(|m| m.exported)?;
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for arg in &cmd.arguments {
                properties.insert(
                    arg.name.clone(),
                    json!({ "type": arg.arg_type, "description": arg.description }),
                );
                if arg.required {
                    required.push(json!(arg.name));
                }
            }

            for opt in &cmd.options {
                properties.insert(
                    opt.name.clone(),
                    json!({ "type": opt.opt_type, "description": opt.description }),
                );
            }

            Some(json!({
                "name": mcp.tool_name,
                "description": cmd.description,
                "command_id": cmd.id,
                "route": cmd.route,
                "pillar": cmd.pillar,
                "inputSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }))
        })
        .collect();

    json!({ "version": m.version, "tools": tools })
}
