//! Remove obsolete upstream connection material without changing recorded tool selections.

use super::{
    domain::{binding_digest, HarnessMediationPlan, MEDIATION_PLAN_VERSION},
    exposure::HarnessExposurePolicy,
};
use rusqlite::params;

pub(crate) fn migrate_bindings(connection: &rusqlite::Transaction<'_>) -> Result<(), String> {
    let mut statement = connection.prepare(
        "SELECT id,harness_snapshot,mediation_plan,configuration_digest FROM session_harness_bindings WHERE json_extract(mediation_plan,'$.contractVersion')=?1"
    ).map_err(|e| e.to_string())?;
    let records = statement
        .query_map([MEDIATION_PLAN_VERSION], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(statement);
    if records.is_empty() {
        return Ok(());
    }
    for (id, snapshot, old_plan, digest) in records {
        if binding_digest(&snapshot, &old_plan) != digest {
            return Err(format!(
                "Harness binding {id} failed digest verification during migration"
            ));
        }
        let resolved: HarnessMediationPlan =
            serde_json::from_str(&old_plan).map_err(|e| e.to_string())?;
        let policy = serde_json::to_string(&HarnessExposurePolicy::from_resolved(&resolved))
            .map_err(|e| e.to_string())?;
        connection.execute("UPDATE session_harness_bindings SET mediation_plan=?2,configuration_digest=?3 WHERE id=?1 AND mediation_plan=?4 AND configuration_digest=?5",
            params![id, policy, binding_digest(&snapshot, &policy), old_plan, digest]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    fn migrate_bindings(connection: &Connection) -> Result<(), String> {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        super::migrate_bindings(&transaction)?;
        transaction.commit().map_err(|e| e.to_string())
    }
    use crate::harness_engine::domain::{
        HarnessMcpExposurePlan, HarnessToolAccess, ManagedMcpUpstreamDescriptor,
    };

    #[test]
    fn migration_preserves_policy_and_is_atomic_when_an_old_digest_is_invalid() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE session_harness_bindings(id TEXT PRIMARY KEY,harness_snapshot TEXT,mediation_plan TEXT,configuration_digest TEXT)").unwrap();
        let plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.into(),
            exposures: vec![HarnessMcpExposurePlan {
                configured_server_name: "workflow".into(),
                proxy_server_name: "session_capability_1".into(),
                upstream: ManagedMcpUpstreamDescriptor {
                    name: "workflow".into(),
                    url: "http://obsolete".into(),
                    bearer_token: "secret".into(),
                    workflow_tool_name: Some("handoff_to_agent".into()),
                    workflow_prepare_url: Some("http://obsolete/prepare".into()),
                    caller_context: false,
                },
                access: HarnessToolAccess::SelectedTools {
                    tool_names: vec!["continuation".into()],
                },
            }],
        })
        .unwrap();
        for (id, digest) in [("a", binding_digest("{}", &plan)), ("b", "invalid".into())] {
            connection
                .execute(
                    "INSERT INTO session_harness_bindings VALUES (?1,'{}',?2,?3)",
                    params![id, plan, digest],
                )
                .unwrap();
        }
        assert!(migrate_bindings(&connection).is_err());
        let read = || {
            connection
                .query_row(
                    "SELECT mediation_plan FROM session_harness_bindings WHERE id='a'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap()
        };
        assert_eq!(read(), plan);
        connection
            .execute("DELETE FROM session_harness_bindings WHERE id='b'", [])
            .unwrap();
        migrate_bindings(&connection).unwrap();
        let migrated = read();
        assert!(!migrated.contains("secret") && !migrated.contains("obsolete"));
        let policy: HarnessExposurePolicy = serde_json::from_str(&migrated).unwrap();
        assert_eq!(
            policy.exposures[0].access,
            HarnessToolAccess::SelectedTools {
                tool_names: vec!["continuation".into()]
            }
        );
        migrate_bindings(&connection).unwrap();
        assert_eq!(read(), migrated);
    }
}
