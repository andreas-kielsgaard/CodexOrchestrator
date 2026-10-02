//! The chat uses completed item snapshots, not duplicate wire fragments and ambient telemetry.
use super::*;

pub(super) fn events(
    connection: &Connection,
    id: &AgentInvocationId,
) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
    let mut statement=connection.prepare("SELECT id,invocation_id,sequence,source,raw_payload_json,normalized_json,recorded_at
      FROM agent_session_runtime_events WHERE invocation_id=?1 AND (
        normalized_json IS NOT NULL OR COALESCE(json_extract(raw_payload_json,'$.method'),'') NOT IN (
          'item/commandExecution/outputDelta','item/agentMessage/delta','item/reasoning/textDelta',
          'item/reasoning/summaryTextDelta','item/reasoning/summaryPartAdded','turn/diff/updated',
          'account/rateLimits/updated','mcpServer/startupStatus/updated','skills/changed','thread/status/changed'
        )) ORDER BY sequence ASC").map_err(sql_unavailable("prepare display event history"))?;
    let rows = statement
        .query_map([id.as_str()], event_row)
        .map_err(sql_unavailable("read display event history"))?;
    let mut events: Vec<AgentRuntimeEvent> = collect_mapped(rows, map_event_row)?;
    for event in &mut events {
        if event.normalized.as_ref().is_some_and(|n| {
            n.kind == super::super::domain::NormalizedRuntimeEventKind::ToolActivity
        }) {
            let mut deferred = compact(&mut event.raw_payload);
            if let Some(details) = event.normalized.as_mut().and_then(|n| n.details.as_mut()) {
                deferred |= compact(details);
            }
            if deferred {
                mark_deferred(&mut event.raw_payload, event.id.as_str());
                if let Some(details) = event.normalized.as_mut().and_then(|n| n.details.as_mut()) {
                    mark_deferred(details, event.id.as_str());
                }
            }
        }
    }
    Ok(events)
}

fn compact(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) if text.len() > 4096 => {
            *text = format!(
                "{}\n[Full data available on expansion]",
                text.chars().take(2000).collect::<String>()
            );
            true
        }
        serde_json::Value::Array(values) => values
            .iter_mut()
            .fold(false, |changed, value| compact(value) || changed),
        serde_json::Value::Object(values) => values
            .values_mut()
            .fold(false, |changed, value| compact(value) || changed),
        _ => false,
    }
}
fn mark_deferred(value: &mut serde_json::Value, id: &str) {
    if let Some(object) = value.as_object_mut() {
        object.insert("_orchidDeferredEventId".into(), id.into());
    } else {
        *value = serde_json::json!({"value":value.take(),"_orchidDeferredEventId":id});
    }
}

impl SqliteAgentSessionRepository {
    pub(crate) fn read_event_details(
        &self,
        session: &AgentSessionId,
        id: &str,
    ) -> Result<Option<AgentRuntimeEvent>, RepositoryError> {
        self.read("read requested action details",|connection|{
            let row=connection.query_row("SELECT e.id,e.invocation_id,e.sequence,e.source,e.raw_payload_json,e.normalized_json,e.recorded_at
              FROM agent_session_runtime_events e JOIN agent_session_invocations i ON i.id=e.invocation_id
              WHERE e.id=?1 AND i.session_id=?2",params![id,session.as_str()],event_row).optional().map_err(sql_unavailable("read action details"))?;
            row.map(map_event_row).transpose()
        })
    }
}
