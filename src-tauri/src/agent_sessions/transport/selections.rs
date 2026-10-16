use crate::{
    agent_sessions::{
        application::{LoadPinnedSessionProfileQuery, SendDirectUserAgentSessionMessageCommand},
        domain::{AgentInvocationId, AgentSessionId},
    },
    execution_configuration::{DirectUserInvocationResolution, SessionCreationResolution},
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadPinnedSessionProfileInput {
    session_id: AgentSessionId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SendDirectUserAgentSessionMessageInput {
    session_id: AgentSessionId,
    submitted_text: String,
    #[serde(default)]
    attachments: Vec<crate::agent_sessions::domain::RuntimeImageAttachment>,
    model: Option<String>,
    reasoning_mode: Option<String>,
    sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PinnedAgentSessionProfileDto {
    session_id: AgentSessionId,
    creation_resolution: SessionCreationResolution,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendDirectUserAgentSessionMessageResultDto {
    session_id: AgentSessionId,
    invocation_id: AgentInvocationId,
    invocation_resolution: DirectUserInvocationResolution,
}

#[tauri::command]
pub(crate) async fn load_pinned_agent_session_profile(
    state: State<'_, super::AgentSessionTauriState>,
    input: LoadPinnedSessionProfileInput,
) -> Result<PinnedAgentSessionProfileDto, String> {
    let application = state.queries.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let history = application.load_session(&input.session_id).map_err(|e|e.to_string())?;
        Ok(PinnedAgentSessionProfileDto {
            session_id: input.session_id,
            creation_resolution: history.session.session_profile.ok_or("This session has no current profile")?,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn send_direct_user_agent_session_message(
    state: State<'_, super::AgentSessionTauriState>,
    input: SendDirectUserAgentSessionMessageInput,
) -> Result<SendDirectUserAgentSessionMessageResultDto, String> {
    let application = state.application.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        application
            .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
                session_id: input.session_id,
                submitted_text: input.submitted_text,
                attachments: input.attachments,
                model: input.model,
                reasoning_mode: input.reasoning_mode,
                sandbox_mode: input.sandbox_mode,
            })
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    Ok(SendDirectUserAgentSessionMessageResultDto {
        session_id: result.acknowledgement.session_id,
        invocation_id: result.acknowledgement.invocation_id,
        invocation_resolution: result.invocation_resolution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_user_input_accepts_only_message_local_runtime_choices() {
        let input: SendDirectUserAgentSessionMessageInput =
            serde_json::from_value(serde_json::json!({
                "sessionId": "session-1",
                "submittedText": "Continue",
                "model": "codex-a",
                "reasoningMode": "high"
            }))
            .unwrap();
        assert_eq!(input.model.as_deref(), Some("codex-a"));

        assert!(
            serde_json::from_value::<SendDirectUserAgentSessionMessageInput>(serde_json::json!({
                "sessionId": "session-1",
                "submittedText": "Continue",
                "sandbox": "danger_full_access"
            }))
            .is_err()
        );
    }
}

impl
    From<crate::agent_sessions::application::configuration::SendDirectUserAgentSessionMessageResult>
    for SendDirectUserAgentSessionMessageResultDto
{
    fn from(
        result: crate::agent_sessions::application::configuration::SendDirectUserAgentSessionMessageResult,
    ) -> Self {
        Self {
            session_id: result.acknowledgement.session_id,
            invocation_id: result.acknowledgement.invocation_id,
            invocation_resolution: result.invocation_resolution,
        }
    }
}

#[tauri::command]
pub(crate) async fn load_current_agent_session_profile(
    state: State<'_, super::AgentSessionTauriState>,
    input: LoadPinnedSessionProfileInput,
) -> Result<PinnedAgentSessionProfileDto, String> {
    let application = state.queries.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let profile=application.current_profile(&input.session_id).map_err(|e|e.to_string())?;
        Ok(PinnedAgentSessionProfileDto {
            session_id: input.session_id,
            creation_resolution: profile,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
