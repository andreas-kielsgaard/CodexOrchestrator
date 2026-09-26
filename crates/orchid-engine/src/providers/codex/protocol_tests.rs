use super::protocol::{CodexJsonlProtocol, JsonlTerminalEvidence};
use crate::contracts::{
    NormalizedRuntimeEventKind, ToolActivityKind, ToolActivityPhase, ToolResultClassification,
};
use serde::Deserialize;
const FIRST_TURN: &str = include_str!("fixtures/codex-cli-0.144.0/first-turn.jsonl");
const RESUME: &str = include_str!("fixtures/codex-cli-0.144.0/resume.jsonl");
const MALFORMED: &str = include_str!("fixtures/codex-cli-0.144.0/malformed-and-unknown.jsonl");
const MCP_TOOL_EVENTS: &str = include_str!("fixtures/codex-cli-0.144.0/mcp-tool-events.jsonl");
#[derive(Deserialize)]
struct ChunkFixture {
    chunks: Vec<String>,
}
#[test]
fn normalizes_mcp_started_and_completed_without_requiring_raw_payload_parsing() {
    let mut protocol = CodexJsonlProtocol::default();
    let outputs = protocol.push(MCP_TOOL_EVENTS.as_bytes());
    let activities = outputs
        .into_iter()
        .flat_map(|output| output.events)
        .map(|event| {
            event
                .normalized
                .expect("normalized")
                .tool_activity
                .expect("MCP semantic activity")
        })
        .collect::<Vec<_>>();
    assert_eq!(activities.len(), 2);
    assert!(activities
        .iter()
        .all(|activity| activity.kind == ToolActivityKind::McpTool));
    assert_eq!(activities[0].phase, ToolActivityPhase::Started);
    assert_eq!(activities[0].server.as_deref(), Some("orchestration"));
    assert_eq!(
        activities[0].tool.as_deref(),
        Some("submit_epic_plan_proposal")
    );
    assert_eq!(activities[1].phase, ToolActivityPhase::Completed);
    assert_eq!(
        activities[1].result_classification,
        ToolResultClassification::Succeeded
    );
}

#[test]
fn every_tool_item_carries_a_neutral_kind_and_lifecycle_identity() {
    let mut protocol = CodexJsonlProtocol::default();
    let lines = [
        r#"{"type":"item.started","item":{"id":"cmd-1","type":"command_execution","command":"cargo test","status":"in_progress"}}"#,
        r#"{"type":"item.completed","item":{"id":"cmd-1","type":"command_execution","command":"cargo test","status":"completed"}}"#,
        r#"{"type":"item.completed","item":{"id":"change-1","type":"file_change","status":"failed","changes":[]}}"#,
        r#"{"type":"item.completed","item":{"id":"search-1","type":"web_search","query":"rust"}}"#,
        r#"{"type":"item.completed","item":{"id":"plan-1","type":"plan_update","summary":"steps"}}"#,
    ];
    let activities = lines
        .iter()
        .flat_map(|line| protocol.push(format!("{line}
").as_bytes()))
        .flat_map(|output| output.events)
        .filter_map(|event| event.normalized.and_then(|normalized| normalized.tool_activity))
        .map(|activity| (activity.kind, activity.phase, activity.item_id, activity.tool, activity.result_classification))
        .collect::<Vec<_>>();
    assert_eq!(
        activities,
        [
            (ToolActivityKind::Command, ToolActivityPhase::Started, Some("cmd-1".into()), None, ToolResultClassification::Unknown),
            (ToolActivityKind::Command, ToolActivityPhase::Completed, Some("cmd-1".into()), None, ToolResultClassification::Succeeded),
            (ToolActivityKind::FileChange, ToolActivityPhase::Completed, Some("change-1".into()), None, ToolResultClassification::Failed),
            (ToolActivityKind::WebSearch, ToolActivityPhase::Completed, Some("search-1".into()), None, ToolResultClassification::Unknown),
            (ToolActivityKind::Plan, ToolActivityPhase::Completed, Some("plan-1".into()), None, ToolResultClassification::Unknown),
        ]
    );
}

#[test]
fn exposes_completed_file_changes_without_treating_tool_arguments_as_authorship() {
    let mut protocol = CodexJsonlProtocol::default();
    let mut output = protocol.push(
        br#"{"type":"item.completed","item":{"type":"file_change","status":"completed","changes":[{"path":"docs/spec.md","kind":"add"},{"path":"src/app.ts","kind":"update"}]}}"#,
    );
    output.extend(protocol.finish());
    let changes = output[0].events[0]
        .normalized
        .as_ref()
        .and_then(|event| event.details.as_ref())
        .and_then(|details| details.get("fileChanges"));
    assert_eq!(
        changes,
        Some(&serde_json::json!([
            {"path":"docs/spec.md","operation":"create"},
            {"path":"src/app.ts","operation":"edit"}
        ]))
    );
}
#[test]
fn frames_jsonl_across_arbitrary_byte_chunks_and_marks_final_output() {
    let fixture: ChunkFixture = serde_json::from_str(include_str!(
        "fixtures/codex-cli-0.144.0/chunk-boundaries.json"
    ))
    .expect("chunk fixture");
    let mut protocol = CodexJsonlProtocol::default();
    let mut outputs = Vec::new();
    for chunk in fixture.chunks {
        outputs.extend(protocol.push(chunk.as_bytes()));
    }
    outputs.extend(protocol.finish());
    let events = outputs
        .iter()
        .flat_map(|output| output.events.iter())
        .collect::<Vec<_>>();
    assert!(events
        .iter()
        .any(|event| event.normalized.as_ref().is_some_and(
            |event| event.kind == NormalizedRuntimeEventKind::RuntimeContextEstablished
        )));
    let final_message = events
        .iter()
        .find(|event| {
            event
                .normalized
                .as_ref()
                .is_some_and(|event| event.kind == NormalizedRuntimeEventKind::AgentMessage)
        })
        .expect("agent message");
    assert_eq!(
        final_message
            .normalized
            .as_ref()
            .and_then(|event| event.details.as_ref())
            .and_then(|details| details.get("role"))
            .and_then(|role| role.as_str()),
        Some("final")
    );
    assert_eq!(
        outputs.iter().filter_map(|output| output.terminal).last(),
        Some(JsonlTerminalEvidence::Completed)
    );
}
#[test]
fn malformed_and_unknown_lines_are_preserved_without_losing_valid_neighbors() {
    let mut protocol = CodexJsonlProtocol::default();
    let mut outputs = protocol.push(MALFORMED.as_bytes());
    outputs.extend(protocol.finish());
    let events = outputs
        .iter()
        .flat_map(|output| output.events.iter())
        .collect::<Vec<_>>();
    assert!(events
        .iter()
        .any(|event| event.raw_payload.get("diagnostic").is_some()));
    assert!(events.iter().any(|event| event
        .raw_payload
        .get("type")
        .and_then(|value| value.as_str())
        == Some("future.event")));
    assert!(events.iter().any(|event| event
        .normalized
        .as_ref()
        .is_some_and(|event| event.kind == NormalizedRuntimeEventKind::AgentMessage)));
    assert_eq!(
        outputs.iter().filter_map(|output| output.terminal).last(),
        Some(JsonlTerminalEvidence::Completed)
    );
}
#[test]
fn recorded_first_and_resume_fixtures_keep_the_same_external_thread_binding() {
    fn context_id(fixture: &str) -> String {
        let mut protocol = CodexJsonlProtocol::default();
        let mut outputs = protocol.push(fixture.as_bytes());
        outputs.extend(protocol.finish());
        outputs
            .into_iter()
            .flat_map(|output| output.events)
            .find_map(|event| {
                event
                    .normalized
                    .and_then(|event| event.external_context_id)
                    .map(|id| id.as_str().to_string())
            })
            .expect("thread binding")
    }
    assert_eq!(context_id(FIRST_TURN), "019f-fixture-first");
    assert_eq!(context_id(RESUME), "019f-fixture-first");
}
