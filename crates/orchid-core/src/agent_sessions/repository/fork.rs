//! Conversation forks copy readable history, never native handles or execution queues.
use super::*;
use crate::agent_sessions::{domain::*, ports::AgentSessionHistory};
use crate::execution_targets::domain::SessionExecutionTarget;
pub const SCHEMA: &str = r#"CREATE TABLE IF NOT EXISTS agent_session_forks(session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id),source_session_id TEXT NOT NULL,source_invocation_id TEXT,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS agent_session_fork_requests(session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id),request_hash TEXT NOT NULL);"#;
impl SqliteAgentSessionRepository {
    pub fn conversation_fork(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<crate::synchronization::ConversationFork>, String> {
        self.database
            .read("read conversation fork", |db| read_fork(db, id))
            .map_err(|e| e.to_string())
    }
    pub fn fork_conversation(
        &self,
        source: &AgentSessionHistory,
        target: SessionExecutionTarget,
    ) -> Result<AgentSession, String> {
        self.fork_conversation_with_id(
            AgentSessionId::new(uuid::Uuid::new_v4().to_string()).map_err(|e| e.to_string())?,
            source,
            target,
        )
    }
    pub fn fork_conversation_with_id(
        &self,
        id: AgentSessionId,
        source: &AgentSessionHistory,
        target: SessionExecutionTarget,
    ) -> Result<AgentSession, String> {
        self.fork_conversation_with_context(id, source, target, None)
    }
    pub fn fork_conversation_with_context(
        &self,
        id: AgentSessionId,
        source: &AgentSessionHistory,
        target: SessionExecutionTarget,
        prefix: Option<super::super::ports::InitialPromptPrefix>,
    ) -> Result<AgentSession, String> {
        use sha2::{Digest, Sha256};
        target.execution.validate()?;
        let fingerprint=format!("{:x}",Sha256::digest(serde_json::to_vec(&serde_json::json!({"source":source.session.id,"turns":source.invocations.iter().filter(|t|t.invocation.status==AgentInvocationStatus::Completed).collect::<Vec<_>>(),"target":target,"prefix":prefix})).map_err(|e|e.to_string())?));
        // Lost acknowledgements can be retried even if the target was moved after acceptance.
        if let Some(existing) = self.get_session(&id).map_err(|e| e.to_string())? {
            let previous = self
                .database
                .read("read accepted fork", |db| {
                    db.query_row(
                        "SELECT request_hash FROM agent_session_fork_requests WHERE session_id=?1",
                        [id.as_str()],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                })
                .map_err(|e| e.to_string())?;
            if previous.as_deref() == Some(&fingerprint) {
                return Ok(existing);
            }
            return Err("Fork identity already used for different source context or target".into());
        }
        if target.execution.is_remote() || !std::path::Path::new(&target.path).is_dir() {
            return Err("Choose an available worktree on the destination device".into());
        }
        let now = chrono::Utc::now();
        let session = AgentSession {
            id,
            title: source.session.title.clone(),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: None,
                runtime_version: None,
            },
            working_directory: Some(target.path.clone()),
            execution_target: Some(target),
            workspace_origin: Some("explicit".into()),
            requested_options: source.session.requested_options.clone(),
            session_profile: None,
            harness_version: None,
            assigned_identity: source.session.assigned_identity.clone(),
            created_at: now,
            updated_at: now,
        };
        self.write("fork synchronized conversation", |tx| {
            if let Some(existing) = get_session_from(tx, &session.id)? {
                let parent: Option<String> = tx
                    .query_row(
                        "SELECT source_session_id FROM agent_session_forks WHERE session_id=?1",
                        [session.id.as_str()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(sql_unavailable("read fork identity"))?;
                let accepted: Option<String> = tx
                    .query_row(
                        "SELECT request_hash FROM agent_session_fork_requests WHERE session_id=?1",
                        [session.id.as_str()],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(sql_unavailable("read fork request"))?;
                if accepted.as_deref() != Some(&fingerprint)
                    || parent.as_deref() != Some(source.session.id.as_str())
                    || existing.execution_target != session.execution_target
                {
                    return Err(RepositoryError::new(
                        RepositoryErrorKind::Conflict,
                        "Fork identity already used for different source or target",
                    ));
                }
                return Ok(existing);
            }
            insert_session(tx, &session)?;
            let mut last = None;
            for (ordinal, turn) in source
                .invocations
                .iter()
                .filter(|turn| turn.invocation.status == AgentInvocationStatus::Completed)
                .enumerate()
            {
                let mut invocation = turn.invocation.clone();
                invocation.id = AgentInvocationId::new(uuid::Uuid::new_v4().to_string())
                    .map_err(contract_error)?;
                invocation.session_id = session.id.clone();
                invocation.started_at = None;
                invocation.completed_at = None;
                invocation.exit_code = None;
                invocation.signal = None;
                invocation.runtime_error = None;
                invocation.diagnostics.clear();
                insert_invocation(tx, &invocation)?;
                super::history::allocate_position(
                    tx,
                    &session.id,
                    Some(invocation.id.as_str()),
                    None,
                )?;
                tx.execute(
                    "INSERT INTO agent_session_imported_turns VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        invocation.id.as_str(),
                        session.id.as_str(),
                        turn.invocation.id.as_str(),
                        ordinal as i64,
                        turn.invocation.started_at.map(|t| t.timestamp()),
                        turn.invocation.completed_at.map(|t| t.timestamp())
                    ],
                )
                .map_err(sql_write("record fork provenance"))?;
                for event in &turn.events {
                    let mut event = event.clone();
                    event.id = AgentRuntimeEventId::new(uuid::Uuid::new_v4().to_string())
                        .map_err(contract_error)?;
                    event.invocation_id = invocation.id.clone();
                    insert_event(tx, &event)?;
                }
                last = Some(turn.invocation.id.as_str());
            }
            tx.execute(
                "INSERT INTO agent_session_forks VALUES(?1,?2,?3,?4)",
                params![
                    session.id.as_str(),
                    source.session.id.as_str(),
                    last,
                    timestamp(now)
                ],
            )
            .map_err(sql_write("record conversation fork"))?;
            tx.execute(
                "INSERT INTO agent_session_fork_requests VALUES(?1,?2)",
                params![session.id.as_str(), fingerprint],
            )
            .map_err(sql_write("record fork acceptance"))?;
            if let Some(prefix) = &prefix {
                tx.execute(
                    "INSERT INTO agent_session_initial_prompt_prefixes VALUES(?1,?2,?3)",
                    params![session.id.as_str(), to_json(prefix)?, timestamp(now)],
                )
                .map_err(sql_write("copy initial conversation context"))?;
            }
            Ok(session.clone())
        })
        .map_err(|e| e.to_string())
    }
}

pub(super) fn read_fork(
    db: &Connection,
    id: &AgentSessionId,
) -> Result<Option<crate::synchronization::ConversationFork>, String> {
    db.query_row("SELECT source_session_id,source_invocation_id,created_at FROM agent_session_forks WHERE session_id=?1",[id.as_str()],|r|Ok(crate::synchronization::ConversationFork{source_session_id:r.get(0)?,source_invocation_id:r.get(1)?,created_at:r.get(2)?})).optional().map_err(|e|e.to_string())
}
