use super::*;
use crate::agent_sessions::{
    domain::{AgentInvocation, AgentInvocationInputProvenance, AgentRuntimeOptions},
    preparation::{PreparationPhase, SessionPreparation},
    queue::{
        validate_queue_entry, SessionQueueEntry, SessionQueueEntryItem, SessionQueueEntryState,
        SessionQueueSnapshot,
    },
    target_operation::{
        planned_actions, replan_actions, validate_target_operation, SessionTargetOperation,
        TargetOperationActionStatus, TargetOperationPhase,
    },
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyTargetTransition {
    session_id: AgentSessionId,
    source_target: SessionExecutionTarget,
    destination_selection: SessionExecutionSelection,
    phase: LegacyTargetTransitionPhase,
    resolved_target: Option<SessionExecutionTarget>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyTargetTransitionPhase {
    Pending,
    Running,
    Ready,
    Failed,
    Canceled,
}

pub(crate) const SCHEMA: &str = r#"
DROP INDEX IF EXISTS agent_session_one_active_invocation;
CREATE UNIQUE INDEX IF NOT EXISTS agent_session_one_running_invocation
ON agent_session_invocations(session_id)
WHERE status = 'running';

CREATE TABLE IF NOT EXISTS agent_session_queue_state (
  session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_session_target_operations (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
);

CREATE INDEX IF NOT EXISTS agent_session_target_operations_session
ON agent_session_target_operations(session_id);

CREATE TABLE IF NOT EXISTS agent_session_queue_entries (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('message', 'target_operation')),
  state TEXT NOT NULL CHECK(state IN ('pending', 'executing', 'blocked', 'completed', 'canceled')),
  invocation_id TEXT REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
  operation_id TEXT REFERENCES agent_session_target_operations(id) ON DELETE CASCADE,
  blocked_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK((kind='message' AND invocation_id IS NOT NULL AND operation_id IS NULL)
     OR (kind='target_operation' AND operation_id IS NOT NULL AND invocation_id IS NULL)),
  UNIQUE(session_id, ordinal),
  UNIQUE(invocation_id),
  UNIQUE(operation_id)
);

CREATE INDEX IF NOT EXISTS agent_session_queue_unfinished
ON agent_session_queue_entries(session_id, ordinal)
WHERE state IN ('pending', 'executing', 'blocked');

INSERT OR IGNORE INTO agent_session_queue_state(session_id,revision,created_at,updated_at)
SELECT id,0,created_at,updated_at FROM agent_sessions;

INSERT OR IGNORE INTO agent_session_queue_entries(
  id,session_id,ordinal,kind,state,invocation_id,operation_id,blocked_reason,created_at,updated_at
)
SELECT 'message:' || i.id,i.session_id,
       COALESCE((SELECT MAX(q.ordinal) FROM agent_session_queue_entries q WHERE q.session_id=i.session_id),-1)
         + ROW_NUMBER() OVER (PARTITION BY i.session_id ORDER BY i.created_at,i.rowid),
       'message',CASE WHEN i.status='running' THEN 'executing' ELSE 'pending' END,
       i.id,NULL,NULL,i.created_at,i.updated_at
FROM agent_session_invocations i
WHERE i.status IN ('pending','running')
ORDER BY i.session_id,i.created_at,i.rowid;
"#;

pub(crate) fn migrate_legacy_target_transitions(connection: &Connection) -> Result<(), String> {
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_session_target_transitions')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("Unable to inspect legacy target transitions: {error}"))?;
    if !exists {
        return Ok(());
    }
    let transitions = {
        let mut statement = connection
            .prepare("SELECT session_id,payload_json FROM agent_session_target_transitions")
            .map_err(|error| {
                format!("Unable to prepare legacy target transition migration: {error}")
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| format!("Unable to read legacy target transitions: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Unable to read legacy target transition: {error}"))?;
        rows
    };
    for (stored_session_id, payload) in transitions {
        let value: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|error| format!("Invalid legacy target transition: {error}"))?;
        let transition: LegacyTargetTransition = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid legacy target transition: {error}"))?;
        if transition.phase == LegacyTargetTransitionPhase::Canceled {
            connection
                .execute(
                    "DELETE FROM agent_session_target_transitions WHERE session_id=?1",
                    [&stored_session_id],
                )
                .map_err(|error| format!("Unable to retire canceled target transition: {error}"))?;
            continue;
        }
        let session = required_session(connection, &transition.session_id)
            .map_err(|error| error.to_string())?;
        ensure_state(connection, &session).map_err(|error| error.to_string())?;
        let now = transition.updated_at;
        let operation_id = format!("legacy-transition:{}", transition.session_id.as_str());
        let actions = planned_actions(
            Some(&transition.source_target),
            &transition.destination_selection,
        );
        let mut operation = SessionTargetOperation {
            id: operation_id.clone(),
            session_id: transition.session_id.clone(),
            source_target: Some(transition.source_target),
            destination_selection: transition.destination_selection,
            phase: TargetOperationPhase::Pending,
            actions,
            staged_target: None,
            activated_binding: None,
            error: None,
            generation: 0,
            created_at: transition.created_at,
            updated_at: now,
        };
        let entry_state = match transition.phase {
            LegacyTargetTransitionPhase::Pending => SessionQueueEntryState::Pending,
            LegacyTargetTransitionPhase::Ready => {
                operation.phase = TargetOperationPhase::Staged;
                operation.staged_target = transition.resolved_target;
                for action in operation.actions.iter_mut().take(4) {
                    action.status = TargetOperationActionStatus::Completed;
                    action.detail = Some("Recovered from the previous target transition".into());
                }
                SessionQueueEntryState::Pending
            }
            LegacyTargetTransitionPhase::Running => {
                operation.phase = TargetOperationPhase::Blocked;
                operation.error = Some(
                    "Application restarted during the previous target transition. Retry continues from this queue entry."
                        .into(),
                );
                SessionQueueEntryState::Blocked
            }
            LegacyTargetTransitionPhase::Failed => {
                operation.phase = TargetOperationPhase::Blocked;
                operation.error = Some(
                    transition
                        .error
                        .unwrap_or_else(|| "The previous target transition failed".into()),
                );
                SessionQueueEntryState::Blocked
            }
            LegacyTargetTransitionPhase::Canceled => unreachable!(),
        };
        insert_operation(connection, &operation).map_err(|error| error.to_string())?;
        let operation_entry = SessionQueueEntry {
            id: format!("target:{operation_id}"),
            session_id: session.id.clone(),
            ordinal: next_ordinal(connection, &session.id).map_err(|error| error.to_string())?,
            state: entry_state,
            item: SessionQueueEntryItem::TargetOperation {
                operation_id: operation_id.clone(),
            },
            blocked_reason: operation.error.clone(),
            created_at: operation.created_at,
            updated_at: now,
        };
        insert_entry(connection, &operation_entry).map_err(|error| error.to_string())?;

        if let Some(prompt) = value.get("queuedPrompt") {
            let text = prompt.get("text").and_then(serde_json::Value::as_str);
            let id = prompt
                .get("clientMessageId")
                .and_then(serde_json::Value::as_str);
            if let (Some(text), Some(id)) = (text, id) {
                let invocation_id =
                    AgentInvocationId::new(id.to_string()).map_err(|error| error.to_string())?;
                let invocation = AgentInvocation { attachments: vec![], id: invocation_id.clone(),
                    session_id: session.id.clone(),
                    submitted_text: text.to_string(),
                    input_provenance: AgentInvocationInputProvenance::User,
                    status: AgentInvocationStatus::Pending,
                    requested_options: AgentRuntimeOptions::default(),
                    effective_options: None,
                    started_at: None,
                    completed_at: None,
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                    diagnostics: Vec::new(),
                    created_at: now,
                    updated_at: now,
                };
                validate_new_invocation(&session, None, &invocation)
                    .map_err(|error| error.to_string())?;
                connection
                    .execute(
                        "INSERT INTO agent_session_invocations (id,session_id,submitted_text,input_provenance,status,requested_options_json,effective_options_json,started_at,completed_at,exit_code,signal,runtime_error_json,created_at,updated_at)
                         VALUES(?1,?2,?3,'user','pending',?4,NULL,NULL,NULL,NULL,NULL,NULL,?5,?5)",
                        params![
                            invocation.id.as_str(),
                            invocation.session_id.as_str(),
                            invocation.submitted_text,
                            to_json(&invocation.requested_options).map_err(|error| error.to_string())?,
                            timestamp(now),
                        ],
                    )
                    .map_err(|error| format!("Unable to migrate queued transition invocation: {error}"))?;
                let preparation = SessionPreparation {
                    invocation_id: invocation_id.clone(),
                    session_id: session.id.clone(),
                    phase: PreparationPhase::Accepted,
                    steps: Vec::new(),
                    error: None,
                    can_retry: false,
                    selection: None,
                    source_target: session.execution_target.clone(),
                    source_binding: session.runtime_binding.clone(),
                    prepared_binding: None,
                    source_provider_binding: None,
                    resolved_target: None,
                    resolved_working_directory: session.working_directory.clone(),
                    accepted_working_directory: session.working_directory.clone(),
                    context: Vec::new(),
                    current_resolution: None,
                    resolution: None,
                    execution_snapshot: None,
                    model: None,
                    reasoning_mode: None,
                    sandbox_mode: None,
                    delivery_started: false,
                };
                connection
                    .execute(
                        "INSERT INTO agent_session_preparations(invocation_id,session_id,payload_json) VALUES(?1,?2,?3)",
                        params![invocation_id.as_str(), session.id.as_str(), to_json(&preparation).map_err(|error| error.to_string())?],
                    )
                    .map_err(|error| format!("Unable to migrate queued transition prompt: {error}"))?;
                let message_entry = SessionQueueEntry {
                    id: format!("message:{}", invocation_id.as_str()),
                    session_id: session.id.clone(),
                    ordinal: next_ordinal(connection, &session.id)
                        .map_err(|error| error.to_string())?,
                    state: SessionQueueEntryState::Pending,
                    item: SessionQueueEntryItem::Message { invocation_id },
                    blocked_reason: None,
                    created_at: now,
                    updated_at: now,
                };
                insert_entry(connection, &message_entry).map_err(|error| error.to_string())?;
            }
        }
        increment_revision(connection, &session.id, now).map_err(|error| error.to_string())?;
        connection
            .execute(
                "DELETE FROM agent_session_target_transitions WHERE session_id=?1",
                [&stored_session_id],
            )
            .map_err(|error| format!("Unable to retire legacy target transition: {error}"))?;
    }
    connection
        .execute_batch("DROP TABLE agent_session_target_transitions")
        .map_err(|error| format!("Unable to remove legacy target transition storage: {error}"))?;
    Ok(())
}

fn state_text(state: SessionQueueEntryState) -> &'static str {
    match state {
        SessionQueueEntryState::Pending => "pending",
        SessionQueueEntryState::Executing => "executing",
        SessionQueueEntryState::Blocked => "blocked",
        SessionQueueEntryState::Completed => "completed",
        SessionQueueEntryState::Canceled => "canceled",
    }
}

fn parse_state(value: &str) -> Result<SessionQueueEntryState, RepositoryError> {
    match value {
        "pending" => Ok(SessionQueueEntryState::Pending),
        "executing" => Ok(SessionQueueEntryState::Executing),
        "blocked" => Ok(SessionQueueEntryState::Blocked),
        "completed" => Ok(SessionQueueEntryState::Completed),
        "canceled" => Ok(SessionQueueEntryState::Canceled),
        _ => Err(invalid_data(format!(
            "invalid Session queue state: {value}"
        ))),
    }
}

fn ensure_state(connection: &Connection, session: &AgentSession) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT OR IGNORE INTO agent_session_queue_state(session_id,revision,created_at,updated_at) VALUES(?1,0,?2,?2)",
            params![session.id.as_str(), timestamp(session.created_at)],
        )
        .map_err(sql_write("initialize Session queue"))?;
    Ok(())
}

fn increment_revision(
    connection: &Connection,
    session_id: &AgentSessionId,
    at: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "UPDATE agent_session_queue_state SET revision=revision+1,updated_at=?2 WHERE session_id=?1",
            params![session_id.as_str(), timestamp(at)],
        )
        .map_err(sql_write("advance Session queue revision"))?;
    touch_session(connection, session_id, at)
}

fn next_ordinal(
    connection: &Connection,
    session_id: &AgentSessionId,
) -> Result<i64, RepositoryError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(ordinal)+1,0) FROM agent_session_queue_entries WHERE session_id=?1",
            [session_id.as_str()],
            |row| row.get(0),
        )
        .map_err(sql_unavailable("read next Session queue position"))
}

fn insert_operation(
    connection: &Connection,
    operation: &SessionTargetOperation,
) -> Result<(), RepositoryError> {
    validate_target_operation(operation)
        .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
    connection
        .execute(
            "INSERT INTO agent_session_target_operations(id,session_id,payload_json) VALUES(?1,?2,?3)",
            params![operation.id, operation.session_id.as_str(), to_json(operation)?],
        )
        .map_err(sql_write("insert target operation"))?;
    Ok(())
}

fn insert_entry(connection: &Connection, entry: &SessionQueueEntry) -> Result<(), RepositoryError> {
    validate_queue_entry(entry)
        .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
    let (kind, invocation, operation) = match &entry.item {
        SessionQueueEntryItem::Message { invocation_id } => {
            ("message", Some(invocation_id.as_str()), None)
        }
        SessionQueueEntryItem::TargetOperation { operation_id } => {
            ("target_operation", None, Some(operation_id.as_str()))
        }
    };
    connection
        .execute(
            "INSERT INTO agent_session_queue_entries(id,session_id,ordinal,kind,state,invocation_id,operation_id,blocked_reason,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                entry.id,
                entry.session_id.as_str(),
                entry.ordinal,
                kind,
                state_text(entry.state),
                invocation,
                operation,
                entry.blocked_reason,
                timestamp(entry.created_at),
                timestamp(entry.updated_at),
            ],
        )
        .map_err(sql_write("insert Session queue entry"))?;
    Ok(())
}

fn read_entries(
    connection: &Connection,
    session_id: &AgentSessionId,
) -> Result<Vec<SessionQueueEntry>, RepositoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id,ordinal,kind,state,invocation_id,operation_id,blocked_reason,created_at,updated_at
             FROM agent_session_queue_entries WHERE session_id=?1 ORDER BY ordinal,id",
        )
        .map_err(sql_unavailable("prepare Session queue"))?;
    let rows = statement
        .query_map([session_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(sql_unavailable("read Session queue"))?;
    rows.map(|row| {
        let (id, ordinal, kind, state, invocation, operation, blocked_reason, created, updated) =
            row.map_err(sql_unavailable("read Session queue entry"))?;
        let item = match kind.as_str() {
            "message" => SessionQueueEntryItem::Message {
                invocation_id: AgentInvocationId::new(
                    invocation.ok_or_else(|| invalid_data("queued message has no invocation"))?,
                )
                .map_err(contract_error)?,
            },
            "target_operation" => SessionQueueEntryItem::TargetOperation {
                operation_id: operation
                    .ok_or_else(|| invalid_data("queued target operation has no operation"))?,
            },
            _ => return Err(invalid_data(format!("invalid Session queue kind: {kind}"))),
        };
        Ok(SessionQueueEntry {
            id,
            session_id: session_id.clone(),
            ordinal,
            state: parse_state(&state)?,
            item,
            blocked_reason,
            created_at: parse_timestamp(&created)?,
            updated_at: parse_timestamp(&updated)?,
        })
    })
    .collect()
}

fn read_snapshot(
    connection: &Connection,
    session_id: &AgentSessionId,
) -> Result<SessionQueueSnapshot, RepositoryError> {
    required_session(connection, session_id)?;
    let revision = connection
        .query_row(
            "SELECT revision FROM agent_session_queue_state WHERE session_id=?1",
            [session_id.as_str()],
            |row| row.get::<_, u64>(0),
        )
        .optional()
        .map_err(sql_unavailable("read Session queue revision"))?
        .unwrap_or(0);
    Ok(SessionQueueSnapshot {
        session_id: session_id.clone(),
        revision,
        entries: read_entries(connection, session_id)?,
    })
}

fn require_revision(
    connection: &Connection,
    session_id: &AgentSessionId,
    expected: u64,
) -> Result<(), RepositoryError> {
    let actual: u64 = connection
        .query_row(
            "SELECT revision FROM agent_session_queue_state WHERE session_id=?1",
            [session_id.as_str()],
            |row| row.get(0),
        )
        .map_err(sql_unavailable("read Session queue revision"))?;
    if actual != expected {
        return Err(RepositoryError::new(
            RepositoryErrorKind::Conflict,
            format!("Session queue changed from revision {expected} to {actual}"),
        ));
    }
    Ok(())
}

impl SqliteAgentSessionRepository {
    pub(super) fn accept_queued_preparation_record(
        &self,
        new_session: Option<(
            AgentSession,
            Option<crate::agent_sessions::organization::SessionPlacement>,
        )>,
        invocation: AgentInvocation,
        preparation: crate::agent_sessions::preparation::SessionPreparation,
        operation: Option<SessionTargetOperation>,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("accept queued Agent Session submission", |tx| {
            if let Some((session, placement)) = new_session {
                validate_session(&session).map_err(contract_error)?;
                insert_session(tx, &session)?;
                if let Some(placement) = placement {
                    write_placement(tx, &session.id, &placement)?;
                }
            }
            let session = required_session(tx, &invocation.session_id)?;
            validate_new_invocation(&session, None, &invocation).map_err(contract_error)?;
            insert_invocation(tx, &invocation)?;
            tx.execute(
                "INSERT INTO agent_session_preparations(invocation_id,session_id,payload_json) VALUES(?1,?2,?3)",
                params![invocation.id.as_str(),session.id.as_str(),to_json(&preparation)?],
            )
            .map_err(sql_write("accept queued preparation"))?;
            ensure_state(tx, &session)?;
            let mut ordinal = next_ordinal(tx, &session.id)?;
            if let Some(operation) = operation {
                insert_operation(tx, &operation)?;
                insert_entry(
                    tx,
                    &SessionQueueEntry {
                        id: format!("target:{}", operation.id),
                        session_id: session.id.clone(),
                        ordinal,
                        state: SessionQueueEntryState::Pending,
                        item: SessionQueueEntryItem::TargetOperation {
                            operation_id: operation.id,
                        },
                        blocked_reason: None,
                        created_at: at,
                        updated_at: at,
                    },
                )?;
                ordinal += 1;
            }
            insert_entry(
                tx,
                &SessionQueueEntry {
                    id: format!("message:{}", invocation.id.as_str()),
                    session_id: session.id.clone(),
                    ordinal,
                    state: SessionQueueEntryState::Pending,
                    item: SessionQueueEntryItem::Message {
                        invocation_id: invocation.id.clone(),
                    },
                    blocked_reason: None,
                    created_at: at,
                    updated_at: at,
                },
            )?;
            increment_revision(tx, &session.id, at)?;
            read_snapshot(tx, &session.id)
        })
    }

    pub(super) fn read_queue_snapshot_record(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.read("read Session queue", |connection| {
            read_snapshot(connection, session_id)
        })
    }

    pub(super) fn enqueue_target_operation_record(
        &self,
        operation: SessionTargetOperation,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("enqueue Session target operation", |tx| {
            let session = required_session(tx, &operation.session_id)?;
            ensure_state(tx, &session)?;
            let ordinal = next_ordinal(tx, &session.id)?;
            insert_operation(tx, &operation)?;
            insert_entry(
                tx,
                &SessionQueueEntry {
                    id: format!("target:{}", operation.id),
                    session_id: session.id.clone(),
                    ordinal,
                    state: SessionQueueEntryState::Pending,
                    item: SessionQueueEntryItem::TargetOperation {
                        operation_id: operation.id,
                    },
                    blocked_reason: None,
                    created_at: at,
                    updated_at: at,
                },
            )?;
            increment_revision(tx, &session.id, at)?;
            read_snapshot(tx, &session.id)
        })
    }

    pub(super) fn read_target_operation_record(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionTargetOperation>, RepositoryError> {
        self.read("read target operation", |connection| {
            let json: Option<String> = connection
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [operation_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read target operation"))?;
            json.map(|value| from_json(&value, "target operation"))
                .transpose()
        })
    }

    pub(super) fn save_target_operation_record(
        &self,
        operation: &SessionTargetOperation,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        validate_target_operation(operation)
            .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
        self.write("save target operation", |tx| {
            let changed = tx
                .execute(
                    "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1 AND session_id=?3",
                    params![operation.id,to_json(operation)?,operation.session_id.as_str()],
                )
                .map_err(sql_write("save target operation"))?;
            if changed != 1 {
                return Err(not_found("Target operation not found"));
            }
            increment_revision(tx, &operation.session_id, operation.updated_at)?;
            read_snapshot(tx, &operation.session_id)
        })
    }

    pub(super) fn set_queue_entry_state_record(
        &self,
        entry_id: &str,
        state: SessionQueueEntryState,
        blocked_reason: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("update Session queue entry", |tx| {
            let session: String = tx
                .query_row(
                    "SELECT session_id FROM agent_session_queue_entries WHERE id=?1",
                    [entry_id],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("load Session queue entry"))?;
            let session_id = AgentSessionId::new(session).map_err(contract_error)?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state=?2,blocked_reason=?3,updated_at=?4 WHERE id=?1",
                params![entry_id,state_text(state),blocked_reason,timestamp(at)],
            )
            .map_err(sql_write("update Session queue entry"))?;
            increment_revision(tx, &session_id, at)?;
            read_snapshot(tx, &session_id)
        })
    }

    pub(super) fn retry_queue_entry_record(
        &self,
        entry_id: &str,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("retry Session queue entry", |tx| {
            let (session, kind, state, invocation_id, operation_id): (
                String,
                String,
                String,
                Option<String>,
                Option<String>,
            ) = tx
                .query_row(
                    "SELECT session_id,kind,state,invocation_id,operation_id FROM agent_session_queue_entries WHERE id=?1",
                    [entry_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .map_err(sql_unavailable("load blocked Session queue entry"))?;
            if state != "blocked" {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Only blocked queue entries can be retried",
                ));
            }
            let session_id = AgentSessionId::new(session).map_err(contract_error)?;
            match kind.as_str() {
                "message" => {
                    let invocation_id = AgentInvocationId::new(
                        invocation_id.ok_or_else(|| invalid_data("Queued message has no invocation"))?,
                    )
                    .map_err(contract_error)?;
                    let mut preparation = super::preparation::read_record(tx, &invocation_id)?
                        .ok_or_else(|| not_found("Preparation not found"))?;
                    if !matches!(
                        preparation.phase,
                        crate::agent_sessions::preparation::PreparationPhase::Failed
                            | crate::agent_sessions::preparation::PreparationPhase::Canceled
                    ) || !preparation.can_retry
                        || preparation.delivery_started
                        || invocation_launch_accepted_at_from(tx, &invocation_id)?.is_some()
                    {
                        return Err(RepositoryError::new(
                            RepositoryErrorKind::Conflict,
                            "Only known preparation failures can be retried",
                        ));
                    }
                    tx.execute(
                        "UPDATE agent_session_invocations SET status='pending',started_at=NULL,
                         completed_at=NULL,runtime_error_json=NULL,effective_options_json=NULL,
                         exit_code=NULL,signal=NULL,updated_at=?2 WHERE id=?1",
                        params![invocation_id.as_str(), timestamp(at)],
                    )
                    .map_err(sql_write("retry queued invocation"))?;
                    preparation.phase =
                        crate::agent_sessions::preparation::PreparationPhase::Accepted;
                    preparation.error = None;
                    preparation.can_retry = false;
                    tx.execute(
                        "UPDATE agent_session_preparations SET payload_json=?2 WHERE invocation_id=?1",
                        params![invocation_id.as_str(), to_json(&preparation)?],
                    )
                    .map_err(sql_write("retry queued preparation"))?;
                }
                "target_operation" => {
                    let operation_id = operation_id
                        .ok_or_else(|| invalid_data("Queued target operation has no operation"))?;
                    let payload: String = tx
                        .query_row(
                            "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                            [&operation_id],
                            |row| row.get(0),
                        )
                        .map_err(sql_unavailable("load blocked target operation"))?;
                    let mut operation: SessionTargetOperation =
                        from_json(&payload, "target operation")?;
                    if operation.phase != TargetOperationPhase::Blocked {
                        return Err(RepositoryError::new(
                            RepositoryErrorKind::Conflict,
                            "Only blocked target operations can be retried",
                        ));
                    }
                    operation.phase = TargetOperationPhase::Pending;
                    operation.error = None;
                    operation.updated_at = at;
                    operation.actions = replan_actions(
                        &operation.actions,
                        operation.source_target.as_ref(),
                        &operation.destination_selection,
                    );
                    tx.execute(
                        "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                        params![operation_id, to_json(&operation)?],
                    )
                    .map_err(sql_write("retry target operation"))?;
                }
                _ => return Err(invalid_data("Unknown Session queue entry kind")),
            }
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='pending',blocked_reason=NULL,updated_at=?2 WHERE id=?1",
                params![entry_id, timestamp(at)],
            )
            .map_err(sql_write("retry Session queue entry"))?;
            increment_revision(tx, &session_id, at)?;
            read_snapshot(tx, &session_id)
        })
    }

    pub(super) fn queue_entry_for_invocation_record(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<SessionQueueEntry>, RepositoryError> {
        self.read("read invocation queue entry", |connection| {
            let session: Option<String> = connection
                .query_row(
                    "SELECT session_id FROM agent_session_queue_entries WHERE invocation_id=?1",
                    [invocation_id.as_str()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read invocation queue entry"))?;
            let Some(session) = session else { return Ok(None) };
            let session_id = AgentSessionId::new(session).map_err(contract_error)?;
            Ok(read_entries(connection, &session_id)?.into_iter().find(|entry| {
                matches!(&entry.item, SessionQueueEntryItem::Message { invocation_id: id } if id == invocation_id)
            }))
        })
    }

    pub(super) fn queue_entry_for_operation_record(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionQueueEntry>, RepositoryError> {
        self.read("read operation queue entry", |connection| {
            let session: Option<String> = connection
                .query_row(
                    "SELECT session_id FROM agent_session_queue_entries WHERE operation_id=?1",
                    [operation_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read operation queue entry"))?;
            let Some(session) = session else { return Ok(None) };
            let session_id = AgentSessionId::new(session).map_err(contract_error)?;
            Ok(read_entries(connection, &session_id)?.into_iter().find(|entry| {
                matches!(&entry.item, SessionQueueEntryItem::TargetOperation { operation_id: id } if id == operation_id)
            }))
        })
    }

    pub(super) fn reorder_queue_record(
        &self,
        session_id: &AgentSessionId,
        expected_revision: u64,
        entry_ids: &[String],
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("reorder Session queue", |tx| {
            require_revision(tx, session_id, expected_revision)?;
            let entries = read_entries(tx, session_id)?;
            let unfinished = entries
                .iter()
                .filter(|entry| entry.state.is_unfinished())
                .collect::<Vec<_>>();
            if let Some(pinned) = unfinished
                .iter()
                .find(|entry| entry.state == SessionQueueEntryState::Executing)
            {
                if entry_ids.first().map(String::as_str) != Some(pinned.id.as_str()) {
                    return Err(RepositoryError::new(
                        RepositoryErrorKind::Conflict,
                        "The executing queue entry is pinned",
                    ));
                }
            }
            let expected = unfinished
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<std::collections::HashSet<_>>();
            let supplied = entry_ids
                .iter()
                .map(String::as_str)
                .collect::<std::collections::HashSet<_>>();
            if expected != supplied || supplied.len() != entry_ids.len() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Reorder must include every unfinished queue entry exactly once",
                ));
            }
            let slots = unfinished
                .iter()
                .map(|entry| entry.ordinal)
                .collect::<Vec<_>>();
            for (index, id) in entry_ids.iter().enumerate() {
                tx.execute(
                    "UPDATE agent_session_queue_entries SET ordinal=?2 WHERE id=?1 AND session_id=?3",
                    params![id,-1_i64-(index as i64),session_id.as_str()],
                )
                .map_err(sql_write("stage Session queue reorder"))?;
            }
            for (id, ordinal) in entry_ids.iter().zip(slots) {
                tx.execute(
                    "UPDATE agent_session_queue_entries SET ordinal=?2,updated_at=?3 WHERE id=?1 AND session_id=?4",
                    params![id,ordinal,timestamp(at),session_id.as_str()],
                )
                .map_err(sql_write("apply Session queue reorder"))?;
            }
            increment_revision(tx, session_id, at)?;
            read_snapshot(tx, session_id)
        })
    }

    pub(super) fn update_queued_message_record(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
        expected_revision: u64,
        submitted_text: &str,
        model: Option<String>,
        reasoning_mode: Option<String>,
        sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("edit queued Agent Session message", |tx| {
            require_revision(tx, session_id, expected_revision)?;
            if submitted_text.trim().is_empty() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::InvalidState,
                    "Queued message cannot be empty",
                ));
            }
            let (invocation_id, state): (String, String) = tx
                .query_row(
                    "SELECT invocation_id,state FROM agent_session_queue_entries WHERE id=?1 AND session_id=?2 AND kind='message'",
                    params![entry_id,session_id.as_str()],
                    |row| Ok((row.get(0)?,row.get(1)?)),
                )
                .map_err(sql_unavailable("load queued message"))?;
            if !matches!(state.as_str(), "pending" | "blocked") {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Message execution has already started",
                ));
            }
            let invocation_id = AgentInvocationId::new(invocation_id).map_err(contract_error)?;
            if invocation_launch_accepted_at_from(tx, &invocation_id)?.is_some() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Delivered message cannot be edited",
                ));
            }
            let mut preparation = super::preparation::read_record(tx, &invocation_id)?
                .ok_or_else(|| not_found("Preparation not found"))?;
            if preparation.delivery_started {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Message delivery has already started",
                ));
            }
            preparation.model = model;
            preparation.reasoning_mode = reasoning_mode;
            preparation.sandbox_mode = sandbox_mode;
            preparation.phase = crate::agent_sessions::preparation::PreparationPhase::Accepted;
            preparation.steps.clear();
            preparation.error = None;
            preparation.can_retry = false;
            preparation.resolved_target = None;
            preparation.current_resolution = None;
            preparation.resolution = None;
            preparation.execution_snapshot = None;
            preparation.prepared_binding = None;
            tx.execute(
                "UPDATE agent_session_invocations SET submitted_text=?2,status='pending',started_at=NULL,completed_at=NULL,effective_options_json=NULL,runtime_error_json=NULL,exit_code=NULL,signal=NULL,updated_at=?3 WHERE id=?1",
                params![invocation_id.as_str(),submitted_text,timestamp(at)],
            )
            .map_err(sql_write("edit queued message"))?;
            tx.execute(
                "UPDATE agent_session_preparations SET payload_json=?2 WHERE invocation_id=?1",
                params![invocation_id.as_str(),to_json(&preparation)?],
            )
            .map_err(sql_write("edit queued message choices"))?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='pending',blocked_reason=NULL,updated_at=?2 WHERE id=?1",
                params![entry_id,timestamp(at)],
            )
            .map_err(sql_write("reset edited queue entry"))?;
            increment_revision(tx, session_id, at)?;
            read_snapshot(tx, session_id)
        })
    }

    pub(super) fn replace_target_operation_record(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
        expected_revision: u64,
        destination: crate::execution_targets::domain::SessionExecutionSelection,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("edit queued target operation", |tx| {
            require_revision(tx, session_id, expected_revision)?;
            let (operation_id, state): (String, String) = tx
                .query_row(
                    "SELECT operation_id,state FROM agent_session_queue_entries WHERE id=?1 AND session_id=?2 AND kind='target_operation'",
                    params![entry_id,session_id.as_str()],
                    |row| Ok((row.get(0)?,row.get(1)?)),
                )
                .map_err(sql_unavailable("load queued target operation"))?;
            if !matches!(state.as_str(), "pending" | "blocked") {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Target activation has already started",
                ));
            }
            let json: String = tx
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [&operation_id],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("read target operation"))?;
            let mut operation: SessionTargetOperation = from_json(&json, "target operation")?;
            operation.destination_selection = destination;
            operation.phase = crate::agent_sessions::target_operation::TargetOperationPhase::Pending;
            operation.actions = crate::agent_sessions::target_operation::planned_actions(
                operation.source_target.as_ref(),
                &operation.destination_selection,
            );
            operation.staged_target = None;
            operation.activated_binding = None;
            operation.error = None;
            operation.generation = operation.generation.saturating_add(1);
            operation.updated_at = at;
            validate_target_operation(&operation)
                .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
            tx.execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![operation_id,to_json(&operation)?],
            )
            .map_err(sql_write("edit target operation"))?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='pending',blocked_reason=NULL,updated_at=?2 WHERE id=?1",
                params![entry_id,timestamp(at)],
            )
            .map_err(sql_write("reset target operation queue entry"))?;
            increment_revision(tx, session_id, at)?;
            read_snapshot(tx, session_id)
        })
    }

    pub(super) fn activate_target_operation_record(
        &self,
        operation_id: &str,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("activate queued target", |tx| {
            let json: String = tx
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [operation_id],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("read target operation"))?;
            let mut operation: SessionTargetOperation = from_json(&json, "target operation")?;
            if !matches!(operation.phase, crate::agent_sessions::target_operation::TargetOperationPhase::Staged | crate::agent_sessions::target_operation::TargetOperationPhase::Activating) {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Target operation is not staged",
                ));
            }
            let target = operation
                .staged_target
                .clone()
                .ok_or_else(|| not_found("Staged target unavailable"))?;
            let session = required_session(tx, &operation.session_id)?;
            if let (Some(source), Some(external_context_id)) = (
                session.execution_target.as_ref(),
                session.runtime_binding.external_context_id.clone(),
            ) {
                super::native_conversations::retain_source_provider_binding(
                    tx,
                    &operation.session_id,
                    Some(&crate::agent_sessions::domain::ProviderSessionBinding {
                        external_context_id,
                        runtime_version: session.runtime_binding.runtime_version.clone(),
                        location: source.execution.clone(),
                        workspace: Some(crate::agent_sessions::domain::ProviderWorkspaceIdentity::from(source)),
                        last_invocation_id: None,
                    }),
                    at,
                )?;
            }
            tx.execute(
                "UPDATE agent_sessions SET execution_target_json=?2,working_directory=?3,workspace_origin='explicit',external_context_id=NULL,runtime_version=NULL,updated_at=?4 WHERE id=?1",
                params![operation.session_id.as_str(),to_json(&target)?,target.path,timestamp(at)],
            )
            .map_err(sql_write("activate Session target"))?;
            operation.phase = crate::agent_sessions::target_operation::TargetOperationPhase::Completed;
            operation.error = None;
            operation.updated_at = at;
            for action in &mut operation.actions {
                if matches!(
                    action.kind,
                    crate::agent_sessions::target_operation::TargetOperationActionKind::SelectProviderContext
                        | crate::agent_sessions::target_operation::TargetOperationActionKind::ActivateTarget
                ) {
                    action.status = crate::agent_sessions::target_operation::TargetOperationActionStatus::Completed;
                    action.error = None;
                }
            }
            tx.execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![operation_id,to_json(&operation)?],
            )
            .map_err(sql_write("complete target operation"))?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='completed',blocked_reason=NULL,updated_at=?2 WHERE operation_id=?1",
                params![operation_id,timestamp(at)],
            )
            .map_err(sql_write("complete target operation queue entry"))?;
            increment_revision(tx, &operation.session_id, at)?;
            read_snapshot(tx, &operation.session_id)
        })
    }
}
