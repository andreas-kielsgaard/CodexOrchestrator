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
    for (work_unit_id, materialization_id, accepted_revision_id, sprint_id) in units {
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
}
