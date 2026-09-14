//! Session creation and explicit historical context selection.
use super::{AgentSessionApplication, AgentSessionApplicationError};
use crate::agent_sessions::{domain::AgentSessionId, workspace::SessionWorkspaces};

impl AgentSessionApplication {
    pub(super) fn add_workspace_capabilities(
        &self,
        extension: Option<crate::agent_sessions::ports::RuntimeLaunchExtension>,
    ) -> Option<crate::agent_sessions::ports::RuntimeLaunchExtension> {
        let Some(workspaces) = &self.workspaces else {
            return extension;
        };
        let mut extension = extension.unwrap_or_default();
        let root = workspaces.skills_root();
        if !extension.skill_roots.contains(&root) {
            extension.skill_roots.push(root);
        }
        Some(extension)
    }
    pub(crate) fn resolve_working_directory(
        &self,
        id: &AgentSessionId,
        directory: String,
    ) -> Result<(), AgentSessionApplicationError> {
        if directory.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "Choose an explicit working directory",
            ));
        }
        let history = self.load_session(id)?;
        if history.session.execution_target.is_some() { return Err(AgentSessionApplicationError::conflict("This session is bound to its selected worktree")); }
        if history
            .invocations
            .iter()
            .any(|i| i.invocation.status.is_active())
        {
            return Err(AgentSessionApplicationError::conflict(
                "Cancel the active invocation before selecting its working context",
            ));
        }
        let directory = self
            .prepare_working_directory(id, Some(directory))?
            .ok_or_else(|| {
                AgentSessionApplicationError::invalid("Choose an explicit working directory")
            })?;
        self.repository
            .resolve_working_directory(id, &directory, "explicit", self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        Ok(())
    }
    pub(crate) fn with_workspaces(mut self, workspaces: SessionWorkspaces) -> Self {
        self.workspaces = Some(workspaces);
        self
    }

    pub(super) fn prepare_working_directory(
        &self,
        id: &AgentSessionId,
        explicit: Option<String>,
    ) -> Result<Option<String>, AgentSessionApplicationError> {
        match &self.workspaces {
            Some(workspaces) => workspaces
                .prepare(id, explicit)
                .map(Some)
                .map_err(AgentSessionApplicationError::invalid),
            None => Ok(super::creation::normalize_optional(explicit)),
        }
    }
}
