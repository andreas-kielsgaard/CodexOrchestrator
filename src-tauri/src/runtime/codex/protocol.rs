use crate::agent_sessions::{
    domain::{
        AgentRuntimeEventSource, AgentRuntimeUsage, ExternalRuntimeContextId,
        NormalizedRuntimeEvent, NormalizedRuntimeEventKind, NormalizedToolActivity,
        ToolActivityPhase, ToolResultClassification,
    },
    ports::RuntimeEventDraft,
};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum JsonlTerminalEvidence {
    Completed,
    Failed,
    Error,
}

#[derive(Debug)]
pub(super) struct ProtocolOutput {
    pub(super) events: Vec<RuntimeEventDraft>,
    pub(super) terminal: Option<JsonlTerminalEvidence>,
}

#[derive(Default)]
pub(super) struct CodexJsonlProtocol {
    buffer: Vec<u8>,
    line_number: u64,
    pending_agent_message: Option<(Value, String)>,
}

impl CodexJsonlProtocol {
    pub(super) fn push(&mut self, bytes: &[u8]) -> Vec<ProtocolOutput> {
        self.buffer.extend_from_slice(bytes);
        let mut outputs = Vec::new();
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.buffer.drain(..=newline).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            outputs.push(self.parse_line(line));
        }
        outputs
    }

    pub(super) fn finish(&mut self) -> Vec<ProtocolOutput> {
        let mut outputs = Vec::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            outputs.push(self.parse_line(line));
        }
        if let Some(event) = self.take_agent_message("intermediate") {
            outputs.push(ProtocolOutput {
                events: vec![event],
                terminal: None,
            });
        }
        outputs
    }

    fn parse_line(&mut self, bytes: Vec<u8>) -> ProtocolOutput {
        self.line_number += 1;
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return ProtocolOutput {
                events: Vec::new(),
                terminal: None,
            };
        }
        let line = String::from_utf8_lossy(&bytes).into_owned();
        let raw = match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Object(raw)) => Value::Object(raw),
            Ok(value) => {
                return malformed(
                    line,
                    self.line_number,
                    "event line must be a JSON object",
                    Some(value),
                )
            }
            Err(error) => {
                return malformed(
                    line,
                    self.line_number,
                    &format!("invalid JSON: {error}"),
                    None,
                )
            }
        };
        self.normalize(raw)
    }

    fn normalize(&mut self, raw: Value) -> ProtocolOutput {
        let event_type = raw.get("type").and_then(Value::as_str).map(str::to_string);
        let Some(event_type) = event_type else {
            return malformed(
                raw.to_string(),
                self.line_number,
                "event type must be a string",
                Some(raw),
            );
        };
        let mut events = Vec::new();
        let mut terminal = None;
        match event_type.as_str() {
            "thread.started" => match raw.get("thread_id").and_then(Value::as_str) {
                Some(id) if !id.trim().is_empty() => match ExternalRuntimeContextId::new(id) {
                    Ok(id) => events.push(draft(
                        raw,
                        normalized(
                            NormalizedRuntimeEventKind::RuntimeContextEstablished,
                            None,
                            Some(id),
                            None,
                            None,
                        ),
                    )),
                    Err(error) => {
                        return malformed(
                            raw.to_string(),
                            self.line_number,
                            &error.to_string(),
                            Some(raw),
                        )
                    }
                },
                _ => {
                    return malformed(
                        raw.to_string(),
                        self.line_number,
                        "thread.started thread_id must be a non-empty string",
                        Some(raw),
                    )
                }
            },
            "turn.started" => events.push(draft(
                raw,
                normalized(
                    NormalizedRuntimeEventKind::ProcessingStarted,
                    None,
                    None,
                    None,
                    None,
                ),
            )),
            "turn.completed" => {
                if let Some(message) = self.take_agent_message("final") {
                    events.push(message);
                }
                if let Some(usage_value) = raw.get("usage") {
                    if let Some(usage_object) = usage_value.as_object() {
                        events.push(draft(
                            raw.clone(),
                            normalized(
                                NormalizedRuntimeEventKind::Usage,
                                None,
                                None,
                                Some(parse_usage(usage_object)),
                                Some(usage_value.clone()),
                            ),
                        ));
                    } else {
                        events.push(diagnostic(
                            raw.clone(),
                            self.line_number,
                            "turn.completed usage must be a JSON object",
                        ));
                    }
                }
                events.push(draft(
                    raw,
                    normalized(
                        NormalizedRuntimeEventKind::InvocationCompleted,
                        None,
                        None,
                        None,
                        None,
                    ),
                ));
                terminal = Some(JsonlTerminalEvidence::Completed);
            }
            "turn.failed" => {
                if let Some(message) = self.take_agent_message("intermediate") {
                    events.push(message);
                }
                events.push(runtime_error(raw, "Codex reported turn.failed", "failed"));
                terminal = Some(JsonlTerminalEvidence::Failed);
            }
            "error" => {
                events.push(runtime_error(raw, "Codex reported an error event", "error"));
                terminal = Some(JsonlTerminalEvidence::Error);
            }
            event if event.starts_with("item.") => {
                let Some(item) = raw.get("item").and_then(Value::as_object) else {
                    return malformed(
                        raw.to_string(),
                        self.line_number,
                        &format!("{event} item must be a JSON object"),
                        Some(raw),
                    );
                };
                let Some(item_type) = item.get("type").and_then(Value::as_str).map(str::to_string)
                else {
                    return malformed(
                        raw.to_string(),
                        self.line_number,
                        "item type must be a string",
                        Some(raw),
                    );
                };
                match item_type.as_str() {
                    "agent_message" => {
                        let Some(text) =
                            item.get("text").and_then(Value::as_str).map(str::to_string)
                        else {
                            return malformed(
                                raw.to_string(),
                                self.line_number,
                                "agent_message text must be a string",
                                Some(raw),
                            );
                        };
                        if event == "item.completed" {
                            if let Some(previous) = self.take_agent_message("intermediate") {
                                events.push(previous);
                            }
                            self.pending_agent_message = Some((raw, text));
                        } else {
                            events.push(agent_message(raw, &text, "intermediate"));
                        }
                    }
                    "reasoning" => {
                        let text = item_text(item);
                        let details = json!({"itemType": item_type, "eventType": event});
                        events.push(draft(
                            raw,
                            normalized(
                                NormalizedRuntimeEventKind::ProcessingUpdate,
                                text,
                                None,
                                None,
                                Some(details),
                            ),
                        ));
                    }
                    "mcp_tool_call" => {
                        let text = item_text(item);
                        let details = json!({"itemType": item_type, "eventType": event});
                        let activity = mcp_tool_activity(item, event);
                        events.push(draft(
                            raw,
                            normalized_with_tool(
                                NormalizedRuntimeEventKind::ToolActivity,
                                text,
                                None,
                                None,
                                Some(details),
                                activity,
                            ),
                        ));
                    }
                    "command_execution" | "file_change" | "web_search" | "plan_update" => {
                        let text = item_text(item);
                        let details = json!({"itemType": item_type, "eventType": event});
                        events.push(draft(
                            raw,
                            normalized(
                                NormalizedRuntimeEventKind::ToolActivity,
                                text,
                                None,
                                None,
                                Some(details),
                            ),
                        ));
                    }
                    _ => events.push(draft(
                        raw,
                        normalized(
                            NormalizedRuntimeEventKind::Unknown,
                            None,
                            None,
                            None,
                            Some(json!({"unknownItemType": item_type, "eventType": event})),
                        ),
                    )),
                }
            }
            _ => events.push(draft(
                raw,
                normalized(
                    NormalizedRuntimeEventKind::Unknown,
                    None,
                    None,
                    None,
                    Some(json!({"unknownEventType": event_type})),
                ),
            )),
        }
        ProtocolOutput { events, terminal }
    }

    fn take_agent_message(&mut self, role: &str) -> Option<RuntimeEventDraft> {
        self.pending_agent_message
            .take()
            .map(|(raw, text)| agent_message(raw, &text, role))
    }
}

fn malformed(
    line: String,
    line_number: u64,
    message: &str,
    parsed: Option<Value>,
) -> ProtocolOutput {
    let raw = json!({"raw": line, "parsed": parsed, "diagnostic": {"code": "codex_jsonl_malformed", "line": line_number, "message": message}});
    ProtocolOutput {
        events: vec![draft(
            raw,
            normalized(
                NormalizedRuntimeEventKind::Unknown,
                None,
                None,
                None,
                Some(json!({"diagnostic": message, "line": line_number})),
            ),
        )],
        terminal: None,
    }
}

fn diagnostic(raw: Value, line: u64, message: &str) -> RuntimeEventDraft {
    draft(
        raw,
        normalized(
            NormalizedRuntimeEventKind::Unknown,
            None,
            None,
            None,
            Some(json!({"diagnostic": message, "line": line})),
        ),
    )
}

fn runtime_error(raw: Value, fallback: &str, terminal: &str) -> RuntimeEventDraft {
    let text = raw
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string();
    draft(
        raw,
        normalized(
            NormalizedRuntimeEventKind::RuntimeError,
            Some(text),
            None,
            None,
            Some(json!({"providerTerminal": terminal})),
        ),
    )
}

fn agent_message(raw: Value, text: &str, role: &str) -> RuntimeEventDraft {
    draft(
        raw,
        normalized(
            NormalizedRuntimeEventKind::AgentMessage,
            Some(text.to_string()),
            None,
            None,
            Some(json!({"role": role})),
        ),
    )
}

fn draft(raw_payload: Value, normalized: NormalizedRuntimeEvent) -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload,
        normalized: Some(normalized),
    }
}

fn normalized(
    kind: NormalizedRuntimeEventKind,
    text: Option<String>,
    external_context_id: Option<ExternalRuntimeContextId>,
    usage: Option<AgentRuntimeUsage>,
    details: Option<Value>,
) -> NormalizedRuntimeEvent {
    NormalizedRuntimeEvent {
        kind,
        text,
        external_context_id,
        usage,
        details,
        tool_activity: None,
    }
}

fn normalized_with_tool(
    kind: NormalizedRuntimeEventKind,
    text: Option<String>,
    external_context_id: Option<ExternalRuntimeContextId>,
    usage: Option<AgentRuntimeUsage>,
    details: Option<Value>,
    tool_activity: NormalizedToolActivity,
) -> NormalizedRuntimeEvent {
    NormalizedRuntimeEvent {
        kind,
        text,
        external_context_id,
        usage,
        details,
        tool_activity: Some(tool_activity),
    }
}

fn mcp_tool_activity(item: &Map<String, Value>, event: &str) -> NormalizedToolActivity {
    let status = text_at(item, &["status", "result", "outcome"]).or_else(|| {
        item.get("result")
            .and_then(Value::as_object)
            .and_then(|result| text_at(result, &["status", "outcome"]))
    });
    let phase = match event {
        "item.started" => ToolActivityPhase::Started,
        "item.completed" => ToolActivityPhase::Completed,
        _ => ToolActivityPhase::Unknown,
    };
    let result_classification = match status.as_deref() {
        Some("completed" | "success" | "succeeded" | "persisted") => {
            ToolResultClassification::Succeeded
        }
        Some("failed" | "error" | "errored") => ToolResultClassification::Failed,
        _ => ToolResultClassification::Unknown,
    };
    NormalizedToolActivity {
        phase,
        item_id: text_at(item, &["id"]),
        server: text_at(item, &["server", "server_name", "serverName"]),
        tool: text_at(item, &["tool", "tool_name", "toolName", "name"]),
        status,
        result_classification,
    }
}

fn text_at(item: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| item.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn item_text(item: &Map<String, Value>) -> Option<String> {
    ["text", "command", "query", "summary"]
        .iter()
        .find_map(|key| item.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn parse_usage(usage: &Map<String, Value>) -> AgentRuntimeUsage {
    AgentRuntimeUsage {
        input_tokens: token(usage, &["input_tokens", "inputTokens"]),
        cached_input_tokens: token(usage, &["cached_input_tokens", "cachedInputTokens"]),
        output_tokens: token(usage, &["output_tokens", "outputTokens"]),
    }
}

fn token(usage: &Map<String, Value>, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| usage.get(*key).and_then(Value::as_u64))
}
