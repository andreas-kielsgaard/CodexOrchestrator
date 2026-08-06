use super::{
    arguments::{build_args_from_effective_options, prepare_options, InvocationCommand},
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
            ToolActivityPhase, ToolResultClassification,
        },
        ports::{
            AgentAccessCapabilities, AgentAccessCapabilityDiscovery, AgentAccessCapabilitySnapshot,
            AgentRuntime, AgentRuntimeUpdateSink, CapabilityDiscoveryState, CapabilityProvenance,
            CapabilitySupport, InvocationCapabilities, RuntimeInvocationMode,
            RuntimeInvocationRequest, RuntimeLaunchExtension, RuntimePortError,
            RuntimePortErrorKind, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
        },
    },
    runtime::processes::{
        ChildProcessFactory, ProcessExit, ProcessFailureKind, ProcessLaunchSpec,
        ProcessTerminalOutcome, SpawnedProcess, SupervisedChild,
    },
};
use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    io::{self, Cursor},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};

const FIRST_TURN: &str = include_str!("fixtures/codex-cli-0.144.0/first-turn.jsonl");
const RESUME: &str = include_str!("fixtures/codex-cli-0.144.0/resume.jsonl");
const MALFORMED: &str = include_str!("fixtures/codex-cli-0.144.0/malformed-and-unknown.jsonl");
const MCP_TOOL_EVENTS: &str = include_str!("fixtures/codex-cli-0.144.0/mcp-tool-events.jsonl");

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

fn invocation_capabilities(resume: bool) -> InvocationCapabilities {
    InvocationCapabilities {
        structured_events: CapabilitySupport::Supported,
        model_selection: CapabilitySupport::Supported,
        sandbox_selection: if resume {
            CapabilitySupport::Unsupported
        } else {
            CapabilitySupport::Supported
        },
    }
}

struct SnapshotDiscovery {
    count: AtomicUsize,
    start: InvocationCapabilities,
    resume: InvocationCapabilities,
    state: CapabilityDiscoveryState,
}

struct MutableSnapshotDiscovery {
    count: AtomicUsize,
    capabilities: Mutex<AgentAccessCapabilities>,
}

impl AgentAccessCapabilityDiscovery for MutableSnapshotDiscovery {
    fn discover_capabilities(
        &self,
        observed_at: chrono::DateTime<Utc>,
    ) -> AgentAccessCapabilitySnapshot {
        self.count.fetch_add(1, Ordering::SeqCst);
        AgentAccessCapabilitySnapshot {
            capabilities: self.capabilities.lock().expect("capabilities").clone(),
            discovery_state: CapabilityDiscoveryState::Observed,
            provenance: CapabilityProvenance {
                source: "mutable_test_codex_probe".to_string(),
                runtime_version: Some("codex-cli mutable-test".to_string()),
            },
            observed_at,
            valid_until: observed_at + ChronoDuration::minutes(30),
            unavailable_reason: None,
        }
    }
}

impl AgentAccessCapabilityDiscovery for SnapshotDiscovery {
    fn discover_capabilities(
        &self,
        observed_at: chrono::DateTime<Utc>,
    ) -> AgentAccessCapabilitySnapshot {
        self.count.fetch_add(1, Ordering::SeqCst);
        AgentAccessCapabilitySnapshot {
            capabilities: AgentAccessCapabilities {
                start: self.start.clone(),
                resume: self.resume.clone(),
            },
            discovery_state: self.state,
            provenance: CapabilityProvenance {
                source: "test_codex_probe".to_string(),
                runtime_version: (self.state == CapabilityDiscoveryState::Observed)
                    .then(|| "codex-cli test".to_string()),
            },
            observed_at,
            valid_until: observed_at + ChronoDuration::minutes(30),
            unavailable_reason: (self.state == CapabilityDiscoveryState::Unavailable)
                .then(|| "test probe unavailable".to_string()),
        }
    }
}

fn observed_discovery() -> Arc<SnapshotDiscovery> {
    Arc::new(SnapshotDiscovery {
        count: AtomicUsize::new(0),
        start: invocation_capabilities(false),
        resume: invocation_capabilities(true),
        state: CapabilityDiscoveryState::Observed,
    })
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
fn failed_codex_probe_is_unavailable_with_unknown_semantics() {
    let missing = std::env::temp_dir().join(format!(
        "missing-codex-capability-probe-{}",
        uuid::Uuid::new_v4()
    ));
    let snapshot = CodexCliCapabilityProbe::new(missing.to_string_lossy().into_owned())
        .discover_capabilities(Utc::now());

    assert_eq!(
        snapshot.discovery_state,
        CapabilityDiscoveryState::Unavailable
    );
    assert_eq!(
        snapshot.capabilities.start,
        InvocationCapabilities::default()
    );
    assert_eq!(
        snapshot.capabilities.resume,
        InvocationCapabilities::default()
    );
    assert!(snapshot.unavailable_reason.is_some());
}

#[test]
fn builds_supported_first_turn_and_resume_commands() {
    let options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    let start_effective =
        prepare_options(&options, &invocation_capabilities(false)).expect("start preflight");
    let start = build_args_from_effective_options(
        InvocationCommand::Start,
        "hello",
        &start_effective,
        None,
    );
    assert_eq!(
        start,
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
    assert_eq!(start_effective, options);
    let context = ExternalRuntimeContextId::new("thread-external").expect("context");
    let resume_options = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: None,
    };
    let resume_effective =
        prepare_options(&resume_options, &invocation_capabilities(true)).expect("resume preflight");
    let resume = build_args_from_effective_options(
        InvocationCommand::Resume(&context),
        "continue",
        &resume_effective,
        None,
    );
    assert_eq!(
        resume,
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
    assert_eq!(resume_effective, resume_options);
}

#[test]
fn resume_assembles_sandbox_through_the_supported_strict_config_surface() {
    let options = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let effective = prepare_options(
        &options,
        &InvocationCapabilities {
            structured_events: CapabilitySupport::Supported,
            model_selection: CapabilitySupport::Supported,
            sandbox_selection: CapabilitySupport::Supported,
        },
    )
    .unwrap();
    let context = ExternalRuntimeContextId::new("thread-resume").unwrap();
    assert_eq!(
        build_args_from_effective_options(
            InvocationCommand::Resume(&context),
            "continue",
            &effective,
            None,
        ),
        [
            "exec",
            "resume",
            "--json",
            "-c",
            "sandbox_mode=\"read-only\"",
            "thread-resume",
            "continue",
        ]
    );
}

#[test]
fn launch_provenance_redacts_the_exact_workspace_from_project_trust_configuration() {
    assert_eq!(
        super::runtime::sanitized_configuration_key(
            r#"projects.'c:\isolated\execution-workspace'.trust_level"#
        ),
        "projects.<application-bound-workspace>.trust_level"
    );
    assert_eq!(
        super::runtime::sanitized_configuration_key("approval_policy"),
        "approval_policy"
    );
}

#[test]
fn omits_optional_options_when_capability_data_is_absent() {
    let options = AgentRuntimeOptions {
        model: Some("unverified-model".to_string()),
        sandbox: None,
    };
    let effective =
        prepare_options(&options, &InvocationCapabilities::default()).expect("preflight defaults");
    let args =
        build_args_from_effective_options(InvocationCommand::Start, "hello", &effective, None);
    assert_eq!(args, ["exec", "--json", "hello"]);
    assert_eq!(effective, AgentRuntimeOptions::default());
}

#[test]
fn rejects_requested_sandbox_when_support_is_unknown() {
    let options = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let error = prepare_options(&options, &InvocationCapabilities::default())
        .expect_err("sandbox enforcement must fail closed");
    assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
    assert!(error.message.contains("sandbox support is unknown"));
}

#[test]
fn assembles_enforced_plan_builder_runtime_and_child_configuration() {
    let effective = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let extension = RuntimeLaunchExtension {
        additional_args: vec![
            "-c".into(),
            "approval_policy=\"never\"".into(),
            "-c".into(),
            "mcp_servers.role.required=true".into(),
        ],
        environment: vec![],
        initial_prompt_prefix: None,
    };
    assert_eq!(
        build_args_from_effective_options(
            InvocationCommand::Start,
            "plan",
            &effective,
            Some(&extension),
        ),
        [
            "exec",
            "--json",
            "--sandbox",
            "read-only",
            "-c",
            "approval_policy=\"never\"",
            "-c",
            "mcp_servers.role.required=true",
            "plan",
        ]
    );
}

#[test]
fn resume_places_child_configuration_before_the_session_id() {
    let context = ExternalRuntimeContextId::new("thread-resume").unwrap();
    let extension = RuntimeLaunchExtension {
        additional_args: vec!["-c".into(), "mcp_servers.plan_builder.required=true".into()],
        environment: vec![],
        initial_prompt_prefix: None,
    };
    assert_eq!(
        build_args_from_effective_options(
            InvocationCommand::Resume(&context),
            "build",
            &AgentRuntimeOptions::default(),
            Some(&extension),
        ),
        [
            "exec",
            "resume",
            "--json",
            "-c",
            "mcp_servers.plan_builder.required=true",
            "thread-resume",
            "build",
        ]
    );
}

#[test]
fn rejects_a_confirmed_unsupported_resume_sandbox() {
    let options = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };
    let error =
        prepare_options(&options, &invocation_capabilities(true)).expect_err("unsupported sandbox");
    assert!(error.message.contains("sandbox"));
}

#[test]
fn runtime_reuses_discovery_and_exposes_refresh_and_invalidation() {
    let discovery = observed_discovery();
    let factory = Arc::new(FixtureFactory::default());
    let runtime = CodexCliRuntime::new_with_discovery("fixture-codex", discovery.clone(), factory);

    runtime
        .preflight_invocation(
            RuntimeInvocationMode::Start,
            &AgentRuntimeOptions::default(),
        )
        .expect("first preflight");
    runtime
        .preflight_invocation(
            RuntimeInvocationMode::Resume,
            &AgentRuntimeOptions::default(),
        )
        .expect("cached resume preflight");
    assert_eq!(discovery.count.load(Ordering::SeqCst), 1);

    runtime.refresh_capabilities();
    assert_eq!(discovery.count.load(Ordering::SeqCst), 2);
    runtime.invalidate_capabilities();
    runtime
        .preflight_invocation(
            RuntimeInvocationMode::Start,
            &AgentRuntimeOptions::default(),
        )
        .expect("preflight after invalidation");
    assert_eq!(discovery.count.load(Ordering::SeqCst), 3);
}

#[test]
fn unavailable_discovery_rejects_a_required_sandbox_before_launch() {
    let discovery = Arc::new(SnapshotDiscovery {
        count: AtomicUsize::new(0),
        start: InvocationCapabilities::default(),
        resume: InvocationCapabilities::default(),
        state: CapabilityDiscoveryState::Unavailable,
    });
    let runtime = CodexCliRuntime::new_with_discovery(
        "fixture-codex",
        discovery.clone(),
        Arc::new(FixtureFactory::default()),
    );
    let requested = AgentRuntimeOptions {
        model: Some("unverified-model".to_string()),
        sandbox: Some(RuntimeSandboxMode::ReadOnly),
    };

    let error = runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &requested)
        .expect_err("unknown sandbox support must fail closed");

    assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
    assert_eq!(discovery.count.load(Ordering::SeqCst), 1);
}

#[test]
fn start_and_resume_use_distinct_capability_surfaces() {
    let discovery = observed_discovery();
    let runtime = CodexCliRuntime::new_with_discovery(
        "fixture-codex",
        discovery,
        Arc::new(FixtureFactory::default()),
    );
    let requested = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };

    assert_eq!(
        runtime
            .preflight_invocation(RuntimeInvocationMode::Start, &requested)
            .expect("start sandbox")
            .effective_options,
        requested
    );
    let error = runtime
        .preflight_invocation(RuntimeInvocationMode::Resume, &requested)
        .expect_err("resume sandbox is independently unsupported");
    assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
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
    stderr: Mutex<VecDeque<Vec<u8>>>,
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
        let stderr = self
            .stderr
            .lock()
            .expect("stderr")
            .pop_front()
            .unwrap_or_default();
        Ok(SpawnedProcess {
            child: Arc::new(ImmediateChild),
            stdout: Box::new(Cursor::new(stdout)),
            stderr: Box::new(Cursor::new(stderr)),
        })
    }
}

#[test]
fn launch_uses_persisted_preflight_options_without_rediscovery() {
    let discovery = Arc::new(MutableSnapshotDiscovery {
        count: AtomicUsize::new(0),
        capabilities: Mutex::new(AgentAccessCapabilities {
            start: invocation_capabilities(false),
            resume: invocation_capabilities(true),
        }),
    });
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .push_back(FIRST_TURN.as_bytes().to_vec());
    let runtime =
        CodexCliRuntime::new_with_discovery("fixture-codex", discovery.clone(), factory.clone());
    let requested = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    let preflight = runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &requested)
        .expect("preflight");
    assert_eq!(discovery.count.load(Ordering::SeqCst), 1);

    *discovery.capabilities.lock().expect("capabilities") = AgentAccessCapabilities {
        start: InvocationCapabilities {
            structured_events: CapabilitySupport::Unsupported,
            model_selection: CapabilitySupport::Unsupported,
            sandbox_selection: CapabilitySupport::Unsupported,
        },
        resume: InvocationCapabilities::default(),
    };
    runtime.refresh_capabilities();
    assert_eq!(discovery.count.load(Ordering::SeqCst), 2);

    let mut invocation = request("preflight-options", "first");
    invocation.options = preflight.effective_options;
    let sink = Arc::new(CollectingSink::default());
    runtime
        .start_invocation(invocation, sink.clone())
        .expect("launch uses persisted effective options");
    sink.wait_finished();

    assert_eq!(discovery.count.load(Ordering::SeqCst), 2);
    let specs = factory.specs.lock().expect("specs");
    assert_eq!(
        specs[0].args,
        [
            "exec",
            "--json",
            "--model",
            "gpt-5",
            "--sandbox",
            "workspace-write",
            "first"
        ]
    );
}

#[test]
fn confirmed_unsupported_requested_option_fails_before_process_launch() {
    let discovery = observed_discovery();
    let factory = Arc::new(FixtureFactory::default());
    let runtime = CodexCliRuntime::new_with_discovery("fixture-codex", discovery, factory.clone());
    let requested = AgentRuntimeOptions {
        model: None,
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    let error = runtime
        .preflight_invocation(RuntimeInvocationMode::Resume, &requested)
        .expect_err("unsupported resume sandbox");

    assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
    assert!(factory.specs.lock().expect("specs").is_empty());
}

#[test]
fn stderr_is_preserved_as_bytes_and_normalized_as_readable_text() {
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .push_back(FIRST_TURN.as_bytes().to_vec());
    factory
        .stderr
        .lock()
        .expect("stderr")
        .push_back(b"readable warning\n".to_vec());
    let runtime = CodexCliRuntime::new("fixture-codex", Some(capabilities()), factory);
    let sink = Arc::new(CollectingSink::default());

    runtime
        .start_invocation(request("stderr-invocation", "first"), sink.clone())
        .expect("start invocation");
    sink.wait_finished();

    let updates = sink.updates.lock().expect("updates");
    let stderr = updates
        .iter()
        .find_map(|update| match update {
            RuntimeUpdate::Event(event)
                if event.source
                    == crate::agent_sessions::domain::AgentRuntimeEventSource::Stderr =>
            {
                Some(event)
            }
            _ => None,
        })
        .expect("stderr event");
    assert_eq!(
        stderr.raw_payload["bytes"],
        serde_json::json!([
            114, 101, 97, 100, 97, 98, 108, 101, 32, 119, 97, 114, 110, 105, 110, 103, 10
        ])
    );
    assert_eq!(
        stderr
            .normalized
            .as_ref()
            .and_then(|normalized| normalized.text.as_deref()),
        Some("readable warning\n")
    );
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

struct RunningOrderSink {
    durable_running: Arc<AtomicBool>,
    inner: CollectingSink,
}

impl AgentRuntimeUpdateSink for RunningOrderSink {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        assert!(
            self.durable_running.load(Ordering::Acquire),
            "runtime update arrived before the durable running transition"
        );
        self.inner.emit_update(invocation_id, update)
    }

    fn report_delivery_failure(
        &self,
        invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.inner.report_delivery_failure(invocation_id, failure);
    }
}

#[test]
fn preflight_allows_durable_running_transition_before_an_immediate_child_can_emit() {
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .push_back(FIRST_TURN.as_bytes().to_vec());
    let runtime = CodexCliRuntime::new("fixture-codex", Some(capabilities()), factory);
    let requested = AgentRuntimeOptions {
        model: Some("gpt-5".to_string()),
        sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
    };
    let preflight = runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &requested)
        .expect("preflight");
    assert_eq!(preflight.effective_options, requested);

    let durable_running = Arc::new(AtomicBool::new(false));
    let sink = Arc::new(RunningOrderSink {
        durable_running: durable_running.clone(),
        inner: CollectingSink::default(),
    });
    durable_running.store(true, Ordering::Release);
    let mut invocation = request("immediate", "first");
    invocation.options = requested;
    runtime
        .start_invocation(invocation, sink.clone())
        .expect("launch");
    sink.inner.wait_finished();
    assert!(sink
        .inner
        .delivery_failures
        .lock()
        .expect("delivery failures")
        .is_empty());
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
            .any(|failure| matches!(failure.update, RuntimeUpdate::Finished(_)))
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
        .any(|failure| matches!(failure.update, RuntimeUpdate::Event(_))));
    assert!(matches!(
        failures.last().map(|failure| &failure.update),
        Some(RuntimeUpdate::Finished(outcome))
            if outcome.status == AgentInvocationTerminalStatus::Completed
    ));
    assert_eq!(
        failures
            .iter()
            .map(|failure| &failure.update)
            .collect::<Vec<_>>(),
        attempts.iter().collect::<Vec<_>>()
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
    let first_preflight = runtime
        .preflight_invocation(
            RuntimeInvocationMode::Start,
            &AgentRuntimeOptions::default(),
        )
        .expect("start preflight");
    runtime
        .start_invocation(request("inv-1", "first"), first_sink.clone())
        .expect("start");
    first_sink.wait_finished();
    let resume_sink = Arc::new(CollectingSink::default());
    let resume_preflight = runtime
        .preflight_invocation(
            RuntimeInvocationMode::Resume,
            &AgentRuntimeOptions::default(),
        )
        .expect("resume preflight");
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
    assert_eq!(
        first_preflight.effective_options,
        AgentRuntimeOptions::default()
    );
    assert_eq!(
        resume_preflight.effective_options,
        AgentRuntimeOptions::default()
    );
}

#[test]
fn launch_observer_sees_resume_external_context_not_local_ids() {
    let factory = Arc::new(FixtureFactory::default());
    factory
        .stdout
        .lock()
        .expect("stdout")
        .push_back(FIRST_TURN.as_bytes().to_vec());
    let observed = Arc::new(Mutex::new(Vec::<ProcessLaunchSpec>::new()));
    let observer_target = observed.clone();
    let runtime = CodexCliRuntime::new("codex", Some(capabilities()), factory)
        .with_launch_observer(Arc::new(move |spec| {
            observer_target
                .lock()
                .expect("observed specs")
                .push(spec.clone());
        }));
    let local_session = AgentSessionId::new("local-session-17").expect("session");
    let local_invocation = AgentInvocationId::new("local-invocation-23").expect("invocation");
    let external_context = ExternalRuntimeContextId::new("codex-thread-41").expect("context");
    let sink = Arc::new(CollectingSink::default());

    runtime
        .resume_invocation(
            RuntimeInvocationRequest {
                session_id: local_session,
                invocation_id: local_invocation,
                submitted_text: "continue".to_string(),
                working_directory: None,
                options: AgentRuntimeOptions::default(),
                launch_extension: None,
            },
            external_context.clone(),
            sink.clone(),
        )
        .expect("resume");
    sink.wait_finished();

    let specs = observed.lock().expect("observed specs");
    assert_eq!(specs.len(), 1);
    assert_eq!(
        specs[0].args,
        ["exec", "resume", "--json", "codex-thread-41", "continue"]
    );
    assert!(specs[0]
        .args
        .contains(&external_context.as_str().to_string()));
    assert!(!specs[0].args.contains(&"local-session-17".to_string()));
    assert!(!specs[0].args.contains(&"local-invocation-23".to_string()));
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
        launch_extension: None,
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
    assert_eq!(discovered.resume_sandbox, Some(true));
}
