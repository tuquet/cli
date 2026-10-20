use serde_json::{Value, json};

use super::protocol::JsonRpcResponse;
use super::tools::{faker, inbox, runner, status, tree, workflow};

pub async fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let p = match params {
        Some(v) => v,
        None => {
            return JsonRpcResponse::err(id, -32602, "Invalid params: missing arguments");
        }
    };

    let tool_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = p.get("arguments").cloned().unwrap_or(json!({}));

    let call_result = match tool_name {
        "status" => status::execute_status().await,
        "workflow_list" => workflow::execute_workflow_list(&arguments).await,
        "workflow_inspect" => workflow::execute_workflow_inspect(&arguments).await,
        "workflow_run" => workflow::execute_workflow_run(&arguments).await,
        "runner_probe" => runner::execute_runner_probe().await,
        "cloud_whoami" => status::execute_cloud_whoami().await,
        "browser_status" => status::execute_browser_status().await,
        "tree" => tree::execute_tree(&arguments).await,
        "faker_generate" => faker::execute_faker_generate(&arguments).await,
        "inbox_otp" => inbox::execute_inbox_otp(&arguments).await,
        "inbox_status" => inbox::execute_inbox_status().await,
        other => {
            return JsonRpcResponse::err(id, -32601, format!("Tool not found: {}", other));
        }
    };

    match call_result {
        Ok(text_output) => JsonRpcResponse::ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": text_output
                    }
                ],
                "isError": false
            }),
        ),
        Err(err_msg) => JsonRpcResponse::ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Error: {}", err_msg)
                    }
                ],
                "isError": true
            }),
        ),
    }
}
