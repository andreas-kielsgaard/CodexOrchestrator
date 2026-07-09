use super::*;

pub(crate) fn update_conversation_from_runtime_result(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    runtime_result: &CodexRuntimeResult,
) -> Result<(), String> {
    conn.execute(
        "
UPDATE conversations
SET external_thread_id = COALESCE(?1, external_thread_id), summary = ?2, updated_at = ?3
WHERE id = ?4
",
        params![
            runtime_result.summary.thread_id.as_deref(),
            summarize_conversation(runtime_result),
            now_iso(),
            started.conversation_id
        ],
    )
    .map_err(sql_error("update Codex conversation"))?;

    Ok(())
}

pub(crate) fn summarize_conversation(runtime_result: &CodexRuntimeResult) -> String {
    let prefix = if runtime_result.status == CodexRuntimeStatus::Completed {
        "Codex completed".to_string()
    } else {
        format!(
            "Codex {}: {}",
            runtime_result.status.as_str(),
            runtime_result.status_reason
        )
    };

    match runtime_result.summary.final_agent_message_text.as_deref() {
        Some(final_message) if !final_message.trim().is_empty() => {
            truncate(&format!("{prefix}: {}", final_message.trim()), 240)
        }
        _ => prefix,
    }
}

pub(crate) fn codex_failure_reason(runtime_result: &CodexRuntimeResult) -> String {
    let stderr = runtime_result.stderr.trim();

    if stderr.is_empty() {
        return runtime_result.status_reason.clone();
    }

    truncate(&format!("{}: {stderr}", runtime_result.status_reason), 500)
}

pub(crate) fn classify_codex_exec_result(
    process_result: &CodexCommandRunResult,
    summary: &CodexJsonlSummary,
) -> (CodexRuntimeStatus, String) {
    match summary.terminal_status {
        Some(CodexJsonlTerminalStatus::Error { .. }) => {
            return (
                CodexRuntimeStatus::Error,
                "Codex emitted an error event".to_string(),
            );
        }
        Some(CodexJsonlTerminalStatus::Failed { .. }) => {
            return (
                CodexRuntimeStatus::Failed,
                "Codex emitted a turn.failed event".to_string(),
            );
        }
        _ => {}
    }

    if let Some(signal) = &process_result.signal {
        return (
            CodexRuntimeStatus::Failed,
            format!("Codex process exited on signal {signal}"),
        );
    }

    if process_result.exit_code != Some(0) {
        return (
            CodexRuntimeStatus::Failed,
            match process_result.exit_code {
                Some(exit_code) => format!("Codex process exited with code {exit_code}"),
                None => "Codex process exited without an exit code".to_string(),
            },
        );
    }

    if matches!(
        summary.terminal_status,
        Some(CodexJsonlTerminalStatus::Completed { .. })
    ) {
        return (
            CodexRuntimeStatus::Completed,
            "Codex emitted a turn.completed event".to_string(),
        );
    }

    (
        CodexRuntimeStatus::Failed,
        "Codex output did not include a terminal event".to_string(),
    )
}

pub(crate) fn parse_codex_jsonl_summary(jsonl: &str) -> Result<CodexJsonlSummary, String> {
    let mut summary = CodexJsonlSummary::default();
    let normalized_jsonl = jsonl.replace("\r\n", "\n").replace('\r', "\n");

    for (index, line) in normalized_jsonl.split('\n').enumerate() {
        let line_number = index + 1;

        if line.trim().is_empty() {
            continue;
        }

        let parsed = serde_json::from_str::<Value>(line)
            .map_err(|error| format!("Line {line_number}: Invalid JSON: {error}"))?;
        let object = parsed
            .as_object()
            .ok_or_else(|| format!("Line {line_number}: Event line must be a JSON object"))?;
        let event_type = object
            .get("type")
            .ok_or_else(|| format!("Line {line_number}: Event type is required"))?
            .as_str()
            .ok_or_else(|| format!("Line {line_number}: Event type must be a string"))?;

        match event_type {
            "thread.started" => {
                let thread_id = object
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|thread_id| !thread_id.is_empty())
                    .ok_or_else(|| {
                        format!("Line {line_number}: thread.started thread_id must be a string")
                    })?;
                summary.thread_id = Some(thread_id.to_string());
            }
            "turn.completed" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Completed { line_number });
                if let Some(usage) = object.get("usage") {
                    summary.token_usage = Some(usage.as_object().cloned().ok_or_else(|| {
                        format!("Line {line_number}: turn.completed usage must be a JSON object")
                    })?);
                }
            }
            "turn.failed" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Failed { line_number });
            }
            "error" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Error { line_number });
            }
            _ if event_type.starts_with("item.") => {
                let item = object
                    .get("item")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        format!("Line {line_number}: {event_type} item must be a JSON object")
                    })?;
                let item_type = item
                    .get("type")
                    .ok_or_else(|| format!("Line {line_number}: Item type is required"))?
                    .as_str()
                    .ok_or_else(|| format!("Line {line_number}: Item type must be a string"))?;
                *summary
                    .item_counts_by_type
                    .entry(item_type.to_string())
                    .or_insert(0) += 1;

                if event_type == "item.completed" && item_type == "agent_message" {
                    if let Some(text) = item.get("text") {
                        summary.final_agent_message_text = Some(
                            text.as_str()
                                .ok_or_else(|| {
                                    format!(
                                        "Line {line_number}: agent_message text must be a string"
                                    )
                                })?
                                .to_string(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    Ok(summary)
}
