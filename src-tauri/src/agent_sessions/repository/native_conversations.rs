//! Parked native conversations of providers a Session no longer runs on, and the initial prompt
//! prefix a provider new to the Session receives.
use super::*;
use crate::agent_sessions::{domain::ParkedNativeConversation, ports::InitialPromptPrefix};

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_parked_conversations (
 session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
 provider TEXT NOT NULL,
 conversation_json TEXT NOT NULL CHECK(json_valid(conversation_json)),
 parked_at TEXT NOT NULL,
 PRIMARY KEY (session_id, provider)
);
CREATE TABLE IF NOT EXISTS agent_session_initial_prompt_prefixes (
 session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
 prefix_json TEXT NOT NULL CHECK(json_valid(prefix_json)),
 recorded_at TEXT NOT NULL
);
"#;

/// Within a prepared-binding commit: park the source provider's conversation and make the
/// destination provider's parked conversation current by removing it from the parked set.
pub(super) fn swap_parked_conversations(
    tx: &rusqlite::Transaction<'_>,
    session_id: &AgentSessionId,
    parked_source: Option<&ParkedNativeConversation>,
    destination_provider: &str,
    at: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    if let Some(parked) = parked_source {
        tx.execute(
            "INSERT INTO agent_session_parked_conversations(session_id,provider,conversation_json,parked_at)
             VALUES(?1,?2,?3,?4) ON CONFLICT(session_id,provider)
             DO UPDATE SET conversation_json=excluded.conversation_json,parked_at=excluded.parked_at",
            params![session_id.as_str(), parked.provider, to_json(parked)?, timestamp(at)],
        )
        .map_err(sql_write("park native conversation"))?;
    }
    tx.execute(
        "DELETE FROM agent_session_parked_conversations WHERE session_id=?1 AND provider=?2",
        params![session_id.as_str(), destination_provider],
    )
    .map_err(sql_write("resume parked native conversation"))?;
    Ok(())
}

impl SqliteAgentSessionRepository {
    pub(super) fn read_parked_conversation(
        &self,
        session_id: &AgentSessionId,
        provider: &str,
    ) -> Result<Option<ParkedNativeConversation>, RepositoryError> {
        self.read("read parked native conversation", |connection| {
            let json: Option<String> = connection
                .query_row(
                    "SELECT conversation_json FROM agent_session_parked_conversations WHERE session_id=?1 AND provider=?2",
                    params![session_id.as_str(), provider],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read parked native conversation"))?;
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
