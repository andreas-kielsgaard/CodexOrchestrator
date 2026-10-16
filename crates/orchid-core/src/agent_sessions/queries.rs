//! Persisted session reads. Construction does not discover or start runtimes.
use super::{
    application::AgentSessionApplicationError,
    domain::{AgentRuntimeEvent, AgentSessionId},
    ports::{
        AgentSessionHistory, AgentSessionRepository, AgentSessionSummary, ListAgentSessionsQuery,
    },
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AgentSessionQueries {
    repository: Arc<dyn AgentSessionRepository>,
    replicas: Option<crate::synchronization::repository::ReplicaStore>,
    source: Option<Arc<dyn crate::synchronization::SessionReplicaSource>>,
}

impl AgentSessionQueries {
    pub fn invocation(
        &self,
        id: &super::domain::AgentInvocationId,
    ) -> Result<Option<super::domain::AgentInvocation>, AgentSessionApplicationError> {
        if let Some(invocation) = self
            .repository
            .get_invocation(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if self
                .repository
                .owns_execution(&invocation.session_id)
                .map_err(AgentSessionApplicationError::repository)?
            {
                return Ok(Some(invocation));
            }
            return Ok(self
                .load_session(&invocation.session_id)?
                .invocations
                .into_iter()
                .find(|turn| &turn.invocation.id == id)
                .map(|turn| turn.invocation));
        }
        if let Some(replicas) = &self.replicas {
            for copy in replicas.list().map_err(storage_error)? {
                if let Some(turn) = copy
                    .session
                    .history
                    .invocations
                    .into_iter()
                    .find(|turn| &turn.invocation.id == id)
                {
                    return Ok(Some(turn.invocation));
                }
            }
        }
        Ok(None)
    }
    pub fn current_profile(
        &self,
        id: &AgentSessionId,
    ) -> Result<
        crate::execution_configuration::SessionCreationResolution,
        AgentSessionApplicationError,
    > {
        if self
            .repository
            .owns_execution(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if let Some(profile) = self
                .repository
                .current_execution_resolution(id)
                .map_err(AgentSessionApplicationError::repository)?
            {
                return Ok(profile);
            }
        } else if let Some(replicas) = &self.replicas {
            if let Some(profile) = replicas
                .get(id.as_str())
                .map_err(storage_error)?
                .and_then(|c| c.session.current_profile)
            {
                return Ok(profile);
            }
        }
        self.load_session(id)?
            .session
            .session_profile
            .ok_or_else(|| {
                AgentSessionApplicationError::not_found("This session has no current profile")
            })
    }
    pub fn new(repository: Arc<dyn AgentSessionRepository>) -> Self {
        Self {
            repository,
            replicas: None,
            source: None,
        }
    }
    pub fn with_replicas(
        mut self,
        replicas: crate::synchronization::repository::ReplicaStore,
    ) -> Self {
        self.replicas = Some(replicas);
        self
    }

    pub fn with_replica_source(
        mut self,
        source: Arc<dyn crate::synchronization::SessionReplicaSource>,
    ) -> Self {
        self.source = Some(source);
        self
    }

    pub fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, AgentSessionApplicationError> {
        let mut summaries = self
            .repository
            .list_session_summaries(query)
            .map_err(AgentSessionApplicationError::repository)?;
        if let Some(replicas) = &self.replicas {
            for summary in replicas.summaries().map_err(storage_error)? {
                if !self
                    .repository
                    .owns_execution(&summary.session.id)
                    .map_err(AgentSessionApplicationError::repository)?
                {
                    summaries.retain(|row| row.session.id != summary.session.id);
                }
                if summaries
                    .iter()
                    .any(|row| row.session.id == summary.session.id)
                    || query
                        .availability
                        .is_some_and(|availability| summary.session.availability != availability)
                {
                    continue;
                }
                summaries.push(summary);
            }
        }
        summaries.sort_by(|a, b| {
            b.session
                .updated_at
                .cmp(&a.session.updated_at)
                .then(a.session.id.as_str().cmp(b.session.id.as_str()))
        });
        if let Some(limit) = query.limit {
            summaries.truncate(limit as usize);
        }
        Ok(summaries)
    }

    pub fn load_session(
        &self,
        id: &AgentSessionId,
    ) -> Result<AgentSessionHistory, AgentSessionApplicationError> {
        if !self
            .repository
            .owns_execution(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if let Some(history) = self.replica_history(id)? {
                return Ok(history);
            }
        }
        if let Some(history) = self
            .repository
            .load_session_history(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            return Ok(history);
        }
        if let Some(history) = self.replica_history(id)? {
            return Ok(history);
        }
        Err(AgentSessionApplicationError::not_found(
            "Agent Session not found",
        ))
    }

    fn replica_history(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, AgentSessionApplicationError> {
        let Some(replicas) = &self.replicas else {
            return Ok(None);
        };
        if let Some(change) = replicas.get(id.as_str()).map_err(storage_error)? {
            return Ok(Some(change.session.history));
        }
        if replicas
            .owner(id.as_str())
            .map_err(storage_error)?
            .is_none()
        {
            return Ok(None);
        }
        let source = self.source.as_ref().ok_or_else(|| storage_error("This conversation has only a cached summary. Connect to the synchronization server to fetch its history.".into()))?;
        let change = source.fetch_session(id.as_str()).map_err(storage_error)?;
        if change.session.history.session.id != *id {
            return Err(storage_error(
                "The server returned another conversation".into(),
            ));
        }
        replicas
            .accept(&change.device_id, &change)
            .map_err(storage_error)?;
        // Read the accepted copy: a concurrent synchronization may already have a newer snapshot.
        replicas
            .get(id.as_str())
            .map_err(storage_error)?
            .map(|copy| Some(copy.session.history))
            .ok_or_else(|| storage_error("The synchronized history is no longer available".into()))
    }

    pub fn load_session_display(
        &self,
        id: &AgentSessionId,
    ) -> Result<AgentSessionHistory, AgentSessionApplicationError> {
        if !self
            .repository
            .owns_execution(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            return self.load_session(id);
        }
        if let Some(history) = self
            .repository
            .load_session_display_history(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            return Ok(history);
        }
        self.load_session(id)
    }

    pub fn event_details(
        &self,
        session: &AgentSessionId,
        event: &str,
    ) -> Result<AgentRuntimeEvent, AgentSessionApplicationError> {
        if self
            .repository
            .owns_execution(session)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if let Some(event) = self
                .repository
                .read_event_details(session, event)
                .map_err(AgentSessionApplicationError::repository)?
            {
                return Ok(event);
            }
        }
        self.load_session(session)?
            .invocations
            .into_iter()
            .flat_map(|turn| turn.events)
            .find(|candidate| candidate.id.as_str() == event)
            .ok_or_else(|| {
                AgentSessionApplicationError::not_found("Action details not found in this session")
            })
    }
}
fn storage_error(message: String) -> AgentSessionApplicationError {
    AgentSessionApplicationError::repository(super::ports::RepositoryError::new(
        super::ports::RepositoryErrorKind::Unavailable,
        message,
    ))
}
