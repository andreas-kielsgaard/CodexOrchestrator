//! Durable Epic settlement. Terminal readiness is an input, not a settlement fact.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

pub(crate) const EPIC_SETTLEMENT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS epic_settlement_requests (
  epic_id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE,
  approved_plan_fingerprint TEXT NOT NULL, terminal_readiness_id TEXT NOT NULL UNIQUE,
  eligibility_fingerprint TEXT NOT NULL UNIQUE, requested_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_settlement_authorizations (
  request_id TEXT PRIMARY KEY REFERENCES epic_settlement_requests(request_id) ON DELETE RESTRICT,
  authorization_id TEXT NOT NULL UNIQUE, authorization_fingerprint TEXT NOT NULL UNIQUE,
  authorized_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_settlement_evidence (
  epic_id TEXT PRIMARY KEY REFERENCES epic_settlement_requests(epic_id) ON DELETE RESTRICT,
  evidence_id TEXT NOT NULL UNIQUE, request_id TEXT NOT NULL UNIQUE REFERENCES epic_settlement_requests(request_id) ON DELETE RESTRICT,
  authorization_id TEXT NOT NULL UNIQUE REFERENCES epic_settlement_authorizations(authorization_id) ON DELETE RESTRICT,
  evidence_fingerprint TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_settlements (
  epic_id TEXT PRIMARY KEY REFERENCES epic_settlement_requests(epic_id) ON DELETE RESTRICT,
  settlement_id TEXT NOT NULL UNIQUE, request_id TEXT NOT NULL UNIQUE REFERENCES epic_settlement_requests(request_id) ON DELETE RESTRICT,
  authorization_id TEXT NOT NULL UNIQUE REFERENCES epic_settlement_authorizations(authorization_id) ON DELETE RESTRICT,
  evidence_id TEXT NOT NULL UNIQUE REFERENCES epic_settlement_evidence(evidence_id) ON DELETE RESTRICT,
  settlement_fingerprint TEXT NOT NULL UNIQUE, persisted_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_settlement_unresolved (
  epic_id TEXT PRIMARY KEY, unresolved_id TEXT NOT NULL UNIQUE, reason_code TEXT NOT NULL,
  resumption_fact TEXT NOT NULL, snapshot_fingerprint TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_settlement_current_states (
  epic_id TEXT PRIMARY KEY, state_kind TEXT NOT NULL CHECK (state_kind IN ('unresolved','settled')),
  settlement_id TEXT UNIQUE REFERENCES epic_settlements(settlement_id) ON DELETE RESTRICT,
  unresolved_id TEXT UNIQUE REFERENCES epic_settlement_unresolved(unresolved_id) ON DELETE RESTRICT,
  source_fingerprint TEXT NOT NULL, updated_at TEXT NOT NULL,
  CHECK ((state_kind='settled' AND settlement_id IS NOT NULL AND unresolved_id IS NULL)
      OR (state_kind='unresolved' AND unresolved_id IS NOT NULL AND settlement_id IS NULL))
);
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EpicSettlementStatus {
    Settled {
        settlement_id: String,
        persisted_at: String,
    },
    Unresolved {
        reason_code: String,
        resumption_fact: String,
        recorded_at: String,
    },
}

pub(crate) fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(EPIC_SETTLEMENT_SCHEMA)
        .map_err(|error| error.to_string())
}

/// Re-evaluates authoritative Epic state in one application-owned transaction.  It records an
/// unresolved fact when the exact settlement authority is not current instead of inferring a
/// settlement from readiness or partial descendant state.
pub(crate) fn reconcile(connection: &mut Connection) -> Result<(), String> {
    let epics = connection
        .prepare("SELECT DISTINCT epic_id FROM initiated_sprints ORDER BY epic_id")
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for epic in epics {
        reconcile_one(connection, &epic)?;
    }
    Ok(())
}

pub(crate) fn statuses(
    connection: &Connection,
) -> Result<Vec<(String, EpicSettlementStatus)>, String> {
    connection
        .prepare("SELECT current.epic_id,current.state_kind,settlement.settlement_id,settlement.persisted_at,unresolved.reason_code,unresolved.resumption_fact,unresolved.recorded_at FROM epic_settlement_current_states current LEFT JOIN epic_settlements settlement ON settlement.settlement_id=current.settlement_id LEFT JOIN epic_settlement_unresolved unresolved ON unresolved.unresolved_id=current.unresolved_id ORDER BY current.epic_id")
        .map_err(|error| error.to_string())?
        .query_map([], |row| {
            let epic_id: String = row.get(0)?;
            let state: String = row.get(1)?;
            let status = match state.as_str() {
                "settled" => EpicSettlementStatus::Settled { settlement_id: row.get(2)?, persisted_at: row.get(3)? },
                "unresolved" => EpicSettlementStatus::Unresolved { reason_code: row.get(4)?, resumption_fact: row.get(5)?, recorded_at: row.get(6)? },
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok((epic_id, status))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn reconcile_one(connection: &mut Connection, epic: &str) -> Result<(), String> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let snapshot = Snapshot::load(&tx, epic)?;
    if let Err((reason, resume, fingerprint)) = snapshot.eligible() {
        persist_unresolved(&tx, epic, &reason, &resume, &fingerprint)?;
        return tx.commit().map_err(|error| error.to_string());
    }
    let eligibility = snapshot.eligibility_fingerprint();
    let request_id = digest(&format!("epic-settlement-request:{epic}:{eligibility}"));
    let authorization_id = digest(&format!("epic-settlement-authorization:{request_id}"));
    let evidence_id = digest(&format!("epic-settlement-evidence:{authorization_id}"));
    let settlement_id = digest(&format!("epic-settlement:{evidence_id}"));
    let existing: Option<(String, String, String, String)> = tx.query_row(
        "SELECT request_id,approved_plan_fingerprint,terminal_readiness_id,eligibility_fingerprint FROM epic_settlement_requests WHERE epic_id=?1",
        [epic], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|error| error.to_string())?;
    if let Some((stored_request, plan, readiness, stored_eligibility)) = existing {
        if stored_request != request_id
            || plan != snapshot.plan_fingerprint
            || readiness != snapshot.readiness_id
            || stored_eligibility != eligibility
        {
            persist_unresolved(&tx, epic, "settlement_authority_superseded", "restore the exact previously requested approved-plan and terminal-readiness authority, then reassess", &eligibility)?;
            return tx.commit().map_err(|error| error.to_string());
        }
    } else {
        tx.execute("INSERT INTO epic_settlement_requests (epic_id,request_id,approved_plan_fingerprint,terminal_readiness_id,eligibility_fingerprint,requested_at) VALUES (?1,?2,?3,?4,?5,?6)", params![epic,request_id,snapshot.plan_fingerprint,snapshot.readiness_id,eligibility,now()]).map_err(|error| error.to_string())?;
    }
    insert_exact(&tx, "INSERT OR IGNORE INTO epic_settlement_authorizations (request_id,authorization_id,authorization_fingerprint,authorized_at) VALUES (?1,?2,?3,?4)", params![request_id,authorization_id,digest(&format!("{request_id}:{eligibility}")),now()], "SELECT EXISTS(SELECT 1 FROM epic_settlement_authorizations WHERE request_id=?1 AND authorization_id=?2 AND authorization_fingerprint=?3)", params![request_id,authorization_id,digest(&format!("{request_id}:{eligibility}"))], "Epic settlement authorization conflict")?;
    let evidence_fingerprint = digest(&format!(
        "{epic}:{request_id}:{authorization_id}:{eligibility}"
    ));
    insert_exact(&tx, "INSERT OR IGNORE INTO epic_settlement_evidence (epic_id,evidence_id,request_id,authorization_id,evidence_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6)", params![epic,evidence_id,request_id,authorization_id,evidence_fingerprint,now()], "SELECT EXISTS(SELECT 1 FROM epic_settlement_evidence WHERE epic_id=?1 AND evidence_id=?2 AND request_id=?3 AND authorization_id=?4 AND evidence_fingerprint=?5)", params![epic,evidence_id,request_id,authorization_id,evidence_fingerprint], "Epic settlement evidence conflict")?;
    let settlement_fingerprint = digest(&format!("{epic}:{evidence_id}:{eligibility}"));
    insert_exact(&tx, "INSERT OR IGNORE INTO epic_settlements (epic_id,settlement_id,request_id,authorization_id,evidence_id,settlement_fingerprint,persisted_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![epic,settlement_id,request_id,authorization_id,evidence_id,settlement_fingerprint,now()], "SELECT EXISTS(SELECT 1 FROM epic_settlements WHERE epic_id=?1 AND settlement_id=?2 AND request_id=?3 AND authorization_id=?4 AND evidence_id=?5 AND settlement_fingerprint=?6)", params![epic,settlement_id,request_id,authorization_id,evidence_id,settlement_fingerprint], "Epic settlement persistence conflict")?;
    let persisted_at: String = tx
        .query_row(
            "SELECT persisted_at FROM epic_settlements WHERE epic_id=?1",
            [epic],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    tx.execute("INSERT INTO epic_settlement_current_states (epic_id,state_kind,settlement_id,unresolved_id,source_fingerprint,updated_at) VALUES (?1,'settled',?2,NULL,?3,?4) ON CONFLICT(epic_id) DO UPDATE SET state_kind=excluded.state_kind,settlement_id=excluded.settlement_id,unresolved_id=NULL,source_fingerprint=excluded.source_fingerprint,updated_at=excluded.updated_at", params![epic,settlement_id,eligibility,persisted_at]).map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM epic_settlement_unresolved WHERE epic_id=?1",
        [epic],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

fn persist_unresolved(
    tx: &rusqlite::Transaction<'_>,
    epic: &str,
    reason: &str,
    resume: &str,
    snapshot: &str,
) -> Result<(), String> {
    let unresolved_id = digest(&format!("epic-settlement-unresolved:{epic}"));
    let changed = tx.execute("INSERT INTO epic_settlement_unresolved (epic_id,unresolved_id,reason_code,resumption_fact,snapshot_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(epic_id) DO UPDATE SET unresolved_id=excluded.unresolved_id,reason_code=excluded.reason_code,resumption_fact=excluded.resumption_fact,snapshot_fingerprint=excluded.snapshot_fingerprint,recorded_at=excluded.recorded_at", params![epic,unresolved_id,reason,resume,snapshot,now()]).map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("Epic settlement unresolved state conflict".into());
    }
    tx.execute("INSERT INTO epic_settlement_current_states (epic_id,state_kind,settlement_id,unresolved_id,source_fingerprint,updated_at) VALUES (?1,'unresolved',NULL,?2,?3,?4) ON CONFLICT(epic_id) DO UPDATE SET state_kind=excluded.state_kind,settlement_id=NULL,unresolved_id=excluded.unresolved_id,source_fingerprint=excluded.source_fingerprint,updated_at=excluded.updated_at", params![epic,unresolved_id,snapshot,now()]).map_err(|error| error.to_string())?;
    Ok(())
}

fn insert_exact(
    tx: &rusqlite::Transaction<'_>,
    insert: &str,
    insert_params: impl rusqlite::Params,
    check: &str,
    check_params: impl rusqlite::Params,
    conflict: &str,
) -> Result<(), String> {
    let changed = tx
        .execute(insert, insert_params)
        .map_err(|error| error.to_string())?;
    if changed == 0
        && !tx
            .query_row(check, check_params, |row| row.get::<_, bool>(0))
            .map_err(|error| error.to_string())?
    {
        return Err(conflict.into());
    }
    Ok(())
}

struct Snapshot {
    plan_fingerprint: String,
    readiness_id: String,
    exact: bool,
    reason: String,
    resume: String,
    fingerprint: String,
}

impl Snapshot {
    fn load(tx: &rusqlite::Transaction<'_>, epic: &str) -> Result<Self, String> {
        let plan = rows(
            tx,
            "SELECT id,CAST(ordinal AS TEXT) FROM initiated_sprints WHERE epic_id=?1 ORDER BY ordinal,id",
            epic,
            2,
        )?;
        if plan.is_empty() {
            return Ok(Self::unresolved(
                "approved_plan_missing",
                "record an approved Epic plan before settlement can be considered",
                "",
            ));
        }
        let canonical_plan = plan
            .iter()
            .enumerate()
            .all(|(index, row)| row[1] == index.to_string());
        let plan_fingerprint = digest(&format!(
            "{epic}:{}",
            plan.iter()
                .map(|row| row.join("\u{1f}"))
                .collect::<Vec<_>>()
                .join("\u{1e}")
        ));
        if !canonical_plan {
            return Ok(Self::unresolved(
                "approved_plan_nonconsecutive",
                "restore one exact consecutive approved Sprint set, then reassess",
                &plan_fingerprint,
            ));
        }
        let settled = rows(tx, "SELECT s.id,c.decision_id,r.result_id FROM initiated_sprints s JOIN sprint_continuation_current_decisions current ON current.sprint_id=s.id JOIN sprint_continuation_decisions c ON c.decision_id=current.decision_id AND c.decision_state='settled' JOIN sprint_upward_results r ON r.decision_id=c.decision_id AND r.sprint_id=s.id AND r.result_kind='settled' WHERE s.epic_id=?1 ORDER BY s.ordinal,s.id", epic, 3)?;
        if settled.len() != plan.len() {
            return Ok(Self::unresolved(
                "approved_sprint_not_currently_settled",
                "record current settled upward results for every approved Sprint, then reassess",
                &plan_fingerprint,
            ));
        }
        let final_sprint = &plan.last().expect("nonempty plan")[0];
        let readiness: Option<String> = tx.query_row("SELECT readiness.readiness_id FROM epic_runner_sprint_result_terminal_readiness readiness JOIN epic_runner_sprint_result_realizations realization ON realization.result_id=readiness.result_id AND realization.outcome_kind='terminal_readiness' JOIN sprint_upward_results result ON result.result_id=readiness.result_id AND result.result_kind='settled' JOIN sprint_continuation_current_decisions current ON current.sprint_id=realization.source_sprint_id AND current.decision_id=result.decision_id JOIN sprint_continuation_decisions decision ON decision.decision_id=result.decision_id AND decision.decision_state='settled' WHERE realization.epic_id=?1 AND realization.source_sprint_id=?2 ORDER BY readiness.recorded_at DESC,readiness.readiness_id DESC LIMIT 1", params![epic,final_sprint], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
        let Some(readiness_id) = readiness else {
            return Ok(Self::unresolved("terminal_readiness_not_current", "record terminal readiness from the current final approved Sprint result, then reassess", &plan_fingerprint));
        };
        let incomplete_descendants: i64 = tx.query_row("SELECT COUNT(*) FROM work_unit_materializations m LEFT JOIN work_slice_execution_graph_completions graph ON graph.materialization_id=m.materialization_id AND graph.accepted_revision_id=m.accepted_revision_id LEFT JOIN work_slice_execution_settlements settlement ON settlement.materialization_id=m.materialization_id AND settlement.graph_completion_materialization_id=graph.materialization_id LEFT JOIN work_slice_planning_point_execution_settlements point ON point.materialization_id=m.materialization_id AND point.planning_point_id=m.planning_point_id LEFT JOIN work_units unit ON unit.materialization_id=m.materialization_id LEFT JOIN work_unit_settlements unit_settlement ON unit_settlement.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND (graph.materialization_id IS NULL OR settlement.materialization_id IS NULL OR point.materialization_id IS NULL OR (unit.work_unit_id IS NOT NULL AND unit_settlement.work_unit_id IS NULL))", [epic], |row| row.get(0)).map_err(|error| error.to_string())?;
        if incomplete_descendants != 0 {
            return Ok(Self::unresolved("descendant_responsibility_unresolved", "record graph, Work Slice, planning-point, and Work Unit settlement for every approved descendant, then reassess", &format!("{plan_fingerprint}:{readiness_id}")));
        }
        let fingerprint = digest(&format!(
            "{plan_fingerprint}:{readiness_id}:{}",
            settled
                .iter()
                .map(|row| row.join("\u{1f}"))
                .collect::<Vec<_>>()
                .join("\u{1e}")
        ));
        Ok(Self {
            plan_fingerprint,
            readiness_id,
            exact: true,
            reason: String::new(),
            resume: String::new(),
            fingerprint,
        })
    }
    fn unresolved(reason: &str, resume: &str, basis: &str) -> Self {
        Self {
            plan_fingerprint: basis.into(),
            readiness_id: String::new(),
            exact: false,
            reason: reason.into(),
            resume: resume.into(),
            fingerprint: digest(&format!("{reason}:{basis}")),
        }
    }
    fn eligible(&self) -> Result<(), (String, String, String)> {
        if self.exact {
            Ok(())
        } else {
            Err((
                self.reason.clone(),
                self.resume.clone(),
                self.fingerprint.clone(),
            ))
        }
    }
    fn eligibility_fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
}

fn rows(
    tx: &rusqlite::Transaction<'_>,
    sql: &str,
    epic: &str,
    columns: usize,
) -> Result<Vec<Vec<String>>, String> {
    tx.prepare(sql)
        .map_err(|error| error.to_string())?
        .query_map([epic], |row| {
            (0..columns)
                .map(|column| row.get(column))
                .collect::<Result<Vec<String>, _>>()
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::{
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };
    use tempfile::TempDir;

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE initiated_sprints (id TEXT PRIMARY KEY,epic_id TEXT NOT NULL,ordinal INTEGER NOT NULL);CREATE TABLE sprint_continuation_current_decisions (sprint_id TEXT PRIMARY KEY,decision_id TEXT NOT NULL);CREATE TABLE sprint_continuation_decisions (decision_id TEXT PRIMARY KEY,decision_state TEXT NOT NULL);CREATE TABLE sprint_upward_results (result_id TEXT PRIMARY KEY,decision_id TEXT NOT NULL,sprint_id TEXT NOT NULL,result_kind TEXT NOT NULL);CREATE TABLE epic_runner_sprint_result_realizations (result_id TEXT PRIMARY KEY,source_sprint_id TEXT NOT NULL,epic_id TEXT NOT NULL,outcome_kind TEXT NOT NULL);CREATE TABLE epic_runner_sprint_result_terminal_readiness (result_id TEXT PRIMARY KEY,readiness_id TEXT NOT NULL,recorded_at TEXT NOT NULL);CREATE TABLE work_unit_materializations (materialization_id TEXT PRIMARY KEY,epic_id TEXT NOT NULL,accepted_revision_id TEXT NOT NULL,planning_point_id TEXT NOT NULL);CREATE TABLE work_slice_execution_graph_completions (materialization_id TEXT PRIMARY KEY,accepted_revision_id TEXT NOT NULL);CREATE TABLE work_slice_execution_settlements (materialization_id TEXT PRIMARY KEY,graph_completion_materialization_id TEXT NOT NULL);CREATE TABLE work_slice_planning_point_execution_settlements (planning_point_id TEXT PRIMARY KEY,materialization_id TEXT NOT NULL);CREATE TABLE work_units (work_unit_id TEXT PRIMARY KEY,materialization_id TEXT NOT NULL);CREATE TABLE work_unit_settlements (work_unit_id TEXT PRIMARY KEY);").unwrap();
        initialize(&connection).unwrap();
        for (sprint, ordinal) in [("sprint-1", 0), ("sprint-2", 1)] {
            connection
                .execute(
                    "INSERT INTO initiated_sprints VALUES (?1,'epic',?2)",
                    params![sprint, ordinal],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sprint_continuation_decisions VALUES (?1,'settled')",
                    [format!("decision-{sprint}")],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sprint_continuation_current_decisions VALUES (?1,?2)",
                    params![sprint, format!("decision-{sprint}")],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sprint_upward_results VALUES (?1,?2,?3,'settled')",
                    params![
                        format!("result-{sprint}"),
                        format!("decision-{sprint}"),
                        sprint
                    ],
                )
                .unwrap();
        }
        connection.execute("INSERT INTO epic_runner_sprint_result_realizations VALUES ('result-sprint-2','sprint-2','epic','terminal_readiness')", []).unwrap();
        connection.execute("INSERT INTO epic_runner_sprint_result_terminal_readiness VALUES ('result-sprint-2','readiness','now')", []).unwrap();
        connection
    }

    #[test]
    fn settles_complete_epic_once_with_separate_durable_facts() {
        let mut connection = fixture();
        reconcile(&mut connection).unwrap();
        reconcile(&mut connection).unwrap();
        for table in [
            "epic_settlement_requests",
            "epic_settlement_authorizations",
            "epic_settlement_evidence",
            "epic_settlements",
        ] {
            assert_eq!(
                connection
                    .query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get(0))
                    .unwrap(),
                1
            );
        }
        assert!(matches!(
            statuses(&connection).unwrap().pop().unwrap().1,
            EpicSettlementStatus::Settled { .. }
        ));
    }

    #[test]
    fn missing_current_sprint_or_descendant_closure_is_unresolved() {
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM sprint_continuation_current_decisions WHERE sprint_id='sprint-1'",
                [],
            )
            .unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="approved_sprint_not_currently_settled")
        );
        connection.execute("INSERT INTO sprint_continuation_current_decisions VALUES ('sprint-1','decision-sprint-1')", []).unwrap();
        connection.execute("INSERT INTO work_unit_materializations VALUES ('materialization','epic','revision','point')", []).unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="descendant_responsibility_unresolved")
        );
    }

    #[test]
    fn attention_is_unresolved_work_not_epic_settlement() {
        let mut connection = fixture();
        connection.execute("UPDATE sprint_continuation_decisions SET decision_state='attention' WHERE decision_id='decision-sprint-1'", []).unwrap();
        connection.execute("UPDATE sprint_upward_results SET result_kind='attention' WHERE result_id='result-sprint-1'", []).unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="approved_sprint_not_currently_settled")
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM epic_settlements", [], |row| row
                    .get(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn stale_readiness_and_supersession_never_create_a_second_settlement() {
        let mut connection = fixture();
        reconcile(&mut connection).unwrap();
        connection
            .execute(
                "DELETE FROM sprint_continuation_current_decisions WHERE sprint_id='sprint-2'",
                [],
            )
            .unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="approved_sprint_not_currently_settled")
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM epic_settlements", [], |row| row
                    .get(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn partial_recovery_reuses_the_same_request_and_settlement_identity() {
        let mut connection = fixture();
        reconcile(&mut connection).unwrap();
        let id: String = connection
            .query_row("SELECT settlement_id FROM epic_settlements", [], |row| {
                row.get(0)
            })
            .unwrap();
        connection.execute_batch("PRAGMA foreign_keys=OFF;DELETE FROM epic_settlements;DELETE FROM epic_settlement_evidence;DELETE FROM epic_settlement_authorizations;PRAGMA foreign_keys=ON;").unwrap();
        reconcile(&mut connection).unwrap();
        assert_eq!(
            connection
                .query_row::<String, _, _>(
                    "SELECT settlement_id FROM epic_settlements",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            id
        );
    }

    #[test]
    fn concurrent_attempts_converge_on_the_same_durable_settlement() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("epic-settlement.sqlite");
        let seed = fixture();
        seed.execute_batch(&format!(
            "VACUUM INTO '{}'",
            path.display().to_string().replace('\'', "''")
        ))
        .unwrap();
        drop(seed);
        let barrier = Arc::new(Barrier::new(2));
        thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let path = path.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        let mut connection = Connection::open(path).unwrap();
                        connection.busy_timeout(Duration::from_secs(2)).unwrap();
                        barrier.wait();
                        for _ in 0..3 {
                            match reconcile(&mut connection) {
                                Ok(()) => return,
                                Err(error) if error.contains("database is locked") => {
                                    thread::sleep(Duration::from_millis(25))
                                }
                                Err(error) => panic!("unexpected reconciliation error: {error}"),
                            }
                        }
                        panic!("concurrent reconciliation did not converge");
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
        });
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM epic_settlements", [], |row| row
                    .get(0))
                .unwrap(),
            1
        );
    }
}
