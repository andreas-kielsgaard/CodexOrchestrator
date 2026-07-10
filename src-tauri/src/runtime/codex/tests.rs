use super::{
    arguments::{build_args, InvocationCommand},
    capabilities::{resolve_program, CodexCliCapabilities, CodexCliCapabilityProbe},
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
            RuntimePortErrorKind, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
            RuntimeUpdateDeliveryKind,
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

#[cfg(windows)]
#[test]
fn explicit_codex_cmd_path_resolves_to_native_npm_binary() {
    let npm_bin = std::env::temp_dir().join(format!("codex-resolution-{}", uuid::Uuid::new_v4()));
    let cmd = npm_bin.join("codex.cmd");
    #[cfg(target_arch = "x86_64")]
    let native = npm_bin.join(
        "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
    );
    #[cfg(target_arch = "aarch64")]
    let native = npm_bin.join(
        "node_modules/@openai/codex/node_modules/@openai/codex-win32-arm64/vendor/aarch64-pc-windows-msvc/bin/codex.exe",
    );
    std::fs::create_dir_all(native.parent().expect("native parent")).expect("fixture directories");
    std::fs::write(&cmd, "@echo off").expect("cmd fixture");
    std::fs::write(&native, []).expect("native fixture");

    assert_eq!(
        std::path::PathBuf::from(
            resolve_program(cmd.to_string_lossy().into_owned()).expect("native resolution"),
        ),
        native
    );

    std::fs::remove_dir_all(npm_bin).expect("remove fixture");
}

#[cfg(windows)]
#[test]
fn explicit_cmd_without_native_binary_is_rejected_before_process_launch() {
    let npm_bin = std::env::temp_dir().join(format!("codex-resolution-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&npm_bin).expect("fixture directory");
    let cmd = npm_bin.join("codex.cmd");
    std::fs::write(&cmd, "@echo off").expect("cmd fixture");

    let error = resolve_program(cmd.to_string_lossy().into_owned()).expect_err("batch rejection");
    assert!(error.contains("no discoverable native npm binary"));

    std::fs::remove_dir_all(npm_bin).expect("remove fixture");
}

#[test]
fn builds_supported_first_turn_and_resume_commands() {
    let options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    let start = build_args(
        InvocationCommand::Start,
        "hello",
        &options,
        Some(&capabilities()),
    )
    .expect("start args");
    assert_eq!(
        start.args,
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
    assert_eq!(start.effective_options, options);
    let context = ExternalRuntimeContextId::new("thread-external").expect("context");
    let resume_options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: None,
    };
    let resume = build_args(
        InvocationCommand::Resume(&context),
        "continue",
        &resume_options,
        Some(&capabilities()),
    )
    .expect("resume args");
    assert_eq!(
        resume.args,
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
    assert_eq!(resume.effective_options, resume_options);
}

#[test]
fn omits_optional_options_when_capability_data_is_absent() {
    let options = AgentRuntimeOptions {
        model: Some("unverified-model".to_string()),
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let prepared = build_args(InvocationCommand::Start, "hello", &options, None).expect("defaults");
    assert_eq!(prepared.args, ["exec", "--json", "hello"]);
    assert_eq!(prepared.effective_options, AgentRuntimeOptions::default());
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
    delivery_failures: Mutex<Vec<RuntimeUpdateDeliveryFailure>>,
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

    fn report_delivery_failure(
        &self,
        _invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.delivery_failures
            .lock()
            .expect("delivery failures")
            .push(failure);
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

#[derive(Default)]
struct FailingDeliverySink {
    attempts: Mutex<Vec<RuntimeUpdate>>,
    failures: Mutex<Vec<RuntimeUpdateDeliveryFailure>>,
    changed: Condvar,
}

impl AgentRuntimeUpdateSink for FailingDeliverySink {
    fn emit_update(
        &self,
        _invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        self.attempts.lock().expect("attempts").push(update);
        Err(RuntimePortError::new(
            RuntimePortErrorKind::EventDeliveryFailed,
            "deterministic delivery failure",
        ))
    }

    fn report_delivery_failure(
        &self,
        _invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.failures.lock().expect("failures").push(failure);
        self.changed.notify_all();
    }
}

impl FailingDeliverySink {
    fn wait_for_terminal_delivery_failure(&self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut failures = self.failures.lock().expect("failures");
        while !failures
            .iter()
            .any(|failure| failure.update_kind == RuntimeUpdateDeliveryKind::Finished)
        {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .expect("timed out");
            let (next, timeout) = self
                .changed
                .wait_timeout(failures, remaining)
                .expect("wait");
            assert!(
                !timeout.timed_out(),
                "timed out waiting for delivery failure"
            );
            failures = next;
        }
    }
}

#[test]
fn event_and_terminal_delivery_failures_are_observed_without_reclassifying_runtime_outcome() {
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .push_back(FIRST_TURN.as_bytes().to_vec());
    let runtime = CodexCliRuntime::new("fixture-codex", Some(capabilities()), factory);
    let concrete = Arc::new(FailingDeliverySink::default());
    let sink: Arc<dyn AgentRuntimeUpdateSink> = concrete.clone();
    runtime
        .start_invocation(request("delivery-failure", "first"), sink)
        .expect("launch remains successful");
    concrete.wait_for_terminal_delivery_failure();

    let attempts = concrete.attempts.lock().expect("attempts");
    assert!(attempts
        .iter()
        .any(|attempt| matches!(attempt, RuntimeUpdate::Event(_))));
    assert!(matches!(
        attempts.last(),
        Some(RuntimeUpdate::Finished(outcome))
            if outcome.status == AgentInvocationTerminalStatus::Completed
    ));
    let failures = concrete.failures.lock().expect("failures");
    assert!(failures
        .iter()
        .any(|failure| failure.update_kind == RuntimeUpdateDeliveryKind::Event));
    assert_eq!(
        failures.last().map(|failure| failure.update_kind),
        Some(RuntimeUpdateDeliveryKind::Finished)
    );
    assert!(failures
        .iter()
        .all(|failure| failure.error.kind == RuntimePortErrorKind::EventDeliveryFailed));
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
    let first_launch = runtime
        .start_invocation(request("inv-1", "first"), first_sink.clone())
        .expect("start");
    first_sink.wait_finished();
    let resume_sink = Arc::new(CollectingSink::default());
    let resume_launch = runtime
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
    assert_eq!(
        first_launch.effective_options,
        AgentRuntimeOptions::default()
    );
    assert_eq!(
        resume_launch.effective_options,
        AgentRuntimeOptions::default()
    );
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
