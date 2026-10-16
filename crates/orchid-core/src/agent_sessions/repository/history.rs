use super::*;
use crate::agent_sessions::history::*;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_history_positions (
 session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
 ordinal INTEGER NOT NULL,
 invocation_id TEXT UNIQUE REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
 operation_id TEXT UNIQUE REFERENCES agent_session_target_operations(id) ON DELETE CASCADE,
 CHECK((invocation_id IS NOT NULL) != (operation_id IS NOT NULL)),
 PRIMARY KEY(session_id,ordinal)
);
INSERT OR IGNORE INTO agent_session_history_positions(session_id,ordinal,invocation_id,operation_id)
SELECT i.session_id,
 COALESCE((SELECT MAX(p.ordinal) FROM agent_session_history_positions p WHERE p.session_id=i.session_id),-1)
 + ROW_NUMBER() OVER(PARTITION BY i.session_id ORDER BY i.created_at,i.id),i.id,NULL
FROM agent_session_invocations i
WHERE i.status NOT IN ('pending','running')
 AND NOT EXISTS(SELECT 1 FROM agent_session_history_positions p WHERE p.invocation_id=i.id)
ORDER BY i.session_id,i.created_at,i.id;
"#;

pub(super) fn allocate_position(
    tx: &Connection,
    session_id: &AgentSessionId,
    invocation_id: Option<&str>,
    operation_id: Option<&str>,
) -> Result<(), RepositoryError> {
    tx.execute("INSERT OR IGNORE INTO agent_session_history_positions(session_id,ordinal,invocation_id,operation_id) VALUES(?1,COALESCE((SELECT MAX(ordinal)+1 FROM agent_session_history_positions WHERE session_id=?1),0),?2,?3)",params![session_id.as_str(),invocation_id,operation_id]).map_err(sql_write("allocate session history position"))?;
    Ok(())
}

pub(super) fn read_timeline(
    connection: &Connection,
    session_id: &AgentSessionId,
) -> Result<SessionTimeline, RepositoryError> {
    let positions = connection.prepare("SELECT ordinal,invocation_id,operation_id FROM agent_session_history_positions WHERE session_id=?1 ORDER BY ordinal").map_err(sql_unavailable("read history positions"))?.query_map([session_id.as_str()],|r|Ok(SessionHistoryPosition { ordinal:r.get(0)?,invocation_id:r.get(1)?,operation_id:r.get(2)? })).map_err(sql_unavailable("read history positions"))?.collect::<Result<Vec<_>,_>>().map_err(sql_unavailable("read history position"))?;
    let receipts: Vec<String> = connection.prepare("SELECT json_extract(o.payload_json,'$.receipt') FROM agent_session_target_operations o JOIN agent_session_history_positions p ON p.operation_id=o.id WHERE p.session_id=?1 AND json_type(o.payload_json,'$.receipt')='object' ORDER BY p.ordinal").map_err(sql_unavailable("read transitions"))?.query_map([session_id.as_str()],|r|r.get(0)).map_err(sql_unavailable("read transitions"))?.collect::<Result<_,_>>().map_err(sql_unavailable("read transition"))?;
    let transitions = receipts
        .into_iter()
        .map(|json| from_json(&json, "workspace receipt"))
        .collect::<Result<_, _>>()?;
    let providers = connection.prepare("SELECT i.id,json_extract(s.snapshot_json,'$.target.execution.provider') FROM agent_session_invocations i JOIN agent_session_invocation_launch_acceptances l ON l.invocation_id=i.id JOIN agent_session_invocation_execution_snapshots s ON s.invocation_id=i.id WHERE i.session_id=?1 AND json_type(s.snapshot_json,'$.target.execution.provider')='text'").map_err(sql_unavailable("read delivered providers"))?.query_map([session_id.as_str()],|r|Ok(InvocationProviderIdentity { invocation_id:r.get(0)?,provider:r.get(1)? })).map_err(sql_unavailable("read delivered providers"))?.collect::<Result<_,_>>().map_err(sql_unavailable("read delivered provider"))?;
    Ok(SessionTimeline {
        positions,
        transitions,
        providers,
    })
}
