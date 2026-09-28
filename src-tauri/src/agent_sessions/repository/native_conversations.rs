//! Provider-owned conversation bindings and the first prompt prefix delivered to a Session.
use super::*;
use crate::{
    agent_sessions::{domain::ProviderSessionBinding, ports::InitialPromptPrefix},
    execution_targets::domain::ExecutionBinding,
};

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_provider_bindings (
 session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
 execution_key TEXT NOT NULL,
 binding_json TEXT NOT NULL CHECK(json_valid(binding_json)),
 updated_at TEXT NOT NULL,
 PRIMARY KEY (session_id, execution_key)
);
CREATE TABLE IF NOT EXISTS agent_session_initial_prompt_prefixes (
 session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
 prefix_json TEXT NOT NULL CHECK(json_valid(prefix_json)),
 recorded_at TEXT NOT NULL
);
"#;

fn execution_key(execution: &ExecutionBinding) -> Result<String, RepositoryError> {
    to_json(execution)
}

/// Within a prepared-binding commit, retain the source route's provider-owned conversation. The
/// destination binding remains cached too; selecting it never consumes or relocates its handle.
pub(super) fn retain_source_provider_binding(
    tx: &rusqlite::Transaction<'_>,
    session_id: &AgentSessionId,
    source: Option<&ProviderSessionBinding>,
    at: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    let Some(source) = source else {
        return Ok(());
    };
    tx.execute(
        "INSERT INTO agent_session_provider_bindings(session_id,execution_key,binding_json,updated_at)
         VALUES(?1,?2,?3,?4) ON CONFLICT(session_id,execution_key)
         DO UPDATE SET binding_json=excluded.binding_json,updated_at=excluded.updated_at",
        params![
            session_id.as_str(),
            execution_key(&source.location)?,
            to_json(source)?,
            timestamp(at)
        ],
    )
    .map_err(sql_write("retain provider session binding"))?;
    Ok(())
}

impl SqliteAgentSessionRepository {
    pub(super) fn read_provider_session_binding(
        &self,
        session_id: &AgentSessionId,
        execution: &ExecutionBinding,
    ) -> Result<Option<ProviderSessionBinding>, RepositoryError> {
        self.read("read provider session binding", |connection| {
            let json: Option<String> = connection
                .query_row(
                    "SELECT binding_json FROM agent_session_provider_bindings WHERE session_id=?1 AND execution_key=?2",
                    params![session_id.as_str(), execution_key(execution)?],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read provider session binding"))?;
            json.map(|json| {
                serde_json::from_str(&json).map_err(|error| {
                    RepositoryError::new(RepositoryErrorKind::InvalidState, error.to_string())
                })
            })
            .transpose()
        })
    }

    pub(super) fn insert_initial_prompt_prefix(
        &self,
        session_id: &AgentSessionId,
        prefix: &InitialPromptPrefix,
    ) -> Result<(), RepositoryError> {
        self.write("record initial prompt prefix", |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO agent_session_initial_prompt_prefixes(session_id,prefix_json,recorded_at) VALUES(?1,?2,?3)",
                params![session_id.as_str(), to_json(prefix)?, timestamp(Utc::now())],
            )
            .map_err(sql_write("record initial prompt prefix"))?;
            Ok(())
        })
    }

    pub(super) fn read_initial_prompt_prefix(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<InitialPromptPrefix>, RepositoryError> {
        self.read("read initial prompt prefix", |connection| {
            let json: Option<String> = connection
                .query_row(
                    "SELECT prefix_json FROM agent_session_initial_prompt_prefixes WHERE session_id=?1",
                    [session_id.as_str()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read initial prompt prefix"))?;
            json.map(|json| {
                serde_json::from_str(&json).map_err(|error| {
                    RepositoryError::new(RepositoryErrorKind::InvalidState, error.to_string())
                })
            })
            .transpose()
        })
    }
}
