//! App-server notifications translated into the shared Codex item normalizer.
use serde_json::{json, Value};

pub(super) fn legacy_event(method: &str, params: &Value) -> Option<Value> {
    match method {
        "thread/tokenUsage/updated" => {
            Some(json!({"type":"usage.updated","usage":params["tokenUsage"]["last"],"tokenUsage":params["tokenUsage"]}))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_context_capacity_and_cumulative_usage() {
        let params = json!({"tokenUsage":{"last":{"inputTokens":10},"total":{"inputTokens":200},"modelContextWindow":272000}});
        let event = legacy_event("thread/tokenUsage/updated", &params).unwrap();
        assert_eq!(event["usage"]["inputTokens"], 10);
        assert_eq!(event["tokenUsage"], params["tokenUsage"]);
    }
}
