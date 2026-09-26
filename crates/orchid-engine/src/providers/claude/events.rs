//! Claude stream-json messages → normalized runtime events. The raw message travels with each
//! event as evidence; product code reads only the normalized form.
use crate::contracts::{
    domain::{
        agent_message_role, AgentRuntimeEventSource, AgentRuntimeUsage, NormalizedRuntimeEvent,
        NormalizedRuntimeEventKind, NormalizedToolActivity, ToolActivityKind, ToolActivityPhase,
        ToolResultClassification,
    },
    ports::RuntimeEventDraft,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// A tool call awaiting its result.
struct ToolCall {
    kind: ToolActivityKind,
    name: String,
    input: Value,
}

#[derive(Default)]
pub(super) struct ClaudeEvents {
    /// The latest agent text. It becomes the reply if nothing follows it before the result.
    pending_message: Option<(Value, String)>,
    tools: HashMap<String, ToolCall>,
}

impl ClaudeEvents {
    /// Events for one `system`, `assistant` or `user` message.
    pub(super) fn message(&mut self, message: &Value) -> Vec<RuntimeEventDraft> {
        // A subagent's own messages stay evidence; its outcome arrives as the parent tool's result.
        if !message["parent_tool_use_id"].is_null() {
            return vec![evidence(message)];
        }
        match (message["type"].as_str(), message["subtype"].as_str()) {
            (Some("system"), Some("init")) => vec![draft(
                message,
                normalized(NormalizedRuntimeEventKind::ProcessingStarted, None, None),
            )],
            (Some("assistant"), _) => self.assistant(message),
            (Some("user"), _) => self.tool_results(message),
            _ => vec![evidence(message)],
        }
    }

    fn assistant(&mut self, message: &Value) -> Vec<RuntimeEventDraft> {
        let mut events = Vec::new();
        let mut held = false;
        for block in blocks(message) {
            match block["type"].as_str() {
                Some("text") => {
                    let text = block["text"].as_str().unwrap_or_default();
                    if text.trim().is_empty() {
                        continue;
                    }
                    events.extend(self.take_message(agent_message_role::INTERMEDIATE));
                    self.pending_message = Some((message.clone(), text.to_owned()));
                    held = true;
                }
                Some("thinking") => {
                    let text = block["thinking"].as_str().unwrap_or_default();
                    if !text.trim().is_empty() {
                        events.push(draft(
                            message,
                            normalized(
                                NormalizedRuntimeEventKind::ProcessingUpdate,
                                Some(text.to_owned()),
                                Some(json!({"itemType": "thinking"})),
                            ),
                        ));
                    }
                }
                Some("tool_use") => {
                    events.extend(self.take_message(agent_message_role::INTERMEDIATE));
                    let id = block["id"].as_str().unwrap_or_default().to_owned();
                    let call = ToolCall {
                        kind: tool_kind(block["name"].as_str().unwrap_or_default()),
                        name: block["name"].as_str().unwrap_or_default().to_owned(),
                        input: block["input"].clone(),
                    };
                    events.push(tool_event(
                        message,
                        &id,
                        &call,
                        ToolActivityPhase::Started,
                        None,
                    ));
                    self.tools.insert(id, call);
                }
                _ => {}
            }
        }
        if events.is_empty() && !held {
            events.push(evidence(message));
        }
        events
    }

    fn tool_results(&mut self, message: &Value) -> Vec<RuntimeEventDraft> {
        let mut events = Vec::new();
        for block in blocks(message).filter(|block| block["type"] == "tool_result") {
            let id = block["tool_use_id"].as_str().unwrap_or_default();
            let Some(call) = self.tools.remove(id) else {
                continue;
            };
            let succeeded = block["is_error"] != true;
            events.push(tool_event(
                message,
                id,
                &call,
                ToolActivityPhase::Completed,
                Some(succeeded),
            ));
        }
        if events.is_empty() {
            events.push(evidence(message));
        }
        events
    }

    /// Events for a `result`. The last result of an invocation carries its reply; an earlier one
    /// answered a message sent before a later steering message was taken in.
    pub(super) fn result(&mut self, message: &Value, last: bool) -> Vec<RuntimeEventDraft> {
        let succeeded = message["subtype"] == "success" && message["is_error"] != true;
        let role = if last && succeeded {
            agent_message_role::FINAL
        } else {
            agent_message_role::INTERMEDIATE
        };
        let mut events: Vec<_> = self.take_message(role).into_iter().collect();
        if let Some(usage) = message["usage"].as_object() {
            let count = |key: &str| usage.get(key).and_then(Value::as_u64);
            let cached = count("cache_read_input_tokens");
            let input = [
                count("input_tokens"),
                count("cache_creation_input_tokens"),
                cached,
            ]
            .into_iter()
            .flatten()
            .reduce(|total, value| total + value);
            events.push(RuntimeEventDraft {
                source: AgentRuntimeEventSource::Stdout,
                raw_payload: message.clone(),
                normalized: Some(NormalizedRuntimeEvent {
                    usage: Some(AgentRuntimeUsage {
                        input_tokens: input,
                        cached_input_tokens: cached,
                        output_tokens: count("output_tokens"),
                    }),
                    ..normalized(NormalizedRuntimeEventKind::Usage, None, None)
                }),
            });
        }
        if last && succeeded {
            events.push(draft(
                message,
                normalized(NormalizedRuntimeEventKind::InvocationCompleted, None, None),
            ));
        }
        events
    }

    fn take_message(&mut self, role: &str) -> Option<RuntimeEventDraft> {
        self.pending_message.take().map(|(raw, text)| {
            draft(
                &raw,
                normalized(
                    NormalizedRuntimeEventKind::AgentMessage,
                    Some(text),
                    Some(json!({"role": role})),
                ),
            )
        })
    }
}

/// The failure text a `result` reports, if it reports one.
pub(super) fn result_error(message: &Value) -> Option<String> {
    if message["subtype"] == "success" && message["is_error"] != true {
        return None;
    }
    let errors = message["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|errors| !errors.is_empty());
    Some(
        message["result"]
            .as_str()
            .map(str::to_owned)
            .or(errors)
            .unwrap_or_else(|| {
                format!(
                    "Claude reported {}",
                    message["subtype"].as_str().unwrap_or("an error")
                )
            }),
    )
}

fn tool_kind(name: &str) -> ToolActivityKind {
    match name {
        "Bash" => ToolActivityKind::Command,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => ToolActivityKind::FileChange,
        "WebSearch" | "WebFetch" => ToolActivityKind::WebSearch,
        "TodoWrite" => ToolActivityKind::Plan,
        name if name.starts_with("mcp__") => ToolActivityKind::McpTool,
        _ => ToolActivityKind::Other,
    }
}

fn tool_event(
    message: &Value,
    id: &str,
    call: &ToolCall,
    phase: ToolActivityPhase,
    succeeded: Option<bool>,
) -> RuntimeEventDraft {
    let input = &call.input;
    let text = [
        "command",
        "query",
        "url",
        "file_path",
        "notebook_path",
        "description",
    ]
    .iter()
    .find_map(|key| input[*key].as_str())
    .map(str::to_owned);
    let mut details = json!({"itemType": call.name});
    if call.kind == ToolActivityKind::FileChange && succeeded == Some(true) {
        if let Some(path) = input["file_path"]
            .as_str()
            .or(input["notebook_path"].as_str())
        {
            let created = call.name == "Write" && message["tool_use_result"]["type"] == "create";
            details["fileChanges"] =
                json!([{"path": path, "operation": if created { "create" } else { "edit" }}]);
        }
    }
    let (server, tool) = match call
        .name
        .strip_prefix("mcp__")
        .and_then(|name| name.split_once("__"))
    {
        Some((server, tool)) => (Some(server.to_owned()), Some(tool.to_owned())),
        None => (None, None),
    };
    let (status, result_classification) = match succeeded {
        None => (None, ToolResultClassification::Unknown),
        Some(true) => (
            Some("completed".to_owned()),
            ToolResultClassification::Succeeded,
        ),
        Some(false) => (Some("failed".to_owned()), ToolResultClassification::Failed),
    };
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: message.clone(),
        normalized: Some(NormalizedRuntimeEvent {
            tool_activity: Some(NormalizedToolActivity {
                kind: call.kind,
                phase,
                item_id: Some(id.to_owned()),
                server,
                tool,
                status,
                result_classification,
            }),
            ..normalized(
                NormalizedRuntimeEventKind::ToolActivity,
                text,
                Some(details),
            )
        }),
    }
}

fn blocks(message: &Value) -> impl Iterator<Item = &Value> {
    message["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
}

fn normalized(
    kind: NormalizedRuntimeEventKind,
    text: Option<String>,
    details: Option<Value>,
) -> NormalizedRuntimeEvent {
    NormalizedRuntimeEvent {
        kind,
        text,
        external_context_id: None,
        usage: None,
        details,
        tool_activity: None,
    }
}

fn draft(raw: &Value, normalized: NormalizedRuntimeEvent) -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: raw.clone(),
        normalized: Some(normalized),
    }
}

/// A message kept as diagnostic evidence only.
pub(super) fn evidence(raw: &Value) -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: raw.clone(),
        normalized: None,
    }
}
