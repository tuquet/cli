use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::infrastructure::db::AutomaDb;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    #[serde(default)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[tuquet-mcp] Starting Tuquet MCP Server (JSON-RPC 2.0 stdio)...");

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let resp_str = serde_json::to_string(&err_resp)?;
                stdout.write_all(resp_str.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        // Notifications don't require responses
        if request.id.is_none() && request.method.starts_with("notifications/") {
            continue;
        }

        let id = request.id.clone();
        let resp = match request.method.as_str() {
            "initialize" => handle_initialize(id),
            "ping" => handle_ping(id),
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, request.params).await,
            "resources/list" => handle_resources_list(id),
            "prompts/list" => handle_prompts_list(id),
            other => {
                eprintln!("[tuquet-mcp] Unknown method requested: {}", other);
                JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", other),
                        data: None,
                    }),
                }
            }
        };

        let resp_str = serde_json::to_string(&resp)?;
        stdout.write_all(resp_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    eprintln!("[tuquet-mcp] Stdio stream closed, exiting.");
    Ok(())
}

fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "tuquet-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        error: None,
    }
}

fn handle_ping(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({})),
        error: None,
    }
}

fn handle_resources_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({ "resources": [] })),
        error: None,
    }
}

fn handle_prompts_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({ "prompts": [] })),
        error: None,
    }
}

fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tools = json!([
        {
            "name": "tuquet_status",
            "description": "Inspect unified status across Tuquet subsystems: Cloud enrollment identity, local Runner daemon health, and isolated Chromium runtime.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "tuquet_workflow_list",
            "description": "List all browser automation workflows stored in the local SQLite database and file vault (~/.tuquet/workflows). Supports optional search filter.",
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
            "name": "tuquet_workflow_inspect",
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
            "name": "tuquet_workflow_run",
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
            "name": "tuquet_runner_probe",
            "description": "Probe local host hardware specs (CPU cores, memory), display capabilities, and runner automation driver capabilities.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "tuquet_cloud_whoami",
            "description": "Get current Tuquet Cloud workstation pairing identity, device ID, tenant, and endpoint configuration.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "tuquet_browser_status",
            "description": "Check the status, version, executable path, and disk usage of the dedicated isolated Chromium runtime.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ]);

    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

async fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let p = match params {
        Some(v) => v,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params: missing arguments".to_string(),
                    data: None,
                }),
            };
        }
    };

    let tool_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = p.get("arguments").cloned().unwrap_or(json!({}));

    let call_result = match tool_name {
        "tuquet_status" => execute_tuquet_status().await,
        "tuquet_workflow_list" => execute_tuquet_workflow_list(&arguments).await,
        "tuquet_workflow_inspect" => execute_tuquet_workflow_inspect(&arguments).await,
        "tuquet_workflow_run" => execute_tuquet_workflow_run(&arguments).await,
        "tuquet_runner_probe" => execute_tuquet_runner_probe().await,
        "tuquet_cloud_whoami" => execute_tuquet_cloud_whoami().await,
        "tuquet_browser_status" => execute_tuquet_browser_status().await,
        other => {
            return JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Tool not found: {}", other),
                    data: None,
                }),
            };
        }
    };

    match call_result {
        Ok(text_output) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "content": [
                    {
                        "type": "text",
                        "text": text_output
                    }
                ],
                "isError": false
            })),
            error: None,
        },
        Err(err_msg) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Error: {}", err_msg)
                    }
                ],
                "isError": true
            })),
            error: None,
        },
    }
}

async fn execute_tuquet_status() -> Result<String, String> {
    let config = AppConfig::load();

    // 1. Cloud
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;
    let cloud_info = if let Some(ref creds) = cloud_creds {
        json!({
            "enrolled": true,
            "device_id": creds.device_id,
            "device_name": creds.name,
            "tenant_id": creds.tenant_id.as_deref().unwrap_or("Personal Workspace"),
            "endpoint": creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co")
        })
    } else {
        json!({
            "enrolled": false,
            "message": "Workstation not enrolled with cloud fleet. Run 'tuquet login' to authenticate."
        })
    };

    // 2. Runner Daemon
    let host = std::env::var("AUTOMA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("AUTOMA_PORT").unwrap_or_else(|_| config.server_port.to_string());
    let daemon_url = format!("http://{}:{}", host, port);
    let health_url = format!("{}/api/v1/health", daemon_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
        .map_err(|e| e.to_string())?;

    let runner_online = match client.get(&health_url).send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    };

    let runner_info = json!({
        "online": runner_online,
        "endpoint": daemon_url,
        "port": port
    });

    // 3. Browser
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let browser_info = json!({
        "installed": browser_status.installed,
        "executable_path": browser_status.executable_path,
        "pinned_version": browser_status.pinned_version,
        "platform": browser_status.platform,
        "size_mb": browser_status.size_mb
    });

    let result = json!({
        "ecosystem": "tuquet",
        "cloud": cloud_info,
        "runner": runner_info,
        "browser": browser_info
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

async fn execute_tuquet_workflow_list(args: &Value) -> Result<String, String> {
    let search = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config = AppConfig::load();

    #[derive(Serialize)]
    struct WfSummary {
        id: String,
        name: String,
        description: String,
        version: String,
        blocks_count: usize,
        source: String,
    }

    let mut workflows: Vec<WfSummary> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // 1. SQLite Database
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
        && let Ok(list) = db.workflows().get_workflows(None, None, search.as_deref())
    {
        for wf in list {
            seen_ids.insert(wf.id.clone());
            let parsed: Option<Value> = serde_json::from_str(&wf.data).ok();
            let blocks_count = parsed.as_ref().map(count_workflow_blocks).unwrap_or(0);
            workflows.push(WfSummary {
                id: wf.id,
                name: wf.name,
                description: wf.description.unwrap_or_default(),
                version: wf.version,
                blocks_count,
                source: "Database".to_string(),
            });
        }
    }

    // 2. Vault Directory ~/.tuquet/workflows
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let tuquet_vault = PathBuf::from(&home).join(".tuquet").join("workflows");
    let config_vault = PathBuf::from(&config.data_dir).join("workflows");

    for vdir in &[tuquet_vault, config_vault] {
        if let Ok(mut entries) = tokio::fs::read_dir(vdir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file()
                    && path.extension().map(|e| e == "json").unwrap_or(false)
                    && let Ok(content) = tokio::fs::read_to_string(&path).await
                    && let Ok(val) = serde_json::from_str::<Value>(&content)
                {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wf");
                    let id = val
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| stem.trim_end_matches(".workflow").to_string());
                    let name = val
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| id.clone());
                    let desc = val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if let Some(ref q) = search {
                        let q_lower = q.to_lowercase();
                        if !id.to_lowercase().contains(&q_lower)
                            && !name.to_lowercase().contains(&q_lower)
                            && !desc.to_lowercase().contains(&q_lower)
                        {
                            continue;
                        }
                    }

                    if seen_ids.contains(&id) {
                        if let Some(item) = workflows.iter_mut().find(|w| w.id == id) {
                            item.source = "Database+Vault".to_string();
                        }
                        continue;
                    }

                    seen_ids.insert(id.clone());
                    let version = val
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string();
                    let blocks_count = count_workflow_blocks(&val);

                    workflows.push(WfSummary {
                        id,
                        name,
                        description: desc,
                        version,
                        blocks_count,
                        source: "Vault".to_string(),
                    });
                }
            }
        }
    }

    Ok(serde_json::to_string_pretty(&workflows).unwrap_or_default())
}

fn count_workflow_blocks(val: &Value) -> usize {
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        nodes.len()
    } else if let Some(drawflow) = val.get("drawflow") {
        if let Some(s) = drawflow.as_str() {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).map(|a| a.len()))
                .unwrap_or(0)
        } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
            nodes.len()
        } else {
            0
        }
    } else {
        0
    }
}

async fn execute_tuquet_workflow_inspect(args: &Value) -> Result<String, String> {
    let target = args
        .get("workflow")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'workflow' argument".to_string())?;

    let (content, source_label) = resolve_workflow_content(target).await?;

    let val: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow JSON: {}", e))?;

    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or(target);
    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or(id);
    let description = val.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

    let nodes = extract_nodes_array(&val);
    let mut block_types = Vec::new();
    let mut triggers = Vec::new();

    for node in &nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let label = node
            .get("label")
            .or_else(|| node.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        block_types.push(json!({
            "id": node_id,
            "type": label,
        }));

        if label == "trigger" {
            let t_type = node.pointer("/data/type").and_then(|v| v.as_str()).unwrap_or("manual");
            let params = node.pointer("/data/parameters").cloned().unwrap_or(json!([]));
            triggers.push(json!({
                "id": node_id,
                "type": t_type,
                "parameters": params
            }));
        }
    }

    let variables = val.get("variables").cloned().unwrap_or(json!({}));

    let inspect_result = json!({
        "id": id,
        "name": name,
        "description": description,
        "version": version,
        "source": source_label,
        "blocks_count": nodes.len(),
        "triggers": triggers,
        "variables": variables,
        "blocks": block_types
    });

    Ok(serde_json::to_string_pretty(&inspect_result).unwrap_or_default())
}

async fn resolve_workflow_content(target: &str) -> Result<(String, String), String> {
    let target_path = Path::new(target);
    if target_path.exists() && target_path.is_file() {
        let text = tokio::fs::read_to_string(target_path)
            .await
            .map_err(|e| format!("Error reading file {:?}: {}", target_path, e))?;
        return Ok((text, format!("File: {}", target_path.display())));
    }

    let config = AppConfig::load();

    // Check DB
    let db_path = PathBuf::from(&config.data_dir).join("automa.sqlite");
    if db_path.exists()
        && let Ok(db) = AutomaDb::new(&db_path)
        && let Ok(Some(wf)) = db.workflows().get_workflow_by_id_or_name(target)
    {
        return Ok((wf.data, format!("Database (ID: {})", wf.id)));
    }

    // Check Vault
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let tuquet_vault = PathBuf::from(&home).join(".tuquet").join("workflows");
    let config_vault = PathBuf::from(&config.data_dir).join("workflows");

    let candidates = [
        tuquet_vault.join(format!("{}.workflow.json", target)),
        tuquet_vault.join(format!("{}.json", target)),
        config_vault.join(format!("{}.workflow.json", target)),
        config_vault.join(format!("{}.json", target)),
    ];

    for c in &candidates {
        if c.exists() && c.is_file()
            && let Ok(text) = tokio::fs::read_to_string(c).await
        {
            return Ok((text, format!("Vault: {}", c.display())));
        }
    }

    Err(format!(
        "Workflow '{}' not found in file system, database, or vault.",
        target
    ))
}

fn extract_nodes_array(val: &Value) -> Vec<Value> {
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        nodes.clone()
    } else if let Some(drawflow) = val.get("drawflow") {
        if let Some(s) = drawflow.as_str() {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|d| d.get("nodes").and_then(|n| n.as_array()).cloned())
                .unwrap_or_default()
        } else if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
            nodes.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

async fn execute_tuquet_workflow_run(args: &Value) -> Result<String, String> {
    let workflow = args
        .get("workflow")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'workflow' argument".to_string())?;

    let headless = args.get("headless").and_then(|v| v.as_bool()).unwrap_or(true);
    let browser = args.get("browser").and_then(|v| v.as_str()).unwrap_or("chrome");
    let timeout = args.get("timeout").and_then(|v| v.as_u64());

    let exe = std::env::current_exe().map_err(|e| format!("Cannot find current exe: {}", e))?;

    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("automa").arg("run").arg(workflow);

    if headless {
        cmd.arg("--headless");
    }

    cmd.arg("--browser").arg(browser);

    if let Some(t) = timeout {
        cmd.arg("--timeout").arg(t.to_string());
    }

    if let Some(vars) = args.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars {
            let val_str = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            cmd.arg("--var").arg(format!("{}={}", k, val_str));
        }
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to execute workflow subprocess: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let status_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();

    let result = json!({
        "success": success,
        "exit_code": status_code,
        "workflow": workflow,
        "stdout": stdout_str,
        "stderr": stderr_str
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

async fn execute_tuquet_runner_probe() -> Result<String, String> {
    let manifest = json!({
        "protocol": "tuquet.automa.v1",
        "name": "runner",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "chromium-extension-worker",
        "status": "ready",
        "capabilities": [
            "browser:chromium",
            "mv3_extension_worker",
            "isolation:profile_sandbox",
            "headless",
            "automation:workflow_graph"
        ],
        "plugin_type": "runner_driver",
        "hardware": {
            "cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            "target_arch": std::env::consts::ARCH
        }
    });

    Ok(serde_json::to_string_pretty(&manifest).unwrap_or_default())
}

async fn execute_tuquet_cloud_whoami() -> Result<String, String> {
    let config = AppConfig::load();
    let cloud_creds = CloudReporter::whoami(&config.data_dir).await;

    if let Some(creds) = cloud_creds {
        let res = json!({
            "enrolled": true,
            "device_id": creds.device_id,
            "name": creds.name,
            "tenant_id": creds.tenant_id,
            "cloud_url": creds.cloud_url,
            "enrolled_at": creds.registered_at
        });
        Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
    } else {
        let res = json!({
            "enrolled": false,
            "message": "Workstation is not paired with Tuquet Cloud. Use 'tuquet login' to connect."
        });
        Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
    }
}

async fn execute_tuquet_browser_status() -> Result<String, String> {
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    let res = json!({
        "installed": browser_status.installed,
        "executable_path": browser_status.executable_path,
        "pinned_version": browser_status.pinned_version,
        "platform": browser_status.platform,
        "size_mb": browser_status.size_mb
    });
    Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
}
