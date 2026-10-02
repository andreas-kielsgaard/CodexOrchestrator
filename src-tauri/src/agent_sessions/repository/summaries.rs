//! Batched sidebar facts; only active turns need request-event projection.
use super::*;
use std::collections::HashMap;

pub(super) fn list(
    repository: &SqliteAgentSessionRepository,
    query: ListAgentSessionsQuery,
) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
    repository.read("list Agent Session summaries", |connection| {
        let tx = connection.unchecked_transaction().map_err(sql_unavailable("begin summary list"))?;
        let sessions = list_sessions_from(&tx, query)?;
        let mut statement = tx.prepare("WITH ranked AS (
            SELECT session_id,id,status,submitted_text,COUNT(*) OVER(PARTITION BY session_id) AS total,
            ROW_NUMBER() OVER(PARTITION BY session_id ORDER BY created_at DESC,id DESC) AS rn FROM agent_session_invocations
        ), completed AS (
            SELECT session_id,id,ROW_NUMBER() OVER(PARTITION BY session_id ORDER BY completed_at DESC,id DESC) AS rn
            FROM agent_session_invocations WHERE status='completed'
        ), queued AS (
            SELECT session_id,COUNT(*) AS total FROM agent_session_queue_entries WHERE state IN ('pending','executing','blocked') GROUP BY session_id
        ) SELECT r.session_id,r.id,r.status,r.submitted_text,r.total,c.id,
            COALESCE(q.total,0)>0,a.seen_completed_id,COALESCE(a.force_unread,0)
            FROM ranked r LEFT JOIN completed c ON c.session_id=r.session_id AND c.rn=1
            LEFT JOIN queued q ON q.session_id=r.session_id
            LEFT JOIN agent_session_attention a ON a.session_id=r.session_id WHERE r.rn=1")
            .map_err(sql_unavailable("prepare sidebar facts"))?;
        let facts = statement.query_map([], |row| Ok((row.get::<_,String>(0)?, (
            row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,u64>(4)?,
            row.get::<_,Option<String>>(5)?,row.get::<_,bool>(6)?,row.get::<_,Option<String>>(7)?,row.get::<_,bool>(8)?
        )))).map_err(sql_unavailable("read sidebar facts"))?.collect::<Result<HashMap<_,_>,_>>()
            .map_err(sql_unavailable("decode sidebar facts"))?;
        let summaries = sessions.into_iter().map(|session| {
            let mut summary = AgentSessionSummary { session, invocation_count:0, latest_completed_id:None,
                latest_invocation_status:None, latest_submitted_text:None, pending_request_count:0, completion_state:None };
            if let Some((id,status,text,total,completed,queued,seen,force)) = facts.get(summary.session.id.as_str()) {
                let status = parse_status(status)?;
                summary.invocation_count = *total;
                summary.latest_invocation_status = Some(status);
                summary.latest_submitted_text = Some(text.clone());
                summary.latest_completed_id = completed.clone();
                // Intermediate messages never contribute: only terminal completed invocations do.
                if completed.is_some() && (*force || completed != seen) {
                    summary.completion_state = if *queued || status.is_active() { Some("continued".into()) }
                        else if status.is_terminal() { Some("done".into()) } else { None };
                }
                if status.is_active() {
                    let id = AgentInvocationId::new(id.clone()).map_err(contract_error)?;
                    if let Some(invocation) = get_invocation_from(&tx, &id)? {
                        summary.pending_request_count = super::super::interactions::pending_request_count(&[AgentInvocationHistory { last_event_sequence: None,
                            invocation, launch_accepted_at:None, events:request_events(&tx,&id)?, import_provenance:None
                        }]);
                    }
                }
            }
            Ok(summary)
        }).collect::<Result<Vec<_>, RepositoryError>>()?;
        drop(statement);
        tx.commit().map_err(sql_unavailable("commit summary list"))?;
        Ok(summaries)
    })
}

fn request_events(
    connection: &Connection,
    id: &AgentInvocationId,
) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
    let mut statement = connection.prepare("SELECT id,invocation_id,sequence,source,raw_payload_json,normalized_json,recorded_at
        FROM agent_session_runtime_events WHERE invocation_id=?1 AND source='runtime'
        AND json_extract(raw_payload_json,'$.kind') IN ('runtime_request_opened','runtime_request_unsupported','runtime_request_response') ORDER BY sequence")
        .map_err(sql_unavailable("prepare pending request facts"))?;
    let rows = statement
        .query_map([id.as_str()], event_row)
        .map_err(sql_unavailable("read pending request facts"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_unavailable("decode pending request facts"))?;
    rows.into_iter()
        .map(map_event_row)
        .collect()
}
