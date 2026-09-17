use crate::otp_api::*;
use serde::Deserialize;
use serde_json::Value;
use std::cmp::Reverse;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    #[serde(default = "newest")]
    ordering: String,
}
fn newest() -> String {
    "newest".into()
}
pub(super) fn validate(value: &Value) -> Result<(), String> {
    let config: Configuration = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if !["newest", "last_addressed"].contains(&config.ordering.as_str()) {
        return Err("Invalid Stop session ordering".into());
    }
    Ok(())
}
pub(super) fn fields() -> Vec<ConfigurationField> {
    vec![ConfigurationField {
        key: "ordering".into(),
        label: "Order running sessions".into(),
        choices: vec!["newest".into(), "last_addressed".into()],
        default_value: newest(),
        when: None,
    }]
}
pub(super) fn invoke(
    context: &InvocationContext,
    input: ToolInput,
    host: &dyn OtpHost,
) -> Result<ToolResult, String> {
    let ToolInput::Action { configuration, .. } = input else {
        return Err("Stop session requires action input".into());
    };
    validate(&configuration)?;
    let config: Configuration = serde_json::from_value(configuration).map_err(|e| e.to_string())?;
    let node = context
        .output_node_id
        .as_deref()
        .ok_or("Stop session requires a destination node")?;
    host.node(context, node)?;
    let mut sessions = host.sessions(context, node)?;
    sessions.retain(|s| s.running);
    sessions.sort_by_key(|s| {
        (
            Reverse(if config.ordering == "last_addressed" {
                s.last_addressed_sequence
            } else {
                None
            }),
            Reverse(s.created_sequence),
            s.id.clone(),
        )
    });
    Ok(match sessions.first() {
        Some(session) => ToolResult {
            stop_requests: vec![SessionStopRequest {
                node_id: node.into(),
                session_id: session.id.clone(),
            }],
            ..ToolResult::default()
        },
        None => ToolResult {
            text: "No running destination session; nothing to stop.".into(),
            ..ToolResult::default()
        },
    })
}
