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
        assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), super::ACTIVE_SCHEMA_VERSION);
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM agent_sessions WHERE execution_target_json IS NOT NULL", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
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

#[test]
fn main_and_remote_prototype_schemas_upgrade_without_losing_session_state() {
    use crate::{
        agent_sessions::{
            domain::AgentSessionId, ports::AgentSessionRepository,
            repository::SqliteAgentSessionRepository,
        },
        execution_targets::domain::{
            ExecutionBinding, ExecutionConnection, SessionExecutionTarget,
        },
        repository_catalog::device_locations::RepositoryDeviceLocations,
        session_navigation::{
            order::NavigationOrderScope, order_repository::NavigationOrderRepository,
        },
    };

    let timestamp = "2026-09-14T10:00:00Z";
    let session_id = AgentSessionId::new("session").unwrap();
    let target = SessionExecutionTarget {
        capability_profile_id: "remote-profile".into(),
        capability_profile_revision: 1,
        execution: ExecutionBinding {
            device_id: "remote".into(),
            device_name: "Remote server".into(),
            provider: "codex".into(),
            configuration_ref: "codex-default".into(),
            connection: ExecutionConnection::Ssh {
                target: "orchid-remote".into(),
                host_executable: "/root/.local/bin/orchid-host".into(),
            },
        },
        repository_id: "repository".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "remote-worktree".into(),
        path: "/remote/worktree".into(),
        head: Some("a".repeat(40)),
    };
    for source_version in [49, 50] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pre-rebase.sqlite");
        let database = crate::product_database::open(&path).unwrap();
        database.write("seed predecessor Session state", |connection| -> Result<(), String> {
            let target_json = (source_version == 49).then(|| serde_json::to_string(&target).unwrap());
            let cwd = if source_version == 49 { target.path.as_str() } else { "C:/repository/worktree" };
            connection.execute(
                "INSERT INTO agent_sessions(id,title,availability,external_context_id,working_directory,requested_options_json,execution_target_json,created_at,updated_at) VALUES('session','Existing session','available','provider-thread',?1,'{}',?2,?3,?3)",
                params![cwd, target_json, timestamp],
            ).unwrap();
            connection.execute(
                "INSERT INTO agent_session_invocations(id,session_id,submitted_text,input_provenance,status,requested_options_json,started_at,completed_at,created_at,updated_at) VALUES('invocation','session','Retained prompt','user','completed','{}',?1,?1,?1,?1)",
                [timestamp],
            ).unwrap();
            connection.execute(
                "INSERT INTO agent_session_runtime_events(id,invocation_id,sequence,source,raw_payload_json,recorded_at) VALUES('event','invocation',0,'stdout','{\"text\":\"Retained output\"}',?1)",
                [timestamp],
            ).unwrap();
            connection.execute(
                "INSERT INTO registered_repositories(repository_id,label,anchor_root,git_common_directory,first_registered_at,last_verified_at) VALUES('repository','Repository','C:/repository','C:/repository/.git',?1,?1)",
                [timestamp],
            ).unwrap();
            if source_version == 49 {
                connection.execute("INSERT INTO repository_device_locations(repository_id,device_id,repository_root) VALUES('repository','remote','/remote/repository')", []).unwrap();
            } else {
                connection.execute("INSERT INTO agent_session_organization(session_id,placement_kind,pinned_at) VALUES('session','unfiled',?1)", [timestamp]).unwrap();
                connection.execute("INSERT INTO session_navigation_order(scope,ordered_ids) VALUES(?1,'[\"session\"]')", [serde_json::to_string(&NavigationOrderScope::Pinned).unwrap()]).unwrap();
            }
            Ok(())
        }).unwrap();
        let repository = SqliteAgentSessionRepository::from_database(database.clone());
        let before = repository
            .load_session_history(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(before.invocations.len(), 1);
        assert_eq!(before.invocations[0].events.len(), 1);
        let organization = repository.list_organization().unwrap();
        let order = NavigationOrderRepository::new(database.clone())
            .load()
            .unwrap();
        let locations = |database| {
            RepositoryDeviceLocations(database)
                .list()
                .unwrap()
                .into_iter()
                .map(|location| {
                    (
                        location.repository_id,
                        location.device_id,
                        location.repository_root,
                    )
                })
                .collect::<Vec<_>>()
        };
        let device_locations = locations(database.clone());
        drop(repository);
        database.write("restore predecessor schema", |connection| -> Result<(), String> {
            connection.execute_batch(if source_version == 49 {
                // The remote prototype predates main's Session navigation tables.
                "DROP TABLE agent_session_organization; DROP TABLE session_navigation_order; PRAGMA user_version=49;"
            } else {
                // Main has navigation but no remote execution target storage.
                "ALTER TABLE agent_sessions DROP COLUMN execution_target_json; DROP TABLE repository_device_locations; PRAGMA user_version=50;"
            }).unwrap();
            Ok(())
        }).unwrap();
        drop(database);

        // Exercise production migration and an additional open without changing Session state.
        for _ in 0..2 {
            let database = crate::product_database::open(&path).unwrap();
            let repository = SqliteAgentSessionRepository::from_database(database.clone());
            assert_eq!(
                repository
                    .load_session_history(&session_id)
                    .unwrap()
                    .unwrap(),
                before
            );
            assert_eq!(repository.list_organization().unwrap(), organization);
            assert_eq!(
                NavigationOrderRepository::new(database.clone())
                    .load()
                    .unwrap(),
                order
            );
            assert_eq!(locations(database.clone()), device_locations);
            database
                .read(
                    "verify migrated schema",
                    |connection| -> Result<(), String> {
                        assert_eq!(
                            connection
                                .pragma_query_value(None, "user_version", |row| row
                                    .get::<_, i64>(0))
                                .unwrap(),
                            ACTIVE_SCHEMA_VERSION
                        );
                        Ok(())
                    },
                )
                .unwrap();
        }
    }
}
