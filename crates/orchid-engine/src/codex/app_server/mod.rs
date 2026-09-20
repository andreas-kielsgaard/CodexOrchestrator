//! Invocation-scoped Codex app-server adapter. Process ownership remains in the supervisor.
mod approval_choices;
mod client;
pub mod configuration;
mod connection;
pub mod continuation;
pub mod environment;
pub mod history;
mod inventory;
pub mod items;
mod notifications;
mod process_context;
mod requests;

use super::protocol::CodexJsonlProtocol;
use crate::{
    contracts::{domain::*, ports::*},
    processes::*,
};
use connection::{unavailable, Connection};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    thread,
    time::Duration,
};

struct Invocation {
    connection: Connection,
    sink: Arc<dyn AgentRuntimeUpdateSink>,
    protocol: Mutex<CodexJsonlProtocol>,
    target: Mutex<Option<RuntimeTurnTarget>>,
    requests: Mutex<HashMap<String, requests::PendingRequest>>,
    finished: AtomicBool,
    prepared_turn: Mutex<Option<Value>>,
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

    fn event(&self, payload: Value) {
        self.emit(RuntimeUpdate::Event(RuntimeEventDraft {
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: payload,
            normalized: None,
        }));
    }

    fn finish(&self, status: AgentInvocationTerminalStatus, error: Option<RuntimePortError>) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        self.connection.finish_turn();
        if let Ok(mut target) = self.target.lock() {
            *target = None;
        }
        self.emit(RuntimeUpdate::Finished(RuntimeInvocationOutcome {
            status,
            exit_code: None,
            signal: None,
            runtime_error: error.map(|e| AgentRuntimeFailure {
                code: "codex_app_server".into(),
                message: e.message,
                details: e.details,
            }),
        }));
    }

    fn notification(&self, message: Value) -> Result<(), RuntimePortError> {
        if self.finished.load(Ordering::Acquire) {
            return Ok(());
        }
        if message.get("id").is_some() {
            return requests::receive(self, message);
        }
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        // Native default resolution can create an ephemeral thread. Only the explicit start/
        // resume response below establishes this application's durable continuation identity.
        if method == "thread/started" {
            return Ok(());
        }
        if method == "turn/started" {
            let target = RuntimeTurnTarget {
                thread_id: params["threadId"].as_str().unwrap_or("").into(),
                turn_id: params["turn"]["id"].as_str().unwrap_or("").into(),
            };
            *self
                .target
                .lock()
                .map_err(|_| unavailable("Turn lock poisoned"))? = Some(target.clone());
            self.event(json!({"kind":"runtime_turn_active","target":target}));
        }
        if let Some(mut raw) = notifications::legacy_event(method, params) {
            // Attach native evidence before normalization. The shared normalizer can defer an
            // agent message until the next item/terminal event; its evidence must travel with it.
            raw["native"] = message.clone();
            let output = self
                .protocol
                .lock()
                .map_err(|_| unavailable("Protocol lock poisoned"))?
                .normalize(raw);
            for event in output.events {
                self.emit(RuntimeUpdate::Event(event));
            }
        } else {
            self.event(message.clone());
        }
        if method == "turn/completed" {
            let status = match params["turn"]["status"].as_str() {
                Some("completed") => AgentInvocationTerminalStatus::Completed,
                Some("interrupted") => AgentInvocationTerminalStatus::Canceled,
                _ => AgentInvocationTerminalStatus::Failed,
            };
            let error = (status == AgentInvocationTerminalStatus::Failed).then(|| {
                unavailable(
                    params["turn"]["error"]["message"]
                        .as_str()
                        .unwrap_or("Codex turn failed"),
                )
            });
            self.finish(status, error);
        }
        Ok(())
    }
}

#[derive(Default)]
struct Coordinator {
    invocations: Mutex<HashMap<AgentInvocationId, Arc<Invocation>>>,
    supervisor: Mutex<Weak<ProcessSupervisor>>,
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
                raw_payload: json!({"text":String::from_utf8_lossy(&output.bytes)}),
                normalized: None,
            }));
            return;
        }
        let result = invocation
            .connection
            .read(&output.bytes)
            .and_then(|messages| {
                for message in messages {
                    invocation.notification(message)?;
                }
                Ok(())
            });
        if let Err(error) = result {
            invocation.finish(AgentInvocationTerminalStatus::Failed, Some(error));
        }
    }

    fn on_terminal(&self, id: &AgentInvocationId, outcome: ProcessTerminalOutcome) {
        let invocation = self
            .invocations
            .lock()
            .ok()
            .and_then(|mut all| all.remove(id));
        let Some(invocation) = invocation else {
            return;
        };
        invocation.connection.disconnected();
        let (classification, exit) = match &outcome {
            ProcessTerminalOutcome::Exited(exit) => ("completed", Some(exit)),
            ProcessTerminalOutcome::Failed { exit, .. } => ("failed", exit.as_ref()),
            ProcessTerminalOutcome::Canceled { exit } => ("canceled", exit.as_ref()),
            ProcessTerminalOutcome::Interrupted { exit } => ("interrupted", exit.as_ref()),
        };
        invocation.event(json!({"kind":"runtime_process_exit","status":classification,"exitCode":exit.and_then(|e| e.exit_code),"signal":exit.and_then(|e| e.signal.as_deref()),"afterTurnCompletion":invocation.finished.load(Ordering::Acquire),"evidence":format!("{outcome:?}")}));
        let status = match outcome {
            ProcessTerminalOutcome::Interrupted { .. } => {
                AgentInvocationTerminalStatus::Interrupted
            }
            ProcessTerminalOutcome::Canceled { .. } => AgentInvocationTerminalStatus::Canceled,
            _ => AgentInvocationTerminalStatus::Failed,
        };
        invocation.finish(
            status,
            Some(unavailable(
                "App-server disconnected without turn completion",
            )),
        );
    }
}

pub struct CodexAppServerRuntime {
    program: Result<String, String>,
    coordinator: Arc<Coordinator>,
    supervisor: Arc<ProcessSupervisor>,
}

impl CodexAppServerRuntime {
    pub fn system(program: impl Into<String>) -> Self {
        Self::new(program, Arc::new(DuplexProcessFactory))
    }

    pub fn new(program: impl Into<String>, factory: Arc<dyn ChildProcessFactory>) -> Self {
        let coordinator = Arc::new(Coordinator::default());
        let supervisor = Arc::new(ProcessSupervisor::new(factory, coordinator.clone()));
        *coordinator.supervisor.lock().expect("new mutex") = Arc::downgrade(&supervisor);
        Self {
            program: super::resolve_program(program.into()),
            coordinator,
            supervisor,
        }
    }

    fn launch(
        &self,
        request: RuntimeInvocationRequest,
        external: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
        preparation: Option<
            std::sync::mpsc::SyncSender<Result<RuntimeInvocationReady, RuntimePortError>>,
        >,
    ) -> Result<(), RuntimePortError> {
        let program = self.program.clone().map_err(unavailable)?;
        let environment = request
            .launch_extension
            .as_ref()
            .map(|e| e.environment.clone())
            .unwrap_or_default();
        let cwd = request
            .working_directory
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| {
                environment
                    .iter()
                    .find(|(key, _)| key == "CODEX_HOME")
                    .map(|(_, value)| PathBuf::from(value))
            });
        if cwd.is_none() {
            return Err(unavailable(
                "Session has no working context; select an explicit directory before continuing",
            ));
        }
        let args = configuration::arguments(
            request.launch_extension.as_ref(),
            cwd.as_deref().expect("working context checked"),
        )?;
        let spec = ProcessLaunchSpec {
            remove_environment: process_context::parent_session_variables(),
            program,
            args,
            working_directory: cwd,
            environment,
        };
        let id = request.invocation_id.clone();
        let invocation = Arc::new(Invocation {
            connection: Connection::new(id.clone(), self.supervisor.clone()),
            sink,
            protocol: Mutex::new(CodexJsonlProtocol::default()),
            target: Mutex::new(None),
            requests: Mutex::new(HashMap::new()),
            finished: AtomicBool::new(false),
            prepared_turn: Mutex::new(None),
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
        let supervisor = self.supervisor.clone();
        invocation.event(json!({"kind":"runtime_transport","transport":"codex_app_server"}));
        let cleanup_id = id.clone();
        let cleanup_invocation = invocation.clone();
        thread::Builder::new()
            .name(format!("codex-initialize-{id}"))
            .spawn(move || {
                let initialized = initialize_turn(&invocation, request, external);
                match (initialized, preparation) {
                    (Ok((ready, turn)), Some(sender)) => {
                        if invocation.finished.load(Ordering::Acquire) {
                            let _ =
                                sender.send(Err(unavailable("Native preparation was canceled")));
                            return;
                        }
                        *invocation.prepared_turn.lock().expect("prepared turn") = Some(turn);
                        if sender.send(Ok(ready)).is_err() {
                            invocation.finish(AgentInvocationTerminalStatus::Canceled, None);
                        }
                    }
                    (Ok((_, turn)), None) => {
                        if let Err(error) = invocation.connection.call("turn/start", turn) {
                            invocation.finish(AgentInvocationTerminalStatus::Failed, Some(error));
                        }
                    }
                    (Err(error), sender) => {
                        if let Some(sender) = sender {
                            let _ = sender.send(Err(error.clone()));
                        }
                        invocation.finish(AgentInvocationTerminalStatus::Failed, Some(error));
                    }
                }
                // EOF normally closes app-server. Retain ownership and bound cleanup if it does not.
                while !invocation.finished.load(Ordering::Acquire) {
                    thread::sleep(Duration::from_millis(100));
                }
                thread::sleep(Duration::from_secs(2));
                if supervisor.is_active(&id).unwrap_or(false) {
                    let _ = supervisor.cancel(&id);
                }
            })
            .map_err(|error| {
                let failure = unavailable(format!(
                    "Unable to start app-server initialization: {error}"
                ));
                cleanup_invocation
                    .finish(AgentInvocationTerminalStatus::Failed, Some(failure.clone()));
                let _ = self.supervisor.cancel(&cleanup_id);
                failure
            })?;
        Ok(())
    }
}

fn initialize_turn(
    invocation: &Invocation,
    mut request: RuntimeInvocationRequest,
    external: Option<ExternalRuntimeContextId>,
) -> Result<(RuntimeInvocationReady, Value), RuntimePortError> {
    let rpc = &invocation.connection;
    rpc.call("initialize", json!({"clientInfo":{"name":"codex_orchestrator","version":"1"},"capabilities":{"experimentalApi":true}}))?;
    rpc.write(json!({"method":"initialized","params":{}}))?;
    if request.working_directory.is_none() {
        let external = external.as_ref().ok_or_else(|| {
            unavailable("Session has no working directory; select one before starting")
        })?;
        let metadata = rpc.call(
            "thread/read",
            json!({"threadId":external.as_str(),"includeTurns":false}),
        )?;
        let cwd = metadata["thread"]["cwd"].as_str().filter(|cwd| PathBuf::from(cwd).is_absolute() && PathBuf::from(cwd).is_dir()).ok_or_else(|| unavailable("Historical working context could not be recovered. Select an explicit working directory before continuing"))?;
        invocation.sink.emit_update(&request.invocation_id, RuntimeUpdate::Event(RuntimeEventDraft { source: AgentRuntimeEventSource::Runtime, raw_payload: json!({"kind":"runtime_working_directory_resolved","cwd":cwd,"threadId":external.as_str()}), normalized: None }))?;
        request.working_directory = Some(cwd.into());
    }
    let mut params = json!({});
    params["config"] = configuration::thread_configuration(
        rpc,
        request.launch_extension.as_ref(),
        request.working_directory.as_deref(),
    )?;
    if let Some(effort) = request
        .launch_extension
        .as_ref()
        .and_then(|e| e.reasoning_mode.as_ref())
    {
        params["config"]["model_reasoning_effort"] = effort.clone().into();
    }
    if let Some(cwd) = &request.working_directory {
        params["cwd"] = cwd.clone().into();
    }
    if let Some(model) = &request.options.model {
        params["model"] = model.clone().into();
    }
    if let Some(sandbox) = request.options.sandbox {
        params["sandbox"] = match sandbox {
            RuntimeSandboxMode::ReadOnly => "read-only",
            RuntimeSandboxMode::WorkspaceWrite => "workspace-write",
            RuntimeSandboxMode::DangerFullAccess => "danger-full-access",
        }
        .into();
    }
    let expected_external = external.clone();
    let method = if let Some(external) = external {
        // CLI 0.144 retains model/effort from history unless resume supplies current values.
        // Ask Codex to resolve a fresh, ephemeral thread with this exact invocation's config.
        let mut fresh_params = params.clone();
        fresh_params["ephemeral"] = true.into();
        let fresh = rpc.call("thread/start", fresh_params)?;
        if !fresh["model"].is_null() {
            params["model"] = fresh["model"].clone();
        }
        params["approvalPolicy"] = fresh["approvalPolicy"].clone();
        configuration::apply_resume_reasoning(&mut params["config"], &fresh["reasoningEffort"]);
        if request.options.sandbox.is_none() {
            if let Some(mode) = match fresh["sandbox"]["type"].as_str() {
                Some("readOnly") => Some("read-only"),
                Some("workspaceWrite") => Some("workspace-write"),
                Some("dangerFullAccess") => Some("danger-full-access"),
                _ => None,
            } {
                params["sandbox"] = mode.into();
            }
        }
        params["threadId"] = external.as_str().into();
        "thread/resume"
    } else {
        "thread/start"
    };
    let result = rpc.call(method, params)?;
    let thread_id = result["thread"]["id"]
        .as_str()
        .ok_or_else(|| unavailable("App-server did not return a thread identity"))?;
    if expected_external
        .as_ref()
        .is_some_and(|expected| expected.as_str() != thread_id)
    {
        return Err(unavailable(
            "Native resume returned a different conversation identity",
        ));
    }
    if let (Some(requested), Some(actual)) =
        (request.working_directory.as_deref(), result["cwd"].as_str())
    {
        let requested =
            std::fs::canonicalize(requested).unwrap_or_else(|_| PathBuf::from(requested));
        let actual = std::fs::canonicalize(actual).unwrap_or_else(|_| PathBuf::from(actual));
        if requested != actual {
            return Err(unavailable(
                "Native resume did not select the requested working directory",
            ));
        }
    }
    // Resume does not necessarily emit thread/started. Always establish durable continuation identity.
    let context = invocation
        .protocol
        .lock()
        .map_err(|_| unavailable("Protocol lock poisoned"))?
        .normalize(json!({"type":"thread.started","thread_id":thread_id}));
    for event in context.events {
        invocation.emit(RuntimeUpdate::Event(event));
    }
    invocation.event(json!({"kind":"runtime_effective_configuration","model":result["model"],"reasoningEffort":result["reasoningEffort"],"cwd":result["cwd"],"approvalPolicy":result["approvalPolicy"],"sandbox":result["sandbox"]}));
    configuration::validate_effective_sandbox(request.options.sandbox, &result["sandbox"])?;
    let mut input = Vec::new();
    if let Some(extension) = request.launch_extension.as_ref() {
        for skill in &extension.skill_inputs {
            let path = PathBuf::from(&skill.path);
            if skill.id.trim().is_empty()
                || skill.name.trim().is_empty()
                || !path.is_absolute()
                || path.file_name().is_none_or(|name| name != "SKILL.md")
            {
                return Err(unavailable("Invalid selected skill input"));
            }
            input.push(json!({"type":"skill","name":skill.name,"path":skill.path}));
        }
    }
    input.push(json!({"type":"text","text":request.submitted_text}));
    let mut turn = json!({"threadId":thread_id,"input":input});
    if let Some(effort) = request
        .launch_extension
        .as_ref()
        .and_then(|e| e.reasoning_mode.as_ref())
    {
        turn["effort"] = effort.clone().into();
    }
    let working_directory = result["cwd"]
        .as_str()
        .or(request.working_directory.as_deref())
        .ok_or_else(|| unavailable("App-server did not return a working directory"))?
        .to_owned();
    Ok((
        RuntimeInvocationReady {
            external_context_id: ExternalRuntimeContextId::new(thread_id)
                .map_err(|error| unavailable(error.to_string()))?,
            working_directory,
        },
        turn,
    ))
}

impl AgentRuntime for CodexAppServerRuntime {
    fn prepare_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<RuntimeInvocationReady, RuntimePortError> {
        let id = request.invocation_id.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.launch(request, external, sink, Some(sender))?;
        match receiver.recv_timeout(Duration::from_secs(120)) {
            Ok(result) => result,
            Err(error) => {
                let _ = self.cancel_invocation(&id);
                Err(unavailable(format!(
                    "Native preparation did not finish: {error}"
                )))
            }
        }
    }
    fn deliver_prepared_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let invocation = self.coordinator.get(id)?;
        if invocation.finished.load(Ordering::Acquire) {
            return Err(unavailable("Prepared invocation has finished"));
        }
        let turn = invocation
            .prepared_turn
            .lock()
            .map_err(|_| unavailable("Prepared turn lock poisoned"))?
            .take()
            .ok_or_else(|| {
                unavailable("Prepared prompt is unavailable or was already delivered")
            })?;
        if let Err(error) = invocation.connection.call("turn/start", turn) {
            invocation.finish(AgentInvocationTerminalStatus::Failed, Some(error.clone()));
            return Err(error);
        }
        Ok(())
    }

    fn preflight_invocation(
        &self,
        _mode: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        self.program.as_ref().map_err(|e| unavailable(e.clone()))?;
        if options.model.as_ref().is_some_and(|m| m.trim().is_empty()) {
            return Err(unavailable("Model cannot be blank"));
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
        self.launch(request, None, sink, None)
    }
    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external: ExternalRuntimeContextId,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.launch(request, Some(external), sink, None)
    }
    fn active_turn(&self, id: &AgentInvocationId) -> Result<RuntimeTurnTarget, RuntimePortError> {
        self.coordinator
            .get(id)?
            .target
            .lock()
            .map_err(|_| unavailable("Turn lock poisoned"))?
            .clone()
            .ok_or_else(|| {
                RuntimePortError::new(
                    RuntimePortErrorKind::NotActive,
                    "No active turn; wait for the turn to start",
                )
            })
    }
    fn steer(
        &self,
        id: &AgentInvocationId,
        target: &RuntimeTurnTarget,
        input_id: &str,
        text: &str,
    ) -> Result<(), RuntimePortError> {
        if self.active_turn(id)? != *target {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "The targeted turn has finished",
            ));
        }
        let result = self.coordinator.get(id)?.connection.call("turn/steer", json!({"threadId":target.thread_id,"expectedTurnId":target.turn_id,"clientUserMessageId":input_id,"input":[{"type":"text","text":text}]}))?;
        if result["turnId"] != target.turn_id {
            return Err(unavailable(
                "Steering acknowledgement named a different turn; delivery is uncertain",
            ));
        }
        Ok(())
    }
    fn respond(
        &self,
        id: &AgentInvocationId,
        request_id: &str,
        response: Value,
    ) -> Result<(), RuntimePortError> {
        requests::respond(self.coordinator.get(id)?.as_ref(), request_id, response)
    }
    fn cancel_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let invocation = self.coordinator.get(id)?;
        if let Ok(target) = self.active_turn(id) {
            if let Err(error) = invocation.connection.call(
                "turn/interrupt",
                json!({"threadId":target.thread_id,"turnId":target.turn_id}),
            ) {
                return if self.supervisor.is_active(id).unwrap_or(false) {
                    self.supervisor
                        .cancel(id)
                        .map_err(|e| unavailable(e.to_string()))
                } else {
                    Err(error)
                };
            }
            let supervisor = self.supervisor.clone();
            let id = id.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(2));
                if !invocation.finished.load(Ordering::Acquire) {
                    let _ = supervisor.cancel(&id);
                }
            });
            Ok(())
        } else {
            self.supervisor
                .cancel(id)
                .map_err(|e| unavailable(e.to_string()))
        }
    }
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.supervisor
            .shutdown()
            .map_err(|e| unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod preparation_tests;
