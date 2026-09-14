use super::protocol::{CodexJsonlProtocol, JsonlTerminalEvidence};
use crate::contracts::{NormalizedRuntimeEventKind, ToolActivityPhase, ToolResultClassification};
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
