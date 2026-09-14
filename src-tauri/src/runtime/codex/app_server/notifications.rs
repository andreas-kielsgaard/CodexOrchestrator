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
            let item = super::items::convert(&params["item"])?;
            Some(
                json!({"type":if method == "item/started" {"item.started"} else {"item.completed"}, "item":item}),
            )
        }
        _ => None,
    }
}
