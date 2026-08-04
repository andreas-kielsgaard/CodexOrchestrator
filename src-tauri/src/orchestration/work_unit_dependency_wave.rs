//! Durable prerequisite-wave eligibility.  This module consumes settled product facts only; it
//! neither invokes accepted integration nor creates any Handler runtime effect.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub(crate) const WORK_UNIT_DEPENDENCY_WAVE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS work_unit_dependency_activation_intents (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  materialization_id TEXT NOT NULL REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  accepted_revision_id TEXT NOT NULL REFERENCES work_slice_proposal_revisions(revision_id) ON DELETE RESTRICT,
  eligibility_state TEXT NOT NULL CHECK (eligibility_state IN ('blocked','eligible')),
  blocked_reason TEXT,
  eligibility_recorded_at TEXT NOT NULL,
  activation_intended_at TEXT,
  CHECK ((eligibility_state='blocked' AND blocked_reason IS NOT NULL)
      OR (eligibility_state='eligible' AND blocked_reason IS NULL AND activation_intended_at IS NOT NULL))
);

-- These are execution facts, deliberately separate from the historical plan materialization
-- settlement.  A graph completion proves only the canonical graph is coherently integrated;
-- it does not authorize a later planning point, Sprint, or Epic effect.
CREATE TABLE IF NOT EXISTS work_unit_execution_states (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  materialization_id TEXT NOT NULL REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  accepted_revision_id TEXT NOT NULL REFERENCES work_slice_proposal_revisions(revision_id) ON DELETE RESTRICT,
  execution_state TEXT NOT NULL CHECK (execution_state IN ('waiting_on_prerequisites','ready','active','retry_authorized','handed_back','settled','attention')),
  reason TEXT,
  recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_slice_execution_graph_completions (
  materialization_id TEXT PRIMARY KEY REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  accepted_revision_id TEXT NOT NULL UNIQUE REFERENCES work_slice_proposal_revisions(revision_id) ON DELETE RESTRICT,
  graph_fingerprint TEXT NOT NULL UNIQUE,
  completed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_slice_execution_settlements (
  materialization_id TEXT PRIMARY KEY REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  graph_completion_materialization_id TEXT NOT NULL UNIQUE REFERENCES work_slice_execution_graph_completions(materialization_id) ON DELETE RESTRICT,
  settled_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_slice_planning_point_execution_settlements (
  planning_point_id TEXT PRIMARY KEY,
  materialization_id TEXT NOT NULL UNIQUE REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  work_slice_execution_materialization_id TEXT NOT NULL UNIQUE REFERENCES work_slice_execution_settlements(materialization_id) ON DELETE RESTRICT,
  settled_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_slice_execution_attentions (
  materialization_id TEXT PRIMARY KEY REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  code TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);
"#;

#[derive(Debug, PartialEq, Eq)]
enum Eligibility {
    Blocked(String),
    Eligible,
}

/// Recomputes the one bounded dependency wave from exact canonical graph edges and settled
/// prerequisite contributions.  The resulting eligibility and activation intent are one durable
/// transaction and deliberately precede the Handler's effectful reconciliation.
pub(crate) fn reconcile_work_unit_dependency_wave(
    connection: &mut Connection,
) -> Result<(), String> {
    connection
        .execute_batch(WORK_UNIT_DEPENDENCY_WAVE_SCHEMA)
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let units = transaction
        .prepare(
            "SELECT u.work_unit_id,u.materialization_id,u.accepted_revision_id,m.sprint_id
             FROM work_units u
             JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id
             WHERE m.settled_at IS NOT NULL
             ORDER BY m.sprint_id,u.lane_ordinal,u.work_unit_id",
        )
        .map_err(|error| error.to_string())?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut graph_issues = std::collections::HashMap::new();
    for (_, materialization_id, accepted_revision_id, _) in &units {
        graph_issues.entry(materialization_id.clone()).or_insert(graph_issue(
            &transaction,
            materialization_id,
            accepted_revision_id,
        )?);
    }
    for (work_unit_id, materialization_id, accepted_revision_id, sprint_id) in units {
        if let Some(issue) = graph_issues.get(&materialization_id).and_then(Clone::clone) {
            persist(
                &transaction,
                &work_unit_id,
                &materialization_id,
                &accepted_revision_id,
                Eligibility::Blocked(issue),
            )?;
            continue;
        }
        let eligibility = eligibility(
            &transaction,
            &work_unit_id,
            &materialization_id,
            &accepted_revision_id,
            &sprint_id,
        )?;
        persist(
            &transaction,
            &work_unit_id,
            &materialization_id,
            &accepted_revision_id,
            eligibility,
        )?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

/// Projects factual execution state and records the three exact terminal facts only after every
/// canonical unit has one coherent accepted-integration settlement and every dependency has its
/// exact outgoing contribution.  This function is idempotent and makes no runtime call.
pub(crate) fn reconcile_work_slice_execution_settlement(
    connection: &mut Connection,
) -> Result<(), String> {
    connection.execute_batch(WORK_UNIT_DEPENDENCY_WAVE_SCHEMA).map_err(|e| e.to_string())?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let materializations = tx.prepare(
        "SELECT materialization_id,planning_point_id,accepted_revision_id
           FROM work_unit_materializations WHERE settled_at IS NOT NULL
           ORDER BY authorization_recorded_at,materialization_id",
    ).map_err(|e| e.to_string())?.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    for (materialization, planning_point, revision) in materializations {
        let issue = existing_attention(&tx, &materialization)?
            .or(graph_issue(&tx, &materialization, &revision)?)
            .or(execution_issue(&tx, &materialization, &revision)?)
            .or(if stalled_generation(&tx, &materialization, &revision)? {
                Some("stalled_execution_generation".into())
            } else {
                None
            });
        project_execution_states(&tx, &materialization, &revision, issue.as_deref())?;
        if let Some(code) = issue {
            exact_attention(&tx, &materialization, &code)?;
            continue;
        }
        let complete = graph_is_complete(&tx, &materialization, &revision)?;
        if !complete { continue; }
        let fingerprint = graph_fingerprint(&tx, &materialization, &revision)?;
        let at = chrono::Utc::now().to_rfc3339();
        exact_insert(&tx,
            "INSERT INTO work_slice_execution_graph_completions(materialization_id,accepted_revision_id,graph_fingerprint,completed_at) VALUES(?1,?2,?3,?4)",
            params![materialization, revision, fingerprint, at],
            "SELECT EXISTS(SELECT 1 FROM work_slice_execution_graph_completions WHERE materialization_id=?1 AND accepted_revision_id=?2 AND graph_fingerprint=?3)",
            params![materialization, revision, fingerprint],
        )?;
        exact_insert(&tx,
            "INSERT INTO work_slice_execution_settlements(materialization_id,graph_completion_materialization_id,settled_at) VALUES(?1,?1,?2)",
            params![materialization, at],
            "SELECT EXISTS(SELECT 1 FROM work_slice_execution_settlements WHERE materialization_id=?1 AND graph_completion_materialization_id=?1)",
            params![materialization],
        )?;
        exact_insert(&tx,
            "INSERT INTO work_slice_planning_point_execution_settlements(planning_point_id,materialization_id,work_slice_execution_materialization_id,settled_at) VALUES(?1,?2,?2,?3)",
            params![planning_point, materialization, at],
            "SELECT EXISTS(SELECT 1 FROM work_slice_planning_point_execution_settlements WHERE planning_point_id=?1 AND materialization_id=?2 AND work_slice_execution_materialization_id=?2)",
            params![planning_point, materialization],
        )?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn existing_attention(
    tx: &rusqlite::Transaction<'_>,
    materialization: &str,
) -> Result<Option<String>, String> {
    tx.query_row(
        "SELECT code FROM work_slice_execution_attentions WHERE materialization_id=?1",
        [materialization],
        |row| row.get(0),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn exact_insert(tx: &rusqlite::Transaction<'_>, insert: &str, insert_params: impl rusqlite::Params, exact: &str, exact_params: impl rusqlite::Params) -> Result<(), String> {
    match tx.execute(insert, insert_params) {
        Ok(1) => Ok(()),
        Ok(_) => Err("execution_settlement_insert_invalid".into()),
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == rusqlite::ErrorCode::ConstraintViolation => {
            if tx.query_row(exact, exact_params, |r| r.get::<_, bool>(0)).map_err(|e| e.to_string())? { Ok(()) } else { Err("execution_settlement_replay_conflict".into()) }
        }
        Err(error) => Err(error.to_string()),
    }
}

fn exact_attention(tx: &rusqlite::Transaction<'_>, materialization: &str, code: &str) -> Result<(), String> {
    let at = chrono::Utc::now().to_rfc3339();
    exact_insert(tx,
        "INSERT INTO work_slice_execution_attentions(materialization_id,code,recorded_at) VALUES(?1,?2,?3)",
        params![materialization, code, at],
        "SELECT EXISTS(SELECT 1 FROM work_slice_execution_attentions WHERE materialization_id=?1 AND code=?2)",
        params![materialization, code],
    )
}

/// A graph problem is global to a materialization: allowing one apparently-independent lane to
/// continue would make a later terminal claim ambiguous.  Pending (missing) contributions are
/// intentionally not errors here; they remain a waiting fact until their prerequisite settles.
fn graph_issue(tx: &rusqlite::Transaction<'_>, materialization: &str, revision: &str) -> Result<Option<String>, String> {
    let invalid_unit: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_units WHERE materialization_id=?1 AND accepted_revision_id<>?2)",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if invalid_unit { return Ok(Some("canonical_work_unit_correlation_invalid".into())); }
    let invalid_edge: bool = tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM work_unit_relationships r
           LEFT JOIN work_units d ON d.work_unit_id=r.from_id
           LEFT JOIN work_units p ON p.work_unit_id=r.to_id
           WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on'
             AND (d.work_unit_id IS NULL OR p.work_unit_id IS NULL
                  OR d.materialization_id<>?1 OR p.materialization_id<>?1
                  OR d.accepted_revision_id<>?2 OR p.accepted_revision_id<>?2))",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if invalid_edge { return Ok(Some("canonical_dependency_edge_invalid".into())); }
    let duplicate: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_unit_relationships WHERE materialization_id=?1 AND relationship_kind='depends_on' GROUP BY from_id,to_id HAVING COUNT(*)<>1)",
        [materialization], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if duplicate { return Ok(Some("duplicate_canonical_dependency_edge".into())); }
    let cyclic: bool = tx.query_row(
        "WITH RECURSIVE walk(start,node) AS (
           SELECT from_id,to_id FROM work_unit_relationships WHERE materialization_id=?1 AND relationship_kind='depends_on'
           UNION
           SELECT walk.start,r.to_id FROM walk JOIN work_unit_relationships r
             ON r.materialization_id=?1 AND r.relationship_kind='depends_on' AND r.from_id=walk.node
         ) SELECT EXISTS(SELECT 1 FROM walk WHERE start=node)",
        [materialization], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if cyclic { return Ok(Some("canonical_dependency_cycle".into())); }
    let invalid_contribution: bool = tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM work_unit_prerequisite_contributions c
           LEFT JOIN work_units dependent ON dependent.work_unit_id=c.dependent_work_unit_id
           LEFT JOIN work_unit_relationships r ON r.relationship_id=c.relationship_id
           LEFT JOIN work_units prerequisite ON prerequisite.work_unit_id=c.prerequisite_work_unit_id
           LEFT JOIN accepted_work_unit_integrations i ON i.integration_id=c.integration_id
           LEFT JOIN work_unit_settlements s ON s.integration_id=c.integration_id AND s.work_unit_id=c.prerequisite_work_unit_id
           WHERE (r.materialization_id=?1 OR dependent.materialization_id=?1 OR prerequisite.materialization_id=?1)
             AND (r.relationship_id IS NULL OR r.materialization_id<>?1 OR r.relationship_kind<>'depends_on'
                  OR dependent.work_unit_id IS NULL OR prerequisite.work_unit_id IS NULL
                  OR r.from_id<>c.dependent_work_unit_id OR r.to_id<>c.prerequisite_work_unit_id
                  OR prerequisite.materialization_id<>?1 OR prerequisite.accepted_revision_id<>?2
                  OR dependent.materialization_id<>?1 OR dependent.accepted_revision_id<>?2
                  OR i.work_unit_id<>c.prerequisite_work_unit_id OR i.stage<>'settled' OR i.settled_at IS NULL
                  OR s.settlement_id IS NULL))",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if invalid_contribution { return Ok(Some("prerequisite_contribution_correlation_invalid".into())); }
    let duplicate_contribution: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_unit_prerequisite_contributions c JOIN work_units d ON d.work_unit_id=c.dependent_work_unit_id WHERE d.materialization_id=?1 GROUP BY c.relationship_id HAVING COUNT(*)<>1)",
        [materialization], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(duplicate_contribution.then_some("duplicate_prerequisite_contribution".into()))
}

fn execution_issue(tx: &rusqlite::Transaction<'_>, materialization: &str, _revision: &str) -> Result<Option<String>, String> {
    let integration_attention: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM accepted_work_unit_integrations i JOIN work_units u ON u.work_unit_id=i.work_unit_id WHERE u.materialization_id=?1 AND (i.stage='attention' OR i.attention_code IS NOT NULL))",
        [materialization], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if integration_attention { return Ok(Some("accepted_integration_attention".into())); }
    let impossible: bool = tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM work_unit_settlements s JOIN work_units u ON u.work_unit_id=s.work_unit_id
           LEFT JOIN accepted_work_unit_integrations i ON i.integration_id=s.integration_id
           WHERE u.materialization_id=?1 AND (i.integration_id IS NULL OR i.work_unit_id<>u.work_unit_id OR i.stage<>'settled' OR i.settled_at IS NULL))",
        [materialization], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(impossible.then_some("impossible_terminal_mixture".into()))
}

fn graph_is_complete(tx: &rusqlite::Transaction<'_>, materialization: &str, revision: &str) -> Result<bool, String> {
    if existing_attention(tx, materialization)?.is_some() {
        return Ok(false);
    }
    let unresolved_handback: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_unit_no_progress_handbacks h JOIN work_units u ON u.work_unit_id=h.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2)",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if unresolved_handback { return Ok(false); }
    let units: i64 = tx.query_row("SELECT COUNT(*) FROM work_units WHERE materialization_id=?1 AND accepted_revision_id=?2", params![materialization, revision], |r| r.get(0)).map_err(|e| e.to_string())?;
    if units == 0 { return Ok(false); }
    let settled: i64 = tx.query_row(
        "SELECT COUNT(*) FROM work_units u JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id JOIN accepted_work_unit_integrations i ON i.integration_id=s.integration_id AND i.work_unit_id=u.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 AND i.stage='settled' AND i.settled_at IS NOT NULL",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if settled != units { return Ok(false); }
    let edges: i64 = tx.query_row("SELECT COUNT(*) FROM work_unit_relationships WHERE materialization_id=?1 AND relationship_kind='depends_on'", [materialization], |r| r.get(0)).map_err(|e| e.to_string())?;
    let contributions: i64 = tx.query_row(
        "SELECT COUNT(*) FROM work_unit_prerequisite_contributions c JOIN work_unit_relationships r ON r.relationship_id=c.relationship_id JOIN work_units d ON d.work_unit_id=c.dependent_work_unit_id JOIN work_units p ON p.work_unit_id=c.prerequisite_work_unit_id JOIN accepted_work_unit_integrations i ON i.integration_id=c.integration_id AND i.work_unit_id=p.work_unit_id JOIN work_unit_settlements s ON s.integration_id=i.integration_id AND s.work_unit_id=p.work_unit_id WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on' AND d.accepted_revision_id=?2 AND p.accepted_revision_id=?2 AND i.stage='settled' AND i.settled_at IS NOT NULL",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(contributions == edges)
}

fn stalled_generation(tx: &rusqlite::Transaction<'_>, materialization: &str, revision: &str) -> Result<bool, String> {
    let units: i64 = tx.query_row("SELECT COUNT(*) FROM work_units WHERE materialization_id=?1 AND accepted_revision_id=?2", params![materialization, revision], |r| r.get(0)).map_err(|e| e.to_string())?;
    let settled: i64 = tx.query_row("SELECT COUNT(*) FROM work_unit_settlements s JOIN work_units u ON u.work_unit_id=s.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2", params![materialization, revision], |r| r.get(0)).map_err(|e| e.to_string())?;
    if units == 0 || settled == units { return Ok(false); }
    let evaluated: i64 = tx.query_row("SELECT COUNT(*) FROM work_unit_dependency_activation_intents i JOIN work_units u ON u.work_unit_id=i.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 AND i.materialization_id=u.materialization_id AND i.accepted_revision_id=u.accepted_revision_id", params![materialization, revision], |r| r.get(0)).map_err(|e| e.to_string())?;
    if evaluated != units { return Ok(false); }
    let handback: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_no_progress_handbacks h JOIN work_units u ON u.work_unit_id=h.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2)", params![materialization, revision], |r| r.get(0)).map_err(|e| e.to_string())?;
    if handback { return Ok(false); }
    let ready: i64 = tx.query_row(
        "SELECT COUNT(*) FROM work_units u JOIN work_unit_dependency_activation_intents i ON i.work_unit_id=u.work_unit_id AND i.materialization_id=u.materialization_id AND i.accepted_revision_id=u.accepted_revision_id LEFT JOIN work_unit_handler_activations h ON h.work_unit_id=u.work_unit_id AND h.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 AND s.settlement_id IS NULL AND i.eligibility_state='eligible' AND h.work_unit_id IS NULL",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let active: i64 = tx.query_row(
        "SELECT COUNT(*) FROM work_units u JOIN work_unit_handler_activations h ON h.work_unit_id=u.work_unit_id AND h.materialization_id=u.materialization_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 AND s.settlement_id IS NULL AND h.handler_ready_at IS NOT NULL",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let retry: i64 = tx.query_row(
        "SELECT COUNT(*) FROM work_unit_retry_attempts r JOIN work_units u ON u.work_unit_id=r.work_unit_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 AND s.settlement_id IS NULL AND r.retry_ready_at IS NOT NULL AND r.failure_reason IS NULL",
        params![materialization, revision], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(ready == 0 && active == 0 && retry == 0)
}

fn graph_fingerprint(tx: &rusqlite::Transaction<'_>, materialization: &str, revision: &str) -> Result<String, String> {
    let values = tx.prepare(
        "SELECT u.work_unit_id,i.integration_id,s.settlement_id FROM work_units u JOIN accepted_work_unit_integrations i ON i.work_unit_id=u.work_unit_id JOIN work_unit_settlements s ON s.integration_id=i.integration_id AND s.work_unit_id=u.work_unit_id WHERE u.materialization_id=?1 AND u.accepted_revision_id=?2 ORDER BY u.work_unit_id",
    ).map_err(|e| e.to_string())?.query_map(params![materialization, revision], |r| Ok(format!("{}:{}:{}", r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    let edges = tx.prepare("SELECT c.relationship_id,c.contribution_id FROM work_unit_prerequisite_contributions c JOIN work_unit_relationships r ON r.relationship_id=c.relationship_id WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on' ORDER BY c.relationship_id")
        .map_err(|e| e.to_string())?.query_map([materialization], |r| Ok(format!("{}:{}", r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(materialization.as_bytes()); hasher.update([0]); hasher.update(revision.as_bytes());
    for value in values.into_iter().chain(edges) { hasher.update([0]); hasher.update(value.as_bytes()); }
    Ok(format!("{:x}", hasher.finalize()))
}

fn project_execution_states(tx: &rusqlite::Transaction<'_>, materialization: &str, revision: &str, graph_attention: Option<&str>) -> Result<(), String> {
    let units = tx.prepare("SELECT work_unit_id FROM work_units WHERE materialization_id=?1 AND accepted_revision_id=?2 ORDER BY lane_ordinal,work_unit_id")
        .map_err(|e| e.to_string())?.query_map(params![materialization, revision], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    for unit in units {
        let existing: Option<(String, String)> = tx.query_row(
            "SELECT materialization_id,accepted_revision_id FROM work_unit_execution_states WHERE work_unit_id=?1",
            [&unit],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| e.to_string())?;
        if let Some((stored_materialization, stored_revision)) = existing.as_ref() {
            if stored_materialization != materialization || stored_revision != revision {
                return Err("work_unit_execution_state_correlation_invalid".into());
            }
        }
        let (state, reason) = if let Some(reason) = graph_attention { ("attention", Some(reason.to_owned())) }
        else {
            let settled: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_settlements s JOIN accepted_work_unit_integrations i ON i.integration_id=s.integration_id JOIN work_units u ON u.work_unit_id=s.work_unit_id WHERE s.work_unit_id=?1 AND u.materialization_id=?2 AND u.accepted_revision_id=?3 AND i.work_unit_id=?1 AND i.stage='settled' AND i.settled_at IS NOT NULL)", params![unit,materialization,revision], |r| r.get(0)).map_err(|e| e.to_string())?;
            let handed_back: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_no_progress_handbacks h JOIN work_units u ON u.work_unit_id=h.work_unit_id WHERE h.work_unit_id=?1 AND u.materialization_id=?2 AND u.accepted_revision_id=?3)", params![unit,materialization,revision], |r| r.get(0)).map_err(|e| e.to_string())?;
            let retry: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_retry_attempts r JOIN work_units u ON u.work_unit_id=r.work_unit_id LEFT JOIN work_unit_settlements s ON s.work_unit_id=u.work_unit_id WHERE r.work_unit_id=?1 AND u.materialization_id=?2 AND u.accepted_revision_id=?3 AND s.settlement_id IS NULL AND r.retry_ready_at IS NOT NULL AND r.failure_reason IS NULL)", params![unit,materialization,revision], |r| r.get(0)).map_err(|e| e.to_string())?;
            let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations h LEFT JOIN work_unit_settlements s ON s.work_unit_id=h.work_unit_id WHERE h.work_unit_id=?1 AND h.materialization_id=?2 AND s.settlement_id IS NULL AND h.handler_ready_at IS NOT NULL)", params![unit,materialization], |r| r.get(0)).map_err(|e| e.to_string())?;
            let intent: Option<(String, Option<String>)> = tx.query_row("SELECT eligibility_state,blocked_reason FROM work_unit_dependency_activation_intents WHERE work_unit_id=?1 AND materialization_id=?2 AND accepted_revision_id=?3", params![unit,materialization,revision], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(|e| e.to_string())?;
            if settled { ("settled", None) } else if handed_back { ("handed_back", Some("work_unit_handed_back".into())) } else if retry { ("retry_authorized", None) } else if active { ("active", None) } else if matches!(intent.as_ref(), Some((state, _)) if state == "eligible") { ("ready", None) } else { ("waiting_on_prerequisites", intent.and_then(|(_, reason)| reason)) }
        };
        let at = chrono::Utc::now().to_rfc3339();
        if existing.is_some() {
            tx.execute("UPDATE work_unit_execution_states SET execution_state=?4,reason=?5,recorded_at=?6 WHERE work_unit_id=?1 AND materialization_id=?2 AND accepted_revision_id=?3", params![unit,materialization,revision,state,reason,at]).map_err(|e| e.to_string())?;
        } else {
            tx.execute("INSERT INTO work_unit_execution_states(work_unit_id,materialization_id,accepted_revision_id,execution_state,reason,recorded_at) VALUES(?1,?2,?3,?4,?5,?6)", params![unit,materialization,revision,state,reason,at]).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn eligibility(
    transaction: &rusqlite::Transaction<'_>,
    work_unit_id: &str,
    materialization_id: &str,
    accepted_revision_id: &str,
    sprint_id: &str,
) -> Result<Eligibility, String> {
    let canonical: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM work_units u JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id
           WHERE u.work_unit_id=?1 AND u.materialization_id=?2 AND u.accepted_revision_id=?3
             AND m.accepted_revision_id=?3 AND m.sprint_id=?4 AND m.settled_at IS NOT NULL)",
        params![work_unit_id, materialization_id, accepted_revision_id, sprint_id],
        |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if !canonical {
        return Ok(Eligibility::Blocked(
            "canonical_work_unit_correlation_invalid".into(),
        ));
    }
    let has_authority: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM initiated_sprint_git_authorities WHERE sprint_id=?1)",
            [sprint_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !has_authority {
        return Ok(Eligibility::Blocked(
            "initiated_sprint_git_authority_missing".into(),
        ));
    }

    let edges = transaction
        .prepare(
            "SELECT r.relationship_id,r.to_id
         FROM work_unit_relationships r
         JOIN work_units prerequisite ON prerequisite.work_unit_id=r.to_id
         WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on' AND r.from_id=?2
         ORDER BY r.relationship_id",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![materialization_id, work_unit_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let malformed_edge: bool = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM work_unit_relationships r
           LEFT JOIN work_units dependent ON dependent.work_unit_id=r.from_id
           LEFT JOIN work_units prerequisite ON prerequisite.work_unit_id=r.to_id
           WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on' AND r.from_id=?2
             AND (dependent.materialization_id<>?1 OR prerequisite.materialization_id<>?1
                  OR dependent.accepted_revision_id<>?3 OR prerequisite.accepted_revision_id<>?3))",
            params![materialization_id, work_unit_id, accepted_revision_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if malformed_edge {
        return Ok(Eligibility::Blocked(
            "canonical_dependency_edge_invalid".into(),
        ));
    }

    let foreign_contribution: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM work_unit_prerequisite_contributions c
           LEFT JOIN work_unit_relationships r ON r.relationship_id=c.relationship_id
           LEFT JOIN accepted_work_unit_integrations i ON i.integration_id=c.integration_id
           LEFT JOIN work_unit_settlements s ON s.integration_id=c.integration_id AND s.work_unit_id=c.prerequisite_work_unit_id
           WHERE c.dependent_work_unit_id=?1
             AND (r.relationship_id IS NULL OR r.materialization_id<>?2 OR r.relationship_kind<>'depends_on'
                  OR r.from_id<>?1 OR r.to_id<>c.prerequisite_work_unit_id
                  OR i.work_unit_id<>c.prerequisite_work_unit_id OR i.stage<>'settled' OR i.settled_at IS NULL
                  OR s.settlement_id IS NULL))",
        params![work_unit_id, materialization_id], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if foreign_contribution {
        return Ok(Eligibility::Blocked(
            "prerequisite_contribution_correlation_invalid".into(),
        ));
    }

    let mut missing = Vec::new();
    for (relationship_id, prerequisite_id) in edges {
        let contributions: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM work_unit_prerequisite_contributions c
             JOIN accepted_work_unit_integrations i ON i.integration_id=c.integration_id
             JOIN work_unit_settlements s ON s.integration_id=c.integration_id AND s.work_unit_id=c.prerequisite_work_unit_id
             WHERE c.relationship_id=?1 AND c.prerequisite_work_unit_id=?2 AND c.dependent_work_unit_id=?3
               AND i.work_unit_id=?2 AND i.stage='settled' AND i.settled_at IS NOT NULL",
            params![relationship_id, prerequisite_id, work_unit_id], |row| row.get(0),
        ).map_err(|error| error.to_string())?;
        if contributions > 1 {
            return Ok(Eligibility::Blocked(
                "prerequisite_contribution_correlation_invalid".into(),
            ));
        }
        if contributions == 0 {
            missing.push(relationship_id);
            continue;
        }
        let prerequisite: Option<(String, String)> = transaction.query_row(
            "SELECT materialization_id,accepted_revision_id FROM work_units WHERE work_unit_id=?1",
            [&prerequisite_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|error| error.to_string())?;
        if prerequisite.as_ref()
            != Some(&(
                materialization_id.to_owned(),
                accepted_revision_id.to_owned(),
            ))
        {
            return Ok(Eligibility::Blocked(
                "prerequisite_contribution_correlation_invalid".into(),
            ));
        }
    }
    if !missing.is_empty() {
        return Ok(Eligibility::Blocked(format!(
            "missing_prerequisite_contributions:{}",
            missing.join(",")
        )));
    }
    Ok(Eligibility::Eligible)
}

fn persist(
    transaction: &rusqlite::Transaction<'_>,
    work_unit_id: &str,
    materialization_id: &str,
    accepted_revision_id: &str,
    eligibility: Eligibility,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let (state, reason, intent) = match eligibility {
        Eligibility::Blocked(reason) => ("blocked", Some(reason), None),
        Eligibility::Eligible => ("eligible", None, Some(now.clone())),
    };
    let existing: Option<(String, String, String, String, Option<String>, Option<String>)> = transaction.query_row(
        "SELECT work_unit_id,materialization_id,accepted_revision_id,eligibility_state,blocked_reason,activation_intended_at
         FROM work_unit_dependency_activation_intents WHERE work_unit_id=?1", [work_unit_id],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
    ).optional().map_err(|error| error.to_string())?;
    if let Some((
        stored_id,
        stored_materialization,
        stored_revision,
        stored_state,
        stored_reason,
        stored_intent,
    )) = existing
    {
        if stored_id != work_unit_id
            || stored_materialization != materialization_id
            || stored_revision != accepted_revision_id
        {
            return Err("dependency_activation_intent_correlation_invalid".into());
        }
        if stored_state == "eligible" && (stored_reason.is_some() || stored_intent.is_none()) {
            return Err("dependency_activation_intent_invalid".into());
        }
        if stored_state == "blocked" && stored_reason.is_none() {
            return Err("dependency_activation_intent_invalid".into());
        }
        if state == "eligible" {
            transaction
                .execute(
                    "UPDATE work_unit_dependency_activation_intents
                 SET eligibility_state='eligible',blocked_reason=NULL,eligibility_recorded_at=?2,
                     activation_intended_at=COALESCE(activation_intended_at,?2)
                 WHERE work_unit_id=?1",
                    params![work_unit_id, now],
                )
                .map_err(|error| error.to_string())?;
        } else {
            transaction
                .execute(
                    "UPDATE work_unit_dependency_activation_intents
                 SET eligibility_state='blocked',blocked_reason=?2,eligibility_recorded_at=?3
                 WHERE work_unit_id=?1",
                    params![work_unit_id, reason, now],
                )
                .map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    transaction.execute(
        "INSERT INTO work_unit_dependency_activation_intents
         (work_unit_id,materialization_id,accepted_revision_id,eligibility_state,blocked_reason,eligibility_recorded_at,activation_intended_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![work_unit_id, materialization_id, accepted_revision_id, state, reason, now, intent],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn seed(connection: &Connection) {
        connection.execute_batch("PRAGMA foreign_keys=OFF;
          CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY);
          CREATE TABLE initiated_sprint_git_authorities(authority_id TEXT PRIMARY KEY,sprint_id TEXT);
          CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,accepted_revision_id TEXT,sprint_id TEXT,settled_at TEXT);
          CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT,accepted_revision_id TEXT,lane_ordinal INTEGER);
          CREATE TABLE work_unit_relationships(relationship_id TEXT PRIMARY KEY,materialization_id TEXT,relationship_kind TEXT,from_id TEXT,to_id TEXT);
          CREATE TABLE accepted_work_unit_integrations(integration_id TEXT PRIMARY KEY,work_unit_id TEXT,stage TEXT,settled_at TEXT);
          CREATE TABLE work_unit_settlements(settlement_id TEXT PRIMARY KEY,work_unit_id TEXT,integration_id TEXT);
          CREATE TABLE work_unit_prerequisite_contributions(contribution_id TEXT PRIMARY KEY,prerequisite_work_unit_id TEXT,dependent_work_unit_id TEXT,integration_id TEXT,relationship_id TEXT);
          INSERT INTO work_slice_proposal_revisions VALUES('revision');
          INSERT INTO initiated_sprint_git_authorities VALUES('authority','sprint');
          INSERT INTO work_unit_materializations VALUES('materialization','revision','sprint','t');
          INSERT INTO work_units VALUES('root','materialization','revision',0),('one','materialization','revision',1),('two','materialization','revision',2),('many','materialization','revision',3);
          INSERT INTO work_unit_relationships VALUES('one-root','materialization','depends_on','one','root'),('two-root','materialization','depends_on','two','root'),('many-root','materialization','depends_on','many','root'),('many-one','materialization','depends_on','many','one');
        ").unwrap();
    }

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        seed(&connection);
        connection
    }

    fn contribute(connection: &Connection, prerequisite: &str, dependent: &str, edge: &str) {
        let integration = format!("integration-{prerequisite}");
        connection
            .execute(
                "INSERT OR IGNORE INTO accepted_work_unit_integrations VALUES(?1,?2,'settled','t')",
                params![integration, prerequisite],
            )
            .unwrap();
        connection
            .execute(
                "INSERT OR IGNORE INTO work_unit_settlements VALUES(?1,?2,?3)",
                params![
                    format!("settlement-{prerequisite}"),
                    prerequisite,
                    integration
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_unit_prerequisite_contributions VALUES(?1,?2,?3,?4,?5)",
                params![
                    format!("contribution-{edge}"),
                    prerequisite,
                    dependent,
                    format!("integration-{prerequisite}"),
                    edge
                ],
            )
            .unwrap();
    }

    fn state(connection: &Connection, work_unit: &str) -> (String, Option<String>, Option<String>) {
        connection.query_row("SELECT eligibility_state,blocked_reason,activation_intended_at FROM work_unit_dependency_activation_intents WHERE work_unit_id=?1", [work_unit], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap()
    }

    fn seed_settlement(connection: &Connection) {
        connection.execute_batch("PRAGMA foreign_keys=OFF;
          CREATE TABLE work_slice_proposal_revisions(revision_id TEXT PRIMARY KEY);
          CREATE TABLE work_unit_materializations(materialization_id TEXT PRIMARY KEY,planning_point_id TEXT,accepted_revision_id TEXT,sprint_id TEXT,settled_at TEXT,authorization_recorded_at TEXT);
          CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY,materialization_id TEXT,accepted_revision_id TEXT,lane_ordinal INTEGER);
          CREATE TABLE work_unit_relationships(relationship_id TEXT PRIMARY KEY,materialization_id TEXT,relationship_kind TEXT,from_id TEXT,to_id TEXT);
          CREATE TABLE accepted_work_unit_integrations(integration_id TEXT PRIMARY KEY,work_unit_id TEXT,stage TEXT,settled_at TEXT,attention_code TEXT);
          CREATE TABLE work_unit_settlements(settlement_id TEXT PRIMARY KEY,work_unit_id TEXT,integration_id TEXT);
          CREATE TABLE work_unit_prerequisite_contributions(contribution_id TEXT PRIMARY KEY,prerequisite_work_unit_id TEXT,dependent_work_unit_id TEXT,integration_id TEXT,relationship_id TEXT);
          CREATE TABLE work_unit_no_progress_handbacks(work_unit_id TEXT);
          CREATE TABLE work_unit_retry_attempts(work_unit_id TEXT,retry_ready_at TEXT,failure_reason TEXT);
          CREATE TABLE work_unit_handler_activations(work_unit_id TEXT,materialization_id TEXT,handler_ready_at TEXT);
          INSERT INTO work_slice_proposal_revisions VALUES('revision');
          INSERT INTO work_unit_materializations VALUES('materialization','point','revision','sprint','t','t');
          INSERT INTO work_units VALUES('root-a','materialization','revision',0),('root-b','materialization','revision',1),('middle-a','materialization','revision',2),('middle-b','materialization','revision',3),('leaf','materialization','revision',4);
          INSERT INTO work_unit_relationships VALUES
            ('middle-a-root-a','materialization','depends_on','middle-a','root-a'),
            ('middle-b-root-b','materialization','depends_on','middle-b','root-b'),
            ('leaf-middle-a','materialization','depends_on','leaf','middle-a'),
            ('leaf-middle-b','materialization','depends_on','leaf','middle-b');
        ").unwrap();
    }

    fn settlement_fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        seed_settlement(&connection);
        connection
    }

    fn settle(connection: &Connection, unit: &str) {
        let integration = format!("integration-{unit}");
        connection.execute("INSERT INTO accepted_work_unit_integrations VALUES(?1,?2,'settled','t',NULL)", params![integration, unit]).unwrap();
        connection.execute("INSERT INTO work_unit_settlements VALUES(?1,?2,?3)", params![format!("settlement-{unit}"), unit, integration]).unwrap();
        let edges = connection.prepare("SELECT relationship_id,from_id FROM work_unit_relationships WHERE relationship_kind='depends_on' AND to_id=?1 ORDER BY relationship_id").unwrap().query_map([unit], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        for (edge, dependent) in edges {
            connection.execute("INSERT INTO work_unit_prerequisite_contributions VALUES(?1,?2,?3,?4,?5)", params![format!("contribution-{edge}"), unit, dependent, format!("integration-{unit}"), edge]).unwrap();
        }
    }

    fn intent(connection: &Connection, unit: &str, state: &str, reason: Option<&str>) {
        connection.execute_batch(WORK_UNIT_DEPENDENCY_WAVE_SCHEMA).unwrap();
        connection.execute(
            "INSERT OR REPLACE INTO work_unit_dependency_activation_intents(work_unit_id,materialization_id,accepted_revision_id,eligibility_state,blocked_reason,eligibility_recorded_at,activation_intended_at) VALUES(?1,'materialization','revision',?2,?3,'t',CASE WHEN ?2='eligible' THEN 't' END)",
            params![unit, state, reason],
        ).unwrap();
    }

    #[test]
    fn zero_one_and_multiple_prerequisites_require_exact_settled_contributions() {
        let mut connection = fixture();
        reconcile_work_unit_dependency_wave(&mut connection).unwrap();
        assert_eq!(state(&connection, "root").0, "eligible");
        assert_eq!(
            state(&connection, "one").1,
            Some("missing_prerequisite_contributions:one-root".into())
        );
        contribute(&connection, "root", "one", "one-root");
        contribute(&connection, "root", "two", "two-root");
        contribute(&connection, "root", "many", "many-root");
        reconcile_work_unit_dependency_wave(&mut connection).unwrap();
        assert_eq!(state(&connection, "one").0, "eligible");
        assert_eq!(state(&connection, "two").0, "eligible");
        assert_eq!(
            state(&connection, "many").1,
            Some("missing_prerequisite_contributions:many-one".into())
        );
        contribute(&connection, "one", "many", "many-one");
        reconcile_work_unit_dependency_wave(&mut connection).unwrap();
        assert_eq!(state(&connection, "many").0, "eligible");
    }

    #[test]
    fn replay_reopen_and_concurrent_callers_preserve_one_activation_intent() {
        let database = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(database.path()).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        seed(&connection);
        contribute(&connection, "root", "one", "one-root");
        drop(connection);
        let barrier = Arc::new(Barrier::new(3));
        let callers = (0..2)
            .map(|_| {
                let path = database.path().to_path_buf();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut connection = Connection::open(path).unwrap();
                    connection
                        .busy_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    barrier.wait();
                    reconcile_work_unit_dependency_wave(&mut connection)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        for caller in callers {
            caller.join().unwrap().unwrap();
        }
        let mut reopened = Connection::open(database.path()).unwrap();
        reconcile_work_unit_dependency_wave(&mut reopened).unwrap();
        let intent = state(&reopened, "one").2.unwrap();
        assert_eq!(reopened.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_dependency_activation_intents WHERE work_unit_id='one'", [], |row| row.get(0)).unwrap(), 1);
        reconcile_work_unit_dependency_wave(&mut reopened).unwrap();
        assert_eq!(state(&reopened, "one").2.as_deref(), Some(intent.as_str()));
    }

    #[test]
    fn malformed_foreign_or_duplicate_contributions_fail_closed() {
        let mut connection = fixture();
        contribute(&connection, "root", "one", "one-root");
        reconcile_work_unit_dependency_wave(&mut connection).unwrap();
        let intent = state(&connection, "one").2;
        connection.execute("INSERT INTO work_unit_prerequisite_contributions VALUES('foreign','root','one','integration-root','many-root')", []).unwrap();
        reconcile_work_unit_dependency_wave(&mut connection).unwrap();
        assert_eq!(
            state(&connection, "one"),
            (
                "blocked".into(),
                Some("prerequisite_contribution_correlation_invalid".into()),
                intent
            )
        );
    }

    #[test]
    fn multi_root_multi_level_graph_settles_each_terminal_fact_exactly_once() {
        let mut connection = settlement_fixture();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_graph_completions", [], |r| r.get(0)).unwrap(), 0);
        for unit in ["root-a", "root-b", "middle-a", "middle-b", "leaf"] { settle(&connection, unit); }
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        for table in ["work_slice_execution_graph_completions", "work_slice_execution_settlements", "work_slice_planning_point_execution_settlements"] {
            assert_eq!(connection.query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap(), 1, "{table}");
        }
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_execution_states WHERE execution_state='settled'", [], |r| r.get(0)).unwrap(), 5);
    }

    #[test]
    fn cyclic_graph_records_attention_without_execution_settlement() {
        let mut connection = settlement_fixture();
        connection.execute("INSERT INTO work_unit_relationships VALUES('root-a-leaf','materialization','depends_on','root-a','leaf')", []).unwrap();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        assert_eq!(connection.query_row::<String, _, _>("SELECT code FROM work_slice_execution_attentions", [], |r| r.get(0)).unwrap(), "canonical_dependency_cycle");
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_settlements", [], |r| r.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_execution_states WHERE execution_state='attention'", [], |r| r.get(0)).unwrap(), 5);
    }

    #[test]
    fn concurrent_reopen_replays_one_execution_settlement() {
        let database = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(database.path()).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        seed_settlement(&connection);
        for unit in ["root-a", "root-b", "middle-a", "middle-b", "leaf"] { settle(&connection, unit); }
        drop(connection);
        let barrier = Arc::new(Barrier::new(3));
        let callers = (0..2).map(|_| {
            let path = database.path().to_path_buf();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut connection = Connection::open(path).unwrap();
                connection.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                barrier.wait();
                reconcile_work_slice_execution_settlement(&mut connection)
            })
        }).collect::<Vec<_>>();
        barrier.wait();
        for caller in callers { caller.join().unwrap().unwrap(); }
        let mut reopened = Connection::open(database.path()).unwrap();
        reconcile_work_slice_execution_settlement(&mut reopened).unwrap();
        for table in ["work_slice_execution_graph_completions", "work_slice_execution_settlements", "work_slice_planning_point_execution_settlements"] {
            assert_eq!(reopened.query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap(), 1, "{table}");
        }
    }

    #[test]
    fn unresolved_attention_survives_replay_and_prevents_every_terminal_fact() {
        let database = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(database.path()).unwrap();
        seed_settlement(&connection);
        connection.execute_batch(WORK_UNIT_DEPENDENCY_WAVE_SCHEMA).unwrap();
        connection.execute("INSERT INTO work_slice_execution_attentions VALUES('materialization','test_attention','t')", []).unwrap();
        for unit in ["root-a", "root-b", "middle-a", "middle-b", "leaf"] { settle(&connection, unit); }
        drop(connection);
        let mut reopened = Connection::open(database.path()).unwrap();
        reconcile_work_slice_execution_settlement(&mut reopened).unwrap();
        reconcile_work_slice_execution_settlement(&mut reopened).unwrap();
        for table in ["work_slice_execution_graph_completions", "work_slice_execution_settlements", "work_slice_planning_point_execution_settlements"] {
            assert_eq!(reopened.query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap(), 0, "{table}");
        }
        assert_eq!(reopened.query_row::<String, _, _>("SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id='root-a'", [], |r| r.get(0)).unwrap(), "attention");
    }

    #[test]
    fn current_unresolved_facts_distinguish_ready_active_waiting_and_stall() {
        let mut connection = settlement_fixture();
        intent(&connection, "root-a", "eligible", None);
        intent(&connection, "root-b", "eligible", None);
        intent(&connection, "middle-a", "blocked", Some("missing_prerequisite_contributions:middle-a-root-a"));
        intent(&connection, "middle-b", "blocked", Some("missing_prerequisite_contributions:middle-b-root-b"));
        intent(&connection, "leaf", "blocked", Some("missing_prerequisite_contributions:leaf-middle-a"));
        connection.execute("INSERT INTO work_unit_handler_activations VALUES('root-b','materialization','t')", []).unwrap();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        let states = connection.prepare("SELECT work_unit_id,execution_state FROM work_unit_execution_states ORDER BY work_unit_id").unwrap().query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert!(states.contains(&(String::from("root-a"), String::from("ready"))));
        assert!(states.contains(&(String::from("root-b"), String::from("active"))));
        assert!(states.contains(&(String::from("middle-a"), String::from("waiting_on_prerequisites"))));
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_attentions", [], |r| r.get(0)).unwrap(), 0);
        connection.execute_batch("DELETE FROM work_unit_handler_activations; UPDATE work_unit_dependency_activation_intents SET eligibility_state='blocked',blocked_reason='activation_terminal_without_current_progress',activation_intended_at=NULL").unwrap();
        let tx = connection.transaction().unwrap();
        assert!(stalled_generation(&tx, "materialization", "revision").unwrap());
        tx.rollback().unwrap();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        assert_eq!(connection.query_row::<String, _, _>("SELECT code FROM work_slice_execution_attentions", [], |r| r.get(0)).unwrap(), "stalled_execution_generation");
    }

    #[test]
    fn foreign_or_missing_contribution_endpoint_and_state_correlation_fail_closed() {
        let mut connection = settlement_fixture();
        connection.execute("INSERT INTO work_unit_prerequisite_contributions VALUES('bad','ghost','middle-a','integration-ghost','middle-a-root-a')", []).unwrap();
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        assert_eq!(connection.query_row::<String, _, _>("SELECT code FROM work_slice_execution_attentions", [], |r| r.get(0)).unwrap(), "prerequisite_contribution_correlation_invalid");
        let mut foreign = settlement_fixture();
        foreign.execute("INSERT INTO work_units VALUES('foreign-unit','foreign-materialization','foreign-revision',0)", []).unwrap();
        foreign.execute("INSERT INTO work_unit_prerequisite_contributions VALUES('foreign','foreign-unit','middle-a','integration-foreign','middle-a-root-a')", []).unwrap();
        reconcile_work_slice_execution_settlement(&mut foreign).unwrap();
        assert_eq!(foreign.query_row::<String, _, _>("SELECT code FROM work_slice_execution_attentions", [], |r| r.get(0)).unwrap(), "prerequisite_contribution_correlation_invalid");
        let mut correlation = settlement_fixture();
        correlation.execute_batch(WORK_UNIT_DEPENDENCY_WAVE_SCHEMA).unwrap();
        correlation.execute("INSERT INTO work_unit_execution_states VALUES('root-a','foreign-materialization','revision','ready',NULL,'t')", []).unwrap();
        assert_eq!(reconcile_work_slice_execution_settlement(&mut correlation).unwrap_err(), "work_unit_execution_state_correlation_invalid");
    }

    #[test]
    fn execution_states_prefer_current_settlement_retry_and_handback_facts() {
        let mut connection = settlement_fixture();
        intent(&connection, "root-a", "eligible", None);
        intent(&connection, "root-b", "eligible", None);
        intent(&connection, "middle-a", "blocked", Some("missing_prerequisite_contributions:middle-a-root-a"));
        intent(&connection, "middle-b", "blocked", Some("missing_prerequisite_contributions:middle-b-root-b"));
        intent(&connection, "leaf", "blocked", Some("missing_prerequisite_contributions:leaf-middle-a"));
        connection.execute("INSERT INTO work_unit_retry_attempts VALUES('middle-a','t',NULL)", []).unwrap();
        connection.execute("INSERT INTO work_unit_no_progress_handbacks VALUES('middle-b')", []).unwrap();
        settle(&connection, "leaf");
        reconcile_work_slice_execution_settlement(&mut connection).unwrap();
        for (unit, expected) in [("root-a", "ready"), ("middle-a", "retry_authorized"), ("middle-b", "handed_back"), ("leaf", "settled")] {
            assert_eq!(connection.query_row::<String, _, _>("SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1", [unit], |r| r.get(0)).unwrap(), expected, "{unit}");
        }
    }
}
