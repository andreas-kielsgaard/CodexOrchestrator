//! Durable, application-owned Sprint continuation decisions.  A decision is neither an Epic
//! receipt nor any higher-level settlement.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

pub(crate) const SPRINT_CONTINUATION_SETTLEMENT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sprint_handback_dependency_routes (handback_id TEXT PRIMARY KEY,work_unit_id TEXT NOT NULL,route_fingerprint TEXT NOT NULL UNIQUE,recorded_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS sprint_continuation_decisions (
  decision_id TEXT PRIMARY KEY,
  sprint_id TEXT NOT NULL,
  decision_sequence INTEGER NOT NULL,
  decision_state TEXT NOT NULL CHECK (decision_state IN ('continuing','attention','settled')),
  continuation_kind TEXT NOT NULL,
  accepted_materialization_count INTEGER NOT NULL,
  input_fingerprint TEXT NOT NULL UNIQUE,
  recorded_at TEXT NOT NULL,
  UNIQUE(sprint_id, decision_sequence)
);
CREATE TABLE IF NOT EXISTS sprint_continuation_current_decisions (
  sprint_id TEXT PRIMARY KEY,
  decision_id TEXT NOT NULL UNIQUE REFERENCES sprint_continuation_decisions(decision_id) ON DELETE RESTRICT,
  decision_state TEXT NOT NULL CHECK (decision_state IN ('continuing','attention','settled')),
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sprint_continuation_attentions (
  decision_id TEXT PRIMARY KEY REFERENCES sprint_continuation_decisions(decision_id) ON DELETE RESTRICT,
  attention_id TEXT NOT NULL UNIQUE,
  attention_code TEXT NOT NULL,
  attention_fingerprint TEXT NOT NULL UNIQUE,
  source_attention_id TEXT,
  recorded_at TEXT NOT NULL
);
-- This is a locally persisted Sprint result only.  It has no delivery, receiver, Epic, or
-- acceptance fields, deliberately preventing it from claiming any higher-level effect.
CREATE TABLE IF NOT EXISTS sprint_upward_results (
  result_id TEXT PRIMARY KEY,
  decision_id TEXT NOT NULL UNIQUE REFERENCES sprint_continuation_decisions(decision_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL,
  result_kind TEXT NOT NULL CHECK (result_kind IN ('continuing','attention','settled')),
  chronology_fingerprint TEXT NOT NULL UNIQUE,
  recorded_at TEXT NOT NULL
);
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SprintContinuationStatus {
    pub(crate) state: String,
    pub(crate) recorded_at: String,
    pub(crate) upward_result_recorded_at: String,
}

pub(crate) fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(SPRINT_CONTINUATION_SETTLEMENT_SCHEMA)
        .map_err(|error| error.to_string())?;
    let columns = connection
        .prepare("PRAGMA table_info(sprint_continuation_attentions)")
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if !columns.iter().any(|column| column == "source_attention_id") {
        connection
            .execute(
                "ALTER TABLE sprint_continuation_attentions ADD COLUMN source_attention_id TEXT",
                [],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn statuses(
    connection: &Connection,
) -> Result<Vec<(String, SprintContinuationStatus)>, String> {
    connection
        .prepare(
            "SELECT c.sprint_id,c.decision_state,c.recorded_at,r.recorded_at
         FROM sprint_continuation_current_decisions current
         JOIN sprint_continuation_decisions c ON c.decision_id=current.decision_id
         JOIN sprint_upward_results r ON r.decision_id=c.decision_id
         ORDER BY c.sprint_id",
        )
        .map_err(|error| error.to_string())?
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                SprintContinuationStatus {
                    state: row.get(1)?,
                    recorded_at: row.get(2)?,
                    upward_result_recorded_at: row.get(3)?,
                },
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

/// Reconcile from authoritative durable rows only.  The function intentionally fails closed:
/// an incomplete, foreign, or malformed materialization cannot become a settlement.
pub(crate) fn reconcile(connection: &mut Connection) -> Result<(), String> {
    let sprint_ids = connection
        .prepare(
            "SELECT s.id FROM initiated_sprints s
         JOIN sprint_runner_transitions t ON t.sprint_id=s.id AND t.epic_id=s.epic_id
         ORDER BY s.id",
        )
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for sprint_id in sprint_ids {
        reconcile_unbound_dependency_routes(connection, &sprint_id)?;
        reconcile_one(connection, &sprint_id)?;
    }
    Ok(())
}

fn reconcile_one(connection: &mut Connection, sprint_id: &str) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let snapshot = Snapshot::load(&transaction, sprint_id)?;
    let (state, kind) = snapshot.decision();
    let source_attention_id = if state == "attention"
        && kind == "structured_human_or_external_attention"
        && snapshot.structured_attention_source_ids.len() == 1
    {
        Some(snapshot.structured_attention_source_ids[0].clone())
    } else {
        None
    };
    let fingerprint = digest(&format!(
        "{sprint_id}:{}:{}",
        state,
        snapshot.fingerprint_input()
    ));
    let existing: Option<(String, String, String, i64)> = transaction.query_row(
        "SELECT decision_id,sprint_id,decision_state,decision_sequence FROM sprint_continuation_decisions WHERE input_fingerprint=?1", [&fingerprint], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
    ).optional().map_err(|error| error.to_string())?;
    let (decision_id, recorded_at, decision_sequence) = if let Some((
        id,
        existing_sprint,
        existing_state,
        sequence,
    )) = existing
    {
        let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_continuation_decisions WHERE decision_id=?1 AND sprint_id=?2 AND decision_state=?3 AND continuation_kind=?4 AND accepted_materialization_count=?5 AND input_fingerprint=?6)", params![id,sprint_id,state,kind,snapshot.materialization_count,fingerprint], |row| row.get(0)).map_err(|error| error.to_string())?;
        if !exact || existing_sprint != sprint_id || existing_state != state {
            return Err("Sprint continuation decision conflict".into());
        }
        let at = transaction
            .query_row(
                "SELECT recorded_at FROM sprint_continuation_decisions WHERE decision_id=?1",
                [&id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        (id, at, sequence)
    } else {
        let sequence: i64 = transaction.query_row("SELECT COALESCE(MAX(decision_sequence),0)+1 FROM sprint_continuation_decisions WHERE sprint_id=?1", [sprint_id], |row| row.get(0)).map_err(|error| error.to_string())?;
        let id = digest(&format!(
            "sprint-continuation-decision:{sprint_id}:{sequence}:{fingerprint}"
        ));
        let now = chrono::Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO sprint_continuation_decisions (decision_id,sprint_id,decision_sequence,decision_state,continuation_kind,accepted_materialization_count,input_fingerprint,recorded_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, sprint_id, sequence, state, kind, snapshot.materialization_count, fingerprint, now],
        ).map_err(|error| error.to_string())?;
        (id, now, sequence)
    };
    let current: Option<(String, String, i64)> = transaction.query_row("SELECT c.decision_id,c.decision_state,c.decision_sequence FROM sprint_continuation_current_decisions current JOIN sprint_continuation_decisions c ON c.decision_id=current.decision_id WHERE current.sprint_id=?1", [sprint_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|error| error.to_string())?;
    if let Some((current_id, current_state, current_sequence)) = current {
        if current_id == decision_id {
            if current_state != state {
                return Err("Sprint continuation current-pointer state conflict".into());
            }
        } else if current_sequence >= decision_sequence || current_state == "settled" {
            return Err("Sprint continuation current-pointer transition conflict".into());
        }
    }
    transaction.execute(
        "INSERT INTO sprint_continuation_current_decisions (sprint_id,decision_id,decision_state,updated_at) VALUES (?1,?2,?3,?4)
         ON CONFLICT(sprint_id) DO UPDATE SET decision_id=excluded.decision_id,decision_state=excluded.decision_state,updated_at=excluded.updated_at",
        params![sprint_id, decision_id, state, recorded_at],
    ).map_err(|error| error.to_string())?;
    if state == "attention" {
        let attention_id = digest(&format!("sprint-continuation-attention:{decision_id}"));
        let attention_fingerprint = digest(&format!("{decision_id}:{kind}"));
        let changed = transaction.execute(
            "INSERT OR IGNORE INTO sprint_continuation_attentions (decision_id,attention_id,attention_code,attention_fingerprint,source_attention_id,recorded_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![decision_id, attention_id, kind, attention_fingerprint, source_attention_id, recorded_at],
        ).map_err(|error| error.to_string())?;
        if changed == 0 {
            let exact:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_continuation_attentions WHERE decision_id=?1 AND attention_id=?2 AND attention_code=?3 AND attention_fingerprint=?4 AND source_attention_id IS ?5 AND recorded_at=?6)",params![decision_id,attention_id,kind,attention_fingerprint,source_attention_id,recorded_at],|row|row.get(0)).map_err(|error|error.to_string())?;
            if !exact {
                return Err("Sprint continuation attention conflict".into());
            }
        }
    }
    let result_id = digest(&format!("sprint-upward-result:{decision_id}"));
    let chronology = digest(&format!("{sprint_id}:{decision_id}:{state}:{recorded_at}"));
    let changed = transaction.execute(
        "INSERT OR IGNORE INTO sprint_upward_results (result_id,decision_id,sprint_id,result_kind,chronology_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![result_id, decision_id, sprint_id, state, chronology, recorded_at],
    ).map_err(|error| error.to_string())?;
    if changed == 0 {
        let exact:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_upward_results WHERE result_id=?1 AND decision_id=?2 AND sprint_id=?3 AND result_kind=?4 AND chronology_fingerprint=?5 AND recorded_at=?6)",params![result_id,decision_id,sprint_id,state,chronology,recorded_at],|row|row.get(0)).map_err(|error|error.to_string())?;
        if !exact {
            return Err("Sprint upward result conflict".into());
        }
    }
    transaction.commit().map_err(|error| error.to_string())
}

struct Snapshot {
    identity: Vec<String>,
    structured_attention_source_ids: Vec<String>,
    materialization_count: i64,
    correlated_materialization_count: i64,
    terminal_materialization_count: i64,
    malformed_chronology: bool,
    unresolved_retry: bool,
    unresolved_handback: bool,
    agent_dependency_wait: bool,
    dependency_route_unavailable: bool,
    durable_attention: bool,
    eligible_work: bool,
    outstanding_continuation: bool,
    stale_epic_context: bool,
}

impl Snapshot {
    fn load(tx: &rusqlite::Transaction<'_>, sprint: &str) -> Result<Self, String> {
        let mut identity = canonical_rows(tx, "SELECT m.materialization_id,m.planning_point_id,m.accepted_revision_id,m.epic_id,m.sprint_id,COALESCE(r.accepted_at,'') FROM work_unit_materializations m LEFT JOIN work_slice_proposal_revisions r ON r.revision_id=m.accepted_revision_id WHERE m.sprint_id=?1 ORDER BY m.materialization_id", sprint, 6)?;
        identity.extend(canonical_rows(tx, "SELECT m.materialization_id,COALESCE(g.accepted_revision_id,''),COALESCE(w.graph_completion_materialization_id,''),COALESCE(p.planning_point_id,'') FROM work_unit_materializations m LEFT JOIN work_slice_execution_graph_completions g ON g.materialization_id=m.materialization_id LEFT JOIN work_slice_execution_settlements w ON w.materialization_id=m.materialization_id LEFT JOIN work_slice_planning_point_execution_settlements p ON p.materialization_id=m.materialization_id WHERE m.sprint_id=?1 ORDER BY m.materialization_id", sprint, 4)?);
        identity.extend(canonical_rows(tx, "SELECT h.work_unit_id,h.eligibility_state,COALESCE(h.handler_ready_at,'') FROM work_unit_handler_activations h WHERE h.sprint_id=?1 ORDER BY h.work_unit_id", sprint, 3)?);
        identity.extend(canonical_rows(tx, "SELECT h.handback_id,h.work_unit_id,h.context_fingerprint,COALESCE(d.movement_kind,''),COALESCE(d.details_json,'') FROM work_unit_no_progress_handbacks h JOIN work_units u ON u.work_unit_id=h.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id LEFT JOIN sprint_runner_handback_dispositions d ON d.handback_id=h.handback_id WHERE m.sprint_id=?1 ORDER BY h.handback_id", sprint, 5)?);
        identity.extend(canonical_rows(tx, "SELECT r.handback_id,r.epic_id,COALESCE(r.correlation_fingerprint,''),COALESCE(d.movement_kind,''),COALESCE(d.details_json,''),COALESCE(q.request_json,'') FROM epic_runner_escalation_receivers r LEFT JOIN epic_runner_escalation_dispositions d ON d.handback_id=r.handback_id LEFT JOIN epic_runner_escalation_downstream_requests q ON q.handback_id=r.handback_id WHERE r.sprint_id=?1 ORDER BY r.handback_id", sprint, 6)?);
        let structured_attention_source_ids = structured_attention_source_ids(tx, sprint)?;
        identity.extend(
            structured_attention_source_ids
                .iter()
                .map(|source| format!("structured-attention-source\u{1e}{source}")),
        );
        let materialization_count = count(
            tx,
            "SELECT COUNT(*) FROM work_unit_materializations WHERE sprint_id=?1",
            sprint,
        )?;
        let correlated_materialization_count = count(tx, "SELECT COUNT(*) FROM work_unit_materializations m JOIN initiated_sprints s ON s.id=m.sprint_id AND s.epic_id=m.epic_id JOIN work_slice_proposal_revisions r ON r.revision_id=m.accepted_revision_id AND r.planning_point_id=m.planning_point_id AND r.accepted_at IS NOT NULL JOIN work_slice_planning_episodes e ON e.planning_point_id=m.planning_point_id AND e.sprint_id=m.sprint_id WHERE m.sprint_id=?1", sprint)?;
        let terminal_materialization_count = count(tx, "SELECT COUNT(*) FROM work_unit_materializations m JOIN work_slice_execution_graph_completions g ON g.materialization_id=m.materialization_id AND g.accepted_revision_id=m.accepted_revision_id JOIN work_slice_execution_settlements w ON w.materialization_id=m.materialization_id AND w.graph_completion_materialization_id=g.materialization_id JOIN work_slice_planning_point_execution_settlements p ON p.materialization_id=m.materialization_id AND p.work_slice_execution_materialization_id=w.materialization_id AND p.planning_point_id=m.planning_point_id WHERE m.sprint_id=?1", sprint)?;
        let malformed_chronology = exists(tx, "SELECT 1 FROM work_slice_execution_settlements w JOIN work_unit_materializations m ON m.materialization_id=w.materialization_id LEFT JOIN work_slice_execution_graph_completions g ON g.materialization_id=w.graph_completion_materialization_id AND g.accepted_revision_id=m.accepted_revision_id WHERE m.sprint_id=?1 AND g.materialization_id IS NULL UNION ALL SELECT 1 FROM work_slice_planning_point_execution_settlements p JOIN work_unit_materializations m ON m.materialization_id=p.materialization_id LEFT JOIN work_slice_execution_settlements w ON w.materialization_id=p.work_slice_execution_materialization_id AND w.materialization_id=m.materialization_id WHERE m.sprint_id=?1 AND w.materialization_id IS NULL", sprint)?;
        let unresolved_retry = exists(tx, "SELECT 1 FROM work_unit_retry_attempts r JOIN work_units u ON u.work_unit_id=r.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE m.sprint_id=?1 AND s.work_unit_id IS NULL", sprint)?;
        let unresolved_handback = exists(tx, "SELECT 1 FROM work_unit_no_progress_handbacks h JOIN work_units u ON u.work_unit_id=h.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE m.sprint_id=?1 AND s.work_unit_id IS NULL", sprint)?;
        let dependency_wait = dependency_wait_state(tx, sprint)?;
        identity.extend(dependency_wait.identity);
        let durable_attention = exists(tx, "SELECT 1 FROM work_slice_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.sprint_id=?1 UNION ALL SELECT 1 FROM work_unit_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.sprint_id=?1 UNION ALL SELECT 1 FROM epic_runner_escalation_attentions a JOIN epic_runner_escalation_receivers r ON r.handback_id=a.handback_id WHERE r.sprint_id=?1", sprint)?;
        let eligible_work = exists(tx, "SELECT 1 FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements s ON s.work_unit_id=h.work_unit_id WHERE h.sprint_id=?1 AND h.eligibility_state='eligible' AND h.handler_ready_at IS NOT NULL AND s.work_unit_id IS NULL", sprint)?;
        let outstanding_continuation = exists(tx, "SELECT 1 FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements s ON s.work_unit_id=h.work_unit_id WHERE h.sprint_id=?1 AND s.work_unit_id IS NULL UNION ALL SELECT 1 FROM work_unit_implementer_activations i JOIN work_units u ON u.work_unit_id=i.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE m.sprint_id=?1 AND s.work_unit_id IS NULL", sprint)?;
        let stale_epic_context = exists(tx, "SELECT 1 FROM epic_runner_escalation_receivers r JOIN sprint_runner_transitions t ON t.sprint_id=r.sprint_id WHERE r.sprint_id=?1 AND r.epic_id<>t.epic_id", sprint)?;
        Ok(Self {
            identity,
            structured_attention_source_ids,
            materialization_count,
            correlated_materialization_count,
            terminal_materialization_count,
            malformed_chronology,
            unresolved_retry,
            unresolved_handback,
            agent_dependency_wait: dependency_wait.active,
            dependency_route_unavailable: dependency_wait.unavailable,
            durable_attention,
            eligible_work,
            outstanding_continuation,
            stale_epic_context,
        })
    }

    fn decision(&self) -> (&'static str, &'static str) {
        if self.durable_attention {
            return ("attention", "structured_human_or_external_attention");
        }
        if self.stale_epic_context {
            return ("attention", "stale_epic_context");
        }
        if self.dependency_route_unavailable {
            return ("attention", "dependency_route_unavailable");
        }
        if self.malformed_chronology
            || self.correlated_materialization_count != self.materialization_count
        {
            return ("attention", "correlation_or_chronology_unavailable");
        }
        if self.materialization_count > 0
            && self.terminal_materialization_count == self.materialization_count
            && !self.unresolved_retry
            && !self.unresolved_handback
            && !self.agent_dependency_wait
            && !self.outstanding_continuation
        {
            return ("settled", "all_authoritative_sprint_work_settled");
        }
        if self.agent_dependency_wait {
            return ("continuing", "wait_for_agent_dependency");
        }
        if self.eligible_work {
            return ("continuing", "continue_eligible_work");
        }
        if self.unresolved_retry {
            return ("continuing", "retry_reassessment_pending");
        }
        if self.outstanding_continuation {
            return ("continuing", "continuation_pending");
        }
        if self.unresolved_handback {
            return ("attention", "unresolved_handback");
        }
        ("continuing", "planning_or_execution_pending")
    }

    fn fingerprint_input(&self) -> String {
        let aggregate = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.materialization_count,
            self.correlated_materialization_count,
            self.terminal_materialization_count,
            self.malformed_chronology as u8,
            self.unresolved_retry as u8,
            self.unresolved_handback as u8,
            self.agent_dependency_wait as u8,
            self.dependency_route_unavailable as u8,
            self.durable_attention as u8,
            self.eligible_work as u8,
            self.outstanding_continuation as u8,
            self.stale_epic_context as u8
        );
        format!("{aggregate}:{}", self.identity.join("\u{1f}"))
    }
}

fn structured_attention_source_ids(
    tx: &rusqlite::Transaction<'_>,
    sprint: &str,
) -> Result<Vec<String>, String> {
    let columns = tx
        .prepare("PRAGMA table_info(epic_runner_escalation_attentions)")
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if !columns.iter().any(|column| column == "attention_id") {
        return Ok(Vec::new());
    }
    let sources = tx
        .prepare(
            "SELECT attention.attention_id FROM epic_runner_escalation_receivers receiver JOIN epic_runner_escalation_attentions attention ON attention.handback_id=receiver.handback_id WHERE receiver.sprint_id=?1 ORDER BY attention.attention_id",
        )
        .map_err(|error| error.to_string())?
        .query_map([sprint], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if sources.iter().any(|source| source.trim().is_empty())
        || sources.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err("malformed structured Sprint attention correlation".into());
    }
    Ok(sources)
}

fn count(tx: &rusqlite::Transaction<'_>, query: &str, sprint: &str) -> Result<i64, String> {
    tx.query_row(query, [sprint], |row| row.get(0))
        .map_err(|error| error.to_string())
}
fn exists(tx: &rusqlite::Transaction<'_>, query: &str, sprint: &str) -> Result<bool, String> {
    tx.query_row(&format!("SELECT EXISTS({query})"), [sprint], |row| {
        row.get(0)
    })
    .map_err(|error| error.to_string())
}
fn canonical_rows(
    tx: &rusqlite::Transaction<'_>,
    query: &str,
    sprint: &str,
    columns: usize,
) -> Result<Vec<String>, String> {
    tx.prepare(query)
        .map_err(|error| error.to_string())?
        .query_map([sprint], |row| {
            (0..columns)
                .map(|index| row.get::<_, String>(index))
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join("\u{1e}"))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
#[derive(Default)]
struct DependencyWaitState { active: bool, unavailable: bool, identity: Vec<String> }

fn dependency_wait_state(tx: &rusqlite::Transaction<'_>, sprint: &str) -> Result<DependencyWaitState, String> {
    let rows = tx.prepare("SELECT 'handback',d.handback_id,d.details_json,COALESCE(route.work_unit_id,''),COALESCE(route.route_fingerprint,''),COALESCE(h.sprint_id,''),COALESCE(h.eligibility_state,''),COALESCE(h.handler_ready_at,''),CASE WHEN settled.work_unit_id IS NULL THEN '' ELSE 'settled' END FROM sprint_runner_handback_dispositions d JOIN sprint_runner_handback_deliveries delivery ON delivery.handback_id=d.handback_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=d.handback_id LEFT JOIN work_unit_handler_activations h ON h.work_unit_id=route.work_unit_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=h.work_unit_id WHERE delivery.sprint_id=?1 AND d.movement_kind='wait_for_agent_dependency' UNION ALL SELECT 'epic',q.handback_id,q.request_json,COALESCE(route.work_unit_id,''),COALESCE(route.route_fingerprint,''),COALESCE(h.sprint_id,''),COALESCE(h.eligibility_state,''),COALESCE(h.handler_ready_at,''),CASE WHEN settled.work_unit_id IS NULL THEN '' ELSE 'settled' END FROM epic_runner_escalation_downstream_requests q JOIN epic_runner_escalation_receivers receiver ON receiver.handback_id=q.handback_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=q.handback_id LEFT JOIN work_unit_handler_activations h ON h.work_unit_id=route.work_unit_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=h.work_unit_id WHERE receiver.sprint_id=?1 AND q.request_kind='existing_agent_achievable_dependency' ORDER BY 1,2").map_err(|error| error.to_string())?.query_map([sprint], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,String>(8)?))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    let mut state = DependencyWaitState::default();
    for (kind, handback, json, route, fingerprint, handler_sprint, eligibility, ready, settled) in rows {
        validate_dependency_wait_payload(&json, kind.as_str())?;
        let route_state = if route.is_empty()
            || fingerprint != dependency_route_fingerprint(&handback, &route)
            || handler_sprint != sprint
        {
            state.unavailable = true;
            "unavailable"
        } else if eligibility == "eligible" && !ready.is_empty() && settled.is_empty() {
            state.active = true;
            "active"
        } else if !settled.is_empty() {
            "resolved"
        } else {
            state.unavailable = true;
            "unavailable"
        };
        state.identity.push(format!("dependency-wait\u{1e}{kind}\u{1e}{handback}\u{1e}{json}\u{1e}{route}\u{1e}{fingerprint}\u{1e}{handler_sprint}\u{1e}{eligibility}\u{1e}{ready}\u{1e}{settled}\u{1e}{route_state}"));
    }
    Ok(state)
}

fn reconcile_unbound_dependency_routes(connection: &mut Connection, sprint: &str) -> Result<(), String> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
    let waits = transaction.prepare("SELECT 'handback',d.handback_id,d.details_json FROM sprint_runner_handback_dispositions d JOIN sprint_runner_handback_deliveries delivery ON delivery.handback_id=d.handback_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=d.handback_id WHERE delivery.sprint_id=?1 AND d.movement_kind='wait_for_agent_dependency' AND route.handback_id IS NULL UNION ALL SELECT 'epic',q.handback_id,q.request_json FROM epic_runner_escalation_downstream_requests q JOIN epic_runner_escalation_receivers receiver ON receiver.handback_id=q.handback_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=q.handback_id WHERE receiver.sprint_id=?1 AND q.request_kind='existing_agent_achievable_dependency' AND route.handback_id IS NULL ORDER BY 1,2").map_err(|error| error.to_string())?.query_map([sprint], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    for (kind, handback, json) in waits {
        validate_dependency_wait_payload(&json, kind.as_str())?;
        let routes = transaction.prepare("SELECT h.work_unit_id FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=h.work_unit_id WHERE h.sprint_id=?1 AND h.eligibility_state='eligible' AND h.handler_ready_at IS NOT NULL AND settled.work_unit_id IS NULL ORDER BY h.work_unit_id").map_err(|error| error.to_string())?.query_map([sprint], |row| row.get::<_,String>(0)).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
        if routes.len() != 1 { continue; }
        let route = &routes[0];
        let fingerprint = dependency_route_fingerprint(&handback, route);
        let changed = transaction.execute("INSERT OR IGNORE INTO sprint_handback_dependency_routes (handback_id,work_unit_id,route_fingerprint,recorded_at) VALUES (?1,?2,?3,?4)",params![handback,route,fingerprint,chrono::Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        if changed == 0 {
            let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_handback_dependency_routes WHERE handback_id=?1 AND work_unit_id=?2 AND route_fingerprint=?3)",params![handback,route,fingerprint],|row|row.get(0)).map_err(|error| error.to_string())?;
            if !exact { return Err("Sprint dependency-route conflict".into()); }
        }
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn validate_dependency_wait_payload(json: &str, kind: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| "malformed agent dependency route".to_owned())?;
    let has = |key: &str| value.get(key).and_then(|item| item.as_str()).is_some_and(|item| !item.trim().is_empty());
    let valid = match kind {
        "handback" => has("dependencyOwner") && has("dependencyOwnerClassification") && has("enablingResult") && has("resumptionPath"),
        "epic" => value.get("target").and_then(|item| item.as_str()) == Some("existing_agent_achievable_dependency") && has("dependency") && has("request") && has("resumptionPath"),
        _ => false,
    };
    valid.then_some(()).ok_or_else(|| "malformed agent dependency route".to_owned())
}

fn dependency_route_fingerprint(handback: &str, route: &str) -> String {
    let prefix = "sprint-handback-dependency-route";
    let mut hash = Sha256::new();
    hash.update(prefix.as_bytes());
    hash.update([0]);
    hash.update(format!("{handback}:{route}").as_bytes());
    format!("{prefix}-{:x}", hash.finalize())
}
fn digest(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("scs-{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::thread;
    use tempfile::TempDir;

    fn legacy_file_schema(path: &Path) -> Connection {
        let connection = Connection::open(path).unwrap();
        // Deliberately omits the SCS route table: this is an accepted pre-SCS durable shape.
        connection.execute_batch("CREATE TABLE initiated_sprints(id TEXT PRIMARY KEY,epic_id TEXT);CREATE TABLE sprint_runner_transitions(sprint_id TEXT,epic_id TEXT);CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_revision_id TEXT,epic_id TEXT,sprint_id TEXT);CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_at TEXT);CREATE TABLE work_slice_planning_episodes(planning_point_id TEXT PRIMARY KEY,sprint_id TEXT);CREATE TABLE work_slice_execution_graph_completions(materialization_id TEXT,accepted_revision_id TEXT);CREATE TABLE work_slice_execution_settlements(materialization_id TEXT,graph_completion_materialization_id TEXT);CREATE TABLE work_slice_planning_point_execution_settlements(planning_point_id TEXT,materialization_id TEXT,work_slice_execution_materialization_id TEXT);CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT);CREATE TABLE work_unit_settlements(work_unit_id TEXT);CREATE TABLE work_unit_retry_attempts(work_unit_id TEXT);CREATE TABLE work_unit_no_progress_handbacks(handback_id TEXT,work_unit_id TEXT,context_fingerprint TEXT);CREATE TABLE sprint_runner_handback_deliveries(handback_id TEXT,sprint_id TEXT);CREATE TABLE sprint_runner_handback_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE epic_runner_escalation_downstream_requests(handback_id TEXT,request_kind TEXT,request_json TEXT);CREATE TABLE epic_runner_escalation_receivers(handback_id TEXT,sprint_id TEXT,epic_id TEXT,correlation_fingerprint TEXT);CREATE TABLE epic_runner_escalation_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE work_slice_execution_attentions(materialization_id TEXT);CREATE TABLE work_unit_execution_attentions(materialization_id TEXT);CREATE TABLE epic_runner_escalation_attentions(handback_id TEXT);CREATE TABLE work_unit_handler_activations(work_unit_id TEXT,sprint_id TEXT,eligibility_state TEXT,handler_ready_at TEXT);CREATE TABLE work_unit_implementer_activations(work_unit_id TEXT);INSERT INTO initiated_sprints VALUES('sprint','epic');INSERT INTO sprint_runner_transitions VALUES('sprint','epic');").unwrap();
        connection
    }

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE initiated_sprints(id TEXT PRIMARY KEY,epic_id TEXT);CREATE TABLE sprint_runner_transitions(sprint_id TEXT,epic_id TEXT);CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_revision_id TEXT,epic_id TEXT,sprint_id TEXT);CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_at TEXT);CREATE TABLE work_slice_planning_episodes(planning_point_id TEXT PRIMARY KEY,sprint_id TEXT);CREATE TABLE work_slice_execution_graph_completions(materialization_id TEXT,accepted_revision_id TEXT);CREATE TABLE work_slice_execution_settlements(materialization_id TEXT,graph_completion_materialization_id TEXT);CREATE TABLE work_slice_planning_point_execution_settlements(planning_point_id TEXT,materialization_id TEXT,work_slice_execution_materialization_id TEXT);CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT);CREATE TABLE work_unit_settlements(work_unit_id TEXT);CREATE TABLE work_unit_retry_attempts(work_unit_id TEXT);CREATE TABLE work_unit_no_progress_handbacks(handback_id TEXT,work_unit_id TEXT,context_fingerprint TEXT);CREATE TABLE sprint_runner_handback_deliveries(handback_id TEXT,sprint_id TEXT);CREATE TABLE sprint_runner_handback_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE sprint_handback_dependency_routes(handback_id TEXT PRIMARY KEY,work_unit_id TEXT NOT NULL,route_fingerprint TEXT NOT NULL UNIQUE,recorded_at TEXT NOT NULL);CREATE TABLE epic_runner_escalation_downstream_requests(handback_id TEXT,request_kind TEXT,request_json TEXT);CREATE TABLE epic_runner_escalation_receivers(handback_id TEXT,sprint_id TEXT,epic_id TEXT,correlation_fingerprint TEXT);CREATE TABLE epic_runner_escalation_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE work_slice_execution_attentions(materialization_id TEXT);CREATE TABLE work_unit_execution_attentions(materialization_id TEXT);CREATE TABLE epic_runner_escalation_attentions(handback_id TEXT);CREATE TABLE work_unit_handler_activations(work_unit_id TEXT,sprint_id TEXT,eligibility_state TEXT,handler_ready_at TEXT);CREATE TABLE work_unit_implementer_activations(work_unit_id TEXT);INSERT INTO initiated_sprints VALUES('sprint','epic');INSERT INTO sprint_runner_transitions VALUES('sprint','epic');").unwrap();
        connection.execute_batch("ALTER TABLE epic_runner_escalation_attentions ADD COLUMN attention_id TEXT;ALTER TABLE epic_runner_escalation_attentions ADD COLUMN attention_json TEXT;ALTER TABLE epic_runner_escalation_attentions ADD COLUMN requested_at TEXT;").unwrap();
        initialize(&connection).unwrap();
        connection
    }
    fn accepted_materialization(connection: &Connection) {
        connection.execute_batch("INSERT INTO work_slice_proposal_revisions VALUES('revision','point','now');INSERT INTO work_slice_planning_episodes VALUES('point','sprint');INSERT INTO work_unit_materializations VALUES('materialization','point','revision','epic','sprint');INSERT INTO work_units VALUES('unit','materialization');").unwrap();
    }
    fn second_accepted_materialization(connection: &Connection) {
        connection.execute_batch("INSERT INTO work_slice_proposal_revisions VALUES('revision-2','point-2','later');INSERT INTO work_slice_planning_episodes VALUES('point-2','sprint');INSERT INTO work_unit_materializations VALUES('materialization-2','point-2','revision-2','epic','sprint');INSERT INTO work_units VALUES('unit-2','materialization-2');").unwrap();
    }
    fn structured_source(connection: &Connection, handback: &str, attention_id: &str) {
        connection.execute("INSERT INTO epic_runner_escalation_receivers VALUES(?1,'sprint','epic',?2)", params![handback, format!("correlation-{handback}")]).unwrap();
        connection.execute("INSERT INTO epic_runner_escalation_attentions (handback_id,attention_id,attention_json,requested_at) VALUES(?1,?2,?3,?4)", params![handback, attention_id, "{\"reason\":\"A bounded external decision is required.\",\"authorityNeeded\":\"designated authority\",\"evidenceContext\":\"the exact unresolved Sprint concern\",\"resumptionPath\":\"resume this exact Sprint decision\"}", "2030-01-01T00:00:00Z"]).unwrap();
    }
    fn terminal_facts(connection: &Connection) {
        connection.execute_batch("INSERT INTO work_slice_execution_graph_completions VALUES('materialization','revision');INSERT INTO work_slice_execution_settlements VALUES('materialization','materialization');INSERT INTO work_slice_planning_point_execution_settlements VALUES('point','materialization','materialization');INSERT INTO work_unit_settlements VALUES('unit');").unwrap();
    }

    #[test]
    fn continuing_paths_preserve_eligible_retry_and_dependency_waits() {
        let mut c = fixture();
        accepted_materialization(&c);
        c.execute(
            "INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now')",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "continuing");
        c.execute("DELETE FROM work_unit_handler_activations", [])
            .unwrap();
        c.execute("INSERT INTO work_unit_retry_attempts VALUES('unit')", [])
            .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "continuing");
        c.execute("DELETE FROM work_unit_retry_attempts", [])
            .unwrap();
        c.execute(
            "INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now')",
            [],
        )
        .unwrap();
        c.execute_batch("INSERT INTO sprint_runner_handback_deliveries VALUES('handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('handback','wait_for_agent_dependency','{\"dependencyOwner\":\"handler\",\"dependencyOwnerClassification\":\"work_unit_handler\",\"enablingResult\":\"review\",\"resumptionPath\":\"reassess\"}');").unwrap();
        c.execute("INSERT INTO sprint_handback_dependency_routes VALUES('handback','unit',?1,'now')", [dependency_route_fingerprint("handback", "unit")]).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1",[],|r|r.get(0)).unwrap(),"wait_for_agent_dependency");
        c.execute("DELETE FROM work_unit_handler_activations", [])
            .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1",[],|r|r.get(0)).unwrap(),"dependency_route_unavailable");
    }

    #[test]
    fn exact_handback_and_epic_waits_fail_closed_until_each_handler_settles() {
        let mut c = fixture();
        accepted_materialization(&c);
        second_accepted_materialization(&c);
        terminal_facts(&c);
        c.execute_batch("INSERT INTO work_slice_execution_graph_completions VALUES('materialization-2','revision-2');INSERT INTO work_slice_execution_settlements VALUES('materialization-2','materialization-2');INSERT INTO work_slice_planning_point_execution_settlements VALUES('point-2','materialization-2','materialization-2');INSERT INTO work_unit_settlements VALUES('unit-2');").unwrap();
        c.execute("DELETE FROM work_unit_settlements", []).unwrap();
        c.execute_batch("INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now');INSERT INTO work_unit_handler_activations VALUES('unit-2','sprint','eligible','now');INSERT INTO sprint_runner_handback_deliveries VALUES('handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('handback','wait_for_agent_dependency','{\"dependencyOwner\":\"handler\",\"dependencyOwnerClassification\":\"work_unit_handler\",\"enablingResult\":\"review\",\"resumptionPath\":\"reassess\"}');INSERT INTO epic_runner_escalation_receivers VALUES('epic-wait','sprint','epic','correlation');INSERT INTO epic_runner_escalation_downstream_requests VALUES('epic-wait','existing_agent_achievable_dependency','{\"target\":\"existing_agent_achievable_dependency\",\"dependency\":\"handler result\",\"request\":\"continue\",\"resumptionPath\":\"reassess\"}');").unwrap();
        c.execute("INSERT INTO sprint_handback_dependency_routes VALUES('handback','unit',?1,'now')", [dependency_route_fingerprint("handback", "unit")]).unwrap();
        c.execute("INSERT INTO sprint_handback_dependency_routes VALUES('epic-wait','unit-2',?1,'now')", [dependency_route_fingerprint("epic-wait", "unit-2")]).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "continuing");
        c.execute("UPDATE work_unit_handler_activations SET eligibility_state='ineligible' WHERE work_unit_id='unit'", []).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |r| r.get(0)).unwrap(), "dependency_route_unavailable");
        assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_upward_results WHERE result_kind='settled'", [], |r| r.get(0)).unwrap(), 0);
        c.execute("UPDATE work_unit_handler_activations SET eligibility_state='eligible',handler_ready_at=NULL WHERE work_unit_id='unit'", []).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |r| r.get(0)).unwrap(), "dependency_route_unavailable");
        c.execute("UPDATE work_unit_handler_activations SET handler_ready_at='ready' WHERE work_unit_id='unit'", []).unwrap();
        c.execute("UPDATE work_unit_handler_activations SET handler_ready_at=NULL WHERE work_unit_id='unit-2'", []).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |r| r.get(0)).unwrap(), "dependency_route_unavailable");
        c.execute_batch("INSERT INTO work_unit_settlements VALUES('unit');INSERT INTO work_unit_settlements VALUES('unit-2');").unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "settled");
        let counts: (i64, i64) = c.query_row("SELECT (SELECT COUNT(*) FROM sprint_continuation_decisions),(SELECT COUNT(*) FROM sprint_upward_results)", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<(i64,i64),_,_>("SELECT (SELECT COUNT(*) FROM sprint_continuation_decisions),(SELECT COUNT(*) FROM sprint_upward_results)", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap(), counts);
    }

    #[test]
    fn fresh_reconciliation_binds_one_legacy_handback_route_and_releases_only_that_route() {
        let mut c = fixture();
        accepted_materialization(&c);
        terminal_facts(&c);
        c.execute("DELETE FROM work_unit_settlements", []).unwrap();
        c.execute_batch("INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now');INSERT INTO sprint_runner_handback_deliveries VALUES('legacy-handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('legacy-handback','wait_for_agent_dependency','{\"dependencyOwner\":\"handler\",\"dependencyOwnerClassification\":\"work_unit_handler\",\"enablingResult\":\"review\",\"resumptionPath\":\"reassess\"}');").unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "continuing");
        assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_handback_dependency_routes WHERE handback_id='legacy-handback' AND work_unit_id='unit'", [], |row| row.get(0)).unwrap(), 1);
        let active_input: String = c.query_row("SELECT input_fingerprint FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        let before: (i64, i64) = c.query_row("SELECT (SELECT COUNT(*) FROM sprint_handback_dependency_routes),(SELECT COUNT(*) FROM sprint_continuation_decisions)", [], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<(i64,i64),_,_>("SELECT (SELECT COUNT(*) FROM sprint_handback_dependency_routes),(SELECT COUNT(*) FROM sprint_continuation_decisions)", [], |row| Ok((row.get(0)?,row.get(1)?))).unwrap(), before);
        c.execute("INSERT INTO work_unit_settlements VALUES('unit')", []).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "settled");
        assert_ne!(c.query_row::<String,_,_>("SELECT input_fingerprint FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |row| row.get(0)).unwrap(), active_input);
    }

    #[test]
    fn conflicting_persisted_dependency_route_fails_closed_without_overwrite() {
        let mut c = fixture();
        accepted_materialization(&c);
        terminal_facts(&c);
        c.execute("DELETE FROM work_unit_settlements", []).unwrap();
        c.execute_batch("INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now');INSERT INTO sprint_runner_handback_deliveries VALUES('conflict-handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('conflict-handback','wait_for_agent_dependency','{\"dependencyOwner\":\"handler\",\"dependencyOwnerClassification\":\"work_unit_handler\",\"enablingResult\":\"review\",\"resumptionPath\":\"reassess\"}');").unwrap();
        reconcile(&mut c).unwrap();
        c.execute("UPDATE sprint_handback_dependency_routes SET route_fingerprint='conflict' WHERE handback_id='conflict-handback'", []).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
        assert_eq!(c.query_row::<String,_,_>("SELECT route_fingerprint FROM sprint_handback_dependency_routes WHERE handback_id='conflict-handback'", [], |row| row.get(0)).unwrap(), "conflict");
        assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_upward_results WHERE result_kind='settled'", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn fresh_reconciliation_keeps_legacy_epic_wait_unavailable_when_route_is_ambiguous() {
        let mut c = fixture();
        accepted_materialization(&c);
        terminal_facts(&c);
        c.execute_batch("INSERT INTO work_unit_handler_activations VALUES('unit-a','sprint','eligible','now');INSERT INTO work_unit_handler_activations VALUES('unit-b','sprint','eligible','now');INSERT INTO epic_runner_escalation_receivers VALUES('legacy-epic','sprint','epic','correlation');INSERT INTO epic_runner_escalation_downstream_requests VALUES('legacy-epic','existing_agent_achievable_dependency','{\"target\":\"existing_agent_achievable_dependency\",\"dependency\":\"handler result\",\"request\":\"continue\",\"resumptionPath\":\"reassess\"}');").unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1", [], |row| row.get(0)).unwrap(), "dependency_route_unavailable");
        assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_handback_dependency_routes", [], |row| row.get(0)).unwrap(), 0);
        let before: (i64, i64, i64) = c.query_row("SELECT (SELECT COUNT(*) FROM sprint_continuation_decisions),(SELECT COUNT(*) FROM sprint_upward_results),(SELECT COUNT(*) FROM sprint_continuation_attentions)", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<(i64,i64,i64),_,_>("SELECT (SELECT COUNT(*) FROM sprint_continuation_decisions),(SELECT COUNT(*) FROM sprint_upward_results),(SELECT COUNT(*) FROM sprint_continuation_attentions)", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap(), before);
        assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='epic_settlements'", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn fresh_open_recovers_legacy_handback_and_epic_dependency_waits_without_settlement() {
        let directory = TempDir::new().unwrap();
        let handback_path = directory.path().join("legacy-handback.sqlite");
        {
            let c = legacy_file_schema(&handback_path);
            accepted_materialization(&c);
            terminal_facts(&c);
            c.execute_batch("DELETE FROM work_unit_settlements;INSERT INTO work_unit_handler_activations VALUES('unit','sprint','eligible','now');INSERT INTO sprint_runner_handback_deliveries VALUES('legacy-handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('legacy-handback','wait_for_agent_dependency','{\"dependencyOwner\":\"handler\",\"dependencyOwnerClassification\":\"work_unit_handler\",\"enablingResult\":\"review\",\"resumptionPath\":\"reassess\"}');").unwrap();
            initialize(&c).unwrap();
        }
        {
            let mut c = Connection::open(&handback_path).unwrap();
            reconcile(&mut c).unwrap();
            assert_eq!(statuses(&c).unwrap()[0].1.state, "continuing");
            assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_handback_dependency_routes WHERE handback_id='legacy-handback' AND work_unit_id='unit'", [], |row| row.get(0)).unwrap(), 1);
        }
        {
            let mut c = Connection::open(&handback_path).unwrap();
            c.execute("INSERT INTO work_unit_settlements VALUES('unit')", []).unwrap();
            reconcile(&mut c).unwrap();
            assert_eq!(statuses(&c).unwrap()[0].1.state, "settled");
            assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_upward_results WHERE result_kind='settled'", [], |row| row.get(0)).unwrap(), 1);
        }

        let epic_path = directory.path().join("legacy-epic.sqlite");
        {
            let c = legacy_file_schema(&epic_path);
            accepted_materialization(&c);
            terminal_facts(&c);
            c.execute_batch("DELETE FROM work_unit_settlements;INSERT INTO work_unit_handler_activations VALUES('unit-a','sprint','eligible','now');INSERT INTO work_unit_handler_activations VALUES('unit-b','sprint','eligible','now');INSERT INTO epic_runner_escalation_receivers VALUES('legacy-epic','sprint','epic','correlation');INSERT INTO epic_runner_escalation_downstream_requests VALUES('legacy-epic','existing_agent_achievable_dependency','{\"target\":\"existing_agent_achievable_dependency\",\"dependency\":\"handler result\",\"request\":\"continue\",\"resumptionPath\":\"reassess\"}');").unwrap();
            initialize(&c).unwrap();
        }
        for _ in 0..2 {
            let mut c = Connection::open(&epic_path).unwrap();
            reconcile(&mut c).unwrap();
            assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
            assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_handback_dependency_routes", [], |row| row.get(0)).unwrap(), 0);
            assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_upward_results WHERE result_kind='settled'", [], |row| row.get(0)).unwrap(), 0);
            assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='epic_settlements'", [], |row| row.get(0)).unwrap(), 0);
        }
    }
    #[test]
    fn structured_attention_and_handback_are_not_settlement() {
        let mut c = fixture();
        accepted_materialization(&c);
        c.execute(
            "INSERT INTO work_unit_no_progress_handbacks VALUES('handback','unit','context')",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
        assert_eq!(
            c.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM sprint_continuation_attentions",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            1
        );
        c.execute(
            "INSERT INTO work_slice_execution_attentions VALUES('materialization')",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
        let mut c = fixture();
        accepted_materialization(&c);
        terminal_facts(&c);
        c.execute(
            "DELETE FROM work_slice_planning_point_execution_settlements",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_ne!(statuses(&c).unwrap()[0].1.state, "settled");
        let mut c = fixture();
        accepted_materialization(&c);
        c.execute(
            "INSERT INTO epic_runner_escalation_receivers VALUES('handback','sprint','foreign','correlation')",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
    }
    #[test]
    fn repeated_same_source_attention_survives_canonical_change_and_reopen() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("repeated-source.sqlite");
        let mut c = fixture();
        accepted_materialization(&c);
        structured_source(&c, "source-handback", "source-attention");
        reconcile(&mut c).unwrap();
        second_accepted_materialization(&c);
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_decisions", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_attentions WHERE source_attention_id='source-attention'", [], |row| row.get(0)).unwrap(), 2);
        c.execute("VACUUM INTO ?1", [path.to_str().unwrap()]).unwrap();
        drop(c);

        let mut reopened = Connection::open(&path).unwrap();
        reconcile(&mut reopened).unwrap();
        assert_eq!(reopened.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_decisions", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(reopened.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_attentions WHERE source_attention_id='source-attention'", [], |row| row.get(0)).unwrap(), 2);
    }
    #[test]
    fn multiple_sources_before_decisions_never_receive_positional_attention_correlation() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("ambiguous-sources.sqlite");
        let mut c = fixture();
        accepted_materialization(&c);
        structured_source(&c, "source-handback-a", "source-attention-a");
        structured_source(&c, "source-handback-b", "source-attention-b");
        reconcile(&mut c).unwrap();
        second_accepted_materialization(&c);
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_decisions", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_attentions WHERE source_attention_id IS NULL", [], |row| row.get(0)).unwrap(), 2);
        c.execute("VACUUM INTO ?1", [path.to_str().unwrap()]).unwrap();
        drop(c);

        let mut reopened = Connection::open(&path).unwrap();
        reconcile(&mut reopened).unwrap();
        assert_eq!(reopened.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_attentions WHERE source_attention_id IS NULL", [], |row| row.get(0)).unwrap(), 2);
    }
    #[test]
    fn conflicting_source_attention_correlation_fails_closed() {
        let mut c = fixture();
        accepted_materialization(&c);
        structured_source(&c, "source-handback", "source-attention");
        reconcile(&mut c).unwrap();
        c.execute(
            "UPDATE sprint_continuation_attentions SET source_attention_id='conflicting-source'",
            [],
        )
        .unwrap();
        assert!(reconcile(&mut c).is_err());
        assert_eq!(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_upward_results WHERE result_kind='settled'", [], |row| row.get(0)).unwrap(), 0);
    }
    #[test]
    fn exact_terminal_facts_settle_and_emit_only_sprint_result() {
        let mut c = fixture();
        accepted_materialization(&c);
        terminal_facts(&c);
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "settled");
        assert_eq!(
            c.query_row::<String, _, _>("SELECT result_kind FROM sprint_upward_results", [], |r| r
                .get(0))
                .unwrap(),
            "settled"
        );
        assert_eq!(
            c.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='epic_settlements'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            0
        );
    }
    #[test]
    fn foreign_or_malformed_terminal_facts_fail_closed() {
        let mut c = fixture();
        accepted_materialization(&c);
        c.execute(
            "UPDATE work_unit_materializations SET epic_id='foreign'",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
        let mut c = fixture();
        accepted_materialization(&c);
        c.execute(
            "INSERT INTO work_slice_execution_settlements VALUES('materialization','missing')",
            [],
        )
        .unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(statuses(&c).unwrap()[0].1.state, "attention");
    }
    #[test]
    fn conflicting_persisted_upward_result_fails_closed_without_overwrite() {
        let mut c = fixture();
        accepted_materialization(&c);
        reconcile(&mut c).unwrap();
        c.execute(
            "UPDATE sprint_upward_results SET result_kind='attention'",
            [],
        )
        .unwrap();
        assert!(reconcile(&mut c).is_err());
        assert_eq!(
            c.query_row::<String, _, _>(
                "SELECT result_kind FROM sprint_upward_results",
                [],
                |row| row.get(0)
            )
            .unwrap(),
            "attention"
        );
    }
    #[test]
    fn reopen_and_replay_keep_one_decision_and_result() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("state.sqlite");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE initiated_sprints(id TEXT PRIMARY KEY,epic_id TEXT);CREATE TABLE sprint_runner_transitions(sprint_id TEXT,epic_id TEXT);CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_revision_id TEXT,epic_id TEXT,sprint_id TEXT);CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_at TEXT);CREATE TABLE work_slice_planning_episodes(planning_point_id TEXT PRIMARY KEY,sprint_id TEXT);CREATE TABLE work_slice_execution_graph_completions(materialization_id TEXT,accepted_revision_id TEXT);CREATE TABLE work_slice_execution_settlements(materialization_id TEXT,graph_completion_materialization_id TEXT);CREATE TABLE work_slice_planning_point_execution_settlements(planning_point_id TEXT,materialization_id TEXT,work_slice_execution_materialization_id TEXT);CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT);CREATE TABLE work_unit_settlements(work_unit_id TEXT);CREATE TABLE work_unit_retry_attempts(work_unit_id TEXT);CREATE TABLE work_unit_no_progress_handbacks(handback_id TEXT,work_unit_id TEXT,context_fingerprint TEXT);CREATE TABLE sprint_runner_handback_deliveries(handback_id TEXT,sprint_id TEXT);CREATE TABLE sprint_runner_handback_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE epic_runner_escalation_downstream_requests(handback_id TEXT,request_kind TEXT,request_json TEXT);CREATE TABLE epic_runner_escalation_receivers(handback_id TEXT,sprint_id TEXT,epic_id TEXT,correlation_fingerprint TEXT);CREATE TABLE epic_runner_escalation_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE work_slice_execution_attentions(materialization_id TEXT);CREATE TABLE work_unit_execution_attentions(materialization_id TEXT);CREATE TABLE epic_runner_escalation_attentions(handback_id TEXT);CREATE TABLE work_unit_handler_activations(work_unit_id TEXT,sprint_id TEXT,eligibility_state TEXT,handler_ready_at TEXT);CREATE TABLE work_unit_implementer_activations(work_unit_id TEXT);INSERT INTO initiated_sprints VALUES('sprint','epic');INSERT INTO sprint_runner_transitions VALUES('sprint','epic');").unwrap();
            initialize(&c).unwrap();
        }
        {
            let mut c = Connection::open(&path).unwrap();
            reconcile(&mut c).unwrap();
            c.execute_batch("DELETE FROM sprint_continuation_current_decisions;DELETE FROM sprint_upward_results;")
                .unwrap();
        }
        let first = path.clone();
        let second = path.clone();
        let one = thread::spawn(move || {
            let mut c = Connection::open(first).unwrap();
            c.busy_timeout(std::time::Duration::from_secs(1)).unwrap();
            reconcile(&mut c)
        });
        let two = thread::spawn(move || {
            let mut c = Connection::open(second).unwrap();
            c.busy_timeout(std::time::Duration::from_secs(1)).unwrap();
            reconcile(&mut c)
        });
        let one = one.join().unwrap();
        let two = two.join().unwrap();
        assert!(one.is_ok() && two.is_ok());
        let mut c = Connection::open(&path).unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(
            c.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM sprint_continuation_decisions",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_upward_results", [], |r| r
                .get(0))
                .unwrap(),
            1
        );
    }
}
