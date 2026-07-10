use super::{
    arguments::{build_args, InvocationCommand},
    capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe},
    protocol::{CodexJsonlProtocol, JsonlTerminalEvidence},
    runtime::reconcile_terminal,
    CodexCliRuntime,
};
use crate::{
    agent_sessions::{
        domain::{
            AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeOptions, AgentSessionId,
            ExternalRuntimeContextId, NormalizedRuntimeEventKind, RuntimeSandboxMode,
        },
        ports::{
            AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationRequest, RuntimePortError,
            RuntimeUpdate,
        },
    },
    runtime::processes::{
        ChildProcessFactory, ProcessExit, ProcessFailureKind, ProcessLaunchSpec,
        ProcessTerminalOutcome, SpawnedProcess, SupervisedChild,
    },
};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    io::{self, Cursor},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

const FIRST_TURN: &str = include_str!("fixtures/codex-cli-0.144.0/first-turn.jsonl");
const RESUME: &str = include_str!("fixtures/codex-cli-0.144.0/resume.jsonl");
const MALFORMED: &str = include_str!("fixtures/codex-cli-0.144.0/malformed-and-unknown.jsonl");

fn capabilities() -> CodexCliCapabilities {
    CodexCliCapabilities {
        version: Some("codex-cli 0.144.0".to_string()),
        exec_json: Some(true),
        resume_json: Some(true),
        exec_model: Some(true),
        resume_model: Some(true),
        exec_sandbox: Some(true),
        resume_sandbox: Some(false),
    }
}

#[test]
fn builds_supported_first_turn_and_resume_commands() {
    let options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    assert_eq!(
        build_args(
            InvocationCommand::Start,
            "hello",
            &options,
            Some(&capabilities())
        )
        .expect("start args"),
        [
            "exec",
            "--json",
            "--model",
            "gpt-5",
            "--sandbox",
            "workspace-write",
            "hello"
        ]
    );
    let context = ExternalRuntimeContextId::new("thread-external").expect("context");
    let resume_options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: None,
    };
    assert_eq!(
        build_args(
            InvocationCommand::Resume(&context),
            "continue",
            &resume_options,
            Some(&capabilities())
        )
        .expect("resume args"),
        [
            "exec",
            "resume",
            "--json",
            "--model",
            "gpt-5",
            "thread-external",
            "continue"
        ]
    );
}

#[test]
fn omits_optional_options_when_capability_data_is_absent() {
    let options = AgentRuntimeOptions {
        model: Some("unverified-model".to_string()),
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    assert_eq!(
        build_args(InvocationCommand::Start, "hello", &options, None).expect("defaults"),
        ["exec", "--json", "hello"]
    );
}

#[test]
fn rejects_a_confirmed_unsupported_resume_sandbox() {
    let context = ExternalRuntimeContextId::new("thread-external").expect("context");
    let options = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let error = build_args(
        InvocationCommand::Resume(&context),
        "continue",
        &options,
        Some(&capabilities()),
    )
    .expect_err("unsupported sandbox");
    assert!(error.message.contains("sandbox"));
}

#[derive(Deserialize)]
struct ChunkFixture {
    chunks: Vec<String>,
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

#[derive(Default)]
struct ImmediateChild;

impl SupervisedChild for ImmediateChild {
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        Ok(Some(ProcessExit {
            exit_code: Some(0),
            signal: None,
        }))
    }
    fn terminate(&self) -> io::Result<()> {
        Ok(())
    }
    fn wait_after_termination(&self) -> io::Result<ProcessExit> {
        Ok(ProcessExit {
            exit_code: Some(0),
            signal: None,
        })
    }
}

#[derive(Default)]
struct FixtureFactory {
    stdout: Mutex<VecDeque<Vec<u8>>>,
    specs: Mutex<Vec<ProcessLaunchSpec>>,
}

impl ChildProcessFactory for FixtureFactory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        self.specs.lock().expect("specs").push(spec.clone());
        let stdout = self
            .stdout
            .lock()
            .expect("stdout")
            .pop_front()
            .expect("fixture configured");
        Ok(SpawnedProcess {
            child: Arc::new(ImmediateChild),
            stdout: Box::new(Cursor::new(stdout)),
            stderr: Box::new(Cursor::new(Vec::<u8>::new())),
        })
    }
}

#[derive(Default)]
struct CollectingSink {
    updates: Mutex<Vec<RuntimeUpdate>>,
    changed: Condvar,
}

impl AgentRuntimeUpdateSink for CollectingSink {
    fn emit_update(
        &self,
        _invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        self.updates.lock().expect("updates").push(update);
        self.changed.notify_all();
        Ok(())
    }
}

impl CollectingSink {
    fn wait_finished(&self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut updates = self.updates.lock().expect("updates");
        while !updates
            .iter()
            .any(|update| matches!(update, RuntimeUpdate::Finished(_)))
        {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .expect("timed out");
            let (next, timeout) = self.changed.wait_timeout(updates, remaining).expect("wait");
            assert!(!timeout.timed_out(), "timed out waiting for runtime finish");
            updates = next;
        }
    }
}

#[test]
fn runtime_uses_supervisor_for_first_turn_and_resume_with_explicit_working_directory() {
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .extend([FIRST_TURN.as_bytes().to_vec(), RESUME.as_bytes().to_vec()]);
    let runtime = CodexCliRuntime::new("codex", Some(capabilities()), factory.clone());
    let first_sink = Arc::new(CollectingSink::default());
    runtime
        .start_invocation(request("inv-1", "first"), first_sink.clone())
        .expect("start");
    first_sink.wait_finished();
    let resume_sink = Arc::new(CollectingSink::default());
    runtime
        .resume_invocation(
            request("inv-2", "again"),
            ExternalRuntimeContextId::new("019f-fixture-first").expect("context"),
            resume_sink.clone(),
        )
        .expect("resume");
    resume_sink.wait_finished();

    let specs = factory.specs.lock().expect("specs");
    assert_eq!(
        specs[0]
            .working_directory
            .as_ref()
            .and_then(|path| path.to_str()),
        Some("C:/work/project")
    );
    assert_eq!(specs[0].args, ["exec", "--json", "first"]);
    assert_eq!(
        specs[1].args,
        ["exec", "resume", "--json", "019f-fixture-first", "again"]
    );
    assert!(first_sink.updates.lock().expect("updates").iter().any(|update| matches!(update, RuntimeUpdate::Finished(outcome) if outcome.status == AgentInvocationTerminalStatus::Completed)));
}

#[test]
fn terminal_reconciliation_requires_both_protocol_and_process_success() {
    let clean_exit = || {
        ProcessTerminalOutcome::Exited(ProcessExit {
            exit_code: Some(0),
            signal: None,
        })
    };
    assert_eq!(
        reconcile_terminal(Some(JsonlTerminalEvidence::Completed), clean_exit()).status,
        AgentInvocationTerminalStatus::Completed
    );
    assert_eq!(
        reconcile_terminal(None, clean_exit()).status,
        AgentInvocationTerminalStatus::Failed
    );
    assert_eq!(
        reconcile_terminal(Some(JsonlTerminalEvidence::Failed), clean_exit()).status,
        AgentInvocationTerminalStatus::Failed
    );
    assert_eq!(
        reconcile_terminal(
            Some(JsonlTerminalEvidence::Completed),
            ProcessTerminalOutcome::Failed {
                kind: ProcessFailureKind::NonZeroExit,
                exit: Some(ProcessExit {
                    exit_code: Some(9),
                    signal: None
                }),
                message: "process exited with code 9".to_string(),
            },
        )
        .status,
        AgentInvocationTerminalStatus::Failed
    );
}

fn request(invocation: &str, prompt: &str) -> RuntimeInvocationRequest {
    RuntimeInvocationRequest {
        session_id: AgentSessionId::new("local-session-id").expect("session"),
        invocation_id: AgentInvocationId::new(invocation).expect("invocation"),
        submitted_text: prompt.to_string(),
        working_directory: Some("C:/work/project".to_string()),
        options: AgentRuntimeOptions::default(),
    }
}

#[test]
#[ignore = "non-agent compatibility probe; requires installed codex CLI"]
fn installed_codex_help_is_compatible_without_running_an_agent_prompt() {
    let discovered = CodexCliCapabilityProbe::new("codex").discover();
    assert!(
        discovered
            .version
            .as_deref()
            .is_some_and(|version| version.starts_with("codex-cli ")),
        "discovered capabilities: {discovered:?}"
    );
    assert_eq!(discovered.exec_json, Some(true));
    assert_eq!(discovered.resume_json, Some(true));
    assert_eq!(discovered.exec_model, Some(true));
    assert_eq!(discovered.resume_model, Some(true));
    assert_eq!(discovered.exec_sandbox, Some(true));
    assert_eq!(discovered.resume_sandbox, Some(false));
}
