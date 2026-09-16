//! Transactional acceptance and truthful successful execution bindings.
use super::*;
use crate::agent_sessions::preparation::{PreparationPhase, SessionPreparation};
pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_preparations (
 invocation_id TEXT PRIMARY KEY REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
 session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
);
CREATE INDEX IF NOT EXISTS agent_session_preparations_session ON agent_session_preparations(session_id);
CREATE TABLE IF NOT EXISTS agent_session_current_execution (
 session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
 resolution_json TEXT NOT NULL CHECK(json_valid(resolution_json))
);
"#;
fn read_record(
    conn: &Connection,
    id: &AgentInvocationId,
) -> Result<Option<SessionPreparation>, RepositoryError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM agent_session_preparations WHERE invocation_id=?1",
            [id.as_str()],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql_unavailable("read preparation"))?;
    json.map(|j| {
        serde_json::from_str(&j)
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::InvalidState, e.to_string()))
    })
    .transpose()
}
impl SqliteAgentSessionRepository {
    pub(super) fn accept_preparation_record(
        &self,
        new_session: Option<(
            AgentSession,
            Option<crate::agent_sessions::organization::SessionPlacement>,
        )>,
        invocation: AgentInvocation,
        preparation: SessionPreparation,
    ) -> Result<(), RepositoryError> {
        self.write("accept ordinary submission", |tx| {
            if let Some((session, placement)) = new_session {
                validate_session(&session).map_err(contract_error)?;
                insert_session(tx, &session)?;
                if let Some(placement) = placement {
                    write_placement(tx, &session.id, &placement)?;
                }
            }
            let session = required_session(tx, &invocation.session_id)?;
            let active = active_invocation(tx, &session.id)?;
            validate_new_invocation(&session, active.as_ref(), &invocation)
                .map_err(contract_error)?;
            insert_invocation(tx, &invocation)?;
            tx.execute(
                "INSERT INTO agent_session_preparations(invocation_id,session_id,payload_json)
                 VALUES(?1,?2,?3)",
                params![
                    invocation.id.as_str(),
                    session.id.as_str(),
                    to_json(&preparation)?
                ],
            )
            .map_err(sql_write("accept preparation"))?;
            touch_session(tx, &session.id, invocation.created_at)?;
            Ok(())
        })
    }
    pub(super) fn read_preparation(
        &self,
        id: &AgentInvocationId,
    ) -> Result<Option<SessionPreparation>, RepositoryError> {
        self.read("read preparation", |c| read_record(c, id))
    }
    pub(super) fn read_latest_preparation(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<SessionPreparation>, RepositoryError> {
        self.read("read Session preparation", |connection| {
            let invocation: Option<String> = connection
                .query_row(
                    "SELECT p.invocation_id FROM agent_session_preparations p
                 JOIN agent_session_invocations i ON i.id=p.invocation_id
                 WHERE p.session_id=?1 ORDER BY i.created_at DESC,i.rowid DESC LIMIT 1",
                    [id.as_str()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read latest preparation"))?;
            invocation
                .map(|id| {
                    AgentInvocationId::new(id)
                        .map_err(contract_error)
                        .and_then(|id| read_record(connection, &id))
                })
                .transpose()
                .map(Option::flatten)
        })
    }

    pub(super) fn save_preparation_record(
        &self,
        p: &SessionPreparation,
    ) -> Result<(), RepositoryError> {
        self.write("save preparation", |tx| {
            let n = tx
                .execute(
                    "UPDATE agent_session_preparations SET payload_json=?2 WHERE invocation_id=?1",
                    params![p.invocation_id.as_str(), to_json(p)?],
                )
                .map_err(sql_write("save preparation"))?;
            if n != 1 {
                return Err(not_found("Preparation not found"));
            }
            Ok(())
        })
    }
    pub(super) fn commit_prepared_binding_record(
        &self,
        p: &SessionPreparation,
        binding: AgentRuntimeBinding,
        at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("commit ready Session target", |tx| {
            let invocation = required_invocation(tx, &p.invocation_id)?;
            if !invocation.status.is_active() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Submission is no longer active",
                ));
            }
            let profile = p
                .current_resolution
                .as_ref()
                .ok_or_else(|| not_found("Resolved profile unavailable"))?;
            let options = p
                .resolution
                .as_ref()
                .ok_or_else(|| not_found("Invocation resolution unavailable"))?;
            tx.execute(
                "UPDATE agent_sessions SET execution_target_json=?2,working_directory=?3,
                 workspace_origin='explicit',external_context_id=?4,runtime_version=?5,
                 session_profile_json=COALESCE(session_profile_json,?6),updated_at=?7 WHERE id=?1",
                params![
                    p.session_id.as_str(),
                    p.resolved_target.as_ref().map(to_json).transpose()?,
                    p.resolved_working_directory,
                    binding.external_context_id.as_ref().map(|id| id.as_str()),
                    binding.runtime_version,
                    to_json(profile)?,
                    timestamp(at),
                ],
            )
            .map_err(sql_write("commit ready target"))?;
            tx.execute(
                "INSERT INTO agent_session_current_execution(session_id,resolution_json)
                 VALUES(?1,?2) ON CONFLICT(session_id)
                 DO UPDATE SET resolution_json=excluded.resolution_json",
                params![p.session_id.as_str(), to_json(profile)?],
            )
            .map_err(sql_write("commit current execution profile"))?;
            tx.execute(
                "UPDATE agent_session_invocations SET requested_options_json=?2 WHERE id=?1",
                params![
                    p.invocation_id.as_str(),
                    to_json(&crate::agent_sessions::application::runtime_options(
                        &options.selections
                    ))?,
                ],
            )
            .map_err(sql_write("resolve accepted invocation options"))?;
            tx.execute(
                "UPDATE agent_session_preparations SET payload_json=?2 WHERE invocation_id=?1",
                params![p.invocation_id.as_str(), to_json(p)?],
            )
            .map_err(sql_write("freeze invocation execution"))?;
            required_session(tx, &p.session_id)
        })
    }

    pub(super) fn retry_preparation_record(
        &self,
        id: &AgentInvocationId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.write("retry preparation", |tx| {
            let mut preparation =
                read_record(tx, id)?.ok_or_else(|| not_found("Preparation not found"))?;
            if !matches!(
                preparation.phase,
                PreparationPhase::Failed | PreparationPhase::Canceled
            ) || !preparation.can_retry
                || preparation.delivery_started
                || invocation_launch_accepted_at_from(tx, id)?.is_some()
            {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Only known preparation failures can be retried",
                ));
            }
            let latest: String = tx
                .query_row(
                    "SELECT id FROM agent_session_invocations WHERE session_id=?1
                 ORDER BY created_at DESC,rowid DESC LIMIT 1",
                    [preparation.session_id.as_str()],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("check retry boundary"))?;
            if latest != id.as_str() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "A later submission exists; send a new prompt instead",
                ));
            }
            let active = active_invocation(tx, &preparation.session_id)?;
            if active
                .as_ref()
                .is_some_and(|invocation| invocation.id != *id)
            {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Another submission is active",
                ));
            }
            tx.execute(
                "UPDATE agent_session_invocations SET status='pending',started_at=NULL,
                 completed_at=NULL,runtime_error_json=NULL,effective_options_json=NULL,
                 exit_code=NULL,signal=NULL,updated_at=?2 WHERE id=?1",
                params![id.as_str(), timestamp(at)],
            )
            .map_err(sql_write("retry pending invocation"))?;
            preparation.phase = PreparationPhase::Accepted;
            preparation.error = None;
            preparation.can_retry = false;
            tx.execute(
                "UPDATE agent_session_preparations SET payload_json=?2 WHERE invocation_id=?1",
                params![id.as_str(), to_json(&preparation)?],
            )
            .map_err(sql_write("retry preparation state"))?;
            Ok(())
        })
    }
}
