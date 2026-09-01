use crate::{
    agent_sessions::{
        application::{
            AgentSessionProfileApplication, LoadPinnedSessionProfileQuery,
            SendDirectUserAgentSessionMessageCommand,
        },
        domain::{AgentInvocationId, AgentSessionId},
    },
    execution_configuration::{DirectUserInvocationResolution, SessionCreationResolution},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

pub(crate) struct AgentSessionProfileTauriState {
    application: Arc<AgentSessionProfileApplication>,
}

impl AgentSessionProfileTauriState {
    pub(crate) fn new(application: Arc<AgentSessionProfileApplication>) -> Self {
        Self { application }
    }
}

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
    model: Option<String>,
    reasoning_mode: Option<String>,
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
pub(crate) fn load_pinned_agent_session_profile(
    state: State<'_, AgentSessionProfileTauriState>,
    input: LoadPinnedSessionProfileInput,
) -> Result<PinnedAgentSessionProfileDto, String> {
    let pinned = state
        .application
        .load_pinned_session_profile(LoadPinnedSessionProfileQuery {
            session_id: input.session_id,
        })
        .map_err(|error| error.to_string())?;
    Ok(PinnedAgentSessionProfileDto {
        session_id: pinned.session_id,
        creation_resolution: pinned.creation_resolution,
    })
}

#[tauri::command]
pub(crate) fn send_direct_user_agent_session_message(
    state: State<'_, AgentSessionProfileTauriState>,
    input: SendDirectUserAgentSessionMessageInput,
) -> Result<SendDirectUserAgentSessionMessageResultDto, String> {
    let result = state
        .application
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: input.session_id,
            submitted_text: input.submitted_text,
            model: input.model,
            reasoning_mode: input.reasoning_mode,
        })
        .map_err(|error| error.to_string())?;
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
