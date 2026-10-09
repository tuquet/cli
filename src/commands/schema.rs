use crate::cli::SchemaSubcommands;
use crate::infrastructure::manifest::{manifest, raw_json, CommandDef};
use crate::ui::{badge_online, colors, Card, Column, Table};
use serde_json::json;

pub async fn handle(
    command: Option<SchemaSubcommands>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        None => {
            if json_output {
                let pretty = serde_json::to_string_pretty(&raw_json())?;
                println!("{}", pretty);
            } else {
                show_overview();
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
            if fmt.is_table() {
                show_commands_table(&filtered, pillar.as_deref());
            } else {
                let pretty = serde_json::to_string_pretty(&filtered)?;
                println!("{}", pretty);
            }
        }
        Some(SchemaSubcommands::Config { service }) => {
            match service {
                Some(srv) => {
                    if let Some(schema) = get_config_schema(&srv) {
                        println!("{}", schema.trim());
                    } else {
                        eprintln!(
                            "  {RED}Error:{RESET} Unknown service '{srv}'. Available services: bridge, browser, automa, runner, faker, system, inbox",
                            RED = colors::RED,
                            RESET = colors::RESET,
                        );
                        std::process::exit(1);
                    }
                }
                None => {
                    show_config_overview();
                }
            }
        }
        Some(SchemaSubcommands::Mcp) => {
            let mcp_tools = export_mcp_tools();
            let pretty = serde_json::to_string_pretty(&mcp_tools)?;
            println!("{}", pretty);
        }
    }

    Ok(())
}

fn show_overview() {
    let m = manifest();
    let root = crate::config::canonical_specter_dir();

    println!();
    let mut card = Card::new("SPECTER CLI SCHEMA MANIFEST");
    card.with_badge(badge_online(&format!("v{}", m.version)));
    card.with_min_width(70);

    card.add_kv("Specification", "VS Code Contributes Standard (Draft 2020-12)");
    card.add_kv("Schema ID", m.schema.as_deref().unwrap_or("cli.manifest.schema.json"));
    card.add_kv("Ecosystem Root", root.display().to_string());
    card.add_kv("Total Commands", m.commands.len().to_string());

    for p in &m.pillars {
        let count = m.commands.iter().filter(|c| c.pillar.eq_ignore_ascii_case(p)).count();
        card.add_kv(
            format!("Pillar: {}", p),
            format!("{} command{}", count, if count == 1 { "" } else { "s" }),
        );
    }

    card.with_footer("Inspect: 'specter schema commands' | Export: 'specter schema --json'");
    card.print();
    println!();
}

fn show_commands_table(commands: &[&CommandDef], pillar_filter: Option<&str>) {
    println!();
    let title = if let Some(p) = pillar_filter {
        format!("MANIFEST COMMANDS (PILLAR: {})", p.to_uppercase())
    } else {
        "MANIFEST COMMANDS CATALOG (VS CODE CONTRIBUTES)".to_string()
    };
    println!(
        "  {BOLD}{CYAN}{title}{RESET} {MUTED}— VS Code Contributes Standard{RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        RESET = colors::RESET,
        MUTED = colors::MUTED,
    );

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
            "{CYAN}specter {}{RESET}",
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
            format!("{GREEN}✓{RESET}", GREEN = colors::GREEN, RESET = colors::RESET)
        } else {
            format!("{MUTED}—{RESET}", MUTED = colors::MUTED, RESET = colors::RESET)
        };

        table.add_row(vec![route_str, pillar_badge, title_and_desc, mcp_indicator]);
    }

    let footer_text = format!(
        "Total: {} commands matching filter. Use --json for machine export.",
        commands.len()
    );
    let table = table.with_footer(footer_text);
    println!("{}", table.render());
    println!();
}

fn show_config_overview() {
    println!();
    let mut card = Card::new("MICROSERVICE CONFIGURATION SCHEMAS");
    card.with_badge(badge_online("JSON SCHEMA DRAFT 2020-12"));
    card.with_min_width(70);

    card.add_kv("Standard", "JSON Schema Specification (Draft 2020-12)");
    card.add_kv("Supported Services", "bridge, browser, automa, runner, faker, system, inbox");
    card.add_line("");
    card.add_line(format!(
        "  {BOLD_WHITE}Usage Example:{RESET}",
        BOLD_WHITE = colors::BOLD_WHITE,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}specter schema config bridge{RESET}   Output bridge.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}specter schema config browser{RESET}  Output browser.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));
    card.add_line(format!(
        "    {CYAN}specter schema config automa{RESET}   Output automa.json schema",
        CYAN = colors::CYAN,
        RESET = colors::RESET,
    ));

    card.with_footer("Pass service name argument to export raw JSON Schema definition.");
    card.print();
    println!();
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
                properties.insert(arg.name.clone(), json!({ "type": arg.arg_type, "description": arg.description }));
                if arg.required {
                    required.push(json!(arg.name));
                }
            }

            for opt in &cmd.options {
                properties.insert(opt.name.clone(), json!({ "type": opt.opt_type, "description": opt.description }));
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
