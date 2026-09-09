//! App-server notifications translated into the shared Codex item normalizer.
use serde_json::{json, Value};

pub(super) fn legacy_event(method: &str, params: &Value) -> Option<Value> {
    match method {
        "thread/tokenUsage/updated" => {
            Some(json!({"type":"usage.updated","usage":params["tokenUsage"]["last"]}))
        }
        "turn/plan/updated" => Some(
            json!({"type":"item.updated","item":{"id":"turn-plan","type":"plan_update","text":params["explanation"],"plan":params["plan"]}}),
        ),
        "thread/started" => {
            Some(json!({"type":"thread.started","thread_id":params["thread"]["id"]}))
        }
        "turn/started" => Some(json!({"type":"turn.started"})),
        "turn/completed" => Some(
            json!({"type":if params["turn"]["status"] == "completed" {"turn.completed"} else {"turn.failed"},"error":params["turn"]["error"]}),
        ),
        "item/started" | "item/completed" => {
            let mut item = params["item"].clone();
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
            Some(
                json!({"type":if method == "item/started" {"item.started"} else {"item.completed"}, "item":item}),
            )
        }
        _ => None,
    }
}
