//! Session creation and delivery use cases used by the Session Events adapter.
use super::*;
use crate::{
    agent_sessions::{domain::*, ports::*, references::session_reference},
    execution_configuration::{
        DirectUserInvocationRequest, SessionCreationRequest, SessionProfileResolver,
    },
    identities::AssignedAgentIdentity,
    session_events::{SessionCreationSpec, SessionDirectoryEntry, SessionDirectoryError},
};

impl AgentSessionApplication {
    pub(crate) fn create_addressed_profiled_session(
        &self,
        addresses: &dyn AgentSessionAddressStore,
        request: SessionCreationSpec,
        session_id: AgentSessionId,
        mut creation: SessionCreationRequest,
        working_directory: Option<String>,
        title: Option<String>,
        assigned_identity: Option<AssignedAgentIdentity>,
    ) -> Result<SessionDirectoryEntry, SessionDirectoryError> {
        if let Some(existing) =
            self.existing_addressed_creation(addresses, &request, &session_id, &creation)?
        {
            return Ok(existing);
        }
        let source = self
            .configuration_source(&creation.capability_profile.execution.provider)
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
        creation.session_skill_inputs = self
            .compile_capability_skill_inputs(
                &creation.capability_profile,
                &creation.capability_profile.execution.configuration_ref,
                working_directory.as_deref(),
            )
            .map_err(SessionDirectoryError::new)?;
        let resolution = SessionProfileResolver::resolve_creation(
            source.as_ref(),
            working_directory.as_deref(),
            creation.clone(),
        )
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
        let requested_options = runtime_options(resolution.session_profile().pinned_defaults());
        let session = self.prepare_session_with_id(
            CreateAgentSessionCommand {
                title,
                working_directory,
                requested_options,
            },
            session_id.clone(),
            AgentSessionOwnership {
                execution_target: None,
                harness_version: None,
                assigned_identity,
                session_profile: Some(resolution.clone()),
            },
        );
        let session = session.map_err(|error| SessionDirectoryError::new(error.to_string()))?;
        let session_ref = session_reference(session.id.as_str())?;
        let (_session, created_sequence) = match addresses.create_addressed_session(
            session,
            &request.logical_address,
            &request.created_by_event,
            request.created_by_session.as_ref(),
        ) {
            Ok(created) => created,
            Err(error)
                if error.kind == crate::agent_sessions::ports::RepositoryErrorKind::Conflict =>
            {
                return self
                    .existing_addressed_creation(addresses, &request, &session_id, &creation)?
                    .ok_or_else(|| {
                        SessionDirectoryError::new(
                            "Session Event creation conflicted without an addressed Session",
                        )
                    });
            }
            Err(error) => return Err(SessionDirectoryError::new(error.to_string())),
        };
        Ok(SessionDirectoryEntry {
            session: session_ref,
            logical_address: Some(request.logical_address),
            running: false,
            created_sequence,
            last_addressed_sequence: None,
            created_by_event: Some(request.created_by_event),
            created_by_session: request.created_by_session,
        })
    }

    fn existing_addressed_creation(
        &self,
        addresses: &dyn AgentSessionAddressStore,
        request: &SessionCreationSpec,
        id: &AgentSessionId,
        creation: &SessionCreationRequest,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
        let Some(session) = self
            .repository
            .get_session(id)
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?
        else {
            return Ok(None);
        };
        let pinned = session.session_profile.as_ref().ok_or_else(|| SessionDirectoryError::new("Session Event creation identity belongs to a session without creation configuration"))?;
        pinned
            .verify_digest()
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
        // A retry compares the original request against original native evidence. Refreshing
        // native discovery here would rewrite creation semantics as the environment changes.
        let profile = pinned.session_profile();
        let original_runtime = crate::execution_configuration::RuntimeProfileSnapshot {
            contract_version: 1,
            configuration: profile.configuration().clone(),
            exposure: profile.attached_runtime_capabilities().clone(),
            locked: profile.attached_runtime_locked().clone(),
            provider_options: profile.provider_options().cloned(),
        };
        let mut creation = creation.clone();
        creation.session_skill_inputs = profile.session_skill_inputs().to_vec();
        let expected = SessionProfileResolver::resolve_snapshot(original_runtime, creation)
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
        if &expected != pinned {
            return Err(SessionDirectoryError::new(
                "Session Event creation identity was already used for a different pinned profile",
            ));
        }
        let existing = addresses.find_session_entry(id)?.ok_or_else(|| {
            SessionDirectoryError::new(
                "Session Event creation identity belongs to an unaddressed session",
            )
        })?;
        if existing.logical_address.as_ref() != Some(&request.logical_address)
            || existing.created_by_event.as_ref() != Some(&request.created_by_event)
            || existing.created_by_session.as_ref() != request.created_by_session.as_ref()
        {
            return Err(SessionDirectoryError::new("Session Event creation identity was already used for different addressing semantics"));
        }
        Ok(Some(existing))
    }
}

use crate::session_events::{DirectUserInvocationOptions, SessionInvocationError};
impl AgentSessionApplication {
    pub(crate) fn deliver_profiled_message(
        &self,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        prompt: String,
        initial_prompt: Option<String>,
        event_group_id: String,
        direct_user_options: Option<DirectUserInvocationOptions>,
        direct_user: bool,
    ) -> Result<SendAgentSessionMessageLaunchResult, SessionInvocationError> {
        let history = self
            .load_session(&session_id)
            .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        let creation = history.session.session_profile.as_ref().ok_or_else(|| {
            SessionInvocationError::new(
                "Session Event delivery requires an immutable pinned Session Profile",
            )
        })?;
        let source = self
            .configuration_source(&creation.session_profile().configuration().provider)
            .map_err(|e| SessionInvocationError::new(e.to_string()))?;
        let source = source.as_ref();
        let cwd = history.session.working_directory.as_deref();
        let selections = match direct_user_options {
            Some(options) => {
                SessionProfileResolver::validate_direct_user_invocation(
                    source,
                    cwd,
                    creation,
                    DirectUserInvocationRequest {
                        contract_version: 1,
                        model: options.model,
                        reasoning_mode: options.reasoning_mode,
                        sandbox_mode: None,
                    },
                )
                .map_err(|error| SessionInvocationError::new(error.to_string()))?
                .selections
            }
            None => {
                SessionProfileResolver::validate_pinned_session(source, cwd, creation)
                    .map_err(|error| SessionInvocationError::new(error.to_string()))?;
                creation.session_profile().pinned_defaults().clone()
            }
        };
        let requested_options = runtime_options(&selections);
        let mut extension = reasoning_launch_extension(&selections).unwrap_or_default();
        if let Some(initial_prompt) = initial_prompt {
            extension.initial_prompt_prefix = Some(InitialPromptPrefix {
                source: format!("session_event:{event_group_id}"),
                version: 1,
                content: initial_prompt,
            });
        }
        let command = SendIdempotentApplicationAgentSessionMessageCommand {
            invocation_id,
            message: SendAgentSessionMessageCommand {
                session_id: Some(session_id),
                submitted_text: prompt,
                title: None,
                working_directory: history.session.working_directory,
                requested_options: Some(requested_options),
            },
        };
        let extension = (extension != RuntimeLaunchExtension::default()).then_some(extension);
        if direct_user {
            self.send_idempotent_user_message_with_launch_observation(command, extension)
        } else {
            self.send_idempotent_application_message_with_launch_observation(command, extension)
        }
        .map_err(|error| SessionInvocationError::new(error.to_string()))
    }
}
