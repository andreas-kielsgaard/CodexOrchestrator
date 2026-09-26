//! Stored Codex history and native forks; never starts a model turn.
use orchid_engine::providers::codex::app_server::{history as native_history, items};
use crate::agent_sessions::{domain::*, imports::*, ports::CodexHistorySource};
use orchid_engine::providers::codex::protocol::CodexJsonlProtocol;
use serde_json::{json, Value};
use std::path::PathBuf;

pub(crate) struct CodexHistoryReader(pub String);
impl CodexHistorySource for CodexHistoryReader {
    fn read(&self, home: &ImportHome, id: &str) -> Result<ImportedThread, String> {
        let result = native_history::read_thread(&self.0, PathBuf::from(&home.path), id)
            .map_err(|e| e.to_string())?;
        decode(&result["thread"])
    }
    fn fork(
        &self,
        home: &ImportHome,
        id: &str,
        last: &str,
        cwd: &str,
    ) -> Result<ImportedThread, String> {
        let result = native_history::fork_thread(&self.0, PathBuf::from(&home.path), id, last, cwd)
            .map_err(|e| e.to_string())?;
        decode(&result["thread"])
    }
}

pub(crate) fn decode(thread: &Value) -> Result<ImportedThread, String> {
    let id = thread["id"]
        .as_str()
        .ok_or("Codex did not return a thread ID")?;
    let turns = thread["turns"]
        .as_array()
        .ok_or("Codex did not return full history")?;
    let mut decoded = Vec::new();
    for turn in turns {
        // Fork through the last settled boundary; never import an unfinished turn.
        if turn["status"] == "inProgress" {
            break;
        }
        let status = match turn["status"].as_str() {
            Some("completed") => AgentInvocationStatus::Completed,
            Some("failed") => AgentInvocationStatus::Failed,
            Some("interrupted") => AgentInvocationStatus::Interrupted,
            _ => return Err("Codex returned an unsupported historical turn status".into()),
        };
        if turn["itemsView"].as_str().is_some_and(|v| v != "full") {
            return Err("This Codex version did not provide full turn items".into());
        }
        let native_items = turn["items"]
            .as_array()
            .ok_or("Codex returned no turn items")?;
        let mut result = Vec::new();
        for native in native_items {
            let kind = native["type"].as_str().unwrap_or("unknown");
            let (display_kind, text) = match kind {
                "userMessage" => ("user", user_text(native)),
                "agentMessage" => (
                    "assistant",
                    native["text"].as_str().unwrap_or("").to_owned(),
                ),
                _ => ("activity", activity_text(native)),
            };
            let normalized = items::convert(native).and_then(|item| {
                let mut protocol = CodexJsonlProtocol::default();
                let mut events = protocol
                    .normalize(json!({"type":"item.completed","item":item}))
                    .events;
                events.extend(protocol.normalize(json!({"type":"turn.completed"})).events);
                events.into_iter().find_map(|e| {
                    e.normalized.filter(|n| {
                        !matches!(
                            n.kind,
                            NormalizedRuntimeEventKind::InvocationCompleted
                                | NormalizedRuntimeEventKind::RuntimeError
                        )
                    })
                })
            });
            result.push(ImportedItem {
                kind: display_kind.into(),
                text,
                raw: native.clone(),
                normalized,
            });
        }
        decoded.push(ImportedTurn {
            id: turn["id"]
                .as_str()
                .ok_or("Codex returned a turn without identity")?
                .into(),
            status,
            started_at: turn["startedAt"].as_i64(),
            completed_at: turn["completedAt"].as_i64(),
            items: result,
        });
    }
    Ok(ImportedThread {
        id: id.into(),
        title: thread["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| thread["preview"].as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Imported Codex conversation")
            .chars()
            .take(160)
            .collect(),
        cwd: thread["cwd"].as_str().map(str::to_owned),
        version: thread["cliVersion"].as_str().map(str::to_owned),
        turns: decoded,
    })
}
fn user_text(item: &Value) -> String {
    item["content"]
        .as_array()
        .map(|content| {
            content
                .iter()
                .map(|part| {
                    if part["type"] == "text" {
                        part["text"].as_str().unwrap_or("").to_owned()
                    } else {
                        format!(
                            "[{} attachment retained in source history]",
                            part["type"].as_str().unwrap_or("Unsupported")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "[Unavailable user content]".into())
}
fn activity_text(item: &Value) -> String {
    match item["type"].as_str() {
        Some("commandExecution") => format!(
            "Command: {}\n{}",
            item["command"].as_str().unwrap_or(""),
            item["aggregatedOutput"].as_str().unwrap_or("")
        ),
        Some("mcpToolCall") => format!(
            "Tool: {} / {}",
            item["server"].as_str().unwrap_or(""),
            item["tool"].as_str().unwrap_or("")
        ),
        Some("reasoning") => item["summary"]
            .as_array()
            .map(|s| {
                s.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or("Reasoning summary unavailable".into()),
        Some("fileChange") => format!("File changes: {}", item["changes"]),
        Some("webSearch") => format!("Web search: {}", item["query"].as_str().unwrap_or("")),
        kind => format!(
            "{} — details retained in imported history",
            kind.unwrap_or("Unsupported item")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_preserves_steering_items_order_and_unknown_content() {
        let thread = json!({"id":"source","cwd":"C:/gone","turns":[
            {"id":"settled","status":"completed","items":[
                {"type":"userMessage","content":[{"type":"text","text":"First"},{"type":"image","url":"unavailable"}]},
                {"type":"agentMessage","text":"Working"},
                {"type":"userMessage","content":[{"type":"text","text":"Steering"}]},
                {"type":"futureItem","payload":"retain"},
                {"type":"agentMessage","text":"Done"}
            ]},
            {"id":"active","status":"inProgress","items":[]}
        ]});
        let decoded = decode(&thread).unwrap();
        assert_eq!(decoded.turns.len(), 1);
        assert_eq!(decoded.turns[0].items.len(), 5);
        assert!(decoded.turns[0].items[0].text.contains("image attachment"));
        assert_eq!(decoded.turns[0].items[2].text, "Steering");
        assert_eq!(decoded.turns[0].items[3].raw["payload"], "retain");
        assert_eq!(decoded.turns[0].started_at, None);
        assert_eq!(
            decoded.turns[0].items[4].normalized.as_ref().unwrap().kind,
            NormalizedRuntimeEventKind::AgentMessage
        );
    }
    #[test]
    fn summary_only_history_is_rejected() {
        assert!(decode(&json!({"id":"x","turns":[{"id":"t","status":"completed","itemsView":"summary","items":[]}]})).is_err());
    }
}
