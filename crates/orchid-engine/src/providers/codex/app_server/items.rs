//! Pure native item conversion shared by live and stored history.
use serde_json::Value;
pub fn convert(native: &Value) -> Option<Value> {
    let mut item = native.clone();
    let kind = match item["type"].as_str()? {
        "agentMessage" => "agent_message",
        "commandExecution" => "command_execution",
        "fileChange" => "file_change",
        "mcpToolCall" => "mcp_tool_call",
        "webSearch" => "web_search",
        "reasoning" => "reasoning",
        _ => return None,
    };
    item["type"] = kind.into();
    if kind == "command_execution" {
        item["aggregated_output"] = item["aggregatedOutput"].clone();
        item["exit_code"] = item["exitCode"].clone();
    }
    if let Some(changes) = item.get_mut("changes").and_then(Value::as_array_mut) {
        for change in changes {
            if let Some(kind) = change["kind"]["type"].as_str().map(str::to_string) {
                change["kind"] = kind.into();
            }
        }
    }
    Some(item)
}
