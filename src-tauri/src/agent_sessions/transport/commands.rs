//! Session creation, metadata, turn submission and cancellation.
use super::*;
use base64::Engine;
use sha2::{Digest, Sha256};

#[tauri::command]
pub(crate) async fn read_agent_session_image(
    path: String,
) -> Result<crate::agent_sessions::domain::RuntimeImageAttachment, String> {
    tauri::async_runtime::spawn_blocking(move || {
        const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
        let path = std::path::PathBuf::from(path);
        let metadata =
            std::fs::metadata(&path).map_err(|error| format!("Unable to read image: {error}"))?;
        if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
            return Err("Images must be files no larger than 10 MB".into());
        }
        let media_type = match path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => return Err("Supported images are PNG, JPEG, GIF, and WebP".into()),
        };
        let bytes =
            std::fs::read(&path).map_err(|error| format!("Unable to read image: {error}"))?;
        let id = format!("{:x}", Sha256::digest(&bytes));
        Ok(crate::agent_sessions::domain::RuntimeImageAttachment {
            id,
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("image")
                .to_owned(),
            media_type: media_type.into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn create_agent_session(
    state: State<'_, AgentSessionTauriState>,
    input: CreateAgentSessionCommandDto,
) -> Result<AgentSessionDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ownership = input.ownership();
        application
            .create_default_session(input.into(), ownership)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_agent_session_harness(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionHarnessCommandDto,
) -> Result<AgentSessionDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .update_session_harness(input.into())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_agent_session_identity(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionIdentityCommandDto,
) -> Result<AgentSessionDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .update_session_identity(input.into())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn rename_agent_session(
    state: State<'_, AgentSessionTauriState>,
    input: RenameAgentSessionCommandDto,
) -> Result<AgentSessionDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .rename_session(input.into())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_agent_session_model_override(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateAgentSessionModelOverrideCommandDto,
) -> Result<AgentSessionDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .update_session_model_override(input.into())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn send_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: SendAgentSessionMessageCommandDto,
) -> Result<SendAgentSessionMessageResultDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let command: crate::agent_sessions::application::SendAgentSessionMessageCommand =
            input.into();
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
            return application
                .start_direct_user_session_with_target_and_attachments(
                    command.submitted_text,
                    command.attachments,
                    command.title,
                    command.working_directory,
                    choices.model,
                    None,
                    sandbox_mode,
                    None,
                    None,
                )
                .map(|result| result.acknowledgement.into())
                .map_err(|e| e.to_string());
        }
        let id = command.session_id.as_ref().expect("existing session");
        if application
            .load_session(id)
            .map_err(|e| e.to_string())?
            .session
            .session_profile
            .is_some()
        {
            return application
                .send_direct_user_message(
                    crate::agent_sessions::application::SendDirectUserAgentSessionMessageCommand {
                        session_id: id.clone(),
                        submitted_text: command.submitted_text,
                        attachments: command.attachments,
                        model: choices.model,
                        reasoning_mode: None,
                        sandbox_mode,
                    },
                )
                .map(|result| result.acknowledgement.into())
                .map_err(|e| e.to_string());
        }
        application
            .send_message(command)
            .map(Into::into)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn cancel_agent_invocation(
    state: State<'_, AgentSessionTauriState>,
    input: CancelAgentInvocationCommandDto,
) -> Result<AgentInvocationDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .cancel_invocation(input.into())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
