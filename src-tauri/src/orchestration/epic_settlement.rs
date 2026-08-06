//! Durable Epic settlement. Terminal readiness is an input, not a settlement fact.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpicSettlementProjection {
    pub(crate) epic_id: String,
    pub(crate) state: EpicSettlementProjectionState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum EpicSettlementProjectionState {
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

/// Returns only current, privacy-safe settlement facts. An absent complete schema is an absent
/// legacy capability; a partial schema or malformed durable chain is an unavailable query.
pub(crate) fn native_projection(
    connection: &Connection,
    initiated_epic_ids: &[String],
) -> Result<Option<Vec<EpicSettlementProjection>>, String> {
    const TABLES: [&str; 6] = [
        "epic_settlement_requests",
        "epic_settlement_authorizations",
        "epic_settlement_evidence",
        "epic_settlements",
        "epic_settlement_unresolved",
        "epic_settlement_current_states",
    ];
    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('epic_settlement_requests','epic_settlement_authorizations','epic_settlement_evidence','epic_settlements','epic_settlement_unresolved','epic_settlement_current_states')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if table_count == 0 {
        return Ok(None);
    }
    if table_count != TABLES.len() as i64 {
        return Err("Productive Epic settlement projection tables are incomplete".into());
    }
    let initiated: std::collections::HashSet<&str> =
        initiated_epic_ids.iter().map(String::as_str).collect();
    let mut current = connection
        .prepare("SELECT epic_id,state_kind,settlement_id,unresolved_id,source_fingerprint,updated_at FROM epic_settlement_current_states ORDER BY epic_id")
        .map_err(|error| error.to_string())?;
    let rows = current
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut projection = Vec::new();
    for row in rows {
        let (epic_id, state_kind, settlement_id, unresolved_id, source_fingerprint, updated_at) =
            row.map_err(|error| error.to_string())?;
        if epic_id.is_empty()
            || source_fingerprint.is_empty()
            || updated_at.is_empty()
            || !initiated.contains(epic_id.as_str())
        {
            return Err("Epic settlement current state correlation is invalid".into());
        }
        let state = match state_kind.as_str() {
            "settled" => {
                let settlement_id = settlement_id
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "Epic settlement settled state is incomplete".to_string())?;
                if unresolved_id.is_some() {
                    return Err("Epic settlement current state contradicts settled shape".into());
                }
                let chain: Option<(
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                )> = connection
                    .query_row(
                        "SELECT s.epic_id,s.settlement_id,s.request_id,s.authorization_id,s.evidence_id,s.settlement_fingerprint,s.persisted_at,r.epic_id,r.request_id,r.approved_plan_fingerprint,r.terminal_readiness_id,r.eligibility_fingerprint,r.requested_at,a.request_id,a.authorization_id,a.authorization_fingerprint,a.authorized_at,e.epic_id,e.evidence_id,e.request_id,e.authorization_id,e.evidence_fingerprint,e.recorded_at FROM epic_settlements s JOIN epic_settlement_requests r ON r.epic_id=s.epic_id AND r.request_id=s.request_id JOIN epic_settlement_authorizations a ON a.request_id=s.request_id AND a.authorization_id=s.authorization_id JOIN epic_settlement_evidence e ON e.epic_id=s.epic_id AND e.request_id=s.request_id AND e.authorization_id=s.authorization_id AND e.evidence_id=s.evidence_id WHERE s.settlement_id=?1",
                        [settlement_id.as_str()],
                        |row| {
                            Ok((
                                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                                row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                                row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?,
                                row.get(12)?, row.get(13)?, row.get(14)?, row.get(15)?,
                                row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?,
                                row.get(20)?, row.get(21)?, row.get(22)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                let Some((
                    settled_epic,
                    settled_id,
                    request_id,
                    authorization_id,
                    evidence_id,
                    settlement_fingerprint,
                    persisted_at,
                    request_epic,
                    request_id_again,
                    approved_plan_fingerprint,
                    terminal_readiness_id,
                    eligibility_fingerprint,
                    requested_at,
                    authorization_request_id,
                    authorization_id_again,
                    authorization_fingerprint,
                    authorized_at,
                    evidence_epic,
                    evidence_id_again,
                    evidence_request_id,
                    evidence_authorization_id,
                    evidence_fingerprint,
                    recorded_at,
                )) = chain
                else {
                    return Err("Epic settlement durable chain is incomplete".into());
                };
                let expected_request_id = digest(&format!(
                    "epic-settlement-request:{epic_id}:{eligibility_fingerprint}"
                ));
                let expected_authorization_id = digest(&format!(
                    "epic-settlement-authorization:{expected_request_id}"
                ));
                let expected_authorization_fingerprint =
                    digest(&format!("{expected_request_id}:{eligibility_fingerprint}"));
                let expected_evidence_id = digest(&format!(
                    "epic-settlement-evidence:{expected_authorization_id}"
                ));
                let expected_evidence_fingerprint = digest(&format!(
                    "{epic_id}:{expected_request_id}:{expected_authorization_id}:{eligibility_fingerprint}"
                ));
                let expected_settlement_id =
                    digest(&format!("epic-settlement:{expected_evidence_id}"));
                let expected_settlement_fingerprint = digest(&format!(
                    "{epic_id}:{expected_evidence_id}:{eligibility_fingerprint}"
                ));
                if settled_epic != epic_id
                    || request_epic != epic_id
                    || evidence_epic != epic_id
                    || source_fingerprint != eligibility_fingerprint
                    || settled_id != settlement_id
                    || settled_id != expected_settlement_id
                    || request_id != expected_request_id
                    || request_id_again != expected_request_id
                    || authorization_request_id != expected_request_id
                    || evidence_request_id != expected_request_id
                    || authorization_id != expected_authorization_id
                    || authorization_id_again != expected_authorization_id
                    || evidence_authorization_id != expected_authorization_id
                    || evidence_id != expected_evidence_id
                    || evidence_id != evidence_id_again
                    || settlement_fingerprint != expected_settlement_fingerprint
                    || authorization_fingerprint != expected_authorization_fingerprint
                    || evidence_fingerprint != expected_evidence_fingerprint
                    || approved_plan_fingerprint.is_empty()
                    || terminal_readiness_id.is_empty()
                    || eligibility_fingerprint.is_empty()
                    || requested_at.is_empty()
                    || authorized_at.is_empty()
                    || recorded_at.is_empty()
                    || persisted_at.is_empty()
                    || updated_at != persisted_at
                {
                    return Err("Epic settlement durable chain is contradictory".into());
                }
                EpicSettlementProjectionState::Settled {
                    settlement_id,
                    persisted_at,
                }
            }
            "unresolved" => {
                let unresolved_id = unresolved_id
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "Epic settlement unresolved state is incomplete".to_string())?;
                if settlement_id.is_some() {
                    return Err("Epic settlement current state contradicts unresolved shape".into());
                }
                let unresolved: Option<(String, String, String, String, String)> = connection
                    .query_row(
                        "SELECT epic_id,reason_code,resumption_fact,snapshot_fingerprint,recorded_at FROM epic_settlement_unresolved WHERE unresolved_id=?1",
                        [unresolved_id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                let Some((
                    unresolved_epic,
                    reason_code,
                    resumption_fact,
                    snapshot_fingerprint,
                    recorded_at,
                )) = unresolved
                else {
                    return Err("Epic settlement unresolved chain is incomplete".into());
                };
                if unresolved_epic != epic_id
                    || reason_code.trim().is_empty()
                    || resumption_fact.trim().is_empty()
                    || snapshot_fingerprint.is_empty()
                    || snapshot_fingerprint != source_fingerprint
                    || recorded_at.is_empty()
                {
                    return Err("Epic settlement unresolved chain is contradictory".into());
                }
                EpicSettlementProjectionState::Unresolved {
                    reason_code,
                    resumption_fact,
                    recorded_at,
                }
            }
            _ => return Err("Epic settlement current state has an unknown variant".into()),
        };
        projection.push(EpicSettlementProjection { epic_id, state });
    }
    Ok(Some(projection))
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
        if exists_epic(tx, "SELECT 1 FROM work_slice_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.epic_id=?1 UNION ALL SELECT 1 FROM work_unit_execution_attentions a JOIN work_unit_materializations m ON m.materialization_id=a.materialization_id WHERE m.epic_id=?1 UNION ALL SELECT 1 FROM epic_runner_escalation_attentions a JOIN epic_runner_escalation_receivers r ON r.handback_id=a.handback_id WHERE r.epic_id=?1", epic)? {
            return Ok(Self::unresolved("structured_attention_unresolved", "resolve the recorded structured human or external attention, then reassess", &plan_fingerprint));
        }
        if exists_epic(tx, "SELECT 1 FROM work_unit_retry_attempts retry JOIN work_units unit ON unit.work_unit_id=retry.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=unit.materialization_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND settled.work_unit_id IS NULL", epic)? {
            return Ok(Self::unresolved("descendant_retry_unresolved", "complete or return the exact retry responsibility through its authoritative Sprint lifecycle, then reassess", &plan_fingerprint));
        }
        if exists_epic(tx, "SELECT 1 FROM work_unit_no_progress_handbacks handback JOIN work_units unit ON unit.work_unit_id=handback.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=unit.materialization_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND settled.work_unit_id IS NULL", epic)? {
            return Ok(Self::unresolved("descendant_handback_unresolved", "resolve the recorded Handback through its authoritative reassessment route, then reassess", &plan_fingerprint));
        }
        if exists_epic(tx, "SELECT 1 FROM sprint_runner_handback_dispositions disposition JOIN sprint_runner_handback_deliveries delivery ON delivery.handback_id=disposition.handback_id JOIN initiated_sprints sprint ON sprint.id=delivery.sprint_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=disposition.handback_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=route.work_unit_id WHERE sprint.epic_id=?1 AND disposition.movement_kind='wait_for_agent_dependency' AND (route.work_unit_id IS NULL OR settled.work_unit_id IS NULL) UNION ALL SELECT 1 FROM epic_runner_escalation_downstream_requests request JOIN epic_runner_escalation_receivers receiver ON receiver.handback_id=request.handback_id LEFT JOIN sprint_handback_dependency_routes route ON route.handback_id=request.handback_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=route.work_unit_id WHERE receiver.epic_id=?1 AND request.request_kind='existing_agent_achievable_dependency' AND (route.work_unit_id IS NULL OR settled.work_unit_id IS NULL)", epic)? {
            return Ok(Self::unresolved("agent_dependency_unresolved", "wait for the exact routed agent-achievable dependency to settle, then reassess", &plan_fingerprint));
        }
        if exists_epic(tx, "SELECT 1 FROM work_unit_handler_activations handler JOIN work_units unit ON unit.work_unit_id=handler.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=unit.materialization_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND settled.work_unit_id IS NULL UNION ALL SELECT 1 FROM work_unit_implementer_activations implementer JOIN work_units unit ON unit.work_unit_id=implementer.work_unit_id JOIN work_unit_materializations m ON m.materialization_id=unit.materialization_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND settled.work_unit_id IS NULL UNION ALL SELECT 1 FROM epic_runner_sprint_result_downstream_requests request JOIN epic_runner_sprint_result_realizations realization ON realization.result_id=request.result_id LEFT JOIN sprint_runner_transitions successor ON successor.sprint_id=realization.successor_sprint_id AND successor.epic_id=realization.epic_id AND successor.request_id=realization.successor_request_id WHERE realization.epic_id=?1 AND COALESCE((realization.outcome_kind='successor_request' AND realization.successor_sprint_id IS NOT NULL AND realization.successor_request_id IS NOT NULL AND request.request_id=realization.successor_request_id AND successor.sprint_id=realization.successor_sprint_id AND successor.epic_id=realization.epic_id AND successor.request_id=realization.successor_request_id),0)=0", epic)? {
            return Ok(Self::unresolved("descendant_continuation_unresolved", "complete the recorded descendant continuation or downstream request through its authoritative route, then reassess", &plan_fingerprint));
        }
        let settled = rows(tx, "SELECT s.id,c.decision_id,r.result_id FROM initiated_sprints s JOIN sprint_continuation_current_decisions current ON current.sprint_id=s.id AND current.decision_state='settled' JOIN sprint_continuation_decisions c ON c.decision_id=current.decision_id AND c.sprint_id=s.id AND c.decision_state='settled' JOIN sprint_upward_results r ON r.decision_id=c.decision_id AND r.sprint_id=s.id AND r.result_kind='settled' WHERE s.epic_id=?1 ORDER BY s.ordinal,s.id", epic, 3)?;
        if settled.len() != plan.len()
            || settled
                .iter()
                .zip(plan.iter())
                .any(|(settled, approved)| settled[0] != approved[0])
        {
            return Ok(Self::unresolved(
                "current_sprint_result_correlation_unresolved",
                "record one current settled decision and correlated upward result for every approved Sprint, then reassess",
                &plan_fingerprint,
            ));
        }
        let final_sprint = &plan.last().expect("nonempty plan")[0];
        let readiness: Option<String> = tx.query_row("SELECT readiness.readiness_id FROM epic_runner_sprint_result_terminal_readiness readiness JOIN epic_runner_sprint_result_realizations realization ON realization.result_id=readiness.result_id AND realization.outcome_kind='terminal_readiness' JOIN sprint_upward_results result ON result.result_id=readiness.result_id AND result.decision_id=realization.decision_id AND result.result_kind='settled' JOIN sprint_continuation_current_decisions current ON current.sprint_id=realization.source_sprint_id AND current.decision_id=result.decision_id AND current.decision_state='settled' JOIN sprint_continuation_decisions decision ON decision.decision_id=result.decision_id AND decision.sprint_id=realization.source_sprint_id AND decision.decision_state='settled' WHERE realization.epic_id=?1 AND realization.source_sprint_id=?2 ORDER BY readiness.recorded_at DESC,readiness.readiness_id DESC LIMIT 1", params![epic,final_sprint], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
        let Some(readiness_id) = readiness else {
            return Ok(Self::unresolved("terminal_readiness_correlation_unresolved", "record terminal readiness correlated to the exact current final approved Sprint decision and result, then reassess", &plan_fingerprint));
        };
        if exists_epic(tx, "SELECT 1 FROM initiated_sprints sprint LEFT JOIN work_unit_materializations m ON m.epic_id=sprint.epic_id AND m.sprint_id=sprint.id WHERE sprint.epic_id=?1 AND m.materialization_id IS NULL", epic)? {
            return Ok(Self::unresolved("approved_sprint_descendant_materialization_unresolved", "materialize every approved Sprint through its accepted planning point, then reassess", &format!("{plan_fingerprint}:{readiness_id}")));
        }
        if exists_epic(tx, "SELECT 1 FROM work_unit_materializations m LEFT JOIN initiated_sprints sprint ON sprint.id=m.sprint_id AND sprint.epic_id=m.epic_id LEFT JOIN work_slice_execution_graph_completions graph ON graph.materialization_id=m.materialization_id AND graph.accepted_revision_id=m.accepted_revision_id LEFT JOIN work_slice_execution_settlements settlement ON settlement.materialization_id=m.materialization_id AND settlement.graph_completion_materialization_id=graph.materialization_id LEFT JOIN work_slice_planning_point_execution_settlements point ON point.materialization_id=m.materialization_id AND point.work_slice_execution_materialization_id=settlement.materialization_id AND point.planning_point_id=m.planning_point_id WHERE m.epic_id=?1 AND (sprint.id IS NULL OR graph.materialization_id IS NULL OR settlement.materialization_id IS NULL OR point.materialization_id IS NULL)", epic)? {
            return Ok(Self::unresolved("descendant_execution_chain_unresolved", "record the exact graph-completion, Work Slice settlement, and planning-point settlement chain for every materialization, then reassess", &format!("{plan_fingerprint}:{readiness_id}")));
        }
        if exists_epic(tx, "SELECT 1 FROM work_units unit JOIN work_unit_materializations m ON m.materialization_id=unit.materialization_id LEFT JOIN work_unit_settlements settled ON settled.work_unit_id=unit.work_unit_id WHERE m.epic_id=?1 AND settled.work_unit_id IS NULL", epic)? {
            return Ok(Self::unresolved("work_unit_closure_unresolved", "record Work Unit settlement for every materialized descendant, then reassess", &format!("{plan_fingerprint}:{readiness_id}")));
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
fn exists_epic(tx: &rusqlite::Transaction<'_>, sql: &str, epic: &str) -> Result<bool, String> {
    tx.query_row(&format!("SELECT EXISTS({sql})"), [epic], |row| row.get(0))
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
        connection.execute_batch("CREATE TABLE initiated_sprints (id TEXT PRIMARY KEY,epic_id TEXT NOT NULL,ordinal INTEGER NOT NULL);CREATE TABLE sprint_continuation_current_decisions (sprint_id TEXT PRIMARY KEY,decision_id TEXT NOT NULL,decision_state TEXT NOT NULL);CREATE TABLE sprint_continuation_decisions (decision_id TEXT PRIMARY KEY,sprint_id TEXT NOT NULL,decision_state TEXT NOT NULL);CREATE TABLE sprint_upward_results (result_id TEXT PRIMARY KEY,decision_id TEXT NOT NULL,sprint_id TEXT NOT NULL,result_kind TEXT NOT NULL);CREATE TABLE epic_runner_sprint_result_realizations (result_id TEXT PRIMARY KEY,decision_id TEXT NOT NULL,source_sprint_id TEXT NOT NULL,epic_id TEXT NOT NULL,outcome_kind TEXT NOT NULL,successor_sprint_id TEXT,successor_request_id TEXT);CREATE TABLE epic_runner_sprint_result_terminal_readiness (result_id TEXT PRIMARY KEY,readiness_id TEXT NOT NULL,recorded_at TEXT NOT NULL);CREATE TABLE sprint_runner_transitions (sprint_id TEXT PRIMARY KEY,epic_id TEXT NOT NULL,request_id TEXT NOT NULL);CREATE TABLE work_unit_materializations (materialization_id TEXT PRIMARY KEY,epic_id TEXT NOT NULL,sprint_id TEXT NOT NULL,accepted_revision_id TEXT NOT NULL,planning_point_id TEXT NOT NULL);CREATE TABLE work_slice_execution_graph_completions (materialization_id TEXT PRIMARY KEY,accepted_revision_id TEXT NOT NULL);CREATE TABLE work_slice_execution_settlements (materialization_id TEXT PRIMARY KEY,graph_completion_materialization_id TEXT NOT NULL);CREATE TABLE work_slice_planning_point_execution_settlements (planning_point_id TEXT PRIMARY KEY,materialization_id TEXT NOT NULL,work_slice_execution_materialization_id TEXT NOT NULL);CREATE TABLE work_units (work_unit_id TEXT PRIMARY KEY,materialization_id TEXT NOT NULL);CREATE TABLE work_unit_settlements (work_unit_id TEXT PRIMARY KEY);CREATE TABLE work_unit_retry_attempts (work_unit_id TEXT);CREATE TABLE work_unit_no_progress_handbacks (handback_id TEXT,work_unit_id TEXT);CREATE TABLE sprint_runner_handback_dispositions (handback_id TEXT,movement_kind TEXT);CREATE TABLE sprint_runner_handback_deliveries (handback_id TEXT,sprint_id TEXT);CREATE TABLE sprint_handback_dependency_routes (handback_id TEXT,work_unit_id TEXT);CREATE TABLE epic_runner_escalation_downstream_requests (handback_id TEXT,request_kind TEXT);CREATE TABLE epic_runner_escalation_receivers (handback_id TEXT,epic_id TEXT);CREATE TABLE work_unit_handler_activations (work_unit_id TEXT);CREATE TABLE work_unit_implementer_activations (work_unit_id TEXT);CREATE TABLE epic_runner_sprint_result_downstream_requests (result_id TEXT,request_id TEXT);CREATE TABLE work_slice_execution_attentions (materialization_id TEXT);CREATE TABLE work_unit_execution_attentions (materialization_id TEXT);CREATE TABLE epic_runner_escalation_attentions (handback_id TEXT);").unwrap();
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
                    "INSERT INTO sprint_continuation_decisions VALUES (?1,?2,'settled')",
                    params![format!("decision-{sprint}"), sprint],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sprint_continuation_current_decisions VALUES (?1,?2,'settled')",
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
        for sprint in ["sprint-1", "sprint-2"] {
            let materialization = format!("materialization-{sprint}");
            let revision = format!("revision-{sprint}");
            let point = format!("point-{sprint}");
            let unit = format!("unit-{sprint}");
            connection
                .execute(
                    "INSERT INTO work_unit_materializations VALUES (?1,'epic',?2,?3,?4)",
                    params![materialization, sprint, revision, point],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO work_slice_execution_graph_completions VALUES (?1,?2)",
                    params![materialization, revision],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO work_slice_execution_settlements VALUES (?1,?1)",
                    [&materialization],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO work_slice_planning_point_execution_settlements VALUES (?1,?2,?2)",
                    params![point, materialization],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO work_units VALUES (?1,?2)",
                    params![unit, materialization],
                )
                .unwrap();
            connection
                .execute("INSERT INTO work_unit_settlements VALUES (?1)", [unit])
                .unwrap();
        }
        connection.execute("INSERT INTO epic_runner_sprint_result_realizations VALUES ('result-sprint-2','decision-sprint-2','sprint-2','epic','terminal_readiness',NULL,NULL)", []).unwrap();
        connection.execute("INSERT INTO epic_runner_sprint_result_terminal_readiness VALUES ('result-sprint-2','readiness','now')", []).unwrap();
        connection
    }

    fn assert_unresolved(connection: &mut Connection, expected: &str) {
        reconcile(connection).unwrap();
        match statuses(connection).unwrap().pop().unwrap().1 {
            EpicSettlementStatus::Unresolved {
                reason_code,
                resumption_fact,
                ..
            } => {
                assert_eq!(reason_code, expected);
                assert!(!resumption_fact.is_empty());
            }
            status => panic!("expected unresolved {expected}, got {status:?}"),
        }
    }

    fn prior_successor_request(
        connection: &Connection,
        realization_request: Option<&str>,
        retained_request: &str,
        transition_request: Option<&str>,
    ) {
        connection.execute("INSERT INTO epic_runner_sprint_result_realizations VALUES ('result-sprint-1','decision-sprint-1','sprint-1','epic','successor_request','sprint-2',?1)", [realization_request]).unwrap();
        connection.execute("INSERT INTO epic_runner_sprint_result_downstream_requests VALUES ('result-sprint-1',?1)", [retained_request]).unwrap();
        if let Some(request) = transition_request {
            connection
                .execute(
                    "INSERT INTO sprint_runner_transitions VALUES ('sprint-2','epic',?1)",
                    [request],
                )
                .unwrap();
        }
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
    fn native_projection_exposes_only_the_current_settlement_variant() {
        let mut connection = fixture();
        reconcile(&mut connection).unwrap();
        let projection = native_projection(&connection, &["epic".into()])
            .unwrap()
            .unwrap();
        assert_eq!(projection.len(), 1);
        assert_eq!(projection[0].epic_id, "epic");
        assert!(matches!(
            projection[0].state,
            EpicSettlementProjectionState::Settled { .. }
        ));
    }

    #[test]
    fn native_projection_rejects_contradictory_settled_authority_and_timestamps() {
        let cases = [
            (
                "current source",
                "UPDATE epic_settlement_current_states SET source_fingerprint='altered-source'",
            ),
            (
                "request identity",
                "PRAGMA foreign_keys=OFF; UPDATE epic_settlement_requests SET request_id='altered-request'; UPDATE epic_settlement_authorizations SET request_id='altered-request'; UPDATE epic_settlement_evidence SET request_id='altered-request'; UPDATE epic_settlements SET request_id='altered-request'; PRAGMA foreign_keys=ON;",
            ),
            (
                "authorization identity",
                "PRAGMA foreign_keys=OFF; UPDATE epic_settlement_authorizations SET authorization_id='altered-authorization'; UPDATE epic_settlement_evidence SET authorization_id='altered-authorization'; UPDATE epic_settlements SET authorization_id='altered-authorization'; PRAGMA foreign_keys=ON;",
            ),
            (
                "evidence identity",
                "PRAGMA foreign_keys=OFF; UPDATE epic_settlement_evidence SET evidence_id='altered-evidence'; UPDATE epic_settlements SET evidence_id='altered-evidence'; PRAGMA foreign_keys=ON;",
            ),
            (
                "settlement identity",
                "PRAGMA foreign_keys=OFF; UPDATE epic_settlements SET settlement_id='altered-settlement'; UPDATE epic_settlement_current_states SET settlement_id='altered-settlement'; PRAGMA foreign_keys=ON;",
            ),
            (
                "request fingerprint",
                "UPDATE epic_settlement_requests SET eligibility_fingerprint='altered-request-fingerprint'",
            ),
            (
                "authorization fingerprint",
                "UPDATE epic_settlement_authorizations SET authorization_fingerprint='altered-authorization-fingerprint'",
            ),
            (
                "evidence fingerprint",
                "UPDATE epic_settlement_evidence SET evidence_fingerprint='altered-evidence-fingerprint'",
            ),
            (
                "settlement fingerprint",
                "UPDATE epic_settlements SET settlement_fingerprint='altered-settlement-fingerprint'",
            ),
            (
                "request timestamp",
                "UPDATE epic_settlement_requests SET requested_at=''",
            ),
            (
                "authorization timestamp",
                "UPDATE epic_settlement_authorizations SET authorized_at=''",
            ),
            (
                "evidence timestamp",
                "UPDATE epic_settlement_evidence SET recorded_at=''",
            ),
            (
                "settlement timestamp",
                "UPDATE epic_settlements SET persisted_at=''",
            ),
            (
                "current timestamp",
                "UPDATE epic_settlement_current_states SET updated_at=''",
            ),
        ];
        for (label, mutation) in cases {
            let mut connection = fixture();
            reconcile(&mut connection).unwrap();
            connection.execute_batch(mutation).unwrap();
            assert!(
                native_projection(&connection, &["epic".into()]).is_err(),
                "projection accepted altered {label}"
            );
        }
    }

    #[test]
    fn native_projection_returns_none_for_legacy_databases_without_settlement_tables() {
        let connection = Connection::open_in_memory().unwrap();
        assert_eq!(
            native_projection(&connection, &["epic".into()]).unwrap(),
            None
        );
    }

    #[test]
    fn native_projection_fails_closed_on_partial_chain_and_foreign_current_state() {
        let partial_schema = Connection::open_in_memory().unwrap();
        partial_schema
            .execute_batch("CREATE TABLE epic_settlement_requests (epic_id TEXT PRIMARY KEY);")
            .unwrap();
        assert_eq!(
            native_projection(&partial_schema, &["epic".into()]).unwrap_err(),
            "Productive Epic settlement projection tables are incomplete"
        );

        let mut connection = fixture();
        reconcile(&mut connection).unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys=OFF;DELETE FROM epic_settlement_evidence;PRAGMA foreign_keys=ON;")
            .unwrap();
        assert_eq!(
            native_projection(&connection, &["epic".into()]).unwrap_err(),
            "Epic settlement durable chain is incomplete"
        );

        let connection = fixture();
        connection
            .execute_batch("INSERT INTO epic_settlement_unresolved VALUES ('foreign-epic','foreign-unresolved','foreign_reason','resume','snapshot','recorded');INSERT INTO epic_settlement_current_states VALUES ('foreign-epic','unresolved',NULL,'foreign-unresolved','snapshot','updated');")
            .unwrap();
        assert_eq!(
            native_projection(&connection, &["epic".into()]).unwrap_err(),
            "Epic settlement current state correlation is invalid"
        );
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
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="current_sprint_result_correlation_unresolved")
        );
        connection.execute("INSERT INTO sprint_continuation_current_decisions VALUES ('sprint-1','decision-sprint-1','settled')", []).unwrap();
        connection.execute("INSERT INTO work_unit_materializations VALUES ('materialization','epic','sprint-1','revision','point')", []).unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="descendant_execution_chain_unresolved")
        );
    }

    #[test]
    fn attention_is_unresolved_work_not_epic_settlement() {
        let mut connection = fixture();
        connection
            .execute(
                "INSERT INTO work_slice_execution_attentions VALUES ('materialization-sprint-1')",
                [],
            )
            .unwrap();
        reconcile(&mut connection).unwrap();
        assert!(
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="structured_attention_unresolved")
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
            matches!(statuses(&connection).unwrap().pop().unwrap().1, EpicSettlementStatus::Unresolved { ref reason_code, .. } if reason_code=="current_sprint_result_correlation_unresolved")
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM epic_settlements", [], |row| row
                    .get(0))
                .unwrap(),
            1
        );
        connection.execute("INSERT INTO sprint_continuation_current_decisions VALUES ('sprint-2','decision-sprint-2','settled')", []).unwrap();
        connection.execute("UPDATE epic_runner_sprint_result_terminal_readiness SET readiness_id='replacement-readiness'", []).unwrap();
        assert_unresolved(&mut connection, "settlement_authority_superseded");
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
    fn plan_and_current_result_correlation_fail_closed() {
        let mut connection = fixture();
        connection
            .execute(
                "UPDATE initiated_sprints SET ordinal=3 WHERE id='sprint-2'",
                [],
            )
            .unwrap();
        assert_unresolved(&mut connection, "approved_plan_nonconsecutive");
        let mut connection = fixture();
        connection.execute("UPDATE sprint_upward_results SET sprint_id='sprint-1' WHERE result_id='result-sprint-2'", []).unwrap();
        assert_unresolved(
            &mut connection,
            "current_sprint_result_correlation_unresolved",
        );
        let mut connection = fixture();
        connection.execute("UPDATE sprint_continuation_current_decisions SET decision_id='foreign-decision' WHERE sprint_id='sprint-2'", []).unwrap();
        assert_unresolved(
            &mut connection,
            "current_sprint_result_correlation_unresolved",
        );
        let mut connection = fixture();
        connection.execute("UPDATE epic_runner_sprint_result_realizations SET decision_id='decision-sprint-1' WHERE result_id='result-sprint-2'", []).unwrap();
        assert_unresolved(&mut connection, "terminal_readiness_correlation_unresolved");
    }

    #[test]
    fn exact_descendant_chain_and_work_unit_closure_are_required() {
        let mut connection = fixture();
        connection.execute("UPDATE work_slice_execution_settlements SET graph_completion_materialization_id='foreign' WHERE materialization_id='materialization-sprint-1'", []).unwrap();
        assert_unresolved(&mut connection, "descendant_execution_chain_unresolved");
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM work_unit_settlements WHERE work_unit_id='unit-sprint-1'",
                [],
            )
            .unwrap();
        assert_unresolved(&mut connection, "work_unit_closure_unresolved");
        let mut connection = fixture();
        connection.execute("DELETE FROM work_unit_materializations WHERE materialization_id='materialization-sprint-1'", []).unwrap();
        assert_unresolved(
            &mut connection,
            "approved_sprint_descendant_materialization_unresolved",
        );
    }

    #[test]
    fn each_pending_descendant_category_retains_its_resumption_path() {
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM work_unit_settlements WHERE work_unit_id='unit-sprint-1'",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_unit_retry_attempts VALUES ('unit-sprint-1')",
                [],
            )
            .unwrap();
        assert_unresolved(&mut connection, "descendant_retry_unresolved");
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM work_unit_settlements WHERE work_unit_id='unit-sprint-1'",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_unit_no_progress_handbacks VALUES ('handback','unit-sprint-1')",
                [],
            )
            .unwrap();
        assert_unresolved(&mut connection, "descendant_handback_unresolved");
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM work_unit_settlements WHERE work_unit_id='unit-sprint-1'",
                [],
            )
            .unwrap();
        connection.execute_batch("INSERT INTO sprint_runner_handback_dispositions VALUES ('dependency','wait_for_agent_dependency');INSERT INTO sprint_runner_handback_deliveries VALUES ('dependency','sprint-1');INSERT INTO sprint_handback_dependency_routes VALUES ('dependency','unit-sprint-1');").unwrap();
        assert_unresolved(&mut connection, "agent_dependency_unresolved");
        let mut connection = fixture();
        connection
            .execute(
                "DELETE FROM work_unit_settlements WHERE work_unit_id='unit-sprint-1'",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_unit_handler_activations VALUES ('unit-sprint-1')",
                [],
            )
            .unwrap();
        assert_unresolved(&mut connection, "descendant_continuation_unresolved");
        let mut connection = fixture();
        connection.execute("INSERT INTO epic_runner_sprint_result_downstream_requests VALUES ('result-sprint-2','unfulfilled')", []).unwrap();
        assert_unresolved(&mut connection, "descendant_continuation_unresolved");
    }

    #[test]
    fn fulfilled_prior_successor_request_does_not_block_final_epic_settlement() {
        let mut connection = fixture();
        prior_successor_request(
            &connection,
            Some("successor-request"),
            "successor-request",
            Some("successor-request"),
        );
        reconcile(&mut connection).unwrap();
        assert!(matches!(
            statuses(&connection).unwrap().pop().unwrap().1,
            EpicSettlementStatus::Settled { .. }
        ));
        reconcile(&mut connection).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM epic_settlements", [], |row| row
                    .get(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn partial_or_foreign_prior_successor_request_remains_unresolved() {
        let mut connection = fixture();
        prior_successor_request(
            &connection,
            None,
            "successor-request",
            Some("successor-request"),
        );
        assert_unresolved(&mut connection, "descendant_continuation_unresolved");
        let mut connection = fixture();
        prior_successor_request(
            &connection,
            Some("successor-request"),
            "foreign-request",
            Some("successor-request"),
        );
        assert_unresolved(&mut connection, "descendant_continuation_unresolved");
        let mut connection = fixture();
        prior_successor_request(
            &connection,
            Some("successor-request"),
            "successor-request",
            Some("foreign-request"),
        );
        assert_unresolved(&mut connection, "descendant_continuation_unresolved");
    }

    #[test]
    fn fresh_open_recovers_partial_effects_with_the_original_identity() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("reopen.sqlite");
        let seed = fixture();
        seed.execute_batch(&format!(
            "VACUUM INTO '{}'",
            path.display().to_string().replace('\'', "''")
        ))
        .unwrap();
        drop(seed);
        let id = {
            let mut connection = Connection::open(&path).unwrap();
            reconcile(&mut connection).unwrap();
            let id: String = connection
                .query_row("SELECT settlement_id FROM epic_settlements", [], |row| {
                    row.get(0)
                })
                .unwrap();
            connection.execute_batch("PRAGMA foreign_keys=OFF;DELETE FROM epic_settlements;DELETE FROM epic_settlement_evidence;DELETE FROM epic_settlement_authorizations;PRAGMA foreign_keys=ON;").unwrap();
            id
        };
        let mut reopened = Connection::open(path).unwrap();
        reconcile(&mut reopened).unwrap();
        assert_eq!(
            reopened
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
