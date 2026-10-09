use serde_json::{json, Value};

use super::protocol::JsonRpcResponse;
use super::tools::{faker, runner, status, tree, workflow};

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
        "specter_status" => status::execute_specter_status().await,
        "specter_workflow_list" => workflow::execute_specter_workflow_list(&arguments).await,
        "specter_workflow_inspect" => workflow::execute_specter_workflow_inspect(&arguments).await,
        "specter_workflow_run" => workflow::execute_specter_workflow_run(&arguments).await,
        "specter_runner_probe" => runner::execute_specter_runner_probe().await,
        "specter_cloud_whoami" => status::execute_specter_cloud_whoami().await,
        "specter_browser_status" => status::execute_specter_browser_status().await,
        "specter_tree" => tree::execute_specter_tree(&arguments).await,
        "specter_faker_generate" => faker::execute_specter_faker_generate(&arguments).await,
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
