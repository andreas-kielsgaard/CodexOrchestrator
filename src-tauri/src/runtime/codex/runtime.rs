use super::{
    arguments::{build_args, InvocationCommand},
    capabilities::{resolve_program, CodexCliCapabilities},
    protocol::{CodexJsonlProtocol, JsonlTerminalEvidence},
};
use crate::{
    agent_sessions::{
        domain::{
            AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeEventSource,
            AgentRuntimeFailure,
        },
        ports::{
            AgentRuntime, AgentRuntimeUpdateSink, RuntimeEventDraft, RuntimeInvocationOutcome,
            RuntimeInvocationRequest, RuntimePortError, RuntimePortErrorKind, RuntimeUpdate,
        },
    },
    runtime::processes::{
        ChildProcessFactory, ProcessEventSink, ProcessFailureKind, ProcessLaunchSpec,
        ProcessOutput, ProcessOutputStream, ProcessSupervisor, ProcessTerminalOutcome,
        SupervisorError, SupervisorErrorKind, SystemProcessFactory,
    },
};
use serde_json::json;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

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
                        normalized: None,
                    });
                }
            }
            (state.sink.clone(), events)
        };
        for event in events {
            let _ = sink.emit_update(invocation_id, RuntimeUpdate::Event(event));
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
                let _ = state
                    .sink
                    .emit_update(invocation_id, RuntimeUpdate::Event(event));
            }
        }
        let finished = reconcile_terminal(state.terminal_evidence, outcome);
        let _ = state
            .sink
            .emit_update(invocation_id, RuntimeUpdate::Finished(finished));
    }
}

/// Codex CLI runtime adapter. Child ownership, output reads, cancellation, and process terminal
/// evidence are delegated exclusively to `ProcessSupervisor`.
pub(crate) struct CodexCliRuntime {
    program: String,
    capabilities: Option<CodexCliCapabilities>,
    coordinator: Arc<RuntimeCoordinator>,
    supervisor: ProcessSupervisor,
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
        let coordinator = Arc::new(RuntimeCoordinator::default());
        let supervisor = ProcessSupervisor::new(factory, coordinator.clone());
        Self {
            program: resolve_program(program.into()),
            capabilities,
            coordinator,
            supervisor,
        }
    }

    fn launch(
        &self,
        request: RuntimeInvocationRequest,
        command: InvocationCommand<'_>,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        let args = build_args(
            command,
            &request.submitted_text,
            &request.options,
            self.capabilities.as_ref(),
        )?;
        let spec = ProcessLaunchSpec {
            program: self.program.clone(),
            args,
            working_directory: request.working_directory.as_deref().map(PathBuf::from),
            environment: Vec::new(),
        };
        self.coordinator
            .register(request.invocation_id.clone(), update_sink)?;
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

impl AgentRuntime for CodexCliRuntime {
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
