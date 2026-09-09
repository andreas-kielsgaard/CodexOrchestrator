use super::*;
use crate::harness_engine::{domain::binding_digest, exposure::HarnessExposurePolicy};
use rusqlite::params;

#[test]
fn main_v47_sessions_upgrade_without_moving_context_or_losing_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("main-v47.sqlite");
    let connection = open_active_database(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE execution_default_capability_profile;
         ALTER TABLE agent_sessions DROP COLUMN workspace_origin;
         PRAGMA user_version=47;",
        )
        .unwrap();
    for (id, cwd) in [
        ("explicit", Some("C:\\existing-work")),
        ("historical", None),
    ] {
        connection.execute(
            "INSERT INTO agent_sessions(id,title,availability,external_context_id,working_directory,requested_options_json,created_at,updated_at) VALUES(?1,?1,'available',?2,?3,'{}','before','before')",
            params![id, format!("thread-{id}"), cwd],
        ).unwrap();
    }
    let plan = serde_json::json!({
        "contractVersion":"harness-mediation-plan/v1",
        "exposures":[{
            "configuredServerName":"workflow_handoff", "proxyServerName":"session_capability_1",
            "upstream":{"name":"workflow_handoff","url":"http://obsolete/mcp","bearerToken":"old-secret","workflowToolName":"handoff_to_agent","workflowPrepareUrl":"http://obsolete/prepare"},
            "access":{"kind":"selected_tools","toolNames":["handoff_to_agent"]}
        }]
    }).to_string();
    // Use the native serializer for the policy enum while retaining the predecessor transport.
    let mut plan: serde_json::Value = serde_json::from_str(&plan).unwrap();
    plan["exposures"][0]["access"] = serde_json::to_value(
        crate::harness_engine::domain::HarnessToolAccess::SelectedTools {
            tool_names: vec!["handoff_to_agent".into()],
        },
    )
    .unwrap();
    let plan = plan.to_string();
    connection.execute(
        "INSERT INTO session_harness_bindings(id,contract_version,session_id,runtime_instance_id,session_instance_token,stage,harness_snapshot,mediation_plan,configuration_digest,harness_token,source_workflow_instance_id,source_recipe_id,source_node_id,prepared_at,bound_at) VALUES('binding','harness-binding/v1','explicit','runtime','instance-token','bound','{}',?1,?2,'stable-token','workflow','recipe','node','before','before')",
        params![plan, binding_digest("{}", &plan)],
    ).unwrap();
    let before = session_rows(&connection);
    drop(connection);

    // Production composition must perform this migration, not only a test repository opener.
    let database = crate::product_database::open(&path).unwrap();
    database.read("verify Session migration", |connection| -> Result<(), String> {
        assert_eq!(session_rows(connection), before);
        assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), 48);
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM execution_default_capability_profile", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM agent_sessions WHERE workspace_origin IS NOT NULL", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        let (policy, token): (String, String) = connection.query_row("SELECT mediation_plan,harness_token FROM session_harness_bindings", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        let policy: HarnessExposurePolicy = serde_json::from_str(&policy).unwrap();
        assert_eq!(policy.exposures[0].configured_server_name, "workflow_handoff");
        assert_eq!(token, "stable-token");
        Ok(())
    }).unwrap();
    drop(database);
    let reopened = crate::product_database::open(&path).unwrap();
    reopened
        .read(
            "verify Session migration reopen",
            |connection| -> Result<(), String> {
                assert_eq!(session_rows(connection), before);
                Ok(())
            },
        )
        .unwrap();
}

fn session_rows(connection: &Connection) -> Vec<(String, Option<String>, String, String)> {
    connection.prepare("SELECT id,working_directory,external_context_id,updated_at FROM agent_sessions ORDER BY id").unwrap()
        .query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap()
        .collect::<Result<_, _>>().unwrap()
}
