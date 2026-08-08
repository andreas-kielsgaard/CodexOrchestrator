use super::{
    arguments::{build_args_from_effective_options, prepare_options, InvocationCommand},
    capabilities::{
        resolve_program, CodexCliCapabilities, CodexCliCapabilityProbe,
        FixedCodexCapabilityDiscovery,
    },
    protocol::{CodexJsonlProtocol, JsonlTerminalEvidence},
};
use crate::{
    agent_sessions::{
        domain::{
            AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeEventSource,
            AgentRuntimeFailure, NormalizedRuntimeEvent, NormalizedRuntimeEventKind,
        },
        ports::{
            AgentAccessCapabilityDiscovery, AgentAccessCapabilitySnapshot, AgentRuntime,
            AgentRuntimeUpdateSink, CapabilityRefresh, InvocationCapabilities, RuntimeEventDraft,
            RuntimeInvocationMode, RuntimeInvocationOutcome, RuntimeInvocationPreflight,
            RuntimeInvocationRequest, RuntimePortError, RuntimePortErrorKind, RuntimeUpdate,
            RuntimeUpdateDeliveryFailure,
        },
    },
    runtime::capabilities::AgentAccessCapabilityCache,
    runtime::processes::{
        ChildProcessFactory, ProcessEventSink, ProcessFailureKind, ProcessLaunchSpec,
        ProcessOutput, ProcessOutputStream, ProcessSupervisor, ProcessTerminalOutcome,
        SupervisorError, SupervisorErrorKind, SystemProcessFactory,
    },
};
use chrono::Utc;
use serde_json::json;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

const GRACEFUL_SHUTDOWN_PERIOD: Duration = Duration::from_secs(2);

struct InvocationState {
    sink: Arc<dyn AgentRuntimeUpdateSink>,
    protocol: CodexJsonlProtocol,
    terminal_evidence: Option<JsonlTerminalEvidence>,
}

#[derive(Default)]
struct RuntimeCoordinator {
    invocations: Mutex<HashMap<AgentInvocationId, InvocationState>>,
}

impl RuntimeCoordinator {
    fn register(
        &self,
        invocation_id: AgentInvocationId,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        let mut invocations = self
            .invocations
            .lock()
            .map_err(|_| unavailable("Codex runtime registry lock was poisoned"))?;
        if invocations.contains_key(&invocation_id) {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::AlreadyActive,
                format!("invocation {invocation_id} is already active"),
            ));
        }
        invocations.insert(
            invocation_id,
            InvocationState {
                sink,
                protocol: CodexJsonlProtocol::default(),
                terminal_evidence: None,
            },
        );
        Ok(())
    }

    fn remove(&self, invocation_id: &AgentInvocationId) {
        if let Ok(mut invocations) = self.invocations.lock() {
            invocations.remove(invocation_id);
        }
    }
}

impl ProcessEventSink for RuntimeCoordinator {
    fn on_output(&self, invocation_id: &AgentInvocationId, output: ProcessOutput) {
        let (sink, events) = {
            let Ok(mut invocations) = self.invocations.lock() else {
                return;
            };
            let Some(state) = invocations.get_mut(invocation_id) else {
                return;
            };
            let mut events = Vec::new();
            match output.stream {
                ProcessOutputStream::Stdout => {
                    for parsed in state.protocol.push(&output.bytes) {
                        if parsed.terminal.is_some() {
                            state.terminal_evidence = parsed.terminal;
                        }
                        events.extend(parsed.events);
                    }
                }
                ProcessOutputStream::Stderr => {
                    let text = String::from_utf8_lossy(&output.bytes).into_owned();
                    events.push(RuntimeEventDraft {
                        source: AgentRuntimeEventSource::Stderr,
                        raw_payload: json!({"bytes": output.bytes, "lossyUtf8": text}),
                        normalized: Some(NormalizedRuntimeEvent {
                            kind: NormalizedRuntimeEventKind::Unknown,
                            text: Some(text),
                            external_context_id: None,
                            usage: None,
                            details: None,
                            tool_activity: None,
                        }),
                    });
                }
            }
            (state.sink.clone(), events)
        };
        for event in events {
            deliver_update(&sink, invocation_id, RuntimeUpdate::Event(event));
        }
    }

    fn on_terminal(&self, invocation_id: &AgentInvocationId, outcome: ProcessTerminalOutcome) {
        let state = self
            .invocations
            .lock()
            .ok()
            .and_then(|mut states| states.remove(invocation_id));
        let Some(mut state) = state else {
            return;
        };
        for parsed in state.protocol.finish() {
            if parsed.terminal.is_some() {
                state.terminal_evidence = parsed.terminal;
            }
            for event in parsed.events {
                deliver_update(&state.sink, invocation_id, RuntimeUpdate::Event(event));
            }
        }
        let finished = reconcile_terminal(state.terminal_evidence, outcome);
        deliver_update(
            &state.sink,
            invocation_id,
            RuntimeUpdate::Finished(finished),
        );
    }
}

fn deliver_update(
    sink: &Arc<dyn AgentRuntimeUpdateSink>,
    invocation_id: &AgentInvocationId,
    update: RuntimeUpdate,
) {
    if let Err(error) = sink.emit_update(invocation_id, update.clone()) {
        let failure = RuntimeUpdateDeliveryFailure { update, error };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sink.report_delivery_failure(invocation_id, failure)
        }));
    }
}

/// Codex CLI runtime adapter. Child ownership, output reads, cancellation, and process terminal
/// evidence are delegated exclusively to `ProcessSupervisor`.
pub(crate) struct CodexCliRuntime {
    program: Result<String, String>,
    capability_discovery: Arc<dyn AgentAccessCapabilityDiscovery>,
    capability_cache: AgentAccessCapabilityCache,
    coordinator: Arc<RuntimeCoordinator>,
    supervisor: ProcessSupervisor,
    #[cfg(test)]
    launch_observer: Option<Arc<dyn Fn(&ProcessLaunchSpec) + Send + Sync>>,
}

impl CodexCliRuntime {
    pub(crate) fn system(
        program: impl Into<String>,
        capabilities: Option<CodexCliCapabilities>,
    ) -> Self {
        Self::new(program, capabilities, Arc::new(SystemProcessFactory))
    }

    pub(crate) fn new(
        program: impl Into<String>,
        capabilities: Option<CodexCliCapabilities>,
        factory: Arc<dyn ChildProcessFactory>,
    ) -> Self {
        let program = resolve_program(program.into());
        let capability_discovery: Arc<dyn AgentAccessCapabilityDiscovery> = match capabilities {
            Some(capabilities) => Arc::new(FixedCodexCapabilityDiscovery::new(capabilities)),
            None => Arc::new(CodexCliCapabilityProbe::from_resolved(program.clone())),
        };
        Self::new_composed(program, capability_discovery, factory)
    }

    fn new_composed(
        program: Result<String, String>,
        capability_discovery: Arc<dyn AgentAccessCapabilityDiscovery>,
        factory: Arc<dyn ChildProcessFactory>,
    ) -> Self {
        let coordinator = Arc::new(RuntimeCoordinator::default());
        let supervisor = ProcessSupervisor::new(factory, coordinator.clone());
        Self {
            program,
            capability_discovery,
            capability_cache: AgentAccessCapabilityCache::default(),
            coordinator,
            supervisor,
            #[cfg(test)]
            launch_observer: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_discovery(
        program: impl Into<String>,
        capability_discovery: Arc<dyn AgentAccessCapabilityDiscovery>,
        factory: Arc<dyn ChildProcessFactory>,
    ) -> Self {
        Self::new_composed(
            resolve_program(program.into()),
            capability_discovery,
            factory,
        )
    }

    /// Forces a fresh Codex version/help observation without launching an agent invocation.
    pub(crate) fn refresh_capabilities(&self) -> AgentAccessCapabilitySnapshot {
        self.resolve_capabilities(CapabilityRefresh::Refresh)
    }

    /// Invalidates application-lifetime evidence after executable or configuration changes.
    pub(crate) fn invalidate_capabilities(&self) {
        self.capability_cache.invalidate();
    }

    fn resolve_capabilities(&self, refresh: CapabilityRefresh) -> AgentAccessCapabilitySnapshot {
        self.capability_cache
            .resolve(refresh, Utc::now(), self.capability_discovery.as_ref())
    }

    fn invocation_capabilities(&self, mode: RuntimeInvocationMode) -> InvocationCapabilities {
        let snapshot = self.resolve_capabilities(CapabilityRefresh::UseFreshCache);
        match mode {
            RuntimeInvocationMode::Start => snapshot.capabilities.start,
            RuntimeInvocationMode::Resume => snapshot.capabilities.resume,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_launch_observer(
        mut self,
        observer: Arc<dyn Fn(&ProcessLaunchSpec) + Send + Sync>,
    ) -> Self {
        self.launch_observer = Some(observer);
        self
    }

    /// Test-only visibility into direct children owned by this runtime's supervisor. This makes
    /// no claim about descendants (notably on Windows, where `Child::kill` is not tree kill).
    #[cfg(test)]
    pub(crate) fn active_direct_child_count(&self) -> Result<usize, RuntimePortError> {
        self.supervisor.active_count().map_err(map_supervisor_error)
    }

    fn launch(
        &self,
        request: RuntimeInvocationRequest,
        command: InvocationCommand<'_>,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        // Application persistence supplies the preflight-approved effective options. Launch must
        // not rediscover or reinterpret capabilities after that durable transition.
        let invocation_mode = match &command {
            InvocationCommand::Start => "start",
            InvocationCommand::Resume(_) => "resume",
        };
        let args = build_args_from_effective_options(
            command,
            &request.submitted_text,
            &request.options,
            request.launch_extension.as_ref(),
        );
        let program = self
            .program
            .clone()
            .map_err(|message| RuntimePortError::new(RuntimePortErrorKind::Unavailable, message))?;
        let spec = ProcessLaunchSpec {
            program,
            args,
            working_directory: request.working_directory.as_deref().map(PathBuf::from),
            environment: request
                .launch_extension
                .as_ref()
                .map(|extension| extension.environment.clone())
                .unwrap_or_default(),
        };
        #[cfg(test)]
        if let Some(observer) = &self.launch_observer {
            observer(&spec);
        }
        self.coordinator
            .register(request.invocation_id.clone(), update_sink)?;
        self.coordinator.record_launch_provenance(
            &request.invocation_id,
            &spec,
            invocation_mode,
        );
        match self
            .supervisor
            .start(request.session_id, request.invocation_id.clone(), spec)
        {
            Ok(()) => Ok(()),
            Err(error) => {
                if error.kind != SupervisorErrorKind::SpawnFailed {
                    self.coordinator.remove(&request.invocation_id);
                }
                Err(map_supervisor_error(error))
            }
        }
    }
}

impl RuntimeCoordinator {
    fn record_launch_provenance(
        &self,
        invocation_id: &AgentInvocationId,
        spec: &ProcessLaunchSpec,
        invocation_mode: &str,
    ) {
        let Some(sink) = self
            .invocations
            .lock()
            .ok()
            .and_then(|states| states.get(invocation_id).map(|state| state.sink.clone()))
        else {
            return;
        };
        let configuration_keys = spec
            .args
            .windows(2)
            .filter_map(|pair| {
                (pair[0] == "-c").then_some(
                    pair[1]
                        .split_once('=')
                        .map_or(pair[1].as_str(), |(key, _)| key),
                )
            })
            .map(sanitized_configuration_key)
            .collect::<Vec<_>>();
        let sandbox = spec
            .args
            .windows(2)
            .find_map(|pair| (pair[0] == "--sandbox").then_some(pair[1].as_str()))
            .or_else(|| {
                spec.args.windows(2).find_map(|pair| {
                    (pair[0] == "-c")
                        .then(|| pair[1].strip_prefix("sandbox_mode=\"")?.strip_suffix('"'))
                        .flatten()
                })
            });
        let working_directory = spec.working_directory.as_ref().map(|path| {
            let value = path.to_string_lossy();
            json!({
                "provided": true,
                "extendedLengthPrefix": value.starts_with(r"\\?\"),
                "absolute": path.is_absolute(),
            })
        }).unwrap_or_else(|| json!({"provided": false}));
        let launch_restrictions = json!({
            "strictConfig": spec.args.iter().any(|argument| argument == "--strict-config"),
            "ignoresUserConfig": spec.args.iter().any(|argument| argument == "--ignore-user-config"),
            "ignoresRules": spec.args.iter().any(|argument| argument == "--ignore-rules"),
            "additionalWritableDirectoryCount": spec.args.iter().filter(|argument| argument.as_str() == "--add-dir").count(),
            "dangerouslyBypassesApprovalsAndSandbox": spec.args.iter().any(|argument| argument == "--dangerously-bypass-approvals-and-sandbox"),
            "dangerouslyBypassesHookTrust": spec.args.iter().any(|argument| argument == "--dangerously-bypass-hook-trust"),
            "liveWebSearchEnabled": spec.args.iter().any(|argument| argument == "--search"),
        });
        deliver_update(
            &sink,
            invocation_id,
            RuntimeUpdate::Event(RuntimeEventDraft {
                source: AgentRuntimeEventSource::Runtime,
                raw_payload: json!({
                    "kind": "codex_launch_provenance",
                    "invocationMode": invocation_mode,
                    "executable": PathBuf::from(&spec.program).file_name().and_then(|name| name.to_str()).unwrap_or("codex"),
                    "sandbox": sandbox,
                    "sandboxAuthorityEvidence": if sandbox == Some("workspace-write") {
                        "unverified_until_application_candidate_evidence"
                    } else {
                        "configuration_only"
                    },
                    "configurationKeys": configuration_keys,
                    "environmentKeys": spec.environment.iter().map(|(key, _)| key).collect::<Vec<_>>(),
                    "inheritsParentEnvironment": true,
                    "parentCodeHomePresent": std::env::var_os("CODEX_HOME").is_some(),
                    "launchRestrictions": launch_restrictions,
                    "workingDirectory": working_directory,
                }),
                normalized: None,
            }),
        );
    }
}

/// Launch provenance intentionally keeps configuration names but never an isolated-worktree
/// path.  The provider still receives the exact launch-only key; persistence records only the
/// bounded purpose of that key.
pub(super) fn sanitized_configuration_key(key: &str) -> String {
    if key.starts_with("projects.") && key.ends_with(".trust_level") {
        "projects.<application-bound-workspace>.trust_level".into()
    } else {
        key.into()
    }
}

impl AgentRuntime for CodexCliRuntime {
    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        requested_options: &crate::agent_sessions::domain::AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        let capabilities = self.invocation_capabilities(mode);
        let effective_options = prepare_options(requested_options, &capabilities)?;
        Ok(RuntimeInvocationPreflight { effective_options })
    }

    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.launch(request, InvocationCommand::Start, update_sink)
    }

    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: crate::agent_sessions::domain::ExternalRuntimeContextId,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.launch(
            request,
            InvocationCommand::Resume(&external_context_id),
            update_sink,
        )
    }

    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        self.supervisor
            .cancel(invocation_id)
            .map_err(map_supervisor_error)
    }

    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.supervisor
            .shutdown_with_grace_period(GRACEFUL_SHUTDOWN_PERIOD)
            .map_err(map_supervisor_error)
    }
}

pub(super) fn reconcile_terminal(
    evidence: Option<JsonlTerminalEvidence>,
    process: ProcessTerminalOutcome,
) -> RuntimeInvocationOutcome {
    match process {
        ProcessTerminalOutcome::Canceled { exit } => {
            let (exit_code, signal) = exit_parts(exit);
            outcome(
                AgentInvocationTerminalStatus::Canceled,
                exit_code,
                signal,
                None,
            )
        }
        ProcessTerminalOutcome::Interrupted { exit } => {
            let (exit_code, signal) = exit_parts(exit);
            outcome(
                AgentInvocationTerminalStatus::Interrupted,
                exit_code,
                signal,
                None,
            )
        }
        ProcessTerminalOutcome::Failed {
            kind,
            exit,
            message,
        } => {
            let code = match kind {
                ProcessFailureKind::SpawnFailed => "codex_spawn_failed",
                ProcessFailureKind::NonZeroExit => "codex_nonzero_exit",
                ProcessFailureKind::ReaderFailed => "codex_output_reader_failed",
                ProcessFailureKind::WaitFailed => "codex_wait_failed",
                ProcessFailureKind::CancellationFailed => "codex_cancellation_failed",
                ProcessFailureKind::SupervisorFailed => "codex_supervisor_failed",
            };
            let (exit_code, signal) = exit
                .map(|exit| (exit.exit_code, exit.signal))
                .unwrap_or_default();
            outcome(
                AgentInvocationTerminalStatus::Failed,
                exit_code,
                signal,
                Some(failure(code, message, evidence)),
            )
        }
        ProcessTerminalOutcome::Exited(exit) => match evidence {
            Some(JsonlTerminalEvidence::Completed) => outcome(
                AgentInvocationTerminalStatus::Completed,
                exit.exit_code,
                exit.signal,
                None,
            ),
            Some(JsonlTerminalEvidence::Failed) => outcome(
                AgentInvocationTerminalStatus::Failed,
                exit.exit_code,
                exit.signal,
                Some(failure(
                    "codex_turn_failed",
                    "Codex reported turn.failed",
                    evidence,
                )),
            ),
            Some(JsonlTerminalEvidence::Error) => outcome(
                AgentInvocationTerminalStatus::Failed,
                exit.exit_code,
                exit.signal,
                Some(failure(
                    "codex_error_event",
                    "Codex reported an error event",
                    evidence,
                )),
            ),
            None => outcome(
                AgentInvocationTerminalStatus::Failed,
                exit.exit_code,
                exit.signal,
                Some(failure(
                    "codex_missing_terminal_event",
                    "Codex exited successfully without JSONL terminal evidence",
                    evidence,
                )),
            ),
        },
    }
}

fn exit_parts(
    exit: Option<crate::runtime::processes::ProcessExit>,
) -> (Option<i32>, Option<String>) {
    exit.map(|exit| (exit.exit_code, exit.signal))
        .unwrap_or_default()
}

fn outcome(
    status: AgentInvocationTerminalStatus,
    exit_code: Option<i32>,
    signal: Option<String>,
    runtime_error: Option<AgentRuntimeFailure>,
) -> RuntimeInvocationOutcome {
    RuntimeInvocationOutcome {
        status,
        exit_code,
        signal,
        runtime_error,
    }
}

fn failure(
    code: &str,
    message: impl Into<String>,
    evidence: Option<JsonlTerminalEvidence>,
) -> AgentRuntimeFailure {
    AgentRuntimeFailure {
        code: code.to_string(),
        message: message.into(),
        details: Some(json!({"jsonlTerminalEvidence": format!("{evidence:?}")})),
    }
}

fn map_supervisor_error(error: SupervisorError) -> RuntimePortError {
    let kind = match error.kind {
        SupervisorErrorKind::AlreadyActive | SupervisorErrorKind::DuplicateInvocation => {
            RuntimePortErrorKind::AlreadyActive
        }
        SupervisorErrorKind::NotActive => RuntimePortErrorKind::NotActive,
        SupervisorErrorKind::SpawnFailed => RuntimePortErrorKind::LaunchFailed,
        SupervisorErrorKind::CancellationFailed => RuntimePortErrorKind::CancellationFailed,
        SupervisorErrorKind::ShuttingDown | SupervisorErrorKind::Internal => {
            RuntimePortErrorKind::Unavailable
        }
    };
    RuntimePortError::new(kind, error.message)
}

fn unavailable(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message)
}
