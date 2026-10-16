use crate::otp_api::*;
use serde::Deserialize;
use serde_json::Value;
use std::{cmp::Reverse, collections::BTreeSet};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FilterOperation {
    Include,
    Exclude,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum SessionFilter {
    IdleSessions {
        operation: FilterOperation,
    },
    CreatedByEvent {
        operation: FilterOperation,
        event_id: String,
    },
    CreatedBySession {
        operation: FilterOperation,
        session_id: String,
    },
}

impl SessionFilter {
    fn kind(&self) -> &'static str {
        match self {
            Self::IdleSessions { .. } => "idle_sessions",
            Self::CreatedByEvent { .. } => "created_by_event",
            Self::CreatedBySession { .. } => "created_by_session",
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::CreatedByEvent { event_id, .. } if event_id.trim().is_empty() => {
                Err("Created by event filter requires an event ID".into())
            }
            Self::CreatedBySession { session_id, .. } if session_id.trim().is_empty() => {
                Err("Created by session filter requires a session ID".into())
            }
            _ => Ok(()),
        }
    }

    fn matches(&self, session: &NodeSession) -> bool {
        let (matches, operation) = match self {
            Self::IdleSessions { operation } => (!session.running, *operation),
            Self::CreatedByEvent {
                operation,
                event_id,
            } => (
                session.created_by_event.as_deref() == Some(event_id.as_str()),
                *operation,
            ),
            Self::CreatedBySession {
                operation,
                session_id,
            } => (
                session.created_by_session.as_deref() == Some(session_id.as_str()),
                *operation,
            ),
        };
        match operation {
            FilterOperation::Include => matches,
            FilterOperation::Exclude => !matches,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    #[serde(default = "select")]
    mode: String,
    #[serde(default = "first")]
    cardinality: String,
    #[serde(default = "newest")]
    ordering: String,
    #[serde(default = "create")]
    missing: String,
    #[serde(default)]
    filters: Vec<SessionFilter>,
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
            "missing",
            c.missing.as_str(),
            vec!["create", "fail", "noop"],
        ),
    ] {
        if !options.contains(&value) {
            return Err(format!("Invalid {label}: {value}"));
        }
    }
    let mut filter_kinds = BTreeSet::new();
    for filter in &c.filters {
        filter.validate()?;
        if !filter_kinds.insert(filter.kind()) {
            return Err(format!("Session filter `{}` is repeated", filter.kind()));
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
            "Sessions to prompt",
            vec!["first", "all"],
            "first",
        ),
        (
            "ordering",
            "Session selection logic",
            vec!["newest", "last_addressed"],
            "newest",
        ),
        (
            "missing",
            "If no session matches",
            vec!["create", "fail", "noop"],
            "create",
        ),
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
        candidates.retain(|session| c.filters.iter().all(|filter| filter.matches(session)));
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
        stop_requests: vec![],
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
