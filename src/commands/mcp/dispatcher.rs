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
        "tuquet_status" => status::execute_tuquet_status().await,
        "tuquet_workflow_list" => workflow::execute_tuquet_workflow_list(&arguments).await,
        "tuquet_workflow_inspect" => workflow::execute_tuquet_workflow_inspect(&arguments).await,
        "tuquet_workflow_run" => workflow::execute_tuquet_workflow_run(&arguments).await,
        "tuquet_runner_probe" => runner::execute_tuquet_runner_probe().await,
        "tuquet_cloud_whoami" => status::execute_tuquet_cloud_whoami().await,
        "tuquet_browser_status" => status::execute_tuquet_browser_status().await,
        "tuquet_tree" => tree::execute_tuquet_tree(&arguments).await,
        "tuquet_faker_generate" => faker::execute_tuquet_faker_generate(&arguments).await,
        "tuquet_inbox_otp" => inbox::execute_tuquet_inbox_otp(&arguments).await,
        "tuquet_inbox_status" => inbox::execute_tuquet_inbox_status().await,
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
