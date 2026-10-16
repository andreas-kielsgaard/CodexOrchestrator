use super::*;

impl SqliteAgentSessionRepository {
    pub fn mark_attention(&self, id: &AgentSessionId, unread: bool) -> Result<(), RepositoryError> {
        self.write("acknowledge Session completion", |tx| {
            if get_session_from(tx, id)?.is_none() {
                return Err(RepositoryError::new(RepositoryErrorKind::NotFound, "Session not found"));
            }
            tx.execute("INSERT INTO agent_session_attention(session_id,seen_completed_id,force_unread)
                VALUES(?1,(SELECT id FROM agent_session_invocations WHERE session_id=?1 AND status='completed' ORDER BY completed_at DESC,id DESC LIMIT 1),?2)
                ON CONFLICT(session_id) DO UPDATE SET seen_completed_id=excluded.seen_completed_id,force_unread=excluded.force_unread",
                params![id.as_str(), unread]).map_err(sql_write("store Session attention"))?;
            Ok(())
        })
    }
}
