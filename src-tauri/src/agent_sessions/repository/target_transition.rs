//! Persistence for one current device/worktree transition per Agent Session.

use super::*;
use crate::agent_sessions::{
    target_transition::{validate_target_transition, SessionTargetTransition},
};

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_target_transitions (
    session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
);
"#;

fn read_record(
    connection: &Connection,
    session_id: &AgentSessionId,
) -> Result<Option<SessionTargetTransition>, RepositoryError> {
    let json: Option<String> = connection
        .query_row(
            "SELECT payload_json FROM agent_session_target_transitions WHERE session_id=?1",
            [session_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_unavailable("read target transition"))?;
    json.map(|json| {
        serde_json::from_str(&json).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorKind::InvalidState,
                format!("Invalid stored target transition: {error}"),
            )
        })
    })
    .transpose()
}

impl SqliteAgentSessionRepository {
    pub(super) fn read_target_transition_record(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<SessionTargetTransition>, RepositoryError> {
        self.read("read target transition", |connection| {
            read_record(connection, session_id)
        })
    }

    pub(super) fn save_target_transition_record(
        &self,
        transition: &SessionTargetTransition,
    ) -> Result<SessionTargetTransition, RepositoryError> {
        validate_target_transition(transition)
            .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
        self.write("save target transition", |transaction| {
            required_session(transaction, &transition.session_id)?;
            transaction
                .execute(
                    "INSERT INTO agent_session_target_transitions(session_id,payload_json)
                     VALUES(?1,?2) ON CONFLICT(session_id)
                     DO UPDATE SET payload_json=excluded.payload_json",
                    params![transition.session_id.as_str(), to_json(transition)?],
                )
                .map_err(sql_write("save target transition"))?;
            touch_session(transaction, &transition.session_id, transition.updated_at)?;
            Ok(transition.clone())
        })
    }

    pub(super) fn clear_target_transition_record(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<(), RepositoryError> {
        self.write("clear target transition", |transaction| {
            required_session(transaction, session_id)?;
            transaction
                .execute(
                    "DELETE FROM agent_session_target_transitions WHERE session_id=?1",
                    [session_id.as_str()],
                )
                .map_err(sql_write("clear target transition"))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{
            domain::{
                AgentRuntimeBinding, AgentRuntimeOptions, AgentSession, AgentSessionAvailability,
            },
            target_transition::{
                QueuedTransitionPrompt, SessionTargetTransition, TargetTransitionPhase,
                TargetTransitionTask, TargetTransitionTaskKind, TargetTransitionTaskStatus,
            },
        },
        execution_targets::domain::{
            ExecutionBinding, ExecutionConnection, SessionExecutionSelection,
            SessionExecutionTarget, SessionWorkspaceSelection,
        },
    };
    use chrono::{DateTime, Utc};

    fn at(second: u32) -> DateTime<Utc> {
        format!("2026-09-17T12:00:{second:02}Z").parse().unwrap()
    }

    fn target(device_id: &str) -> SessionExecutionTarget {
        SessionExecutionTarget {
            capability_profile_id: format!("profile-{device_id}"),
            capability_profile_revision: 1,
            execution: ExecutionBinding {
                device_id: device_id.into(),
                device_name: device_id.into(),
                provider: "codex".into(),
                configuration_ref: "default".into(),
                connection: if device_id == "laptop" {
                    ExecutionConnection::Local
                } else {
                    ExecutionConnection::Ssh {
                        target: "orchid@example.test".into(),
                        host_executable: "orchid-host".into(),
                    }
                },
            },
            repository_id: "orchid".into(),
            branch_ref: "refs/heads/feature/remote".into(),
            worktree_id: format!("{device_id}-worktree"),
            path: format!("/work/{device_id}"),
            head: Some("abc123".into()),
        }
    }

    fn transition(session_id: AgentSessionId) -> SessionTargetTransition {
        let source = target("laptop");
        let destination = target("server");
        SessionTargetTransition {
            session_id,
            source_target: source,
            destination_selection: SessionExecutionSelection {
                capability_profile_id: destination.capability_profile_id.clone(),
                capability_profile_revision: destination.capability_profile_revision,
                execution: destination.execution.clone(),
                workspace: SessionWorkspaceSelection::Existing {
                    target: destination,
                },
            },
            sister_group_id: Some("group".into()),
            phase: TargetTransitionPhase::Pending,
            tasks: vec![TargetTransitionTask {
                kind: TargetTransitionTaskKind::InspectSource,
                status: TargetTransitionTaskStatus::Pending,
                detail: None,
                error: None,
            }],
            snapshot: None,
            transfer_estimate: None,
            queued_prompt: Some(QueuedTransitionPrompt {
                text: "Continue remotely".into(),
                client_message_id: "client-message".into(),
            }),
            resolved_target: None,
            destination_session_id: None,
            error: None,
            created_at: at(1),
            updated_at: at(1),
        }
    }

    fn session(id: AgentSessionId) -> AgentSession {
        AgentSession {
            execution_target: None,
            workspace_origin: None,
            id,
            title: "Session".into(),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: None,
                runtime_version: None,
            },
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
            session_profile: None,
            harness_version: None,
            assigned_identity: None,
            created_at: at(0),
            updated_at: at(0),
        }
    }

    #[test]
    fn one_session_transition_replaces_and_clears_durably() {
        let repository =
            SqliteAgentSessionRepository::new(Connection::open_in_memory().unwrap()).unwrap();
        let session_id = AgentSessionId::new("session").unwrap();
        repository
            .create_session(session(session_id.clone()))
            .unwrap();

        let original = transition(session_id.clone());
        assert_eq!(
            repository.save_target_transition(&original).unwrap(),
            original
        );
        assert_eq!(
            repository.target_transition(&session_id).unwrap(),
            Some(original)
        );

        let mut replacement = transition(session_id.clone());
        replacement.updated_at = at(2);
        replacement.phase = TargetTransitionPhase::Running;
        replacement.queued_prompt = None;
        replacement.tasks[0].status = TargetTransitionTaskStatus::Running;
        assert_eq!(
            repository.save_target_transition(&replacement).unwrap(),
            replacement
        );
        assert_eq!(
            repository.target_transition(&session_id).unwrap(),
            Some(replacement)
        );

        repository.clear_target_transition(&session_id).unwrap();
        assert_eq!(repository.target_transition(&session_id).unwrap(), None);
    }
}
