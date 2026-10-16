use crate::otp_api::*;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Continuation {
    #[serde(default)]
    output_files: Vec<String>,
    #[serde(default = "empty_object")]
    data: Value,
}
fn empty_object() -> Value {
    json!({})
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Handoff {
    file_paths: Vec<String>,
    prompt_text: String,
}

pub(super) fn invoke(
    context: &InvocationContext,
    input: ToolInput,
    host: &dyn OtpHost,
) -> Result<ToolResult, String> {
    let source = context
        .source
        .as_ref()
        .ok_or("This tool requires a Workflow node session")?;
    let node = host.node(context, &source.node_id)?;
    let source_node = json!({"id":node.id,"name":node.name});
    let (output, payload): (&str, Value) = match (context.capability.tool.as_str(), input) {
        ("trigger_workflow_continuation", ToolInput::Mcp(arguments)) => {
            let args: Continuation =
                serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            (
                "continuation",
                json!({"outputFiles":args.output_files,"sourceNode":source_node,"data":args.data}),
            )
        }
        ("handoff_to_agent", ToolInput::Mcp(arguments)) => {
            let args: Handoff = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            (
                "handoff",
                json!({"filePaths":args.file_paths,"promptText":args.prompt_text,"sourceNode":source_node}),
            )
        }
        ("on_invocation_completed", ToolInput::SessionEvent { status, output }) => {
            if status != "completed" {
                return Ok(ToolResult::default());
            }
            (
                "completed",
                json!({"output":output,"sourceNode":source_node}),
            )
        }
        _ => return Err("Tool input does not match the declared Workflow entrypoint".into()),
    };
    let receipt = host.emit(context, output, payload)?;
    Ok(ToolResult {
        text: format!(
            "Workflow output dispatched {} prompt delivery(s) and requested {} cancellation(s).",
            receipt.deliveries, receipt.stops
        ),
        session_requests: vec![],
        stop_requests: vec![],
    })
}
