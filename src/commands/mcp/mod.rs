pub mod dispatcher;
pub mod protocol;
pub mod registry;
pub mod tools;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[specter-mcp] Starting Specter MCP Server (JSON-RPC 2.0 stdio)...");

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
                let err_resp = JsonRpcResponse::err(None, -32700, format!("Parse error: {}", e));
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
            "initialize" => registry::handle_initialize(id),
            "ping" => registry::handle_ping(id),
            "tools/list" => registry::handle_tools_list(id),
            "tools/call" => dispatcher::handle_tools_call(id, request.params).await,
            "resources/list" => registry::handle_resources_list(id),
            "prompts/list" => registry::handle_prompts_list(id),
            other => {
                eprintln!("[specter-mcp] Unknown method requested: {}", other);
                JsonRpcResponse::err(id, -32601, format!("Method not found: {}", other))
            }
        };

        let resp_str = serde_json::to_string(&resp)?;
        stdout.write_all(resp_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    eprintln!("[specter-mcp] Stdio stream closed, exiting.");
    Ok(())
}
