use crate::otp_api::*;
use serde::Deserialize;
use serde_json::Value;
use std::cmp::Reverse;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    #[serde(default = "select")]
    mode: String,
    #[serde(default = "first")]
    cardinality: String,
    #[serde(default = "newest")]
    ordering: String,
    #[serde(default = "any")]
    running: String,
    #[serde(default = "create")]
    missing: String,
    #[serde(default)]
    created_by_event: String,
    #[serde(default)]
    created_by_session: String,
}
fn select() -> String {
    "select".into()
}
fn first() -> String {
    "first".into()
}
fn newest() -> String {
    "newest".into()
}
fn any() -> String {
    "any".into()
}
fn create() -> String {
    "create".into()
}
fn read(value: &Value) -> Result<Configuration, String> {
    let c: Configuration = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    for (label, value, options) in [
        ("mode", c.mode.as_str(), vec!["select", "new"]),
        ("cardinality", c.cardinality.as_str(), vec!["first", "all"]),
        (
            "ordering",
            c.ordering.as_str(),
            vec!["newest", "last_addressed"],
        ),
        (
            "running",
            c.running.as_str(),
            vec!["any", "running_only", "not_running"],
        ),
        (
            "missing",
            c.missing.as_str(),
            vec!["create", "fail", "noop"],
        ),
    ] {
        if !options.contains(&value) {
            return Err(format!("Invalid {label}: {value}"));
        }
    }
    Ok(c)
}
pub(super) fn validate(value: &Value) -> Result<(), String> {
    read(value).map(|_| ())
}

pub(super) fn fields() -> Vec<ConfigurationField> {
    [
        ("mode", "Session mode", vec!["select", "new"], "select"),
        (
            "cardinality",
            "Sessions to select",
            vec!["first", "all"],
            "first",
        ),
        (
            "ordering",
            "Order sessions",
            vec!["newest", "last_addressed"],
            "newest",
        ),
        (
            "running",
            "Invocation state",
            vec!["any", "running_only", "not_running"],
            "any",
        ),
        (
            "missing",
            "If no session matches",
            vec!["create", "fail", "noop"],
            "create",
        ),
        ("createdByEvent", "Created by event", vec![], ""),
        ("createdBySession", "Created by session", vec![], ""),
    ]
    .into_iter()
    .map(|(key, label, choices, default)| ConfigurationField {
        key: key.into(),
        label: label.into(),
        choices: choices.into_iter().map(str::to_string).collect(),
        default_value: default.into(),
        when: (key != "mode").then(|| ("mode".into(), "select".into())),
    })
    .collect()
}

pub(super) fn invoke(
    context: &InvocationContext,
    input: ToolInput,
    host: &dyn OtpHost,
) -> Result<ToolResult, String> {
    let ToolInput::Action {
        configuration,
        inputs,
    } = input
    else {
        return Err("Prompt agent requires action input".into());
    };
    let c = read(&configuration)?;
    let node_id = context
        .output_node_id
        .as_deref()
        .ok_or("Prompt agent requires an output node")?;
    host.node(context, node_id)?;
    let prompt = inputs
        .into_iter()
        .map(|input| {
            Ok(PromptPart {
                reference: input.reference,
                text: match input.value {
                    Value::String(text) => text,
                    value => serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?,
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if prompt.iter().all(|part| part.text.trim().is_empty()) {
        return Err("Prompt agent requires prompt content".into());
    }
    let targets = if c.mode == "new" {
        vec![SessionRequestTarget::New]
    } else {
        let mut candidates = host.sessions(context, node_id)?;
        candidates.retain(|s| {
            (c.running == "any"
                || (c.running == "running_only" && s.running)
                || (c.running == "not_running" && !s.running))
                && (c.created_by_event.is_empty()
                    || s.created_by_event.as_deref() == Some(c.created_by_event.as_str()))
                && (c.created_by_session.is_empty()
                    || s.created_by_session.as_deref() == Some(c.created_by_session.as_str()))
        });
        candidates.sort_by_key(|s| {
            (
                Reverse(if c.ordering == "last_addressed" {
                    s.last_addressed_sequence
                } else {
                    None
                }),
                Reverse(s.created_sequence),
                s.id.clone(),
            )
        });
        if c.cardinality == "first" {
            candidates.truncate(1);
        }
        if candidates.is_empty() {
            match c.missing.as_str() {
                "create" => vec![SessionRequestTarget::New],
                "noop" => vec![],
                _ => return Err("No destination session matches the configured selection".into()),
            }
        } else {
            candidates
                .into_iter()
                .map(|s| SessionRequestTarget::Exact { session_id: s.id })
                .collect()
        }
    };
    Ok(ToolResult {
        text: String::new(),
        session_requests: targets
            .into_iter()
            .map(|target| SessionRequest {
                node_id: node_id.into(),
                target,
                prompt: prompt.clone(),
            })
            .collect(),
    })
}
