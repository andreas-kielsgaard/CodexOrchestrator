//! Session creation, metadata, turn submission and cancellation.
use super::*;
#[tauri::command]
pub(crate) fn create_agent_session(
    state: State<'_, AgentSessionTauriState>,
    input: CreateAgentSessionCommandDto,
) -> Result<AgentSessionDto, String> {
    let ownership = input.ownership();
    state
        .application
        .create_default_session(input.into(), ownership)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_agent_session_harness(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionHarnessCommandDto,
) -> Result<AgentSessionDto, String> {
    state
        .application
        .update_session_harness(input.into())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_agent_session_identity(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionIdentityCommandDto,
) -> Result<AgentSessionDto, String> {
    state
        .application
        .update_session_identity(input.into())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_agent_session_model_override(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionModelOverrideCommandDto,
) -> Result<AgentSessionDto, String> {
    state
        .application
        .update_session_model_override(input.into())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn send_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: SendAgentSessionMessageCommandDto,
) -> Result<SendAgentSessionMessageResultDto, String> {
    let command: crate::agent_sessions::application::SendAgentSessionMessageCommand = input.into();
    let choices = command.requested_options.clone().unwrap_or_default();
    let sandbox_mode = choices.sandbox.map(|mode| match mode {
        crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly => {
            crate::execution_configuration::SandboxMode::ReadOnly
        }
        crate::agent_sessions::domain::RuntimeSandboxMode::WorkspaceWrite => {
            crate::execution_configuration::SandboxMode::WorkspaceWrite
        }
        crate::agent_sessions::domain::RuntimeSandboxMode::DangerFullAccess => {
            crate::execution_configuration::SandboxMode::DangerFullAccess
        }
    });
    if command.session_id.is_none() {
        return state
            .application
            .start_direct_user_session(
                command.submitted_text,
                command.title,
                command.working_directory,
                choices.model,
                None,
                sandbox_mode,
                None,
            )
            .map(|result| result.acknowledgement.into())
            .map_err(|e| e.to_string());
    }
    let id = command.session_id.as_ref().expect("existing session");
    if state
        .application
        .load_session(id)
        .map_err(|e| e.to_string())?
        .session
        .session_profile
        .is_some()
    {
        return state
            .application
            .send_direct_user_message(
                crate::agent_sessions::application::SendDirectUserAgentSessionMessageCommand {
                    session_id: id.clone(),
                    submitted_text: command.submitted_text,
                    model: choices.model,
                    reasoning_mode: None,
                    sandbox_mode,
                },
            )
            .map(|result| result.acknowledgement.into())
            .map_err(|e| e.to_string());
    }
    state
        .application
        .send_message(command)
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn cancel_agent_invocation(
    state: State<'_, AgentSessionTauriState>,
    input: CancelAgentInvocationCommandDto,
) -> Result<AgentInvocationDto, String> {
    state
        .application
        .cancel_invocation(input.into())
        .map_err(|error| error.to_string())
}
