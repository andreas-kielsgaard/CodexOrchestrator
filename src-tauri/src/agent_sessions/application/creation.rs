//! Session creation, metadata and history queries.

use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionOwnership,
    CreateAgentSessionCommand, CreateApplicationAgentSessionCommand, ListAgentSessionsResult,
    UpdateAgentSessionHarnessCommand, UpdateAgentSessionIdentityCommand,
    UpdateAgentSessionModelOverrideCommand,
};
use crate::agent_sessions::domain::{
    AgentRuntimeBinding, AgentSession, AgentSessionAvailability, AgentSessionId,
};
use crate::agent_sessions::ports::{AgentSessionHistory, ListAgentSessionsQuery};

impl AgentSessionApplication {
    pub(crate) fn create_session(
        &self,
        command: CreateAgentSessionCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        self.create_session_with_ownership(command, AgentSessionOwnership::default())
    }

    pub(crate) fn create_session_with_ownership(
        &self,
        command: CreateAgentSessionCommand,
        ownership: AgentSessionOwnership,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        self.create_session_with_id(command, self.ids.session_id(), ownership)
    }

    pub(crate) fn create_application_session(
        &self,
        command: CreateApplicationAgentSessionCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        self.create_application_session_with_ownership(command, AgentSessionOwnership::default())
    }

    pub(crate) fn create_application_session_with_ownership(
        &self,
        command: CreateApplicationAgentSessionCommand,
        ownership: AgentSessionOwnership,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        if let Some(existing) = self
            .repository
            .get_session(&command.session_id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            let expected_title = normalize_title(command.session.title.as_deref(), "Agent Session");
            let expected_directory = self.prepare_working_directory(
                &command.session_id,
                command.session.working_directory,
            )?;
            if existing.title != expected_title
                || existing.working_directory != expected_directory
                || existing.requested_options != command.session.requested_options
                || existing.session_profile != ownership.session_profile
            {
                return Err(AgentSessionApplicationError::conflict(
                    "application Agent Session identity was already used for different semantics",
                ));
            }
            return Ok(existing);
        }
        self.create_session_with_id(command.session, command.session_id, ownership)
    }

    fn create_session_with_id(
        &self,
        command: CreateAgentSessionCommand,
        session_id: AgentSessionId,
        ownership: AgentSessionOwnership,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        let session = self.prepare_session_with_id(command, session_id, ownership)?;
        self.repository
            .create_session(session)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn prepare_session_with_id(
        &self,
        command: CreateAgentSessionCommand,
        session_id: AgentSessionId,
        ownership: AgentSessionOwnership,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        let now = self.clock.now();
        let workspace_origin = if ownership.execution_target.is_some()
            || command
                .working_directory
                .as_ref()
                .is_some_and(|p| !p.trim().is_empty())
        {
            "explicit"
        } else {
            "allocated"
        };
        let working_directory = match &ownership.execution_target {
            Some(target) if target.execution.is_remote() => Some(target.path.clone()),
            Some(target) => {
                self.prepare_working_directory(&session_id, Some(target.path.clone()))?
            }
            None => self.prepare_working_directory(&session_id, command.working_directory)?,
        };
        Ok(AgentSession {
            execution_target: ownership.execution_target,
            workspace_origin: self.workspaces.as_ref().map(|_| workspace_origin.into()),
            id: session_id,
            title: normalize_title(command.title.as_deref(), "Agent Session"),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: None,
                runtime_version: self.runtime_version.clone(),
            },
            working_directory,
            requested_options: command.requested_options,
            session_profile: ownership.session_profile,
            harness_version: ownership.harness_version,
            assigned_identity: ownership.assigned_identity,
            created_at: now,
            updated_at: now,
        })
    }

    pub(crate) fn update_session_harness(
        &self,
        command: UpdateAgentSessionHarnessCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        if self
            .load_session(&command.session_id)?
            .session
            .execution_target
            .as_ref()
            .is_some_and(|target| target.execution.is_remote())
            && command.harness_version.is_some()
        {
            return Err(AgentSessionApplicationError::invalid(
                "Harness workflow integration is unavailable for remote sessions in this prototype",
            ));
        }
        self.repository
            .update_harness_version(
                &command.session_id,
                command.harness_version,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn update_session_identity(
        &self,
        command: UpdateAgentSessionIdentityCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        self.repository
            .update_assigned_identity(
                &command.session_id,
                command.assigned_identity,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn update_session_model_override(
        &self,
        command: UpdateAgentSessionModelOverrideCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        let model = match command.model {
            Some(model) if model.trim().is_empty() => {
                return Err(AgentSessionApplicationError::invalid(
                    "Session model override cannot be blank",
                ));
            }
            Some(model) => Some(model.trim().to_string()),
            None => None,
        };
        self.repository
            .update_session_model_override(&command.session_id, model, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<ListAgentSessionsResult, AgentSessionApplicationError> {
        self.repository
            .list_session_summaries(query)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn load_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<AgentSessionHistory, AgentSessionApplicationError> {
        self.repository
            .load_session_history(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))
    }
}

pub(super) fn normalize_title(title: Option<&str>, fallback: &str) -> String {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub(super) fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn title_from_message(message: &str) -> String {
    let title = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.chars().count() <= 80 {
        title
    } else {
        format!("{}...", title.chars().take(77).collect::<String>())
    }
}
