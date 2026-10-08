use automa_core::cli::Cli;
use automa_core::infrastructure::manifest::{find_by_route, list_by_pillar, manifest, raw_json};
use clap::CommandFactory;

#[test]
fn test_manifest_schema_and_json_validity() {
    let m = manifest();

    // 1. Basic manifest identity & SSOT standards
    assert_eq!(m.name, "tuquet", "Manifest name must be 'tuquet'");
    assert!(!m.version.is_empty(), "Manifest version must not be empty");
    assert!(!m.description.is_empty(), "Manifest description must not be empty");

    // 2. Canonical pillars check (5 microservice storage pillars + execution runtimes)
    let required_pillars = ["system", "automa", "browser", "bridge", "faker", "runner", "cloud"];
    for p in &required_pillars {
        assert!(
            m.pillars.iter().any(|item| item.eq_ignore_ascii_case(p)),
            "Pillar '{}' must be registered in cli.manifest.json pillars list",
            p
        );
    }

    // 3. Raw JSON export check
    let raw = raw_json();
    assert!(raw.is_object(), "raw_json() must return a JSON object");
    assert!(raw.get("commands").is_some(), "raw_json must contain 'commands'");
}

#[test]
fn test_command_manifest_quality_and_schema_contract() {
    let m = manifest();
    assert!(
        m.commands.len() >= 30,
        "Manifest must declare at least 30 commands, found {}",
        m.commands.len()
    );

    let mut seen_ids = std::collections::HashSet::new();

    for cmd in &m.commands {
        // Assert ID uniqueness & convention
        assert!(
            cmd.id.starts_with("tuquet."),
            "Command ID '{}' must start with 'tuquet.'",
            cmd.id
        );
        assert!(
            seen_ids.insert(&cmd.id),
            "Duplicate command ID found in manifest: '{}'",
            cmd.id
        );

        // Assert Pillar membership
        assert!(
            m.pillars.contains(&cmd.pillar),
            "Command '{}' specifies unknown pillar '{}'",
            cmd.id,
            cmd.pillar
        );

        // Assert metadata quality
        assert!(!cmd.title.trim().is_empty(), "Command '{}' title must not be empty", cmd.id);
        assert!(
            cmd.description.trim().len() >= 10,
            "Command '{}' description is too short: '{}'",
            cmd.id,
            cmd.description
        );
        assert!(
            !cmd.route.is_empty(),
            "Command '{}' route must contain at least 1 path segment",
            cmd.id
        );

        // Assert arguments metadata
        for arg in &cmd.arguments {
            assert!(!arg.name.trim().is_empty(), "Command '{}' has arg with empty name", cmd.id);
            assert!(!arg.arg_type.trim().is_empty(), "Command '{}' arg '{}' has empty type", cmd.id, arg.name);
            assert!(!arg.description.trim().is_empty(), "Command '{}' arg '{}' has empty description", cmd.id, arg.name);
        }

        // Assert options metadata
        for opt in &cmd.options {
            assert!(!opt.name.trim().is_empty(), "Command '{}' has option with empty name", cmd.id);
            assert!(!opt.opt_type.trim().is_empty(), "Command '{}' option '{}' has empty type", cmd.id, opt.name);
            assert!(!opt.description.trim().is_empty(), "Command '{}' option '{}' has empty description", cmd.id, opt.name);
        }
    }
}

#[test]
fn test_clap_to_manifest_contract() {
    let m = manifest();
    let app = Cli::command();

    for cmd in &m.commands {
        let route = &cmd.route;
        let mut current_cmd = &app;
        let mut path_traversed = Vec::new();

        for segment in route {
            path_traversed.push(segment.as_str());
            match current_cmd.find_subcommand(segment) {
                Some(sub) => {
                    current_cmd = sub;
                }
                None => {
                    panic!(
                        "Contract Violation: Route {:?} declared in cli.manifest.json failed to resolve at segment '{}' in Clap Cli::command()",
                        route, segment
                    );
                }
            }
        }
    }
}

#[test]
fn test_mcp_exported_tools_contract() {
    let m = manifest();
    let mut seen_tool_names = std::collections::HashSet::new();

    for cmd in &m.commands {
        if let Some(ref mcp) = cmd.mcp {
            if mcp.exported {
                assert!(
                    mcp.tool_name.starts_with("tuquet_"),
                    "MCP tool name '{}' for command '{}' must start with 'tuquet_'",
                    mcp.tool_name,
                    cmd.id
                );
                assert!(
                    seen_tool_names.insert(&mcp.tool_name),
                    "Duplicate MCP tool name '{}' found",
                    mcp.tool_name
                );
            }
        }
    }

    assert!(
        seen_tool_names.len() >= 5,
        "Expected at least 5 exported MCP tools, found {}",
        seen_tool_names.len()
    );
}

#[test]
fn test_runtime_manifest_queries() {
    let browser_cmds = list_by_pillar("browser");
    assert!(!browser_cmds.is_empty(), "Expected browser commands in manifest");

    let bridge_cmds = list_by_pillar("bridge");
    assert!(!bridge_cmds.is_empty(), "Expected bridge commands in manifest");

    let schema_cmd = find_by_route(&["schema", "commands"]);
    assert!(schema_cmd.is_some(), "Expected 'specter schema commands' to resolve via find_by_route");
    assert_eq!(schema_cmd.unwrap().id, "tuquet.schema.commands");
}

#[test]
fn test_config_schemas_contract() {
    let services = ["bridge", "browser", "automa", "runner", "faker", "system"];
    for s in services {
        let path = std::path::Path::new("schema/config").join(format!("{}.schema.json", s));
        assert!(path.exists(), "Schema file {:?} must exist", path);
        let content = std::fs::read_to_string(&path).expect("Read schema file");
        let val: serde_json::Value = serde_json::from_str(&content).expect("Valid JSON");
        assert_eq!(
            val.get("$schema").and_then(|v| v.as_str()),
            Some("https://json-schema.org/draft/2020-12/schema"),
            "Schema for {} must adhere to Draft 2020-12",
            s
        );
        assert!(val.get("title").is_some(), "Schema for {} must have a title", s);
        assert!(val.get("properties").is_some(), "Schema for {} must have properties", s);
    }
}
