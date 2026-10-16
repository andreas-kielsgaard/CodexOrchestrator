//! Addressed application commands. The router never owns preparation or provider processes.
use super::ports::AgentSessionRepository;
use super::{
    application::*,
    domain::{AgentInvocationId, AgentSessionId},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostCommand {
    pub method: String,
    pub input: Value,
}

pub trait DeviceCommandTransport: Send + Sync {
    fn execute(&self, device_id: &str, command: HostCommand) -> Result<Value, String>;
    fn devices(&self) -> Result<Vec<(crate::synchronization::DeviceDescription, bool)>, String> {
        Ok(Vec::new())
    }
}
pub struct AgentSessionCommandClient {
    pub local_id: String,
    pub application: Arc<AgentSessionApplication>,
    pub repository: Arc<super::repository::SqliteAgentSessionRepository>,
    pub replicas: crate::synchronization::repository::ReplicaStore,
    pub remote: Option<Arc<dyn DeviceCommandTransport>>,
}
impl AgentSessionCommandClient {
    pub fn continuation(&self, id: &AgentSessionId) -> Result<Value, String> {
        let local = self
            .repository
            .get_session(id)
            .map_err(|e| e.to_string())?
            .is_some()
            && self
                .repository
                .owns_execution(id)
                .map_err(|e| e.to_string())?;
        let history = self
            .application
            .queries()
            .with_replicas(self.replicas.clone())
            .load_session(id)
            .map_err(|e| e.to_string())?;
        let owner = if local {
            self.local_id.clone()
        } else {
            self.repository
                .execution_owner(id)?
                .or(self.replicas.owner(id.as_str())?)
                .ok_or("Session not found")?
        };
        let online = local
            || self
                .remote
                .as_ref()
                .and_then(|r| r.devices().ok())
                .is_some_and(|devices| {
                    devices
                        .into_iter()
                        .any(|(d, online)| d.id == owner && online)
                });
        let interrupted = history.invocations.last().is_some_and(|turn| {
            turn.invocation.status == super::domain::AgentInvocationStatus::Interrupted
        });
        let fork = self
            .repository
            .conversation_fork(id)?
            .or(self.replicas.get(id.as_str())?.and_then(|r| r.session.fork));
        Ok(
            json!({"deviceId":owner,"online":online,"fork":fork,"requiresFork":!online||interrupted,"reason":if !online {Some("Execution device is unreachable. Fork this conversation and choose a new target.")}else if interrupted {Some("The last turn was interrupted. Fork this conversation to continue.")}else{None}}),
        )
    }
    pub fn execute(&self, device_id: &str, command: HostCommand) -> Result<Value, String> {
        if device_id == "local" || device_id == self.local_id {
            dispatch_on_device(&self.local_id, &self.application, &self.repository, command)
        } else {
            let remote = self
                .remote
                .as_ref()
                .ok_or("No synchronization server is configured")?;
            let requested_session = command
                .input
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let result = remote.execute(device_id, command)?;
            let session = result
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or(requested_session)
                .or_else(|| result.get("id").and_then(Value::as_str).map(str::to_owned));
            if let Some(session) = session {
                // Refresh after acceptance, but a missing refresh must not hide an accepted command.
                if let Ok(snapshot) = remote.execute(
                    device_id,
                    HostCommand {
                        method: "snapshot".into(),
                        input: json!({"sessionId":session}),
                    },
                ) {
                    if let Ok(change) = serde_json::from_value(snapshot) {
                        let _ = self.replicas.accept(device_id, &change);
                    }
                }
            }
            Ok(result)
        }
    }
    pub fn execute_addressed(&self, command: HostCommand) -> Result<Value, String> {
        if let Some(id) = command.input.get("sessionId").and_then(Value::as_str) {
            if let Some(replica) = self.replicas.get(id)? {
                match command.method.as_str() {
                    "preparation" => {
                        return serde_json::to_value(replica.session.preparations.last())
                            .map_err(|e| e.to_string())
                    }
                    "interactions" => {
                        return serde_json::to_value(super::interactions::project_interactions(
                            &replica.session.history,
                        ))
                        .map_err(|e| e.to_string())
                    }
                    "queue" => {
                        return serde_json::to_value(replica.session.queue_view())
                            .map_err(|e| e.to_string())
                    }
                    _ => {}
                }
            }
        }
        let owner = if let Some(id) = command.input.get("invocationId").and_then(Value::as_str) {
            let id = AgentInvocationId::new(id).map_err(|e| e.to_string())?;
            if let Some(invocation) = self
                .repository
                .get_invocation(&id)
                .map_err(|e| e.to_string())?
            {
                self.repository
                    .execution_owner(&invocation.session_id)?
                    .unwrap_or_else(|| self.local_id.clone())
            } else {
                self.replicas
                    .list()?
                    .into_iter()
                    .find(|r| {
                        r.session
                            .history
                            .invocations
                            .iter()
                            .any(|t| t.invocation.id == id)
                    })
                    .map(|r| r.device_id)
                    .ok_or("Invocation owner is unknown; synchronize its history first")?
            }
        } else if let Some(id) = command.input.get("sessionId").and_then(Value::as_str) {
            if self
                .repository
                .get_session(&AgentSessionId::new(id).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                self.repository
                    .execution_owner(&AgentSessionId::new(id).map_err(|e| e.to_string())?)?
                    .unwrap_or_else(|| self.local_id.clone())
            } else {
                self.replicas
                    .owner(id)?
                    .ok_or("Session owner is unknown; synchronize its history first")?
            }
        } else {
            command
                .input
                .pointer("/executionSelection/execution/deviceId")
                .or_else(|| command.input.pointer("/target/execution/deviceId"))
                .and_then(Value::as_str)
                .unwrap_or(&self.local_id)
                .to_owned()
        };
        self.execute(&owner, command)
    }
}

fn field<T: serde::de::DeserializeOwned>(input: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(
        input
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Missing {name}"))?,
    )
    .map_err(|e| e.to_string())
}
fn encode<T: Serialize>(value: Result<T, AgentSessionApplicationError>) -> Result<Value, String> {
    serde_json::to_value(value.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub fn dispatch_on_device(
    device_id: &str,
    app: &AgentSessionApplication,
    repository: &super::repository::SqliteAgentSessionRepository,
    mut command: HostCommand,
) -> Result<Value, String> {
    if command.input.get("sessionId").is_none_or(Value::is_null) {
        if let Some(id) = command.input.get("invocationId").and_then(Value::as_str) {
            let id = AgentInvocationId::new(id).map_err(|e| e.to_string())?;
            if let Some(invocation) = repository.get_invocation(&id).map_err(|e| e.to_string())? {
                command.input["sessionId"] = json!(invocation.session_id);
            }
        }
    }
    if command.method == "sessionEvent" {
        use crate::session_events::SessionInvocationDispatcher;
        let adapter = super::session_event_adapter::AgentSessionEventAdapter::new(
            Arc::new(app.clone()),
            Arc::new(
                super::repository::SqliteAgentSessionRepository::from_database(
                    repository.database(),
                ),
            ),
            crate::identities::service::IdentityService::from_database(repository.database()),
        );
        return serde_json::to_value(
            adapter
                .dispatch(serde_json::from_value(command.input).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string());
    }
    if command.method == "prepareHandoff" {
        let target = field::<crate::execution_targets::domain::SessionExecutionTarget>(
            &command.input,
            "target",
        )?;
        if target.execution.device_id != device_id || !std::path::Path::new(&target.path).is_dir() {
            return Err("Handoff destination is unavailable".into());
        }
        verify_destination(app, device_id, target)?;
        return Ok(Value::Null);
    }
    if command.method == "acceptHandoff" {
        let change: crate::synchronization::SessionChange =
            serde_json::from_value(command.input).map_err(|e| e.to_string())?;
        if !repository.handoff_accepted(&change)? {
            verify_destination(
                app,
                device_id,
                change
                    .session
                    .history
                    .session
                    .execution_target
                    .clone()
                    .ok_or("Missing handoff target")?,
            )?;
        }
        repository.accept_handoff(device_id, &change)?;
        app.wake_transferred_session(change.session.history.session.id);
        return Ok(Value::Null);
    }
    if let Some(id) = command.input.get("sessionId").and_then(Value::as_str) {
        if let Some(invocation) = command.input.get("invocationId").and_then(Value::as_str) {
            let invocation = AgentInvocationId::new(invocation).map_err(|e| e.to_string())?;
            if repository
                .get_invocation(&invocation)
                .map_err(|e| e.to_string())?
                .is_some_and(|turn| turn.session_id.as_str() != id)
            {
                return Err("Invocation belongs to another session".into());
            }
        }
        if !repository
            .owns_execution(&AgentSessionId::new(id).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
            && !matches!(
                command.method.as_str(),
                "history" | "snapshot" | "queue" | "preparation" | "interactions"
            )
        {
            return Err(
                "Session execution has moved to another device; synchronize before retrying".into(),
            );
        }
    }
    if command.method == "refreshSkills" {
        app.refresh_skill_catalogue()?;
        return Ok(Value::Null);
    }
    if command.method.starts_with("device.") {
        return crate::devices::operations::dispatch(
            device_id,
            repository.database(),
            app.execution_targets()?,
            command,
        );
    }
    if command.method == "editDefinition" {
        return serde_json::to_value(
            repository.definitions().edit_on_master(
                &serde_json::from_value(command.input).map_err(|e| e.to_string())?,
            )?,
        )
        .map_err(|e| e.to_string());
    }
    if command.method == "snapshot" {
        return serde_json::to_value(
            repository
                .export_current_session(&field(&command.input, "sessionId")?, device_id.into())?,
        )
        .map_err(|e| e.to_string());
    }
    if command.method == "fork" {
        let source = field::<super::ports::AgentSessionHistory>(&command.input, "history")?;
        let mut target = field::<crate::execution_targets::domain::SessionExecutionTarget>(
            &command.input,
            "target",
        )?;
        if target.execution.device_id != "local" && target.execution.device_id != device_id {
            return Err("Fork target does not belong to this device".into());
        }
        let fork_id = field(&command.input, "forkId")?;
        if repository
            .get_session(&fork_id)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            target = verify_destination(app, device_id, target)?;
        }
        target.execution.connection = crate::execution_targets::domain::ExecutionConnection::Local;
        target.execution.device_id = "local".into();
        let prefix = serde_json::from_value(
            command
                .input
                .get("initialPromptPrefix")
                .cloned()
                .unwrap_or(Value::Null),
        )
        .map_err(|e| e.to_string())?;
        return serde_json::to_value(
            repository.fork_conversation_with_context(fork_id, &source, target, prefix)?,
        )
        .map_err(|e| e.to_string());
    }
    if matches!(command.method.as_str(), "send" | "sendProduct") {
        if let Some(execution) = command.input.pointer_mut("/executionSelection/execution") {
            let selected = execution
                .get("deviceId")
                .and_then(Value::as_str)
                .ok_or("Missing execution device")?;
            if selected != "local" && selected != device_id {
                return Err("Submission targets another device".into());
            }
            execution["deviceId"] = json!("local");
            execution["connection"] = json!({"kind":"local"});
        }
        if let Some(execution) = command
            .input
            .pointer_mut("/executionSelection/workspace/target/execution")
        {
            let selected = execution
                .get("deviceId")
                .and_then(Value::as_str)
                .ok_or("Missing worktree device")?;
            if selected != "local" && selected != device_id {
                return Err("Worktree belongs to another device".into());
            }
            execution["deviceId"] = json!("local");
            execution["connection"] = json!({"kind":"local"});
        }
        let mut input: super::application::preparation::PreparedMessageInput =
            serde_json::from_value(command.input).map_err(|e| e.to_string())?;
        if input.session_id.is_none()
            && input.working_directory.is_none()
            && input.execution_selection.as_ref().is_none_or(|s| {
                matches!(
                    s.workspace,
                    crate::execution_targets::domain::SessionWorkspaceSelection::Auxiliary
                )
            })
        {
            if let Some(folder) = &input.folder_target {
                input.working_directory = Some(repository.working_directory_for_folder(folder)?);
            }
        }
        command.input = serde_json::to_value(input).map_err(|e| e.to_string())?;
    }
    dispatch(app, command)
}
fn verify_destination(
    app: &AgentSessionApplication,
    device: &str,
    mut target: crate::execution_targets::domain::SessionExecutionTarget,
) -> Result<crate::execution_targets::domain::SessionExecutionTarget, String> {
    use crate::execution_targets::domain::*;
    if target.execution.device_id != "local" && target.execution.device_id != device {
        return Err("Destination belongs to another device".into());
    }
    target.execution.device_id = "local".into();
    target.execution.connection = ExecutionConnection::Local;
    let selection = SessionExecutionSelection {
        capability_profile_id: target.capability_profile_id.clone(),
        capability_profile_revision: target.capability_profile_revision,
        execution: target.execution.clone(),
        workspace: SessionWorkspaceSelection::Existing { target },
    };
    app.execution_targets()?
        .materialize_ready_selection(&selection, "verify-existing-target")
}
pub fn dispatch(app: &AgentSessionApplication, command: HostCommand) -> Result<Value, String> {
    let input = command.input;
    let session = || field::<AgentSessionId>(&input, "sessionId");
    let invocation = || field::<AgentInvocationId>(&input, "invocationId");
    match command.method.as_str() {
        "productContent" => encode(app.deliver_product_content_with_id(
            session()?,
            field(&input, "submissionId")?,
            field(&input, "provenance")?,
            field(&input, "content")?,
        )),
        "productInput" => encode(app.deliver_or_steer_product(
            session()?,
            field(&input, "inputId")?,
            field(&input, "content")?,
        )),
        "send" | "sendProduct" => {
            let input = serde_json::from_value(input).map_err(|e| e.to_string())?;
            let result = if command.method == "sendProduct" {
                app.accept_prepared_product(input)
            } else {
                app.accept_prepared_message(input)
            }
            .map_err(|e| e.to_string())?;
            Ok(json!({"sessionId":result.session_id,"invocationId":result.invocation_id}))
        }
        "cancel" => encode(app.cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: invocation()?,
        })),
        "steer" => {
            encode(app.steer_session(serde_json::from_value(input).map_err(|e| e.to_string())?))
        }
        "editSteering" => {
            encode(app.edit_steering(serde_json::from_value(input).map_err(|e| e.to_string())?))
        }
        "respond" => {
            encode(app.respond_to_runtime_request(
                serde_json::from_value(input).map_err(|e| e.to_string())?,
            ))
        }
        "queue" => encode(app.load_session_queue_view(&session()?)),
        "preparation" => encode(app.load_preparation(&session()?)),
        "retryPreparation" => encode(app.retry_preparation(&invocation()?)),
        "cancelPreparation" => encode(app.cancel_preparation(&invocation()?)),
        "rename" => encode(app.rename_session(RenameAgentSessionCommand {
            session_id: session()?,
            title: field(&input, "title")?,
        })),
        "updateHarness" => encode(
            app.update_session_harness(
                serde_json::from_value::<super::api::UpdateAgentSessionHarnessCommandDto>(input)
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
        ),
        "updateIdentity" => encode(
            app.update_session_identity(
                serde_json::from_value::<super::api::UpdateAgentSessionIdentityCommandDto>(input)
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
        ),
        "updateModelOverride" => encode(
            app.update_session_model_override(
                serde_json::from_value::<super::api::UpdateAgentSessionModelOverrideCommandDto>(
                    input,
                )
                .map_err(|e| e.to_string())?
                .into(),
            ),
        ),
        "interactions" => encode(app.session_interactions(&session()?)),
        "history" => encode(app.load_session(&session()?)),
        "resolveWorkingDirectory" => {
            encode(app.resolve_working_directory(&session()?, field(&input, "directory")?))
        }
        "retryQueueEntry" => {
            encode(app.retry_queue_entry(&session()?, &field::<String>(&input, "entryId")?))
        }
        "cancelQueueEntry" => {
            encode(app.cancel_queue_entry(&session()?, &field::<String>(&input, "entryId")?))
        }
        "reorderQueue" => {
            let id = session()?;
            app.reorder_session_queue(
                &id,
                field(&input, "expectedRevision")?,
                &field::<Vec<String>>(&input, "entryIds")?,
            )
            .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "updateQueuedMessage" => {
            let id = session()?;
            app.update_queued_message(serde_json::from_value(input).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "updateTargetOperation" => {
            let id = session()?;
            app.update_target_operation(serde_json::from_value(input).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "enqueueCompaction" => {
            let id = session()?;
            app.enqueue_session_compaction_with_id(&id, field(&input, "submissionId")?)
                .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "runTargetOperation" => {
            let id = session()?;
            app.run_target_operation_now(&id, &field::<String>(&input, "entryId")?)
                .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "enqueueTargetOperation" => {
            let id = session()?;
            app.enqueue_session_target_operation_with_id(
                &id,
                field(&input, "operationId")?,
                field(&input, "request")?,
                field::<Option<String>>(&input, "beforeEntryId")?.as_deref(),
            )
            .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "recalculateTargetProjection" => {
            let id = session()?;
            app.recalculate_target_operation_projection(
                &id,
                &field::<String>(&input, "entryId")?,
                field(&input, "expectedRevision")?,
            )
            .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        "sendQueueEntryNow" => {
            let id = session()?;
            let entry = field::<String>(&input, "entryId")?;
            let revision = field(&input, "expectedRevision")?;
            let edits = input
                .get("message")
                .filter(|v| !v.is_null())
                .map(|message| {
                    Ok::<_, String>(queue::UpdateQueuedMessageInput {
                        session_id: id.clone(),
                        entry_id: entry.clone(),
                        expected_revision: revision,
                        submitted_text: field(message, "submittedText")?,
                        model: message
                            .get("model")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        reasoning_mode: message
                            .get("reasoningMode")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        sandbox_mode: serde_json::from_value(
                            message.get("sandboxMode").cloned().unwrap_or(Value::Null),
                        )
                        .map_err(|e| e.to_string())?,
                    })
                })
                .transpose()?;
            app.send_queued_message_now(&id, &entry, revision, edits)
                .map_err(|e| e.to_string())?;
            encode(app.load_session_queue_view(&id))
        }
        _ => Err(format!("Unknown host command: {}", command.method)),
    }
}
