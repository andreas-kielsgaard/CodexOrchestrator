//! Durable, application-owned Sprint continuation decisions.  A decision is neither an Epic
//! receipt nor any higher-level settlement.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

pub(crate) const SPRINT_CONTINUATION_SETTLEMENT_SCHEMA: &str = r#"
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
        .map_err(|error| error.to_string())
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
            "INSERT OR IGNORE INTO sprint_continuation_attentions (decision_id,attention_id,attention_code,attention_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5)",
            params![decision_id, attention_id, kind, attention_fingerprint, recorded_at],
        ).map_err(|error| error.to_string())?;
        if changed == 0 {
            let exact:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_continuation_attentions WHERE decision_id=?1 AND attention_id=?2 AND attention_code=?3 AND attention_fingerprint=?4 AND recorded_at=?5)",params![decision_id,attention_id,kind,attention_fingerprint,recorded_at],|row|row.get(0)).map_err(|error|error.to_string())?;
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
    materialization_count: i64,
    correlated_materialization_count: i64,
    terminal_materialization_count: i64,
    malformed_chronology: bool,
    unresolved_retry: bool,
    unresolved_handback: bool,
    agent_dependency_wait: bool,
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
        let agent_dependency_wait = exists(tx, "SELECT 1 FROM sprint_runner_handback_dispositions d JOIN sprint_runner_handback_deliveries delivery ON delivery.handback_id=d.handback_id WHERE delivery.sprint_id=?1 AND d.movement_kind='wait_for_agent_dependency' UNION ALL SELECT 1 FROM epic_runner_escalation_downstream_requests r JOIN epic_runner_escalation_receivers receiver ON receiver.handback_id=r.handback_id WHERE receiver.sprint_id=?1 AND r.request_kind='existing_agent_achievable_dependency'", sprint)?;
        let durable_attention = exists(tx, "SELECT 1 FROM work_slice_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.sprint_id=?1 UNION ALL SELECT 1 FROM work_unit_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.sprint_id=?1 UNION ALL SELECT 1 FROM epic_runner_escalation_attentions a JOIN epic_runner_escalation_receivers r ON r.handback_id=a.handback_id WHERE r.sprint_id=?1", sprint)?;
        let eligible_work = exists(tx, "SELECT 1 FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements s ON s.work_unit_id=h.work_unit_id WHERE h.sprint_id=?1 AND h.eligibility_state='eligible' AND h.handler_ready_at IS NOT NULL AND s.work_unit_id IS NULL", sprint)?;
        let outstanding_continuation = exists(tx, "SELECT 1 FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements s ON s.work_unit_id=h.work_unit_id WHERE h.sprint_id=?1 AND s.work_unit_id IS NULL UNION ALL SELECT 1 FROM work_unit_implementer_activations i JOIN work_units u ON u.work_unit_id=i.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE m.sprint_id=?1 AND s.work_unit_id IS NULL", sprint)?;
        let stale_epic_context = exists(tx, "SELECT 1 FROM epic_runner_escalation_receivers r JOIN sprint_runner_transitions t ON t.sprint_id=r.sprint_id WHERE r.sprint_id=?1 AND r.epic_id<>t.epic_id", sprint)?;
        Ok(Self {
            identity,
            materialization_count,
            correlated_materialization_count,
            terminal_materialization_count,
            malformed_chronology,
            unresolved_retry,
            unresolved_handback,
            agent_dependency_wait,
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
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.materialization_count,
            self.correlated_materialization_count,
            self.terminal_materialization_count,
            self.malformed_chronology as u8,
            self.unresolved_retry as u8,
            self.unresolved_handback as u8,
            self.agent_dependency_wait as u8,
            self.durable_attention as u8,
            self.eligible_work as u8,
            self.outstanding_continuation as u8,
            self.stale_epic_context as u8
        );
        format!("{aggregate}:{}", self.identity.join("\u{1f}"))
    }
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
fn digest(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("scs-{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE initiated_sprints(id TEXT PRIMARY KEY,epic_id TEXT);CREATE TABLE sprint_runner_transitions(sprint_id TEXT,epic_id TEXT);CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_revision_id TEXT,epic_id TEXT,sprint_id TEXT);CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_at TEXT);CREATE TABLE work_slice_planning_episodes(planning_point_id TEXT PRIMARY KEY,sprint_id TEXT);CREATE TABLE work_slice_execution_graph_completions(materialization_id TEXT,accepted_revision_id TEXT);CREATE TABLE work_slice_execution_settlements(materialization_id TEXT,graph_completion_materialization_id TEXT);CREATE TABLE work_slice_planning_point_execution_settlements(planning_point_id TEXT,materialization_id TEXT,work_slice_execution_materialization_id TEXT);CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT);CREATE TABLE work_unit_settlements(work_unit_id TEXT);CREATE TABLE work_unit_retry_attempts(work_unit_id TEXT);CREATE TABLE work_unit_no_progress_handbacks(handback_id TEXT,work_unit_id TEXT,context_fingerprint TEXT);CREATE TABLE sprint_runner_handback_deliveries(handback_id TEXT,sprint_id TEXT);CREATE TABLE sprint_runner_handback_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE epic_runner_escalation_downstream_requests(handback_id TEXT,request_kind TEXT,request_json TEXT);CREATE TABLE epic_runner_escalation_receivers(handback_id TEXT,sprint_id TEXT,epic_id TEXT,correlation_fingerprint TEXT);CREATE TABLE epic_runner_escalation_dispositions(handback_id TEXT,movement_kind TEXT,details_json TEXT);CREATE TABLE work_slice_execution_attentions(materialization_id TEXT);CREATE TABLE work_unit_execution_attentions(materialization_id TEXT);CREATE TABLE epic_runner_escalation_attentions(handback_id TEXT);CREATE TABLE work_unit_handler_activations(work_unit_id TEXT,sprint_id TEXT,eligibility_state TEXT,handler_ready_at TEXT);CREATE TABLE work_unit_implementer_activations(work_unit_id TEXT);INSERT INTO initiated_sprints VALUES('sprint','epic');INSERT INTO sprint_runner_transitions VALUES('sprint','epic');").unwrap();
        initialize(&connection).unwrap();
        connection
    }
    fn accepted_materialization(connection: &Connection) {
        connection.execute_batch("INSERT INTO work_slice_proposal_revisions VALUES('revision','point','now');INSERT INTO work_slice_planning_episodes VALUES('point','sprint');INSERT INTO work_unit_materializations VALUES('materialization','point','revision','epic','sprint');INSERT INTO work_units VALUES('unit','materialization');").unwrap();
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
        c.execute_batch("INSERT INTO sprint_runner_handback_deliveries VALUES('handback','sprint');INSERT INTO sprint_runner_handback_dispositions VALUES('handback','wait_for_agent_dependency','{}');").unwrap();
        reconcile(&mut c).unwrap();
        assert_eq!(c.query_row::<String,_,_>("SELECT continuation_kind FROM sprint_continuation_decisions ORDER BY decision_sequence DESC LIMIT 1",[],|r|r.get(0)).unwrap(),"wait_for_agent_dependency");
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
