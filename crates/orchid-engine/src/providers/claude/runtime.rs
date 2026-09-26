//! One `claude` process per invocation. The process keeps its input open while the invocation
//! runs, so steering messages and answers to permission prompts reach the same conversation.
use super::{
    connection::{unavailable, Connection},
    events::{self, ClaudeEvents},
    launch::{self, Conversation},
    requests,
};
use crate::{
    contracts::{control::*, domain::*, ports::*},
    processes::*,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub(super) struct Invocation {
    pub(super) connection: Connection,
    sink: Arc<dyn AgentRuntimeUpdateSink>,
    session_id: String,
    target: RuntimeTurnTarget,
    events: Mutex<ClaudeEvents>,
    pub(super) requests: Mutex<HashMap<String, requests::PendingRequest>>,
    required_servers: Vec<String>,
    /// A prepared prompt awaiting delivery.
    prompt: Mutex<Option<String>>,
    /// User messages written but not yet taken into the conversation. The invocation finishes at
    /// the result that follows the last of them.
    unanswered: AtomicUsize,
    canceled: AtomicBool,
    finished: AtomicBool,
}

impl Invocation {
    fn emit(&self, update: RuntimeUpdate) {
        if let Err(error) = self
            .sink
            .emit_update(&self.connection.invocation_id, update.clone())
        {
            self.sink.report_delivery_failure(
                &self.connection.invocation_id,
                RuntimeUpdateDeliveryFailure { update, error },
            );
        }
    }

    pub(super) fn control(&self, record: RuntimeControlRecord) {
        self.emit(RuntimeUpdate::Event(record.into_draft()));
    }

    pub(super) fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    fn send_user_message(&self, text: &str) -> Result<(), RuntimePortError> {
        self.unanswered.fetch_add(1, Ordering::AcqRel);
        let written = self.connection.write(json!({
            "type": "user",
            "message": {"role": "user", "content": text},
            "parent_tool_use_id": null,
            "session_id": self.session_id,
        }));
        if written.is_err() {
            self.unanswered.fetch_sub(1, Ordering::AcqRel);
        }
        written
    }

    fn finish(&self, status: AgentInvocationTerminalStatus, error: Option<String>) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        self.connection.finish();
        self.emit(RuntimeUpdate::Finished(RuntimeInvocationOutcome {
            status,
            exit_code: None,
            signal: None,
            runtime_error: error.map(|message| AgentRuntimeFailure {
                code: "claude_code".into(),
                message,
                details: None,
            }),
        }));
    }

    fn receive(&self, message: Value) {
        if self.is_finished() {
            return;
        }
        match message["type"].as_str() {
            Some("control_request") => {
                if let Err(error) = requests::receive(self, &message) {
                    self.finish(AgentInvocationTerminalStatus::Failed, Some(error.message));
                }
            }
            Some("user") if message["isReplay"] == true => {
                self.unanswered.fetch_sub(1, Ordering::AcqRel);
            }
            Some("result") => self.result(&message),
            _ => {
                let unavailable_server = (message["type"] == "system"
                    && message["subtype"] == "init")
                    .then(|| self.unavailable_required_server(&message))
                    .flatten();
                let drafts = self
                    .events
                    .lock()
                    .map(|mut events| events.message(&message));
                for draft in drafts.unwrap_or_default() {
                    self.emit(RuntimeUpdate::Event(draft));
                }
                if let Some(server) = unavailable_server {
                    self.finish(
                        AgentInvocationTerminalStatus::Failed,
                        Some(format!("Required MCP server '{server}' is unavailable")),
                    );
                }
            }
        }
    }

    fn unavailable_required_server(&self, init: &Value) -> Option<String> {
        self.required_servers.iter().find_map(|name| {
            let connected = init["mcp_servers"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|server| server["name"] == name.as_str() && server["status"] == "connected");
            (!connected).then(|| name.clone())
        })
    }

    fn result(&self, message: &Value) {
        let last =
            self.canceled.load(Ordering::Acquire) || self.unanswered.load(Ordering::Acquire) == 0;
        let drafts = self
            .events
            .lock()
            .map(|mut events| events.result(message, last));
        for draft in drafts.unwrap_or_default() {
            self.emit(RuntimeUpdate::Event(draft));
        }
        if !last {
            return;
        }
        match events::result_error(message) {
            None => self.finish(AgentInvocationTerminalStatus::Completed, None),
            Some(_) if self.canceled.load(Ordering::Acquire) => {
                self.finish(AgentInvocationTerminalStatus::Canceled, None)
            }
            Some(error) => self.finish(AgentInvocationTerminalStatus::Failed, Some(error)),
        }
    }
}

#[derive(Default)]
struct Coordinator {
    invocations: Mutex<HashMap<AgentInvocationId, Arc<Invocation>>>,
}

impl Coordinator {
    fn get(&self, id: &AgentInvocationId) -> Result<Arc<Invocation>, RuntimePortError> {
        self.invocations
            .lock()
            .map_err(|_| unavailable("Invocation lock poisoned"))?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                RuntimePortError::new(
                    RuntimePortErrorKind::NotActive,
                    "Invocation is no longer connected",
                )
            })
    }
}

impl ProcessEventSink for Coordinator {
    fn on_output(&self, id: &AgentInvocationId, output: ProcessOutput) {
        let Ok(invocation) = self.get(id) else {
            return;
        };
        if output.stream == ProcessOutputStream::Stderr {
            invocation.emit(RuntimeUpdate::Event(RuntimeEventDraft {
                source: AgentRuntimeEventSource::Stderr,
                raw_payload: json!({"text": String::from_utf8_lossy(&output.bytes)}),
                normalized: None,
            }));
            return;
        }
        match invocation.connection.read(&output.bytes) {
            Ok(messages) => messages
                .into_iter()
                .for_each(|message| invocation.receive(message)),
            Err(error) => {
                invocation.finish(AgentInvocationTerminalStatus::Failed, Some(error.message))
            }
        }
    }

    fn on_terminal(&self, id: &AgentInvocationId, outcome: ProcessTerminalOutcome) {
        let Some(invocation) = self
            .invocations
            .lock()
            .ok()
            .and_then(|mut all| all.remove(id))
        else {
            return;
        };
        let (status, exit) = match &outcome {
            ProcessTerminalOutcome::Exited(exit) => (ProcessExitStatus::Completed, Some(exit)),
            ProcessTerminalOutcome::Failed { exit, .. } => {
                (ProcessExitStatus::Failed, exit.as_ref())
            }
            ProcessTerminalOutcome::Canceled { exit } => {
                (ProcessExitStatus::Canceled, exit.as_ref())
            }
            ProcessTerminalOutcome::Interrupted { exit } => {
                (ProcessExitStatus::Interrupted, exit.as_ref())
            }
        };
        invocation.control(RuntimeControlRecord::ProcessExit {
            status,
            exit_code: exit.and_then(|exit| exit.exit_code),
            signal: exit.and_then(|exit| exit.signal.clone()),
            after_turn_completion: invocation.is_finished(),
            evidence: Some(format!("{outcome:?}")),
        });
        let status = match outcome {
            ProcessTerminalOutcome::Interrupted { .. } => {
                AgentInvocationTerminalStatus::Interrupted
            }
            ProcessTerminalOutcome::Canceled { .. } => AgentInvocationTerminalStatus::Canceled,
            _ => AgentInvocationTerminalStatus::Failed,
        };
        invocation.finish(status, Some("Claude exited without a result".into()));
    }
}

pub struct ClaudeRuntime {
    program: String,
    coordinator: Arc<Coordinator>,
    supervisor: Arc<ProcessSupervisor>,
}

impl ClaudeRuntime {
    pub fn system(program: impl Into<String>) -> Self {
        Self::new(program, Arc::new(DuplexProcessFactory))
    }

    pub fn new(program: impl Into<String>, factory: Arc<dyn ChildProcessFactory>) -> Self {
        let coordinator = Arc::new(Coordinator::default());
        Self {
            program: program.into(),
            supervisor: Arc::new(ProcessSupervisor::new(factory, coordinator.clone())),
            coordinator,
        }
    }

    /// Starts the process and establishes its conversation. The prompt is written later.
    fn launch(
        &self,
        request: &RuntimeInvocationRequest,
        conversation: Conversation,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(Arc<Invocation>, RuntimeInvocationReady), RuntimePortError> {
        let working_directory = request.working_directory.clone().ok_or_else(|| {
            unavailable("Session has no working directory; select one before starting")
        })?;
        let extension = request.launch_extension.as_ref();
        let spec = ProcessLaunchSpec {
            remove_environment: launch::parent_session_variables(),
            program: extension
                .and_then(|extension| extension.executable.clone())
                .unwrap_or_else(|| self.program.clone()),
            args: launch::arguments(request, &conversation)?,
            working_directory: Some(PathBuf::from(&working_directory)),
            environment: launch::environment(extension),
        };
        let id = request.invocation_id.clone();
        let session_id = conversation.session_id().to_owned();
        let external_context_id = ExternalRuntimeContextId::new(&session_id)
            .map_err(|error| unavailable(error.to_string()))?;
        let invocation = Arc::new(Invocation {
            connection: Connection::new(id.clone(), self.supervisor.clone()),
            sink,
            target: RuntimeTurnTarget {
                thread_id: session_id.clone(),
                turn_id: id.to_string(),
            },
            session_id,
            events: Mutex::new(ClaudeEvents::default()),
            requests: Mutex::new(HashMap::new()),
            required_servers: launch::required_servers(request),
            prompt: Mutex::new(None),
            unanswered: AtomicUsize::new(0),
            canceled: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        });
        {
            let mut all = self
                .coordinator
                .invocations
                .lock()
                .map_err(|_| unavailable("Invocation lock poisoned"))?;
            if all.contains_key(&id) {
                return Err(RuntimePortError::new(
                    RuntimePortErrorKind::AlreadyActive,
                    "Invocation already connected",
                ));
            }
            all.insert(id.clone(), invocation.clone());
        }
        if let Err(error) = self
            .supervisor
            .start(request.session_id.clone(), id.clone(), spec)
        {
            self.coordinator
                .invocations
                .lock()
                .ok()
                .map(|mut all| all.remove(&id));
            return Err(unavailable(error.to_string()));
        }
        if let Err(error) = invocation.connection.request("initialize") {
            invocation.finish(
                AgentInvocationTerminalStatus::Failed,
                Some(error.message.clone()),
            );
            return Err(error);
        }
        invocation.emit(RuntimeUpdate::Event(RuntimeEventDraft {
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: json!({"kind": "runtime_context", "sessionId": external_context_id.as_str()}),
            normalized: Some(NormalizedRuntimeEvent {
                kind: NormalizedRuntimeEventKind::RuntimeContextEstablished,
                text: None,
                external_context_id: Some(external_context_id.clone()),
                usage: None,
                details: None,
                tool_activity: None,
            }),
        }));
        invocation.control(RuntimeControlRecord::TurnActive {
            target: invocation.target.clone(),
        });
        Ok((
            invocation,
            RuntimeInvocationReady {
                external_context_id,
                working_directory,
            },
        ))
    }

    fn start(
        &self,
        request: RuntimeInvocationRequest,
        conversation: Conversation,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        let (invocation, _) = self.launch(&request, conversation, sink)?;
        if let Err(error) = invocation.send_user_message(&request.submitted_text) {
            invocation.finish(
                AgentInvocationTerminalStatus::Failed,
                Some(error.message.clone()),
            );
            return Err(error);
        }
        Ok(())
    }
}

fn new_conversation() -> Conversation {
    Conversation::New(uuid::Uuid::new_v4().to_string())
}

impl AgentRuntime for ClaudeRuntime {
    fn prepare_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<RuntimeInvocationReady, RuntimePortError> {
        let conversation = match external {
            Some(external) => Conversation::Resume(external.as_str().into()),
            None => new_conversation(),
        };
        let (invocation, ready) = self.launch(&request, conversation, sink)?;
        *invocation
            .prompt
            .lock()
            .map_err(|_| unavailable("Prompt lock poisoned"))? = Some(request.submitted_text);
        Ok(ready)
    }

    fn deliver_prepared_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let invocation = self.coordinator.get(id)?;
        if invocation.is_finished() {
            return Err(unavailable("Prepared invocation has finished"));
        }
        let prompt = invocation
            .prompt
            .lock()
            .map_err(|_| unavailable("Prompt lock poisoned"))?
            .take()
            .ok_or_else(|| {
                unavailable("Prepared prompt is unavailable or was already delivered")
            })?;
        if let Err(error) = invocation.send_user_message(&prompt) {
            invocation.finish(
                AgentInvocationTerminalStatus::Failed,
                Some(error.message.clone()),
            );
            return Err(error);
        }
        Ok(())
    }

    fn preflight_invocation(
        &self,
        _mode: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        if options
            .model
            .as_ref()
            .is_some_and(|model| model.trim().is_empty())
        {
            return Err(unavailable("Model cannot be blank"));
        }
        if options
            .sandbox
            .is_some_and(|mode| mode != RuntimeSandboxMode::DangerFullAccess)
        {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::UnsupportedOptions,
                "Claude Code offers full access only",
            ));
        }
        Ok(RuntimeInvocationPreflight {
            effective_options: options.clone(),
        })
    }

    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.start(request, new_conversation(), sink)
    }

    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external: ExternalRuntimeContextId,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.start(
            request,
            Conversation::Resume(external.as_str().into()),
            sink,
        )
    }

    fn active_turn(&self, id: &AgentInvocationId) -> Result<RuntimeTurnTarget, RuntimePortError> {
        let invocation = self.coordinator.get(id)?;
        if invocation.is_finished() {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "No active turn; the invocation has finished",
            ));
        }
        Ok(invocation.target.clone())
    }

    fn steer(
        &self,
        id: &AgentInvocationId,
        target: &RuntimeTurnTarget,
        _input_id: &str,
        text: &str,
    ) -> Result<(), RuntimePortError> {
        if self.active_turn(id)? != *target {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "The targeted turn has finished",
            ));
        }
        self.coordinator.get(id)?.send_user_message(text)
    }

    fn respond(
        &self,
        id: &AgentInvocationId,
        request_id: &str,
        response: crate::contracts::RuntimeInteractionResponse,
    ) -> Result<(), RuntimePortError> {
        requests::respond(self.coordinator.get(id)?.as_ref(), request_id, response)
    }

    fn cancel_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let invocation = self.coordinator.get(id)?;
        invocation.canceled.store(true, Ordering::Release);
        if invocation.connection.request("interrupt").is_err() {
            return self
                .supervisor
                .cancel(id)
                .map_err(|e| unavailable(e.to_string()));
        }
        let supervisor = self.supervisor.clone();
        let id = id.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(2));
            if !invocation.is_finished() {
                let _ = supervisor.cancel(&id);
            }
        });
        Ok(())
    }

    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.supervisor
            .shutdown()
            .map_err(|e| unavailable(e.to_string()))
    }
}
