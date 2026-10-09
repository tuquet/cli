use serde_json::{json, Value};
use super::protocol::JsonRpcResponse;

pub fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::ok(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "specter-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

pub fn handle_ping(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::ok(id, json!({}))
}

pub fn handle_resources_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::ok(id, json!({ "resources": [] }))
}

pub fn handle_prompts_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::ok(id, json!({ "prompts": [] }))
}

pub fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tools = json!([
        {
            "name": "specter_status",
            "description": "Inspect unified status across Specter subsystems: Cloud enrollment identity, local Runner daemon health, and isolated Chromium runtime.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "specter_workflow_list",
            "description": "List all browser automation workflows stored in the local SQLite database and file vault (~/.specter/automa/workflows). Supports optional search filter.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "search": {
                        "type": "string",
                        "description": "Optional keyword to filter workflows by ID, name, or description."
                    }
                }
            }
        },
        {
            "name": "specter_workflow_inspect",
            "description": "Inspect the structure of a workflow (blocks sequence, triggers, variables, parameters, node connections) by file path or stored workflow ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow": {
                        "type": "string",
                        "description": "Stored workflow ID (e.g. 'hn-crawler') or path to .json workflow file."
                    }
                },
                "required": ["workflow"]
            }
        },
        {
            "name": "specter_workflow_run",
            "description": "Execute a browser automation workflow (by file path or stored ID) in headless or visible browser with optional variables and timeout.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow": {
                        "type": "string",
                        "description": "Workflow ID or path to workflow JSON file."
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "Run in background headless mode. Default is true."
                    },
                    "browser": {
                        "type": "string",
                        "description": "Browser engine to launch (chrome, edge, brave). Default is chrome."
                    },
                    "variables": {
                        "type": "object",
                        "description": "Key-value map of runtime variables to inject into the workflow execution.",
                        "additionalProperties": { "type": "string" }
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Execution timeout in seconds. Default is 300."
                    }
                },
                "required": ["workflow"]
            }
        },
        {
            "name": "specter_runner_probe",
            "description": "Probe local host hardware specs (CPU cores, memory), display capabilities, and runner automation driver capabilities.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "specter_cloud_whoami",
            "description": "Get current Specter Cloud workstation pairing identity, device ID, tenant, and endpoint configuration.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "specter_browser_status",
            "description": "Check the status, version, executable path, and disk usage of the dedicated isolated Chromium runtime.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "specter_tree",
            "description": "Scan directory structure recursively up to a specified depth, ignoring build artifacts (.git, node_modules, target, dist, build, cache) and returning a clean visual tree.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Root directory path to scan. Defaults to current directory ('.')."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum directory traversal depth (default: 3, max: 10)."
                    },
                    "show_hidden": {
                        "type": "boolean",
                        "description": "Whether to include hidden files (starting with dot). Default is false."
                    }
                }
            }
        },
        {
            "name": "specter_faker_generate",
            "description": "Generate synthetic persona and identity test data with legally compliant Vietnamese Citizen Identity numbers (CCCD), cohesive hierarchical addresses, enterprise credentials, and demographic distributions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "Number of profiles to generate (default: 1, max: 1000)."
                    },
                    "gender": {
                        "type": "string",
                        "description": "Gender filter: 'male', 'female', or 'all'. Default is 'all'."
                    },
                    "nat": {
                        "type": "string",
                        "description": "Nationality filter: 'VN', 'US', or 'all'. Default is 'VN'."
                    },
                    "avatar": {
                        "type": "string",
                        "description": "Avatar style: 'real' (portrait photography) or 'svg' (vector avatar). Default is 'real'."
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional custom email domain (e.g. 'flowup.io.vn'). If omitted, follows ~/.specter/faker/faker.json configuration."
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format: 'json' or 'csv'. Default is 'json'."
                    },
                    "output_file": {
                        "type": "string",
                        "description": "Optional file path to save the generated dataset to disk."
                    }
                }
            }
        },
        {
            "name": "specter_inbox_otp",
            "description": "Retrieve or wait for latest OTP verification code for an email address from Catch-All inbox with automatic consumption.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "recipient": {
                        "type": "string",
                        "description": "Target recipient email address (e.g. 'acc01@domain.com')."
                    },
                    "wait": {
                        "type": "boolean",
                        "description": "Whether to wait and poll until email/OTP arrives (default: true)."
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Maximum wait timeout in seconds (default: 60)."
                    }
                },
                "required": ["recipient"]
            }
        },
        {
            "name": "specter_inbox_status",
            "description": "Inspect Catch-All inbox subsystem health, local webhook daemon status, cached email counts, and cloud sync status.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ]);

    JsonRpcResponse::ok(id, json!({ "tools": tools }))
}
